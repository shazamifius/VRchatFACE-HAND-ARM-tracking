use crate::tracking::types::TrackingData;
use crate::tracking::filter::OneEuroFilter;
use crate::tracking::smoothing::InertiaFilter;
use crate::tracking::ik::ArmIK;
use crate::tracking::calibration::{CalibrationManager, UserProfile};
use std::collections::HashMap;
use nalgebra::{Vector3, Vector2, UnitQuaternion};

pub struct Solver {
    // State
    last_rotation: Option<UnitQuaternion<f32>>,
    last_translation: Option<Vector3<f32>>,
    last_sent_rotation: Option<UnitQuaternion<f32>>, // For Dead Zone
    last_sent_translation: Option<Vector3<f32>>,     // For Dead Zone
    
    // IK
    arm_ik: ArmIK,

    // Calibration
    pub calibration: CalibrationManager, // Public to access start()
    pub user_profile: UserProfile,
    
    // Eye Animation State
    saccade_target: (f32, f32), // Pitch, Yaw offset
    saccade_timer: std::time::Instant,
    saccade_duration: std::time::Duration,
    
    // Auto-Blink System
    auto_blink_timer: std::time::Instant,
    auto_blink_next_interval: std::time::Duration, // 3-7s
    auto_blink_active: bool,
    auto_blink_end: std::time::Instant,
    
    // Deterministic PRNG (replaces rand)
    prng_state: u64,
    
    // Micro-Expressions (ambient face life)
    micro_brow_timer: std::time::Instant,
    micro_brow_interval: std::time::Duration,
    micro_brow_target: f32,
    micro_cheek_timer: std::time::Instant,
    micro_cheek_interval: std::time::Duration,
    micro_cheek_target: f32,
    micro_mouth_timer: std::time::Instant,
    micro_mouth_interval: std::time::Duration,
    micro_mouth_target: f32,
    
    // Fallback State (graceful degradation)
    face_lost_at: Option<std::time::Instant>,
    last_face_params: Vec<(String, f32)>,
    left_hand_lost_at: Option<std::time::Instant>,
    right_hand_lost_at: Option<std::time::Instant>,
    last_left_hand_params: Vec<(String, f32)>,
    last_right_hand_params: Vec<(String, f32)>,

    // Structured Filters
    head_filters: HeadFilters,
    eyes_x: OneEuroFilter,
    eyes_y: OneEuroFilter,
    
    // Inertia filters (Blinks, Jaw)
    blink_l: InertiaFilter,
    blink_r: InertiaFilter,
    jaw: InertiaFilter,

    // Hands
    left_hand: HandFilters,
    right_hand: HandFilters,
    
    filter_params: (f32, f32), // (min_cutoff, beta)
}

pub struct HeadFilters {
    pitch: OneEuroFilter,
    yaw: OneEuroFilter,
    roll: OneEuroFilter,
    x: OneEuroFilter,
    y: OneEuroFilter,
    z: OneEuroFilter,
}

pub struct HandFilters {
    // We filter Position (x,y,z) and Elbow (x,y,z)
    // We also handle dead zones here
    pos_x: OneEuroFilter,
    pos_y: OneEuroFilter,
    pos_z: OneEuroFilter,
    elbow_x: OneEuroFilter,
    elbow_y: OneEuroFilter,
    elbow_z: OneEuroFilter,
    
    dead_zone_pos: Option<Vector3<f32>>,
    dead_zone_elbow: Option<Vector3<f32>>,
}

impl HandFilters {
    pub fn new(mc: f32, beta: f32) -> Self {
        Self {
            pos_x: OneEuroFilter::new(mc, beta),
            pos_y: OneEuroFilter::new(mc, beta),
            pos_z: OneEuroFilter::new(mc, beta),
            elbow_x: OneEuroFilter::new(mc, beta),
            elbow_y: OneEuroFilter::new(mc, beta),
            elbow_z: OneEuroFilter::new(mc, beta),
            dead_zone_pos: None,
            dead_zone_elbow: None,
        }
    }
    
    pub fn reset_params(&mut self, mc: f32, beta: f32) {
        // Re-create or update? Simplified: Re-create to clear history too, matching old behavior
        *self = Self::new(mc, beta);
    }
}

