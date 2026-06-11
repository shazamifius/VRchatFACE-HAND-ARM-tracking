use anyhow::Result;
use image::{ImageBuffer, Rgb};
use crate::tracking::blaze::config::{get_face_short_range_config, get_palm_detection_config};
use crate::tracking::blaze::detector::{BlazeDetector};
use crate::tracking::blaze::landmarks::{BlazeLandmark};
use crate::tracking::blaze::utils::{detection2roi};

/// Re-run the (expensive) DETECTOR models only every N frames, or whenever
/// tracking is lost. In between we run only the landmark models on the ROI we
/// already know — this is the standard MediaPipe "detect-then-track" pattern and
/// roughly halves per-frame cost (4 models -> 2 on tracking frames).
const REDETECT_INTERVAL: u64 = 5;

pub struct InferenceEngine {
    detector: Option<BlazeDetector>,
    landmark_model: Option<BlazeLandmark>,
    // Hand Models
    palm_detector: Option<BlazeDetector>,
    hand_landmark_model: Option<BlazeLandmark>,
    // Body pose (BlazePose 33-pt). v1 runs the landmark model on a centered
    // full-frame square and re-anchors the ROI from the landmarks each frame
    // (same detect-then-track feedback the face uses), so we don't yet need the
    // person detector. Phase 1.5 adds pose_detection.onnx for robustness.
    pose_landmark_model: Option<BlazeLandmark>,
    // detect-then-track state
    frame_count: u64,
    face_roi: Option<(f32, f32, f32, f32)>,   // (xc, yc, scale, theta)
    hand_rois: Vec<(f32, f32, f32, f32)>,      // up to 2, from last palm detection
    pose_roi: Option<(f32, f32, f32, f32)>,    // (xc, yc, scale, theta) from last pose
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            detector: None,
            landmark_model: None,
            palm_detector: None,
            hand_landmark_model: None,
            pose_landmark_model: None,
            frame_count: 0,
            face_roi: None,
            hand_rois: Vec::new(),
            pose_roi: None,
        }
    }

    /// Derive the next-frame pose ROI from the current body landmarks so the crop
    /// follows the person between frames. Mirrors `face_roi_from_landmarks` but
    /// with a larger margin (the body crop must comfortably contain limbs).
    fn pose_roi_from_landmarks(landmarks: &[(f32, f32, f32)], prev: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        if landmarks.is_empty() { return prev; }
        let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (x, y, _) in landmarks {
            minx = minx.min(*x); maxx = maxx.max(*x);
            miny = miny.min(*y); maxy = maxy.max(*y);
        }
        let xc = (minx + maxx) * 0.5;
        let yc = (miny + maxy) * 0.5;
        let size = (maxx - minx).max(maxy - miny) * 1.6; // generous margin for limbs
        (xc, yc, size, 0.0)
    }

    /// Derive the next-frame face ROI from the current mesh landmarks so the crop
    /// follows the head between detections. Falls back to the previous ROI if the
    /// landmark set is empty. The periodic re-detect re-anchors any drift.
    fn face_roi_from_landmarks(landmarks: &[(f32, f32, f32)], prev: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        if landmarks.is_empty() { return prev; }
        let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (x, y, _) in landmarks {
            minx = minx.min(*x); maxx = maxx.max(*x);
            miny = miny.min(*y); maxy = maxy.max(*y);
        }
        let xc = (minx + maxx) * 0.5;
        let yc = (miny + maxy) * 0.5;
        let size = (maxx - minx).max(maxy - miny) * 1.5; // margin ~ matches detector dscale
        (xc, yc, size, 0.0)
    }

    pub fn load_models(&mut self, models_dir: &str) -> Result<()> {
        let face_path = std::path::Path::new(models_dir).join("face_detection_short_range.onnx");
        let mesh_path = std::path::Path::new(models_dir).join("face_landmark.onnx");
        
        let palm_path = std::path::Path::new(models_dir).join("palm_detection.onnx");
        let hand_path = std::path::Path::new(models_dir).join("hand_landmark.onnx");

        let pose_path = std::path::Path::new(models_dir).join("pose_landmark.onnx");

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

        // 5. Load Body Pose Landmark (BlazePose). Output 0 is [1,195] = 39 pts x 5
        // (x,y,z,visibility,presence); we parse 39 and keep the first 33 (the body).
        // Output 1 [1,1] is the pose-presence flag (used as the score). Input is
        // 256x256 NHWC; BlazeLandmark auto-detects layout + size from the model.
        if pose_path.exists() {
            match BlazeLandmark::new(pose_path.to_str().unwrap(), 256, 39, 5) {
                Ok(lm) => {
                    self.pose_landmark_model = Some(lm);
                    println!("[Rust] Body Pose BlazeLandmark loaded.");
                },
                Err(e) => println!("[Rust] Failed to load Pose Landmark: {}", e),
            }
        } else {
            println!("[Rust] Warning: Pose Landmark model not found at {:?}", pose_path);
        }

        Ok(())
    }

    /// Run full pipeline: Face + Hands + Body Pose
    /// Returns: (Face, LeftHand, RightHand, BodyPose(33), MeanBrightness, Diagnostic)
    pub fn run_inference(
        &mut self,
        image: &ImageBuffer<Rgb<u8>, Vec<u8>>
    ) -> Result<(Option<Vec<[f32; 3]>>, Option<Vec<[f32; 3]>>, Option<Vec<[f32; 3]>>, Option<Vec<[f32; 3]>>, f32, Option<String>)> {
        
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

        // --- FACE (detect-then-track) ---
        let mut face_landmarks = None;

        // Step 1: (re)detect the face ROI only when lost or on the refresh tick.
        let need_face_detect = self.face_roi.is_none() || self.frame_count % REDETECT_INTERVAL == 0;
        if need_face_detect {
            if let Some(detector) = &mut self.detector {
                match detector.detect(&dyn_img) {
                    Ok(dets) if !dets.is_empty() => {
                        let (_, config) = get_face_short_range_config();
                        self.face_roi = Some(detection2roi(&dets[0], &config));
                    }
                    Ok(_) => {
                        self.face_roi = None;
                        if diagnostic.is_none() { diagnostic = Some("Face Not Detected".to_string()); }
                    }
                    Err(e) => {
                        self.face_roi = None;
                        return Err(e);
                    }
                }
            } else if diagnostic.is_none() {
                diagnostic = Some("Model Not Loaded (Face)".to_string());
            }
        }

        // Step 2: run the mesh on the known ROI (whether just-detected or tracked).
        if let Some((xc, yc, scale, theta)) = self.face_roi {
            if let Some(landmark_model) = &mut self.landmark_model {
                match landmark_model.predict(&dyn_img, xc, yc, scale, theta) {
                    Ok((landmarks, _score, _)) => {
                        // NOTE: this face_landmark model's flag output reads ~0.27 even
                        // for a clearly-tracked face, so we must NOT threshold on it.
                        // (An earlier `score < 0.3` gate silently dropped EVERY face,
                        // leaving the avatar neutral while the detector saw 0.99 faces.)
                        // Lost-tracking is handled by the periodic re-detect, which
                        // clears face_roi when the detector returns no faces.
                        let points: Vec<[f32; 3]> = landmarks.iter().map(|p| [p.0, p.1, p.2]).collect();
                        // Re-detect the face fresh every frame (clear the ROI). The
                        // landmark-derived ROI feedback is disabled for the face: it
                        // added drift risk for little gain now that detection is cheap
                        // and correct. (Hands still use cached ROIs.)
                        self.face_roi = None;
                        face_landmarks = Some(points);
                    }
                    Err(e) => {
                        diagnostic = Some(format!("Face Landmark Error: {}", e));
                        self.face_roi = None;
                    }
                }
            }
        }

        // --- HANDS (detect-then-track) ---
        let mut left_hand = None;
        let mut right_hand = None;

        // Step 1: (re)detect palm ROIs only when none are cached or on the refresh
        // tick. Between detections we reuse the cached ROIs; the periodic re-detect
        // re-anchors them. (Hands move fast, so REDETECT_INTERVAL keeps it honest.)
        let need_hand_detect = self.hand_rois.is_empty() || self.frame_count % REDETECT_INTERVAL == 0;
        if need_hand_detect {
            if let Some(palm_detector) = &mut self.palm_detector {
                if let Ok(palms) = palm_detector.detect(&dyn_img) {
                    let (_, palm_config) = get_palm_detection_config();
                    self.hand_rois = palms.iter().take(2).map(|p| detection2roi(p, &palm_config)).collect();
                }
            }
        }

        // Step 2: run the hand landmark model on each known ROI.
        if let Some(hand_model) = &mut self.hand_landmark_model {
            let rois = self.hand_rois.clone();
            for (xc, yc, scale, theta) in rois {
                if let Ok((landmarks, score, handedness)) = hand_model.predict(&dyn_img, xc, yc, scale, theta) {
                    if score < 0.5 { continue; } // Threshold

                    let points: Vec<[f32; 3]> = landmarks.iter().map(|p| [p.0, p.1, p.2]).collect();

                    // Handedness: model output if present, else mirror heuristic on X.
                    let is_right = if let Some(h_val) = handedness {
                        h_val > 0.5
                    } else {
                        xc < 0.5 * dyn_img.width() as f32
                    };

                    if is_right {
                        if right_hand.is_none() { right_hand = Some(points); }
                    } else {
                        if left_hand.is_none() { left_hand = Some(points); }
                    }
                }
            }
        }

        // --- BODY POSE (BlazePose 33-pt, detect-then-track via landmark feedback) ---
        // v1: no person detector. On the refresh tick (or when lost) we re-anchor
        // the crop to the whole frame (a square covering the larger dimension);
        // between ticks we reuse the ROI derived from the previous frame's body
        // landmarks. Same pattern as the face path. The pose-presence flag is NOT
        // thresholded yet (its scale is model-specific — gating blindly is exactly
        // what once dropped every face), so we accept any non-empty result and will
        // calibrate a real gate from observed scores.
        let mut pose_landmarks = None;
        if let Some(pose_model) = &mut self.pose_landmark_model {
            let (w, h) = (dyn_img.width() as f32, dyn_img.height() as f32);
            let need_pose_redetect = self.pose_roi.is_none() || self.frame_count % REDETECT_INTERVAL == 0;
            let (xc, yc, scale, theta) = if need_pose_redetect {
                (w * 0.5, h * 0.5, w.max(h), 0.0)
            } else {
                self.pose_roi.unwrap()
            };
            match pose_model.predict(&dyn_img, xc, yc, scale, theta) {
                Ok((landmarks, _score, _)) if !landmarks.is_empty() => {
                    // Keep the first 33 (body); points 34-39 are auxiliary refinement.
                    let body: Vec<(f32, f32, f32)> = landmarks.into_iter().take(33).collect();
                    self.pose_roi = Some(Self::pose_roi_from_landmarks(&body, (xc, yc, scale, theta)));
                    pose_landmarks = Some(body.iter().map(|p| [p.0, p.1, p.2]).collect());
                }
                _ => { self.pose_roi = None; }
            }
        }

        self.frame_count = self.frame_count.wrapping_add(1);
        Ok((face_landmarks, left_hand, right_hand, pose_landmarks, mean, diagnostic))
    }
}
