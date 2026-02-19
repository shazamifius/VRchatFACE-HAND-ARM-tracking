use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CameraInfo {
    pub index: i32,
    pub name: String,
    pub backend: Option<String>,
    pub misc: Option<String>, // Device Path
    pub resolution: Option<(u32, u32)>,
    pub fps: Option<u32>,
    pub formats: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraConfig {
    pub index: u32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub format: Option<String>, // "MJPEG", "YUYV", "Auto"
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            index: 0,
            width: 640,
            height: 480,
            fps: 30,
            format: None,
        }
    }
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
    pub phone_connected: bool,
    pub latency_ms: u64,
    pub quality: String,
    // Profiling
    pub solver_ms: f32,
    pub osc_ms: f32,
    // Tracking health
    pub face_lost: bool,
    pub left_hand_lost: bool,
    pub right_hand_lost: bool,
    // Debug / Health
    pub model_loaded: bool,
    pub camera_name: String,
    pub camera_fps_real: f32, // Actual FPS from camera thread
}

/// Raw tracking data from the AI engine (Python/Rust)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrackingData {
    pub face_landmarks: Option<Vec<[f32; 3]>>, // 468 points (x, y, z)
    pub left_hand_landmarks: Option<Vec<[f32; 3]>>, // 21 points
    pub right_hand_landmarks: Option<Vec<[f32; 3]>>, // 21 points
    pub head_rotation: Option<[f32; 4]>, // Quaternion (x, y, z, w)
}

/// Commands sent from Frontend/Tauri to the Tracking Logic
#[derive(Clone, Debug)]
pub enum TrackingCommand {
    Calibrate(String), // "Neutral" or "TPose" or "Center"
    SetQuality(String), // "Low", "Medium", "High"
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CameraBenchmarkResult {
    pub index: i32,
    pub name: String,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub fps_expected: u32,
    pub fps_actual: f32,
    pub success: bool,
    pub error: Option<String>,
}
