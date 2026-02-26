use nalgebra as na;

pub struct SpatialMapper {
    // Calibration parameters
    fov: f32,
    center_x: f32,
    center_y: f32,
}

impl SpatialMapper {
    pub fn new() -> Self {
        Self {
            fov: 60.0,
            center_x: 0.5,
            center_y: 0.5,
        }
    }

    /// Convert 2D image coordinates (0.0-1.0) + Depth to 3D VRChat Space
    pub fn project_3d(&self, x: f32, y: f32, depth: f32) -> na::Point3<f32> {
        // Simple pinhole camera model
        // VRChat Coordinate System: X=Right, Y=Up, Z=Forward (depending on avatar root)
        
        let centered_x = x - self.center_x;
        let centered_y = y - self.center_y; // Invert Y if needed based on UV
        
        // Scale by depth and FOV
        // tan(fov/2 * PI/180)
        let fov_scale = (self.fov.to_radians() / 2.0).tan();
        
        let x_3d = centered_x * depth * fov_scale;
        let y_3d = centered_y * depth * fov_scale;
        let z_3d = depth;

        na::Point3::new(x_3d, y_3d, z_3d)
    }

    /// Apply biomedical limits to rotation (prevent broken neck/arms)
    pub fn apply_constraints(&self, joint: &str, rotation: na::UnitQuaternion<f32>) -> na::UnitQuaternion<f32> {
        let (roll, pitch, yaw) = rotation.euler_angles();

        // nalgebra euler_angles returns (roll, pitch, yaw)
        // But for VRChat/Unity context, we usually map:
        // Pitch = X axis (Nodding)
        // Yaw = Y axis (Shaking head)
        // Roll = Z axis (Tilting head)

        let (mut r, mut p, mut y) = (roll, pitch, yaw);

        let (min_p, max_p, min_y, max_y, min_r, max_r) = match joint {
            "head" => (-60.0f32.to_radians(), 60.0f32.to_radians(), -85.0f32.to_radians(), 85.0f32.to_radians(), -40.0f32.to_radians(), 40.0f32.to_radians()),
            "neck" => (-50.0f32.to_radians(), 50.0f32.to_radians(), -60.0f32.to_radians(), 60.0f32.to_radians(), -30.0f32.to_radians(), 30.0f32.to_radians()),
            "spine" => (-30.0f32.to_radians(), 30.0f32.to_radians(), -30.0f32.to_radians(), 30.0f32.to_radians(), -20.0f32.to_radians(), 20.0f32.to_radians()),
            _ => return rotation,
        };

        p = p.clamp(min_p, max_p);
        y = y.clamp(min_y, max_y);
        r = r.clamp(min_r, max_r);

        na::UnitQuaternion::from_euler_angles(r, p, y)
    }

    /// Reset method for "Lost Tracking"
    pub fn interpolate_to_neutral(&self, current: na::UnitQuaternion<f32>, speed: f32) -> na::UnitQuaternion<f32> {
        // Spherically interpolate towards Identity (neutral rotation)
        current.slerp(&na::UnitQuaternion::identity(), speed)
    }
}
