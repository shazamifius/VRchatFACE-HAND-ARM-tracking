use nalgebra::{Vector2, Vector3, UnitQuaternion, Vector6, Matrix6};

/// Canonical 3D Face Model (in cm)
/// Coordinate System: Right-Handed, Face looking at camera
/// X+ = Screen Right (Face's Left), Y+ = Up, Z+ = Towards Camera
pub fn get_canonical_metric_landmarks() -> Vec<(usize, Vector3<f32>)> {
    vec![
        (1,   Vector3::new(0.0, 0.0, 0.0)),              // Nose Tip
        (33,  Vector3::new(-4.8, 3.0, -4.5)),            // Left Eye Outer (Screen Left)
        (263, Vector3::new(4.8, 3.0, -4.5)),             // Right Eye Outer (Screen Right)
        (152, Vector3::new(0.0, -9.75, -5.25)),          // Chin
        (61,  Vector3::new(-3.75, -3.75, -3.75)),        // Mouth Left
        (291, Vector3::new(3.75, -3.75, -3.75)),         // Mouth Right
        (10,  Vector3::new(0.0, 10.5, -6.0)),            // Top Head
    ]
}

/// Project 3D point to 2D screen coordinates
pub fn project(p_cam: Vector3<f32>, focal_length: f32, center: Vector2<f32>) -> Vector2<f32> {
    if p_cam.z.abs() < 0.001 { return center; }
    let u = (p_cam.x / p_cam.z) * focal_length + center.x;
    let v = (-p_cam.y / p_cam.z) * focal_length + center.y; // Y inverted: screen Y+ is Down
    Vector2::new(u, v)
}

