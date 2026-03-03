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
             // Face Mesh V2 (Short Range/Lite): 128x128, 468 landmarks, 3 dims
             match BlazeLandmark::new(mesh_path.to_str().unwrap(), 128, 468, 3) {
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
    /// Returns: (Face, LeftHand, RightHand, MeanBrightness, Diagnostic)
    pub fn run_inference(
        &mut self, 
        image: &ImageBuffer<Rgb<u8>, Vec<u8>>
    ) -> Result<(Option<Vec<[f32; 3]>>, Option<Vec<[f32; 3]>>, Option<Vec<[f32; 3]>>, f32, Option<String>)> {
        
        let dyn_img = image::DynamicImage::ImageRgb8(image.clone());

        // --- BRIGHTNESS DIAGNOSTIC ---
        let raw = image.as_raw();
        let mut sum: u64 = 0;
        let step = 100; // Sample every 100th pixel for speed
        for i in (0..raw.len()).step_by(step) {
            sum += raw[i] as u64;
        }
        let count = raw.len() / step;
        let mean = if count > 0 { sum as f32 / count as f32 } else { 0.0 };
        
        let mut diagnostic = None;
        if mean < 15.0 {
            diagnostic = Some("Too Dark (Check Lighting)".to_string());
        } else if mean > 245.0 {
            diagnostic = Some("Too Bright (Overexposed)".to_string());
        }

        // Print stats occasionally
        if rand::random::<f32>() < 0.05 {
             println!("[AI] Input Stats: {}x{} mean_brightness={:.1} (0=black)", dyn_img.width(), dyn_img.height(), mean);
        }

        // --- FACE ---
        let mut face_landmarks = None;
        if let Some(detector) = &mut self.detector {
            let detections = detector.detect(&dyn_img)?;
            if !detections.is_empty() {
                if let Some(landmark_model) = &mut self.landmark_model {
                    let detection = &detections[0]; 
                    let (_, config) = get_face_short_range_config();
                    let (xc, yc, scale, theta) = detection2roi(&detection, &config);
                    
                    match landmark_model.predict(&dyn_img, xc, yc, scale, theta) {
                        Ok((landmarks, score, _)) => {
                            let points: Vec<[f32; 3]> = landmarks.iter().map(|p| [p.0, p.1, p.2]).collect();
                            if rand::random::<f32>() < 0.01 {
                                println!("[AI] Face OK: {} landmarks, score={:.4}", points.len(), score);
                            }
                            face_landmarks = Some(points);
                        },
                        Err(e) => {
                            diagnostic = Some(format!("Face Landmark Error: {}", e));
                        }
                    }
                }
            } else {
                // No face detected in this frame
                if diagnostic.is_none() {
                    diagnostic = Some("Face Not Detected".to_string());
                }
                if rand::random::<f32>() < 0.05 {
                    println!("[AI DEBUG] Face Detection empty vector returned from detector.detect()");
                }
            }
        } else if diagnostic.is_none() {
             diagnostic = Some("Model Not Loaded (Face)".to_string());
        }

        // --- HANDS ---
        let mut left_hand = None;
        let mut right_hand = None;

        if let Some(palm_detector) = &mut self.palm_detector {
            let palms_result = palm_detector.detect(&dyn_img);
            if let Ok(palms) = palms_result {
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
                            
                            let is_right = if let Some(h_val) = handedness {
                                h_val > 0.5
                            } else {
                                // If no handedness output, use position relative to image center
                                // X < 0.5 (Image Left) -> Right Hand (Mirror)? Or Left Hand?
                                // In selfie mode (mirror), Image Left is User Right.
                                // Let's assume User Perspective (non-mirror) for simple logic first?
                                // Or just assign first to Left, second to Right?
                                // Better to rely on X coord if handedness missing.
                                // If x < 0.5 -> Right Hand (User's Right is on Image Left in mirror).
                                xc < 0.5 * dyn_img.width() as f32 // Standard mirror heuristic relative to image width
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
        }

        Ok((face_landmarks, left_hand, right_hand, mean, diagnostic))
    }
}