#[derive(Debug, Clone)]
pub struct TrackerData {
    pub id: i32, // 0=Head, 1=LeftHand, 2=RightHand
    pub position: [f32; 3],
    pub rotation: [f32; 4], // x, y, z, w
}

#[derive(Debug, Clone)]
pub struct SolverOutput {
    pub params: Vec<(String, f32)>,
    pub trackers: Vec<TrackerData>,
}

impl Default for Solver {
    fn default() -> Self { Self::new() }
}

impl Solver {
    pub fn new() -> Self {
        let (mc, beta) = (1.5, 0.01); // Default

        Self { 
            // State
            last_rotation: None,
            last_translation: None, 
            last_sent_rotation: None,
            last_sent_translation: None,
            
            // Logic
            arm_ik: ArmIK::new(30.0, 25.0), 
            calibration: CalibrationManager::new(),
            user_profile: UserProfile::default(),

            // Filters (New)
            head_filters: HeadFilters {
                pitch: OneEuroFilter::new(mc, beta),
                yaw: OneEuroFilter::new(mc, beta),
                roll: OneEuroFilter::new(mc, beta),
                x: OneEuroFilter::new(mc, beta),
                y: OneEuroFilter::new(mc, beta),
                z: OneEuroFilter::new(mc, beta),
            },
            eyes_x: OneEuroFilter::new(mc, beta),
            eyes_y: OneEuroFilter::new(mc, beta),
            
            // Inertia (Attack, Decay) - tuned values from old code:
            // Blink: 1.0, 0.2
            blink_l: InertiaFilter::new(1.0, 0.2),
            blink_r: InertiaFilter::new(1.0, 0.2),
            // Jaw: 0.5, 0.05
            jaw: InertiaFilter::new(0.5, 0.05),

            left_hand: HandFilters::new(mc, beta),
            right_hand: HandFilters::new(mc, beta),

            // Eye Animation
            saccade_target: (0.0, 0.0),
            saccade_timer: std::time::Instant::now(),
            saccade_duration: std::time::Duration::from_millis(100),
            
            // Auto-Blink
            auto_blink_timer: std::time::Instant::now(),
            auto_blink_next_interval: std::time::Duration::from_millis(4000),
            auto_blink_active: false,
            auto_blink_end: std::time::Instant::now(),
            prng_state: 0xDEAD_BEEF_CAFE_u64,
            
            // Micro-Expressions
            micro_brow_timer: std::time::Instant::now(),
            micro_brow_interval: std::time::Duration::from_millis(5000),
            micro_brow_target: 0.0,
            micro_cheek_timer: std::time::Instant::now(),
            micro_cheek_interval: std::time::Duration::from_millis(8000),
            micro_cheek_target: 0.0,
            micro_mouth_timer: std::time::Instant::now(),
            micro_mouth_interval: std::time::Duration::from_millis(6000),
            micro_mouth_target: 0.0,
            
            // Fallback
            face_lost_at: None,
            last_face_params: Vec::new(),
            left_hand_lost_at: None,
            right_hand_lost_at: None,
            last_left_hand_params: Vec::new(),
            last_right_hand_params: Vec::new(),
            
            filter_params: (mc, beta),
        }
    }
    

    fn apply_dead_zone_vec3(new_val: Vector3<f32>, last_val: &mut Option<Vector3<f32>>, threshold: f32) -> Vector3<f32> {
        if let Some(last) = last_val {
            if (new_val - *last).norm() < threshold {
                return *last;
            }
        }
        *last_val = Some(new_val);
        new_val
    }

    fn apply_dead_zone_quat(new_val: UnitQuaternion<f32>, last_val: &mut Option<UnitQuaternion<f32>>, threshold_deg: f32) -> UnitQuaternion<f32> {
        if let Some(last) = last_val {
            let angle = new_val.angle_to(last);
            if angle.to_degrees() < threshold_deg {
                return *last;
            }
        }
        *last_val = Some(new_val);
        new_val
    }