/// Robust SolvePnP using Gauss-Newton Iteration
/// 
/// image_points: Map of Landmark Index -> (u, v) pixels
/// focal_length: in pixels
/// center: (cx, cy)
/// 
/// Returns: (Rotation Quaternion, Translation Vector)
pub fn solve_pnp(
    image_points: &[(usize, Vector2<f32>)], 
    last_rotation: Option<UnitQuaternion<f32>>,
    last_translation: Option<Vector3<f32>>,
    focal_length: f32,
    center: Vector2<f32>
) -> (UnitQuaternion<f32>, Vector3<f32>) {
    
    let model_points = get_canonical_metric_landmarks();
    
    // Initial Guess
    let mut q = last_rotation.unwrap_or(UnitQuaternion::identity());
    let mut t = last_translation.unwrap_or(Vector3::new(0.0, 0.0, 50.0)); // 50cm back
    
    // Filter to common points
    let mut correspondences = Vec::new();
    for (idx, p3d) in &model_points {
        // Find 2D point
        if let Some((_, p2d)) = image_points.iter().find(|(i, _)| i == idx) {
            correspondences.push((*p3d, *p2d));
        }
    }
    
    if correspondences.len() < 4 {
        return (q, t); // Not enough points
    }

    // Iterations (20 is enough for Gauss-Newton on a well-conditioned problem)
    let iterations = 20; 
    let mut lambda: f32 = 1.0; // Levenberg-Marquardt damping (adaptive)
    
    for _ in 0..iterations {
        // Build Jacobian Matrix J (2N x 6) and Residual Vector r (2N)
        // variables: [rx, ry, rz, tx, ty, tz] (Lie Algebra perturbation)
        
        let mut jt_j = Matrix6::zeros();
        let mut jt_r = Vector6::zeros();
        
        let rot_mat = q.to_rotation_matrix();
        
        let mut total_err = 0.0;
        
        for (p_obj, p_img) in &correspondences {
            let p_cam = rot_mat * p_obj + t;
            let p_proj = project(p_cam, focal_length, center);
            let residual = p_img - p_proj;
            total_err += residual.norm();
            
            // Derivatives
            // u = (X/Z)*f + cx
            // v = (-Y/Z)*f + cy
            
            // d(u)/d(X) = f/Z
            // d(u)/d(Z) = -X*f/Z^2
            // d(v)/d(Y) = -f/Z
            // d(v)/d(Z) = Y*f/Z^2
            
            let z_inv = 1.0 / p_cam.z;
            let z_sq_inv = z_inv * z_inv;
            let f = focal_length;
            
            /*
            let du_dx = f * z_inv;
            let du_dy = 0.0;
            let du_dz = -p_cam.x * f * z_sq_inv;
            
            let dv_dx = 0.0;
            let dv_dy = -f * z_inv;
            let dv_dz = p_cam.y * f * z_sq_inv;
            */
            
            // Jacobian of Geometric transform wrt Pose params [wx, wy, wz, tx, ty, tz]
            // d(P_cam)/d(theta) = [ -[P_cam]x | I ]
            // [P_cam]x is skew symmetric matrix
            // [  0  -z   y ]
            // [  z   0  -x ]
            // [ -y   x   0 ]
            
            // J_geo = [
            //   0  -z   y   1  0  0
            //   z   0  -x   0  1  0
            //  -y   x   0   0  0  1
            // ]
            
            // Chain rule: J = J_proj * J_geo
            // J_proj = [ 
            //   du/dx du/dy du/dz
            //   dv/dx dv/dy dv/dz
            // ]
            
            let x = p_cam.x;
            let y = p_cam.y;
            
            let row_u = Vector6::new(
                (f * x * y) * z_sq_inv,       // wx
                -f - (f * x * x) * z_sq_inv,  // wy
                (f * y) * z_inv,              // wz
                f * z_inv,                    // tx
                0.0,                          // ty
                -f * x * z_sq_inv             // tz
            );

            let row_v = Vector6::new(
                -f - (f * y * y) * z_sq_inv,  // wx
                (f * x * y) * z_sq_inv,       // wy
                x * f * z_inv,                // wz
                0.0,                          // tx
                -f * z_inv,                   // ty
                f * y * z_sq_inv              // tz
            );
            
            // Outer product manually
            for i in 0..6 {
                for j in 0..6 {
                    jt_j[(i, j)] += row_u[i] * row_u[j] + row_v[i] * row_v[j];
                }
                jt_r[i] += row_u[i] * residual.x + row_v[i] * residual.y;
            }
        }
        
        if total_err < 0.1 { break; } // Converged
        
        // Solve Linear System (JT_J * delta = JT_r)
        // Levenberg-Marquardt damping (adaptive)
        for i in 0..6 { jt_j[(i,i)] += lambda; }
        
        match jt_j.try_inverse() {
            Some(inv) => {
                let delta = inv * jt_r;
                
                // Update State
                // delta = [wx, wy, wz, tx, ty, tz]
                let mut w = Vector3::new(delta[0], delta[1], delta[2]);
                let mut v = Vector3::new(delta[3], delta[4], delta[5]);
                
                // Clamp step size to prevent divergence
                let w_norm = w.norm();
                if w_norm > 0.5 { w *= 0.5 / w_norm; } // max 0.5 rad per step
                let v_norm = v.norm();
                if v_norm > 20.0 { v *= 20.0 / v_norm; } // max 20cm per step
                
                // Update Rotation: q_new = exp(w) * q_old
                // nalgebra UnitQuaternion::new(w) creates exp(w/2) kind of?
                // UnitQuaternion::from_scaled_axis(w) -> represents rotation vector w.
                let dq = UnitQuaternion::from_scaled_axis(w);
                q = dq * q; 
                
                // Update Translation: t_new = t_old + v
                // Wait, if pertubation is local: P' = R(I+[w])P + (t+v)
                // If pertubation is left-multiplied: T_new = dP * T_old ?
                // Usually for translation it's additive if defined in global frame?
                // Or is v also local?
                // For simplicity, let's assume world-aligned update for t or cam-aligned?
                // The Jacobian above dProject/dT assumed T is simply added to P_cam.
                // So v is additive to t.
                t += v;
                
                // Reduce damping on successful step
                lambda *= 0.5;
                lambda = lambda.max(0.001);
            },
            None => {
                lambda *= 2.0; // Increase damping if singular
            }
        }
    }
    
    (q, t)
}
