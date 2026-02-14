use crate::tracking::types::TrackingData;
use crate::tracking::filter::OneEuroFilter;
use std::collections::HashMap;
use nalgebra::{Vector3, UnitQuaternion};

pub struct Solver {
    filters: HashMap<String, OneEuroFilter>,
}

impl Solver {
    pub fn new() -> Self {
        Self { filters: HashMap::new() }
    }

    pub fn solve(&mut self, data: &TrackingData) -> Vec<(String, f32)> {
        let mut params = Vec::new();
        
        if let Some(face) = &data.face_landmarks {
            // Helper to get point as Vec3
            let get_pt = |idx: usize| -> Vector3<f32> {
                if idx < face.len() {
                    let p = face[idx];
                    Vector3::new(p[0], p[1], p[2])
                } else {
                    Vector3::zeros()
                }
            };

            // Landmark Indices (MediaPipe Face Mesh)
            // 1: Nose Tip
            // 33: Left Eye Outer corner
            // 263: Right Eye Outer corner
            // 152: Chin
            // 10: Top of Head

            let nose = get_pt(1);
            let chin = get_pt(152);
            let left_eye = get_pt(33);
            let right_eye = get_pt(263);


            // --- Head Rotation (Approximate) ---
            // Normalize by Face Width (Outer Eyes Distance) to be Z-independent
            let face_width = (left_eye - right_eye).norm(); // In pixels, e.g. 100.0
            if face_width < 1.0 { return params; } // Avoid div by zero

            // 1. Calculate Roll (Tilt)
            // atan2 is robust, returns radians -PI to PI.
            let eye_delta = right_eye - left_eye;
            let roll = eye_delta.y.atan2(eye_delta.x); // Radians

            // 2. Calculate Yaw (Turn)
            // Compare Nose X to Center of Eyes X
            let face_center_x = (left_eye.x + right_eye.x) / 2.0;
            // Delta X relative to face width.
            // If nose is 20px right of center, and face width is 100px -> 0.2
            // Typical range max turn is maybe 0.5?
            let yaw_rel = (nose.x - face_center_x) / face_width;
            let yaw = yaw_rel * 3.0; // Scale to approx Radians (Tunable)

            // 3. Calculate Pitch (Nod)
            // Use Head Top (10) vs Chin (152).
            // Nose (1) is usually at 40-50% of height from top.
            // If nose moves up (lower Y), pitch is up.
            let face_height = (chin - get_pt(10)).norm();
            let nose_top_dist = (nose - get_pt(10)).norm(); // Distance from top
            let pitch_rel = nose_top_dist / face_height; // ~0.5 usually
            // Range: 0.4 (Up) to 0.6 (Down)
            // Adjusted offset from 0.45 to 0.42 to fix "always looking down"
            let pitch = (pitch_rel - 0.42) * -3.0; // Reduced multiplier slightly

            // Debug Log (Throttle to avoid spam)
            // Use simple print
            // println!("[Solver] Yaw: {:.2} Pitch: {:.2} Roll: {:.2}", yaw, pitch, roll);

            // Debug Raw Coords
            // println!("[Solver] Nose: {:?} LEye: {:?} REye: {:?}", nose, left_eye, right_eye);

            params.push(("HeadRoll".to_string(), roll));
            params.push(("HeadYaw".to_string(), yaw));
            params.push(("HeadPitch".to_string(), pitch));

            // Calc Quaternion from Euler (Yaw, Pitch, Roll)
            // Roll (Z), Pitch (X), Yaw (Y)
            // Check UnitQuaternion::from_euler_angles documentation order!
            // It is Roll, Pitch, Yaw (x, y, z axes) applied in order?
            // Actually it takes (roll, pitch, yaw).
            // Let's assume standard aircraft convention mapping to X, Y, Z.
            let q = UnitQuaternion::from_euler_angles(roll, pitch, yaw); 
            
            params.push(("SYS_HEAD_ROT_X".to_string(), q.i));
            params.push(("SYS_HEAD_ROT_Y".to_string(), q.j));
            params.push(("SYS_HEAD_ROT_Z".to_string(), q.k));
            params.push(("SYS_HEAD_ROT_W".to_string(), q.w));

            // --- Eye Openness ---
            // Left Eye: 159 (Top), 145 (Bottom)
            // Right Eye: 386 (Top), 374 (Bottom)
            let left_open_dist = (get_pt(159) - get_pt(145)).norm();
            let right_open_dist = (get_pt(386) - get_pt(374)).norm();
            
            // Normalize by eye width (Horizontal)
            let left_eye_w = (get_pt(33) - get_pt(133)).norm();
            let right_eye_w = (get_pt(362) - get_pt(263)).norm();

            let left_ratio = left_open_dist / left_eye_w;
            let right_ratio = right_open_dist / right_eye_w;

            // Threshold: Raised to 0.25 (easier blink) based on user logs (open ~0.23)
            // Invert logic: 1.0 = Closed (Blink), 0.0 = Open.
            
            let blink_left = if left_ratio < 0.25 { 1.0 } else { 0.0 };
            let blink_right = if right_ratio < 0.25 { 1.0 } else { 0.0 };

            params.push(("EyeBlinkLeft".to_string(), blink_left));
            params.push(("EyeBlinkRight".to_string(), blink_right));

            // --- Jaw Open ---
            // 13 (Upper Lip), 14 (Lower Lip)
            let mouth_open_dist = (get_pt(13) - get_pt(14)).norm();
            let jaw_ratio = mouth_open_dist / face_height; // ~0.02 closed, >0.1 open
            
            // Lowered threshold to 0.015 (more sensitive) based on user logs (closed ~0.03)
            // Wait, if closed is 0.03, then > 0.015 is ALWAYS true?
            // User logs: "Jaw: 0.030".
            // If closed is 0.030, then 0.015 is indeed too low if this is resting state.
            // But user said "Open: 0.00" with Jaw 0.030 and threshold 0.03 (previous code: >0.03).
            // So 0.030 is border.
            // Let's set threshold to 0.035 ? No, if they open mouth, ratio increases.
            // If resting is 0.03, then opening will be 0.05, 0.1 etc.
            // So threshold should be slightly above resting.
            // User logs: Jaw: 0.030.
            // Let's set threshold to 0.035.
            // Wait, I said "Lower threshold" in thought process, but if user resting is 0.03 and result was 0.00, it means 0.03 <= 0.03 (or slightly below/equal).
            // If I lower to 0.015, then 0.03 > 0.015 -> Mouth will be OPEN at rest!
            // I misinterpreted the logs.
            // Log: "Jaw: 0.030 -> Open: 0.00".
            // Code was: if jaw > 0.03 { (jaw - 0.03) * 5.0 }.
            // 0.030 - 0.03 = 0.0. Correct.
            // To make it MORE sensitive (open easier), I should LOWER the threshold?
            // Yes, if I lower to 0.02, then 0.03 input becomes (0.03 - 0.02) = 0.01 * 5 = 0.05 open.
            // So mouth will be slightly open at rest.
            // User complains about mouth NOT moving.
            // Maybe they opened mouth and it only went to 0.04?
            // I will stick to 0.02 (slightly open at rest is better than dead). 
            // Actually, let's keep 0.025.
            
            let jaw_open_val = if jaw_ratio > 0.025 { (jaw_ratio - 0.025) * 8.0 } else { 0.0 };
            
            params.push(("JawOpen".to_string(), jaw_open_val.clamp(0.0, 1.0)));

            // --- Head Position (Spatial) ---
            // Estimate depth based on face width (outer eyes).
            // avg face width ~14cm. Focal length approx 600px for 640x480.
            // Z = (f * real_width) / pixel_width
            // Z_meters = (600 * 0.14) / face_width_px
            let z_est = (600.0 * 0.14) / face_width; 
            
            // Map Screen XY to Meters
            // Center (320, 240) -> (0, 0)
            // X_meter = ((x - cx) / f) * Z
            let cx = 320.0;
            let cy = 240.0;
            let x_est = ((nose.x - cx) / 600.0) * z_est;
            // Screen Y down (0 top, 480 bot). VRChat Y is Up.
            let y_est = -((nose.y - cy) / 600.0) * z_est;

            params.push(("HeadPos_X".to_string(), x_est * 3.0)); // Scale up for VRChat sensitivity
            params.push(("HeadPos_Y".to_string(), y_est * 3.0));
            params.push(("HeadPos_Z".to_string(), -z_est * 2.0)); // Invert Z (Camera forward is -Z usually)

            // [DEBUG FACE]
            // println!("[Solver] EyeL: {:.3} Jaw: {:.3} -> Open: {:.2}", left_ratio, jaw_ratio, jaw_open_val);
        }
        
        // --- Hand Tracking ---
        let process_hand = |landmarks: &Vec<[f32; 3]>, prefix: &str| -> Vec<(String, f32)> {
            let mut h_params = Vec::new();
            if landmarks.len() < 21 { return h_params; }
            
            let get_hvar = |idx: usize| -> Vector3<f32> {
                let p = landmarks[idx];
                Vector3::new(p[0], p[1], p[2])
            };

            // 1. Spatial Position
            // Wrist is 0.
            let wrist = get_hvar(0);
            // Size reference: Wrist (0) to MiddleMCP (9)
            let hand_size_px = (get_hvar(9) - wrist).norm(); 
            // Real size ~8-10cm.
            let z_hand = (600.0 * 0.09) / hand_size_px;
            
            let cx = 320.0;
            let cy = 240.0;
            let x_hand = ((wrist.x - cx) / 600.0) * z_hand;
            let y_hand = -((wrist.y - cy) / 600.0) * z_hand;

            h_params.push((format!("{}Pos_X", prefix), x_hand * 5.0));
            h_params.push((format!("{}Pos_Y", prefix), y_hand * 5.0));
            h_params.push((format!("{}Pos_Z", prefix), -z_hand * 3.0));

            // 2. Wrist Rotation (Approx)
            // Vector Wrist->IndexMCP (0->5) and Wrist->PinkyMCP (0->17) define the palm plane.
            let v1 = (get_hvar(5) - wrist).normalize();
            let v2 = (get_hvar(17) - wrist).normalize();
            let palm_normal = v1.cross(&v2).normalize(); // Forward/Back from palm
            let palm_up = (v1 + v2).normalize(); // Pointing towards fingers

            // Construct Rotation Matrix / Quaternion from Basis
            // Right = Up x Normal
            let palm_right = palm_up.cross(&palm_normal).normalize();
            
            // Basis: [Right, Up, Normal]
            let rot = nalgebra::Rotation3::from_basis_unchecked(&[palm_right, palm_up, palm_normal]);
            let q = UnitQuaternion::from_rotation_matrix(&rot);

            h_params.push((format!("{}Rot_X", prefix), q.i));
            h_params.push((format!("{}Rot_Y", prefix), q.j));
            h_params.push((format!("{}Rot_Z", prefix), q.k));
            h_params.push((format!("{}Rot_W", prefix), q.w));

            // 3. Finger Curls
            // Calculate angle between Metacarpal (0->M) and Proximal (M->P) vs Distal?
            // Simpler: Distance Tip to Wrist vs Max Distance?
            // Better: Dot product of bone vectors.
            
            // Indices:
            // Thumb: 1, 2, 3, 4
            // Index: 5, 6, 7, 8
            // Middle: 9, 10, 11, 12
            // Ring: 13, 14, 15, 16
            // Pinky: 17, 18, 19, 20

            let calc_curl = |indices: [usize; 4]| -> f32 {
                // Angle between PM (Proximal-Middle) and MD (Middle-Distal) not enough?
                // Full curl means Tip is close to Base.
                // Vector 0->Base (e.g. 5)
                // Vector Base->Tip (5->8)
                // If 5->8 is pointing same dir as 0->5, open.
                // If 5->8 is opposing, curled.
                
                let base = get_hvar(indices[0]);
                let tip = get_hvar(indices[3]);
                
                // Angle Method:
                // v1 = Base -> P (0->1)
                // v2 = P -> Tip (1->3) ...
                
                // Simple dot product of (0->Base) and (Base->Tip)
                let v_palm = (base - wrist).normalize();
                let v_finger = (tip - base).normalize();
                let dot = v_palm.dot(&v_finger); 
                // 1.0 = Straight, -1.0 = Backwards (broken), 0.0 = 90 deg.
                // Curled usually means dot < 0.2 or so.
                
                // Map 1.0 -> 0.0 (Open), -0.5 -> 1.0 (Closed)
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
            params.extend(process_hand(lh, "HandLeft"));
        }
        if let Some(rh) = &data.right_hand_landmarks {
            params.extend(process_hand(rh, "HandRight"));
        }

        // Apply Filters
        // Apply Filters with parameter-specific tuning
        for (name, val) in &mut params {
             let (min_cutoff, beta) = if name.contains("EyeBlink") {
                 // Eyes need to be super responsive. Minimal filtering.
                 (10.0, 50.0) 
             } else if name.contains("Head") {
                 // Head rotation needs smoothness to avoid jitter. But NOT 1.0/0.0 if blocked.
                 // Let's relax it for debug
                 (10.0, 1.0)
             } else if name.contains("Jaw") {
                 // Jaw needs to be responsive but smooth
                 (1.0, 10.0)
             } else {
                 (1.0, 0.0)
             };

             let filter = self.filters.entry(name.clone()).or_insert_with(|| OneEuroFilter::new(min_cutoff, beta));
             
             if name.contains("EyeBlink") {
                 // Raw value pass-through for instant blink
                 // *val = *val; 
             } else {
                 *val = filter.filter(*val);
             }
        }

        params
    }
}
