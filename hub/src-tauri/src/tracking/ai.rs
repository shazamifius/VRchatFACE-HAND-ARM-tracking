use anyhow::Result;
use image::{ImageBuffer, Rgb};
use crate::tracking::blaze::config::{get_face_short_range_config};
use crate::tracking::blaze::detector::{BlazeDetector};
use crate::tracking::blaze::landmarks::{BlazeLandmark};
use crate::tracking::blaze::utils::{detection2roi};

pub struct InferenceEngine {
    detector: Option<BlazeDetector>,
    landmark_model: Option<BlazeLandmark>,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            detector: None,
            landmark_model: None,
        }
    }

    pub fn load_models(&mut self, models_dir: &str) -> Result<()> {
        let face_path = std::path::Path::new(models_dir).join("face_detection_short_range.onnx");
        let mesh_path = std::path::Path::new(models_dir).join("face_landmark.onnx"); // [FIX] Use converted ONNX model

        // 1. Load Detector
        if face_path.exists() {
            let (anchor_options, config) = get_face_short_range_config();
            match BlazeDetector::new(face_path.to_str().unwrap(), config, anchor_options) {
                Ok(det) => {
                    self.detector = Some(det);
                    println!("[Rust] BlazeDetector loaded.");
                },
                Err(e) => println!("[Rust] Failed to load BlazeDetector: {}", e),
            }
        } else {
            println!("[Rust] Warning: Face Detection model not found at {:?}", face_path);
        }

        // 2. Load Landmark Model (Face Mesh)
        if mesh_path.exists() {
             // Face Mesh V2: 192x192, 468 landmarks, 3 dims
             match BlazeLandmark::new(mesh_path.to_str().unwrap(), 192, 468, 3) {
                 Ok(lm) => {
                     self.landmark_model = Some(lm);
                     println!("[Rust] BlazeLandmark (Face Mesh) loaded.");
                 },
                 Err(e) => println!("[Rust] Failed to load BlazeLandmark: {}", e),
             }
        } else {
            println!("[Rust] Warning: Face Mesh model not found at {:?}", mesh_path);
        }

        Ok(())
    }

    /// Run full pipeline: Scale -> Detect -> ROI -> Landmarks
    pub fn run_inference(&mut self, image: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<Option<Vec<[f32; 3]>>> {
        // Convert ImageBuffer to DynamicImage for our utils
         let dyn_img = image::DynamicImage::ImageRgb8(image.clone());

        // 1. Detection
        if let Some(detector) = &mut self.detector {
            let detections = detector.detect(&dyn_img)?;
            
            if !detections.is_empty() {
                // println!("[Rust] Face detected! Score: {:.2}", detections[0].score);
                
                // 2. Landmarks
                if let Some(landmark_model) = &mut self.landmark_model {
                    // Process valid detection (heuristic: biggest score is 0 usually)
                    let detection = &detections[0]; 

                    // Get Config
                    let (_, config) = get_face_short_range_config();
                    
                    let (xc, yc, scale, theta) = detection2roi(&detection, &config);
                    
                    match landmark_model.predict(&dyn_img, xc, yc, scale, theta) {
                        Ok((landmarks, _score)) => {
                            // Convert landmarks to [f32; 3]
                            // Landmarks are typically normalized 0..1 or pixel coords?
                            // BlazeLandmark returns PIXEL coordinates in the ROI, or original image?
                            // Checked `landmarks.rs`: it projects them back to original image size.
                            
                            // Return the points
                            let points: Vec<[f32; 3]> = landmarks.iter().map(|p| [p.0, p.1, p.2]).collect();
                            return Ok(Some(points));
                        },
                        Err(e) => println!("[Rust] Landmark inference failed: {}", e),
                    }
                }
            }
        }
        Ok(None)
    }
}
