use serde::{Deserialize, Serialize};

/// Camera info returned to frontend
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CameraInfo {
    pub index: i32,
    pub name: String,
    pub backend: Option<String>,
    pub resolution: Option<(i32, i32)>,
    pub fps: Option<f32>,
}

/// Tracking status data sent to the frontend
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct TrackingStatus {
    pub face_detected: bool,
    pub face_confidence: Option<f32>,
    pub jaw_open: f32,
    pub left_hand_detected: bool,
    pub right_hand_detected: bool,
    pub pose_detected: bool,
    pub pose_confidence: Option<f32>,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub running: bool,
    pub phone_connected: bool, // [NEW]
    pub latency_ms: u64, // [NEW]
}

/// Raw tracking data from the AI engine (Python/Rust)
#[derive(Clone, Debug, Default)]
pub struct TrackingData {
    pub face_landmarks: Option<Vec<[f32; 3]>>, // 468 points (x, y, z)
    pub left_hand_landmarks: Option<Vec<[f32; 3]>>, // 21 points
    pub right_hand_landmarks: Option<Vec<[f32; 3]>>, // 21 points
    pub head_rotation: Option<[f32; 4]>, // Quaternion (x, y, z, w)
}
