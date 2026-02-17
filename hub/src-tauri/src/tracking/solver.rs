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
    
    // Filters
    filters: HashMap<String, OneEuroFilter>,
    inertia: HashMap<String, InertiaFilter>,
    hand_dead_zones: HashMap<String, Vector3<f32>>, // Store last raw hand position
    
    // IK
    arm_ik: ArmIK,

    // Calibration
    pub calibration: CalibrationManager, // Public to access start()
    pub user_profile: UserProfile,
    
    // Eye Animation State
    saccade_target: (f32, f32), // Pitch, Yaw offset
    saccade_timer: std::time::Instant,
    saccade_duration: std::time::Duration,
    
    _last_blink_time: std::time::Instant,
    filter_params: (f32, f32), // (min_cutoff, beta)
}

impl Solver {
    pub fn new() -> Self {
        Self { 
            filters: HashMap::new(),
            inertia: HashMap::new(),
            last_rotation: None,
            last_translation: None, 
            last_sent_rotation: None,
            last_sent_translation: None,
            arm_ik: ArmIK::new(30.0, 25.0), 
            hand_dead_zones: HashMap::new(),
            calibration: CalibrationManager::new(),
            user_profile: UserProfile::default(),

            // Eye Animation
            saccade_target: (0.0, 0.0),
            saccade_timer: std::time::Instant::now(),
            saccade_duration: std::time::Duration::from_millis(100),
            _last_blink_time: std::time::Instant::now(),
            filter_params: (1.5, 0.01), // Default Medium
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

    fn update_saccades(&mut self) -> (f32, f32) {
        // ... existing saccade logic ...
        if self.saccade_timer.elapsed() > self.saccade_duration {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let range = if rng.gen_bool(0.1) { 0.2 } else { 0.02 }; 
            self.saccade_target = (rng.gen_range(-range..range), rng.gen_range(-range..range));
            self.saccade_duration = std::time::Duration::from_millis(rng.gen_range(200..500));
            self.saccade_timer = std::time::Instant::now();
        }
        self.saccade_target
    }

    pub fn set_quality(&mut self, quality: &str) {
        // Tune Filter Parameters based on Quality
        // Ultra: Low Latency, High Jitter allowed (min_cutoff high)
        // Med: Balanced
        // Low: Smooth, High Latency (min_cutoff low) OR Skip IK steps?
        
        let (mc, beta) = match quality {
            "High" => (4.0, 0.05), // Very responsive
            "Medium" => (1.5, 0.01), // Default
            "Low" => (0.5, 0.001), // Very smooth/slow
            _ => (1.5, 0.01),
        };
        
        // Re-configure filters? 
        // OneEuroFilter struct doesn't expose setters maybe?
        // We can just clear them or update logic to use these values next frame.
        // For simplicity, let's clear filters so they respawn with new params.
        self.filters.clear();
        
        // Store quality state if needed, or just hardcode params in solve() via a field.
        // Let's add `filter_params: (f32, f32)` to struct.
        self.filter_params = (mc, beta);
    }



    pub fn solve(&mut self, data: &TrackingData) -> Vec<(String, f32)> {
        let mut params = Vec::new();
        let mut cal_face_data = HashMap::new();
        
        // ... [Face Logic same as before] ...
        if let Some(face) = &data.face_landmarks {
            let get_pt_2d = |idx: usize| -> Vector2<f32> {
                if idx < face.len() {
                    let p = face[idx];
                    Vector2::new(p[0] * 640.0, p[1] * 480.0)
                } else { Vector2::zeros() }
            };

            // 1. PnP Solve
            let pnp_indices = vec![1, 33, 263, 152, 61, 291];
            let mut image_points = Vec::new();
            for &idx in &pnp_indices {
                if idx < face.len() { image_points.push((idx, get_pt_2d(idx))); }
            }

            use crate::tracking::pnp;
            let (q_raw, t_raw) = pnp::solve_pnp(
                &image_points, 
                self.last_rotation, 
                self.last_translation, 
                600.0, Vector2::new(320.0, 240.0)
            );
            
            self.last_rotation = Some(q_raw);
            self.last_translation = Some(t_raw);

            // 2. Dead Zone (Micro-jitter suppression)
            let t_dead = Self::apply_dead_zone_vec3(t_raw, &mut self.last_sent_translation, 0.2);
            let q_dead = Self::apply_dead_zone_quat(q_raw, &mut self.last_sent_rotation, 0.2);

            // 3. One Euro Filter (Smoothing)
            let (mut roll, mut pitch, mut yaw) = q_dead.euler_angles();
            let mut tx = t_dead.x;
            let mut ty = t_dead.y;
            let mut tz = t_dead.z;

            let filter_val = |filters: &mut HashMap<String, OneEuroFilter>, name: &str, val: f32, min_cutoff: f32, beta: f32| -> f32 {
                let f = filters.entry(name.to_string()).or_insert_with(|| OneEuroFilter::new(min_cutoff, beta));
                f.filter(val)
            };

            let h_mc = self.filter_params.0; 
            let h_beta = self.filter_params.1;

            pitch = filter_val(&mut self.filters, "HeadPitch", pitch, h_mc, h_beta);
            yaw = filter_val(&mut self.filters, "HeadYaw", yaw, h_mc, h_beta);
            roll = filter_val(&mut self.filters, "HeadRoll", roll, h_mc, h_beta);

            tx = filter_val(&mut self.filters, "HeadX", tx, h_mc, h_beta);
            ty = filter_val(&mut self.filters, "HeadY", ty, h_mc, h_beta);
            tz = filter_val(&mut self.filters, "HeadZ", tz, h_mc, h_beta);

            // 4. Anatomical Clamping
            yaw = yaw.clamp(-1.5, 1.5);
            pitch = pitch.clamp(-1.0, 1.0);
            
            // Output Head Params
            params.push(("HeadPitch".to_string(), -pitch)); 
            params.push(("HeadYaw".to_string(), -yaw));   
            params.push(("HeadRoll".to_string(), -roll));  

            let q_final = UnitQuaternion::from_euler_angles(roll, pitch, yaw);
            params.push(("SYS_HEAD_ROT_X".to_string(), q_final.i));
            params.push(("SYS_HEAD_ROT_Y".to_string(), q_final.j));
            params.push(("SYS_HEAD_ROT_Z".to_string(), q_final.k));
            params.push(("SYS_HEAD_ROT_W".to_string(), q_final.w));

            params.push(("HeadPos_X".to_string(), tx / 100.0));
            params.push(("HeadPos_Y".to_string(), -ty / 100.0));
            params.push(("HeadPos_Z".to_string(), -tz / 100.0));

            // --- Expressions (Inertia Filtered + Calibrated) ---
            let get_pt = |idx: usize| -> Vector3<f32> {
                 if idx < face.len() {
                    let p = face[idx];
                    Vector3::new(p[0], p[1], p[2])
                } else { Vector3::zeros() }
            };

            // Blink
            let left_ratio = (get_pt(159) - get_pt(145)).norm() / (get_pt(33) - get_pt(133)).norm();
            let right_ratio = (get_pt(386) - get_pt(374)).norm() / (get_pt(362) - get_pt(263)).norm();
            
            // Apply Neutral Offset if calibrated (Blink neutral is usually ~0.25 open?)
            // Actually Blink Logic is threshold based. 
            // Better to calibrate the THRESHOLD?
            // "Neutral Face" usually means eyes open, mouth closed.
            // If user has narrow eyes, left_ratio might be 0.2 naturally.
            // We can adjust threshold based on neutral.
            // But for now, let's keep hard threshold 0.25 (robust).
            
            let raw_blink_l = if left_ratio < 0.25 { 1.0 } else { 0.0 };
            let raw_blink_r = if right_ratio < 0.25 { 1.0 } else { 0.0 };

            let inertia_val = |inertias: &mut HashMap<String, InertiaFilter>, name: &str, val: f32, attack: f32, decay: f32| -> f32 {
                let f = inertias.entry(name.to_string()).or_insert_with(|| InertiaFilter::new(attack, decay));
                f.filter(val)
            };

            let blink_l = inertia_val(&mut self.inertia, "BlinkL", raw_blink_l, 1.0, 0.2);
            let blink_r = inertia_val(&mut self.inertia, "BlinkR", raw_blink_r, 1.0, 0.2);
            
            params.push(("EyeBlinkLeft".to_string(), blink_l));
            params.push(("EyeBlinkRight".to_string(), blink_r));

            // Saccades (Micro-movements)
            // User request: "Yeux qui glissent = faux", "Saccades = vivant"
            let (s_pitch, s_yaw) = self.update_saccades();
            
            // Output Eye transforms (Offsets from Head)
            // VRChat standard: EyesX, EyesY (-1..1)
            // We scale our small radian offsets to be visible.
            params.push(("EyesX".to_string(), s_yaw * 5.0)); 
            params.push(("EyesY".to_string(), s_pitch * 5.0));

            // Jaw
            let face_h = (get_pt(152) - get_pt(10)).norm();
            let mouth_h = (get_pt(13) - get_pt(14)).norm();
            let jaw_ratio = mouth_h / face_h; 
            
            // Calibration: Neural Jaw Ratio
            let neutral_jaw = self.user_profile.neutral_face.get("JawRatio").unwrap_or(&0.025); // Default 0.025
            
            let raw_jaw = if jaw_ratio > *neutral_jaw { (jaw_ratio - neutral_jaw) * 8.0 } else { 0.0 };
            let raw_jaw = raw_jaw.clamp(0.0, 1.0);

            let jaw = inertia_val(&mut self.inertia, "Jaw", raw_jaw, 0.5, 0.05);
            params.push(("JawOpen".to_string(), jaw));
            
            // Collect Calibration Data
            if self.calibration.is_calibrating() {
                cal_face_data.insert("JawRatio".to_string(), jaw_ratio);
                cal_face_data.insert("BlinkLeft".to_string(), left_ratio);
                cal_face_data.insert("BlinkRight".to_string(), right_ratio);
            }
            

        }

        // Helper to get head transform (computed above)
        // We really should store it in struct state or return it from PnP block.
        // It is `self.last_translation` and `self.last_rotation`.
        let head_pos = self.last_translation.unwrap_or(Vector3::new(0.0, 0.0, 50.0)); // cm
        let head_rot = self.last_rotation.unwrap_or(UnitQuaternion::identity());
        
        // --- Hand Tracking with IK (Smoothed) ---

        // Calibration: T-Pose
        let mut arm_span = None;
        if self.calibration.is_calibrating() {
             if let (Some(lh), Some(rh)) = (&data.left_hand_landmarks, &data.right_hand_landmarks) {
                 // Get Wrists (Idx 0)
                 if !lh.is_empty() && !rh.is_empty() {
                     // We need World Space Wrists for accurate length?
                     // Or just consistency?
                     // PnP gives Head in cm.
                     // Hand logic calculates `hand_pos_m`.
                     // We should use the same logic to get `wrist_m`.
                     // Since `process_hand` does it, maybe we can extract it?
                     // Or just re-calc.
                     // Re-calc specific for calibration is safer.
                     
                     // Helper to get wrist meters
                     let get_wrist_m = |landmarks: &Vec<[f32; 3]>| -> Vector3<f32> {
                         let wrist = Vector3::new(landmarks[0][0], landmarks[0][1], landmarks[0][2]);
                         let hand_size_px = (Vector3::new(landmarks[9][0], landmarks[9][1], landmarks[9][2]) - wrist).norm();
                         let z_hand = (600.0 * 0.09) / hand_size_px.max(1.0);
                         let x = ((wrist.x - 320.0) / 600.0) * z_hand;
                         let y = -((wrist.y - 240.0) / 600.0) * z_hand;
                         Vector3::new(x, y, -z_hand)
                     };
                     
                     let l_wrist = get_wrist_m(lh);
                     let r_wrist = get_wrist_m(rh);
                     let dist = (l_wrist - r_wrist).norm() * 100.0; // cm
                     // Note: T-Pose Arm Span includes Chest width.
                     // Chest width ~ 30-40cm.
                     // Arm Length = (Span - ShoulderWidth) / 2.
                     // We handle this math in `CalibrationManager`?
                     // Manager currently does: `avg_dist * 0.55`.
                     // It assumes `avg_dist` is Shoulder->Hand.
                     // IF we pass Span, we must subtract shoulder width.
                     // Let's pass (Span - 30.0) / 2.0 as "One Arm Length".
                     arm_span = Some((dist - 30.0) / 2.0);
                 }
             }
        }
        
        // Update Calibration
        // We need to pass the Face Params captured earlier. 
        // I'll assume we collected them in `cal_data` if I modified the previous block correctly.
        // Wait, previous block had: `self.calibration.update(&cal_data, None);`
        // I need to change that. I can't call update twice if it finishes!
        // The previous block only called update for Face data.
        // I should Move the update call to HERE.
        
        // Correct approach:
        // 1. In Face block, just push to a local `face_cal_data` variable.
        // 2. Here, call `update` with `face_cal_data` and `arm_span`.
        
        // Since I already wrote the previous block to call `update`...
        // I need to FIX the previous block or just assume facial calibration doesn't overlap with T-Pose?
        // `stage` is enum. Can't be both.
        // So calling update with `None` for arm is fine if stage is NeutralFace.
        // Calling update with `None` for face is fine if stage is TPose.
        // So 2 calls is okay-ish, as long as `finish` isn't triggered between them?
        // Update checks `duration`.
        // If I make 2 calls, I might add 2 samples?
        // `update` adds sample.
        // So I should NOT call it twice per frame.
        
        // I must refactor the previous block to NOT call update, but store data.
        // Or I just call it here.
        
        // Let's just add the T-Pose update call here.
        // If stage is TPose, the Face block call (if any) will define Face params?
        // Face block: `if self.calibration.is_calibrating() { ... update(..., None) }`
        // If stage is TPose, Face block will call `update` with `None` arm.
        // Then HERE, if stage is TPose, we call `update` with `arm`.
        // This adds 2 samples per frame if both active?
        // Manager logic:
        // `match self.stage { ... Neutral => samples.push ... TPose => arm_samples.push ... }`
        // So if I call update for TPose, it ignores Face data.
        // If I call update for Neutral, it ignores Arm data.
        // So 2 calls is SAFE.
        
        if let Some(dist) = arm_span {
             if let Some(new_prof) = self.calibration.update(&cal_face_data, Some(dist)) {
                 println!("[Solver] New Profile Applied!");
                 self.user_profile = new_prof;
                 self.arm_ik = ArmIK::new(self.user_profile.arm_upper_len, self.user_profile.arm_lower_len);
             }
        } else {
             if let Some(new_prof) = self.calibration.update(&cal_face_data, None) {
                 println!("[Solver] New Profile Applied!");
                 self.user_profile = new_prof;
                 // Arm lengths might not change if facial calibration, but safe to update or ignore.
                 // Actually neutral face doesn't affect arm len.
             }
        }
        


        let mut process_hand = |landmarks: &Vec<[f32; 3]>, prefix: &str, is_left: bool| -> Vec<(String, f32)> {
            let mut h_params = Vec::new();
            if landmarks.len() < 21 { return h_params; }
            
            let get_hvar = |idx: usize| -> Vector3<f32> {
                let p = landmarks[idx];
                Vector3::new(p[0], p[1], p[2])
            };

            // 1. Raw Wrist Position
            let wrist = get_hvar(0);
            let hand_size_px = (get_hvar(9) - wrist).norm(); 
            let z_hand = (600.0 * 0.09) / hand_size_px.max(1.0); 
            
            let cx = 320.0;
            let cy = 240.0;
            let x_hand = ((wrist.x - cx) / 600.0) * z_hand;
            let y_hand = -((wrist.y - cy) / 600.0) * z_hand;
            let hand_pos_m = Vector3::new(x_hand, y_hand, -z_hand); 
            
            // 2. Estimate Shoulder Logic
            let head_pos_m = head_pos / 100.0; 
            let sign = if is_left { -1.0 } else { 1.0 };
            let shoulder_offset = Vector3::new(sign * 0.15, -0.25, -0.05);
            let (_r, _p, y_head) = head_rot.euler_angles();
            let torso_rot = UnitQuaternion::from_euler_angles(0.0, 0.0, y_head * 0.5);
            let shoulder_pos = head_pos_m + torso_rot * shoulder_offset;
            
            // 3. Solve IK
            let pole = Vector3::new(sign * 0.2, -1.0, -0.3).normalize();
            let shoulder_cm = shoulder_pos * 100.0;
            let hand_cm = hand_pos_m * 100.0;
            let elbow_cm = self.arm_ik.solve(shoulder_cm, hand_cm, pole);
            let elbow_m = elbow_cm / 100.0;

            // 4. Dead Zone & Smoothing
            // Helper for Vectors
            let mut filter_vec3 = |name_suffix: &str, raw: Vector3<f32>| -> Vector3<f32> {
                let key = format!("{}{}", prefix, name_suffix);
                
                // Dead Zone (0.5 cm = 0.005 m)
                let last_val = self.hand_dead_zones.entry(key.clone()).or_insert(raw);
                let current_val = if (raw - *last_val).norm() < 0.005 {
                    *last_val
                } else {
                    *last_val = raw;
                    raw
                };

                // OneEuro (Dynamic Quality)
                let (mc, beta) = self.filter_params;
                let f_x = self.filters.entry(format!("{}_X", key)).or_insert_with(|| OneEuroFilter::new(mc, beta));
                let x = f_x.filter(current_val.x);
                
                let f_y = self.filters.entry(format!("{}_Y", key)).or_insert_with(|| OneEuroFilter::new(mc, beta));
                let y = f_y.filter(current_val.y);
                
                let f_z = self.filters.entry(format!("{}_Z", key)).or_insert_with(|| OneEuroFilter::new(mc, beta));
                let z = f_z.filter(current_val.z);
                
                Vector3::new(x, y, z)
            };

            let smooth_hand = filter_vec3("Pos", hand_pos_m);
            let smooth_elbow = filter_vec3("Elbow", elbow_m);

            // 5. Output
            h_params.push((format!("{}Pos_X", prefix), smooth_hand.x));
            h_params.push((format!("{}Pos_Y", prefix), smooth_hand.y));
            h_params.push((format!("{}Pos_Z", prefix), smooth_hand.z));
            
            h_params.push((format!("{}Elbow_X", prefix), smooth_elbow.x));
            h_params.push((format!("{}Elbow_Y", prefix), smooth_elbow.y));
            h_params.push((format!("{}Elbow_Z", prefix), smooth_elbow.z));

            // Wrist Rot
            let v1 = (get_hvar(5) - wrist).normalize();
            let v2 = (get_hvar(17) - wrist).normalize();
            let palm_normal = v1.cross(&v2).normalize(); 
            let palm_up = (v1 + v2).normalize(); 
            let palm_right = palm_up.cross(&palm_normal).normalize();
            let rot = nalgebra::Rotation3::from_basis_unchecked(&[palm_right, palm_up, palm_normal]);
            let q = UnitQuaternion::from_rotation_matrix(&rot);

            h_params.push((format!("{}Rot_X", prefix), q.i));
            h_params.push((format!("{}Rot_Y", prefix), q.j));
            h_params.push((format!("{}Rot_Z", prefix), q.k));
            h_params.push((format!("{}Rot_W", prefix), q.w));

            // Finger Curls
            let calc_curl = |indices: [usize; 4]| -> f32 {
                let base = get_hvar(indices[0]);
                let tip = get_hvar(indices[3]);
                let v_palm = (base - wrist).normalize();
                let v_finger = (tip - base).normalize();
                let dot = v_palm.dot(&v_finger); 
                let val = (1.0 - dot) * 0.7; 
                val.clamp(0.0, 1.0)
            };
            h_params.push((format!("{}Thumb", prefix), calc_curl([1, 2, 3, 4])));
            h_params.push((format!("{}Index", prefix), calc_curl([5, 6, 7, 8])));
            h_params.push((format!("{}Middle", prefix), calc_curl([9, 10, 11, 12])));
            h_params.push((format!("{}Ring", prefix), calc_curl([13, 14, 15, 16])));
            h_params.push((format!("{}Pinky", prefix), calc_curl([17, 18, 19, 20])));

            h_params
        };

        if let Some(lh) = &data.left_hand_landmarks {
            params.extend(process_hand(lh, "HandLeft", true));
        }
        if let Some(rh) = &data.right_hand_landmarks {
            params.extend(process_hand(rh, "HandRight", false));
        }

        // Filters are already applied in the face logic above (Head/Eyes/Jaw)
        // Hands might need filtering too, but let's stick to Phase 2 (Face/Head) for now.
        // If we wanted to filter hands, we'd do it here or inside process_hand.
        // For now, return params as is.

        params
    }
}
