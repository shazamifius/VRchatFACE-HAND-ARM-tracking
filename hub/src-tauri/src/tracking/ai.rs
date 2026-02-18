use anyhow::Result;
use image::{ImageBuffer, Rgb};
use crate::tracking::blaze::config::{get_face_short_range_config, get_palm_detection_config};
use crate::tracking::blaze::detector::{BlazeDetector};
use crate::tracking::blaze::landmarks::{BlazeLandmark};
use crate::tracking::blaze::utils::{detection2roi};

pub struct InferenceEngine {
    detector: Option<BlazeDetector>,
    landmark_model: Option<BlazeLandmark>,
    // Hand Models
    palm_detector: Option<BlazeDetector>,
    hand_landmark_model: Option<BlazeLandmark>,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            detector: None,
            landmark_model: None,
            palm_detector: None,
            hand_landmark_model: None,
        }
    }

    pub fn load_models(&mut self, models_dir: &str) -> Result<()> {
        let face_path = std::path::Path::new(models_dir).join("face_detection_short_range.onnx");
        let mesh_path = std::path::Path::new(models_dir).join("face_landmark.onnx");
        
        let palm_path = std::path::Path::new(models_dir).join("palm_detection.onnx");
        let hand_path = std::path::Path::new(models_dir).join("hand_landmark.onnx");

        // 1. Load Face Detector
        if face_path.exists() {
            let (anchor_options, config) = get_face_short_range_config();
            match BlazeDetector::new(face_path.to_str().unwrap(), config, anchor_options) {
                Ok(det) => {
                    self.detector = Some(det);
                    println!("[Rust] Face BlazeDetector loaded.");
                },
                Err(e) => println!("[Rust] Failed to load Face Detector: {}", e),
            }
        }

        // 2. Load Face Landmark
        if mesh_path.exists() {
             // Face Mesh V2: 192x192, 468 landmarks, 3 dims
             match BlazeLandmark::new(mesh_path.to_str().unwrap(), 192, 468, 3) {
                 Ok(lm) => {
                     self.landmark_model = Some(lm);
                     println!("[Rust] Face BlazeLandmark loaded.");
                 },
                 Err(e) => println!("[Rust] Failed to load Face Landmark: {}", e),
             }
        }

        // 3. Load Palm Detector
        if palm_path.exists() {
            let (anchor_options, config) = get_palm_detection_config();
            match BlazeDetector::new(palm_path.to_str().unwrap(), config, anchor_options) {
                Ok(det) => {
                    self.palm_detector = Some(det);
                    println!("[Rust] Palm BlazeDetector loaded.");
                },
                Err(e) => println!("[Rust] Failed to load Palm Detector: {}", e),
            }
        } else {
             println!("[Rust] Warning: Palm Detection model not found at {:?}", palm_path);
        }

        // 4. Load Hand Landmark
        if hand_path.exists() {
            // Hand Landmark: 224x224 (Standard), 21 landmarks, 3 dims
            match BlazeLandmark::new(hand_path.to_str().unwrap(), 224, 21, 3) {
                Ok(lm) => {
                    self.hand_landmark_model = Some(lm);
                    println!("[Rust] Hand BlazeLandmark loaded.");
                },
                Err(e) => println!("[Rust] Failed to load Hand Landmark: {}", e),
            }
        } else {
            println!("[Rust] Warning: Hand Landmark model not found at {:?}", hand_path);
        }

        Ok(())
    }

    /// Run full pipeline: Face + Hands
    /// Returns: (Face, LeftHand, RightHand)
    pub fn run_inference(
        &mut self, 
        image: &ImageBuffer<Rgb<u8>, Vec<u8>>
    ) -> Result<(Option<Vec<[f32; 3]>>, Option<Vec<[f32; 3]>>, Option<Vec<[f32; 3]>>)> {
        
        let dyn_img = image::DynamicImage::ImageRgb8(image.clone());

        // --- FACE ---
        let mut face_landmarks = None;
        if let Some(detector) = &mut self.detector {
            let detections = detector.detect(&dyn_img)?;
            if !detections.is_empty() {
                if let Some(landmark_model) = &mut self.landmark_model {
                    // Process biggest face
                    let detection = &detections[0]; 
                    let (_, config) = get_face_short_range_config();
                    let (xc, yc, scale, theta) = detection2roi(&detection, &config);
                    
                    if let Ok((landmarks, _score, _)) = landmark_model.predict(&dyn_img, xc, yc, scale, theta) {
                        let points: Vec<[f32; 3]> = landmarks.iter().map(|p| [p.0, p.1, p.2]).collect();
                        face_landmarks = Some(points);
                    }
                }
            }
        }

        // --- HANDS ---
        let mut left_hand = None;
        let mut right_hand = None;

        if let Some(palm_detector) = &mut self.palm_detector {
            let palms = palm_detector.detect(&dyn_img)?;
            
            // Process up to 2 hands
            // We need to associate landmarks to Left/Right
            if let Some(hand_model) = &mut self.hand_landmark_model {
                let (_, palm_config) = get_palm_detection_config();

                for (i, palm) in palms.iter().enumerate() {
                    if i >= 2 { break; } // Limit to 2 hands

                    let (xc, yc, scale, theta) = detection2roi(&palm, &palm_config);
                    
                    if let Ok((landmarks, score, handedness)) = hand_model.predict(&dyn_img, xc, yc, scale, theta) {
                        if score < 0.5 { continue; } // Threshold

                        let points: Vec<[f32; 3]> = landmarks.iter().map(|p| [p.0, p.1, p.2]).collect();
                        
                        // Handedness logic
                        // MediaPipe: 0.0 - 0.5 = Left? 0.5 - 1.0 = Right?
                        // "handedness" output (index 2) usually:
                        // < 0.5 => Left (Label: "Left")
                        // > 0.5 => Right (Label: "Right")
                        // Note: MediaPipe "Left" means "Left Hand" (User's left).
                        // Mirror mode might affect this? Assuming simple standard for now.
                        
                        let is_right = if let Some(h) = handedness {
                            h > 0.5
                        } else {
                            // If no handedness output, use position relative to image center
                            // X < 0.5 (Image Left) -> Right Hand (Mirror)? Or Left Hand?
                            // In selfie mode (mirror), Image Left is User Right.
                            // Let's assume User Perspective (non-mirror) for simple logic first?
                            // Or just assign first to Left, second to Right?
                            // Better to rely on X coord if handedness missing.
                            // If x < 0.5 -> Right Hand (User's Right is on Image Left in mirror).
                            xc < 0.5
                        };

                        if is_right {
                            if right_hand.is_none() { right_hand = Some(points); }
                        } else {
                            if left_hand.is_none() { left_hand = Some(points); }
                        }
                    }
                }
            }
        }

        Ok((face_landmarks, left_hand, right_hand))
    }
}