    /// Deterministic PRNG: LCG next value, returns 0.0..1.0
    pub fn prng_next(&mut self) -> f32 {
        self.prng_state = self.prng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.prng_state >> 33) as u32 as f32) / (u32::MAX as f32)
    }
    
    /// PRNG in range [lo, hi)
    pub fn prng_range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.prng_next() * (hi - lo)
    }

    fn update_saccades(&mut self) -> (f32, f32) {
        // Disabled "Alive Feel" Saccades during real tracking tracking
        (0.0, 0.0)
    }
    
    /// Auto-blink: fires every 3-7s if no real blink detected, ~150ms duration
    pub fn update_auto_blink(&mut self, real_blink_detected: bool) -> f32 {
        // Disabled "Alive feel" forcing auto-blinks
        0.0
    }

    /// Micro-expressions: subtle ambient face movements to prevent frozen look
    /// Returns (brow_inner_up, cheek_raise, mouth_corner)
    pub fn update_micro_expressions(&mut self) -> (f32, f32, f32) {
        // Disabled "Alive Feel" Micro-Expressions
        (0.0, 0.0, 0.0)
    }

    /// Generate fallback params when face tracking is lost.
    /// Returns cached params with decay toward neutral over 500ms.
    fn generate_fallback_face(&mut self) -> Vec<(String, f32)> {
        let lost_at = self.face_lost_at.get_or_insert_with(std::time::Instant::now);
        let elapsed_ms = lost_at.elapsed().as_millis() as f32;
        
        // Decay factor: 1.0 at t=0, 0.0 at t=500ms
        let decay = (1.0 - elapsed_ms / 500.0).max(0.0);
        
        if decay <= 0.0 {
            // Fully decayed → neutral (zeros)
            return vec![
                ("EyesX".to_string(), 0.0),
                ("EyesY".to_string(), 0.0),
                ("EyeBlinkLeft".to_string(), 0.0),
                ("EyeBlinkRight".to_string(), 0.0),
                ("JawOpen".to_string(), 0.0),
                ("HeadPitch".to_string(), 0.0),
                ("HeadYaw".to_string(), 0.0),
                ("HeadRoll".to_string(), 0.0),
            ];
        }
        
        // Scale last known params toward zero
        self.last_face_params.iter()
            .map(|(k, v)| (k.clone(), v * decay))
            .collect()
    }

    pub fn set_quality(&mut self, quality: &str) {
        let (mc, beta) = match quality {
            "High" => (3.0, 0.02),
            "Medium" => (1.0, 0.005),
            "Low" => (0.5, 0.001), 
            _ => (1.0, 0.005),
        };
        
        // Reset filters
        self.head_filters = HeadFilters {
            pitch: OneEuroFilter::new(mc, beta),
            yaw: OneEuroFilter::new(mc, beta),
            roll: OneEuroFilter::new(mc, beta),
            x: OneEuroFilter::new(mc, beta),
            y: OneEuroFilter::new(mc, beta),
            z: OneEuroFilter::new(mc, beta),
        };
        self.eyes_x = OneEuroFilter::new(mc, beta);
        self.eyes_y = OneEuroFilter::new(mc, beta);
        self.left_hand.reset_params(mc, beta);
        self.right_hand.reset_params(mc, beta);
        
        self.filter_params = (mc, beta);
    }

    pub fn solve(&mut self, data: &TrackingData, cam_w: f32, cam_h: f32) -> SolverOutput {
        // Defensive: invalid dimensions would make hand/PnP math divide by zero
        // and emit NaN/inf, which permanently poisons the OneEuro filters.
        let cam_w = if cam_w > 1.0 { cam_w } else { 640.0 };
        let cam_h = if cam_h > 1.0 { cam_h } else { 480.0 };

        let mut params = Vec::new();
        let mut trackers = Vec::new();

        // 1. Head
        let (head_params, head_tracker) = self.solve_head(data, cam_w, cam_h);
        params.extend(head_params);
        trackers.push(head_tracker);
        
        // 2. Face
        let (face_params, face_cal_data) = self.solve_face(data);
        params.extend(face_params);
        
        // 3. Calibration (T-Pose) checks
        if self.calibration.is_calibrating() {
            // Calculate Arm Span from raw landmarks if available
            let mut arm_span = None;
            if let (Some(lh), Some(rh)) = (&data.left_hand_landmarks, &data.right_hand_landmarks) {
                 if !lh.is_empty() && !rh.is_empty() {
                     let get_wrist_m_raw = |pts: &Vec<[f32; 3]>| -> Vector3<f32> {
                         let w = Vector3::new(pts[0][0], pts[0][1], pts[0][2]);
                         let s = (Vector3::new(pts[9][0], pts[9][1], pts[9][2]) - w).norm();
                         let z = (600.0 * 0.09) / s.max(1.0);
                         let x = ((w.x - (cam_w / 2.0)) / cam_w) * z;
                         let y = -((w.y - (cam_h / 2.0)) / cam_w) * z;
                         Vector3::new(x, y, -z)
                     };
                     let dist = (get_wrist_m_raw(lh) - get_wrist_m_raw(rh)).norm() * 100.0;
                     arm_span = Some((dist - 30.0) / 2.0);
                 }
            }
            
            if let Some(dist) = arm_span {
                 if let Some(new_prof) = self.calibration.update(&face_cal_data, Some(dist), &self.user_profile) {
                     self.user_profile = new_prof;
                     self.arm_ik = ArmIK::new(self.user_profile.arm_upper_len, self.user_profile.arm_lower_len);
                 }
            } else if let Some(new_prof) = self.calibration.update(&face_cal_data, None, &self.user_profile) {
                self.user_profile = new_prof;
            }
        }

        // 4. Hands
        // Need current head transform for shoulder estimation
        let head_pos = self.last_translation.unwrap_or(Vector3::new(0.0, 0.0, 50.0)) / 100.0;
        let head_rot = self.last_rotation.unwrap_or(UnitQuaternion::identity());

        // Left Hand
        if let Some(lh) = &data.left_hand_landmarks {
            let (p, t) = self.solve_hand(lh, "LeftHand", true, head_pos, head_rot, cam_w, cam_h);
            self.last_left_hand_params = p.clone();
            self.left_hand_lost_at = None;
            params.extend(p);
            if let Some(tr) = t { trackers.push(tr); }
        } else {
             self.handle_hand_loss(&mut params, true);
        }

        // Right Hand
        if let Some(rh) = &data.right_hand_landmarks {
            let (p, t) = self.solve_hand(rh, "RightHand", false, head_pos, head_rot, cam_w, cam_h);
            self.last_right_hand_params = p.clone();
            self.right_hand_lost_at = None;
            params.extend(p);
            if let Some(tr) = t { trackers.push(tr); }
        } else {
             self.handle_hand_loss(&mut params, false);
        }

        SolverOutput { params, trackers }
    }

    fn solve_head(&mut self, data: &TrackingData, cam_w: f32, cam_h: f32) -> (Vec<(String, f32)>, TrackerData) {
        let mut params = Vec::new();
        
        let mut q_raw = self.last_rotation.unwrap_or(UnitQuaternion::identity());
        let mut t_raw = self.last_translation.unwrap_or(Vector3::new(0.0, 0.0, 50.0));

        if let Some(face) = &data.face_landmarks {
            let get_pt_2d = |idx: usize| -> Vector2<f32> {
                if idx < face.len() {
                    let p = face[idx];
                    Vector2::new(p[0], p[1])
                } else { Vector2::zeros() }
            };

            let pnp_indices = vec![1, 33, 263, 152, 61, 291];
            let mut image_points = Vec::new();
            for &idx in &pnp_indices {
                if idx < face.len() { image_points.push((idx, get_pt_2d(idx))); }
            }

            use crate::tracking::pnp;
            let (q, t) = pnp::solve_pnp(
                &image_points, 
                self.last_rotation, 
                self.last_translation, 
                cam_w, Vector2::new(cam_w / 2.0, cam_h / 2.0)
            );
            q_raw = q;
            // Clamp the PnP translation to a physically plausible box (cm). Noisy
            // landmarks (esp. in low light) can make Gauss-Newton diverge to huge
            // values (e.g. head "position" of -191 m), and because we feed the
            // result back as the next initial guess, one bad solve poisons all
            // following frames. Clamping keeps the head near the camera and stops
            // the feedback blow-up.
            t_raw = Vector3::new(
                t.x.clamp(-40.0, 40.0),
                t.y.clamp(-40.0, 40.0),
                t.z.clamp(20.0, 150.0),
            );

            self.last_rotation = Some(q_raw);
            self.last_translation = Some(t_raw);
        }
        
        // Dead Zone
        let t_dead = Self::apply_dead_zone_vec3(t_raw, &mut self.last_sent_translation, 0.2);
        let q_dead = Self::apply_dead_zone_quat(q_raw, &mut self.last_sent_rotation, 0.2);

        // Filter
        let (r, p, y) = q_dead.euler_angles();
        let smooth_pitch = self.head_filters.pitch.filter(p).clamp(-1.0, 1.0);
        let smooth_yaw = self.head_filters.yaw.filter(y).clamp(-1.5, 1.5);
        let smooth_roll = self.head_filters.roll.filter(r);
        
        let smooth_x = self.head_filters.x.filter(t_dead.x);
        let smooth_y = self.head_filters.y.filter(t_dead.y);
        let smooth_z = self.head_filters.z.filter(t_dead.z);

        params.push(("HeadPitch".to_string(), -smooth_pitch)); 
        params.push(("HeadYaw".to_string(), -smooth_yaw));   
        params.push(("HeadRoll".to_string(), -smooth_roll));
        
        let q_final = UnitQuaternion::from_euler_angles(smooth_roll, smooth_pitch, smooth_yaw);
        params.push(("SYS_HEAD_ROT_X".to_string(), q_final.i));
        params.push(("SYS_HEAD_ROT_Y".to_string(), q_final.j));
        params.push(("SYS_HEAD_ROT_Z".to_string(), q_final.k));
        params.push(("SYS_HEAD_ROT_W".to_string(), q_final.w));

        params.push(("HeadPos_X".to_string(), smooth_x / 100.0));
        params.push(("HeadPos_Y".to_string(), -smooth_y / 100.0));
        params.push(("HeadPos_Z".to_string(), -smooth_z / 100.0));

        let tracker = TrackerData {
            id: 0,
            position: [smooth_x / 100.0, -smooth_y / 100.0, -smooth_z / 100.0],
            rotation: [q_final.i, q_final.j, q_final.k, q_final.w],
        };
        
        (params, tracker)
    }

    fn solve_face(&mut self, data: &TrackingData) -> (Vec<(String, f32)>, HashMap<String, f32>) {
        let mut params = Vec::new();
        let mut cal_data = HashMap::new();
        
        if let Some(face) = &data.face_landmarks {
            // 2D only: blink / jaw / gaze are screen-space aperture RATIOS. The
            // model's per-vertex Z is noisy depth and was polluting these .norm()
            // distances (e.g. pinning EyeBlink at 1.0). Drop Z for these ratios.
            let get_pt = |idx: usize| -> Vector3<f32> {
                 if idx < face.len() {
                    let p = face[idx];
                    Vector3::new(p[0], p[1], 0.0)
                } else { Vector3::zeros() }
            };

            // Blink
            let left_ratio = (get_pt(159) - get_pt(145)).norm() / (get_pt(33) - get_pt(133)).norm();
            let right_ratio = (get_pt(386) - get_pt(374)).norm() / (get_pt(362) - get_pt(263)).norm();

            let real_blink = left_ratio < 0.25 && right_ratio < 0.25;
            let (s_pitch, s_yaw) = self.update_saccades();
            let auto_blink = self.update_auto_blink(real_blink);
            
            let bl = if left_ratio < 0.25 { 1.0 } else { auto_blink };
            let br = if right_ratio < 0.25 { 1.0 } else { auto_blink };
            
            params.push(("EyeBlinkLeft".to_string(), self.blink_l.filter(bl)));
            params.push(("EyeBlinkRight".to_string(), self.blink_r.filter(br)));

            // Gaze
            let left_eye_center = (get_pt(33) + get_pt(133)) * 0.5;
            let right_eye_center = (get_pt(263) + get_pt(362)) * 0.5;
            let left_iris = (get_pt(159) + get_pt(145)) * 0.5;
            let right_iris = (get_pt(386) + get_pt(374)) * 0.5;
            
            let gaze_x_l = (left_iris.x - left_eye_center.x) / (get_pt(33) - get_pt(133)).norm().max(0.001);
            let gaze_x_r = (right_iris.x - right_eye_center.x) / (get_pt(263) - get_pt(362)).norm().max(0.001);
            // Y needs similar norm? old code used width too.
            let gaze_y_l = (left_iris.y - left_eye_center.y) / (get_pt(33) - get_pt(133)).norm().max(0.001);
            let gaze_y_r = (right_iris.y - right_eye_center.y) / (get_pt(263) - get_pt(362)).norm().max(0.001);
            
            let raw_eyes_x = ((gaze_x_l + gaze_x_r) * 0.5) * 3.0 + s_yaw * 0.5;
            let raw_eyes_y = -((gaze_y_l + gaze_y_r) * 0.5) * 3.0 + s_pitch * 0.5;
            
            let eyes_x = self.eyes_x.filter(raw_eyes_x).clamp(-0.58, 0.58);
            let eyes_y = self.eyes_y.filter(raw_eyes_y).clamp(-0.42, 0.42);
            
            params.push(("EyesX".to_string(), eyes_x));
            params.push(("EyesY".to_string(), eyes_y));
            
            // Directional Look
            params.push(("EyeLookUpLeft".to_string(), eyes_y.max(0.0)));
            params.push(("EyeLookUpRight".to_string(), eyes_y.max(0.0)));
            params.push(("EyeLookDownLeft".to_string(), (-eyes_y).max(0.0)));
            params.push(("EyeLookDownRight".to_string(), (-eyes_y).max(0.0)));
            params.push(("EyeLookInLeft".to_string(), eyes_x.max(0.0)));
            params.push(("EyeLookInRight".to_string(), (-eyes_x).max(0.0)));
            params.push(("EyeLookOutLeft".to_string(), (-eyes_x).max(0.0)));
            params.push(("EyeLookOutRight".to_string(), eyes_x.max(0.0)));

            // Jaw
            let jaw_ratio = (get_pt(13) - get_pt(14)).norm() / (get_pt(152) - get_pt(10)).norm();
            let neutral = self.user_profile.neutral_face.get("JawRatio").unwrap_or(&0.025);
            let raw_jaw = ((jaw_ratio - neutral) * 8.0).clamp(0.0, 1.0);
            params.push(("JawOpen".to_string(), self.jaw.filter(raw_jaw)));
            
            if self.calibration.is_calibrating() {
                cal_data.insert("JawRatio".to_string(), jaw_ratio);
                cal_data.insert("BlinkLeft".to_string(), left_ratio);
                cal_data.insert("BlinkRight".to_string(), right_ratio);
            }
            
            // Micro Expressions
            let (mb, mc, mm) = self.update_micro_expressions();
            params.push(("BrowInnerUp".to_string(), mb));
            params.push(("CheekSquintLeft".to_string(), mc));
            params.push(("CheekSquintRight".to_string(), mc));
            params.push(("MouthCornerPullLeft".to_string(), mm.max(0.0)));
            params.push(("MouthCornerPullRight".to_string(), (-mm).max(0.0)));
            
            self.last_face_params = params.clone();
            self.face_lost_at = None;
            
        } else {
            // Fallback
             params.extend(self.generate_fallback_face());
        }
        
        (params, cal_data)
    }

    fn solve_hand(&mut self, landmarks: &Vec<[f32; 3]>, prefix: &str, is_left: bool, head_pos: Vector3<f32>, head_rot: UnitQuaternion<f32>, cam_w: f32, cam_h: f32) -> (Vec<(String, f32)>, Option<TrackerData>) {
        if landmarks.len() < 21 { return (Vec::new(), None); }
        let mut params = Vec::new();
        
        let get_pt = |idx: usize| -> Vector3<f32> {
             Vector3::new(landmarks[idx][0], landmarks[idx][1], landmarks[idx][2])
        };
        
        // 1. Raw Position (Meters)
        let wrist = get_pt(0);
        let hand_size_px = (get_pt(9) - wrist).norm();
        let z_hand = (cam_w * 0.09) / hand_size_px.max(1.0);
        let x = ((wrist.x - (cam_w / 2.0)) / cam_w) * z_hand;
        let y = -((wrist.y - (cam_h / 2.0)) / cam_w) * z_hand;
        let pos_raw = Vector3::new(x, y, -z_hand);

        // 2. Dead Zone
        let filters = if is_left { &mut self.left_hand } else { &mut self.right_hand };
        let pos_dead = Self::apply_dead_zone_vec3(pos_raw, &mut filters.dead_zone_pos, 0.005);
        
        // 3. Filter
        let smooth_hand = Vector3::new(
            filters.pos_x.filter(pos_dead.x),
            filters.pos_y.filter(pos_dead.y),
            filters.pos_z.filter(pos_dead.z)
        );
        
        // 4. IK
        let sign = if is_left { -1.0 } else { 1.0 };
        let shoulder_offset = Vector3::new(sign * 0.15, -0.25, -0.05);
        let (_r, _p, y_head) = head_rot.euler_angles();
        let torso_rot = UnitQuaternion::from_euler_angles(0.0, 0.0, y_head * 0.5);
        let shoulder_pos = head_pos + torso_rot * shoulder_offset;
        
        let rel_y = smooth_hand.y - shoulder_pos.y;
        let t = (-rel_y * 2.0).clamp(0.0, 1.0);
        let dynamic_pole = Vector3::new(0.0, -1.0, 0.0).lerp(&Vector3::new(0.0, 0.0, -1.0), t).normalize();
        
        let elbow_cm = self.arm_ik.solve(shoulder_pos * 100.0, smooth_hand * 100.0, dynamic_pole);
        let elbow_raw = elbow_cm / 100.0;
        
        // Filter Elbow (Need separate filters for elbow?)
        // The original code used "Elbow_X" keys which implied unique filters.
        // My struct has elbow filters.
        let filters = if is_left { &mut self.left_hand } else { &mut self.right_hand }; // Re-borrow
        
        let elbow_dead = Self::apply_dead_zone_vec3(elbow_raw, &mut filters.dead_zone_elbow, 0.005);
        let smooth_elbow = Vector3::new(
             filters.elbow_x.filter(elbow_dead.x),
             filters.elbow_y.filter(elbow_dead.y),
             filters.elbow_z.filter(elbow_dead.z)
        );

        // 5. Rotation
        let v1 = (get_pt(5) - wrist).normalize();
        let v2 = (get_pt(17) - wrist).normalize();
        let normal = v1.cross(&v2).normalize();
        let up = (v1 + v2).normalize();
        let right = up.cross(&normal).normalize();
        let rot_mat = nalgebra::Rotation3::from_basis_unchecked(&[right, up, normal]);
        let q = UnitQuaternion::from_rotation_matrix(&rot_mat);

        params.push((format!("{}Pos_X", prefix), smooth_hand.x));
        params.push((format!("{}Pos_Y", prefix), smooth_hand.y));
        params.push((format!("{}Pos_Z", prefix), smooth_hand.z));
        params.push((format!("{}Elbow_X", prefix), smooth_elbow.x));
        params.push((format!("{}Elbow_Y", prefix), smooth_elbow.y));
        params.push((format!("{}Elbow_Z", prefix), smooth_elbow.z));
        params.push((format!("{}Rot_X", prefix), q.i));
        params.push((format!("{}Rot_Y", prefix), q.j));
        params.push((format!("{}Rot_Z", prefix), q.k));
        params.push((format!("{}Rot_W", prefix), q.w));

        // Fingers
        let calc_curl = |indices: [usize; 4]| -> f32 {
             let b = get_pt(indices[0]);
             let t = get_pt(indices[3]);
             let vp = (b - wrist).normalize();
             let vf = (t - b).normalize();
             ((1.0 - vp.dot(&vf)) * 0.7).clamp(0.0, 1.0)
        };
        params.push((format!("{}Thumb", prefix), calc_curl([1, 2, 3, 4])));
        params.push((format!("{}Index", prefix), calc_curl([5, 6, 7, 8])));
        params.push((format!("{}Middle", prefix), calc_curl([9, 10, 11, 12])));
        params.push((format!("{}Ring", prefix), calc_curl([13, 14, 15, 16])));
        params.push((format!("{}Pinky", prefix), calc_curl([17, 18, 19, 20])));

        let tracker = TrackerData {
            id: if is_left { 1 } else { 2 },
            position: [smooth_hand.x, smooth_hand.y, smooth_hand.z],
            rotation: [q.i, q.j, q.k, q.w],
        };
        
        (params, Some(tracker))
    }
    
    fn handle_hand_loss(&mut self, params: &mut Vec<(String, f32)>, is_left: bool) {
        let (lost_at, last_params) = if is_left {
            (&mut self.left_hand_lost_at, &self.last_left_hand_params)
        } else {
            (&mut self.right_hand_lost_at, &self.last_right_hand_params)
        };
        
        let start = lost_at.get_or_insert_with(std::time::Instant::now);
        let elapsed = start.elapsed().as_millis() as f32;
        
        if elapsed < 200.0 {
            params.extend(last_params.clone());
        } else if elapsed < 500.0 {
            let decay = 1.0 - (elapsed - 200.0) / 300.0;
            for (k, v) in last_params {
                params.push((k.clone(), v * decay));
            }
        }
    }
}
