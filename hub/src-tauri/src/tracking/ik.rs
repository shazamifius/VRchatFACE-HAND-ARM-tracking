use nalgebra::{Vector3, Rotation3};

pub struct ArmIK {
    upper_arm_len: f32,
    lower_arm_len: f32,
}

impl ArmIK {
    pub fn new(upper: f32, lower: f32) -> Self {
        Self {
            upper_arm_len: upper,
            lower_arm_len: lower,
        }
    }

    /// Solve 2-Bone IK
    /// Returns: (Elbow Position, Elbow Rotation, Shoulder Rotation) - Approximation
    /// Actually we mostly need Elbow Position for VRChat trackers if using Trackers.
    /// Or Bone Rotations if using Vun.
    /// Let's return just Elbow Position for now, as that's the key missing piece.
    pub fn solve(&self, shoulder: Vector3<f32>, hand: Vector3<f32>, pole_vector: Vector3<f32>) -> Vector3<f32> {
        let arm_dist = (hand - shoulder).norm();
        let max_len = self.upper_arm_len + self.lower_arm_len;
        
        // 1. Clamp Target distance (Prevent stretching)
        // If arm cant reach, we pull the hand back towards shoulder?
        // Or we just point arm at hand.
        // Usually we want to clamp the target "conceptually".
        let clamped_dist = arm_dist.min(max_len * 0.999); // 0.999 to avoid div by 0 issues at full stretch
        
        // Direction from Shoulder to Hand
        let dir = (hand - shoulder).normalize();
        
        // 2. Calculate Elbow Angle (Law of Cosines)
        // c^2 = a^2 + b^2 - 2ab cos(C)
        // dist^2 = upper^2 + lower^2 - 2*u*l * cos(elbow_angle_internal)
        // cos(angle) = (u^2 + l^2 - dist^2) / (2ul)
        
        let cos_angle = (self.upper_arm_len.powi(2) + self.lower_arm_len.powi(2) - clamped_dist.powi(2)) 
                        / (2.0 * self.upper_arm_len * self.lower_arm_len);
        
        // Clamp cos just in case
        let _angle_internal = cos_angle.clamp(-1.0, 1.0).acos();
        
        // 3. Solve Triangle relative to Shoulder-Hand axis
        // We need the angle at Shoulder (between Shoulder-Hand vector and Upper Arm)
        // a^2 = b^2 + c^2 - 2bc cos(A)
        // lower^2 = upper^2 + dist^2 - 2*u*dist * cos(shoulder_angle)
        let cos_shoulder = (self.upper_arm_len.powi(2) + clamped_dist.powi(2) - self.lower_arm_len.powi(2))
                           / (2.0 * self.upper_arm_len * clamped_dist);
        let angle_shoulder = cos_shoulder.clamp(-1.0, 1.0).acos();
        
        // 4. Calculate Plane Orientation
        // The elbow lies on a circle around the Shoulder-Hand axis.
        // The "Pole Vector" determines where on that circle.
        // Usually Pole is Down (-Y) or Back (-Z).
        
        // Project Pole onto the plane perpendicular to Dir
        // axis_u = (Pole - (Pole . Dir) * Dir).normalize()
        // But simpler: Cross product.
        // Normal to plane S-H-P:
        let plane_normal = (pole_vector.cross(&dir)).normalize();
        // The elbow is in the plane defined by Shoulder, Hand, Pole? No.
        // The elbow *lies* in the plane formed by Shoulder, Hand, Pole.
        // Then we rotate vector S->H by `angle_shoulder` inside that plane?
        
        // Actually:
        // Axis of rotation is Normal to (S, H, P).
        // If we rotate `dir` (S->H) around `plane_normal` by `angle_shoulder`, we get Upper Arm direction!
        
        let rot_mat = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(plane_normal), angle_shoulder);
        let upper_arm_dir = rot_mat * dir;
        
        // Elbow Pos = Shoulder + UpperArmDir * UpperLen
        let elbow = shoulder + upper_arm_dir * self.upper_arm_len;
        
        elbow
    }
}
