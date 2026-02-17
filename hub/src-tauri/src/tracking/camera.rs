use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType, ApiBackend, CameraFormat};
use nokhwa::Camera;
use anyhow::{Result, anyhow};
use image::{ImageBuffer, Rgb};
use crate::tracking::types::{CameraInfo, CameraConfig};
use std::collections::HashSet;

pub struct CameraManager {
    camera: Option<Camera>,
    pub current_config: Option<CameraConfig>,
}

impl CameraManager {
    pub fn new() -> Self {
        Self {
            camera: None,
            current_config: None,
        }
    }

    /// Aggressively find all available cameras using multiple backends
    pub fn get_cameras(&self) -> Vec<CameraInfo> {
        let mut final_list: Vec<CameraInfo> = Vec::new();
        let mut seen_paths = HashSet::new();

        // 1. Define Backends to Query
        // Priority: Auto -> MsMF -> DShow -> V4L2 (Linux) -> UVC (Mac)
        // On Windows: DShow often finds things MsMF misses (older webcams).
        // MsMF is more modern but sometimes strict.
        let backends = vec![
            ApiBackend::Auto,
            ApiBackend::MediaFoundation,
            ApiBackend::Video4Linux, // Will be ignored on Windows usually
            // ApiBackend::DirectShow, // Only if input-dshow is enabled
        ];

        // We check if DirectShow is available in the current build
        // Nokhwa doesn't let us iterate variants easily without `strum`, so we try manually if supported.
        // If `input-dshow` feature is enabled, we should try it.
        // Ideally we'd add ApiBackend::DirectShow to the list, but let's assume Auto covers it or we add it explicitly.
        // We will try DShow explicitly if 'Auto' didn't yield everything, or just merge them.
        
        let backends_to_try = backends.clone();
        // DirectShow is not supported in this version of nokhwa without custom features
        // We rely on Auto/MsMF


        for backend in backends_to_try {
            if let Ok(cams) = nokhwa::query(backend) {
                println!("[Rust] Querying Backend: {:?} -> Found {}", backend, cams.len());
                for cam_info in cams {
                    // Deduplication logic
                    // Use miscalleneous string (device path) if available, otherwise name+index
                    let unique_id = cam_info.misc().clone();
                    let name = cam_info.human_name();
                    
                    // Skip if we've seen this exact device path before
                    if !unique_id.is_empty() {
                         if seen_paths.contains(&unique_id) {
                             continue;
                         }
                         seen_paths.insert(unique_id.clone());
                    } else {
                        // If no path, use name + index as a fallback key
                        let composite_key = format!("{}_{}", name, cam_info.index());
                        if seen_paths.contains(&composite_key) {
                            continue;
                        }
                        seen_paths.insert(composite_key);
                    }

                    // Log it
                    println!("[Rust] Found Camera: {} [{:?}] (Path: {})", name, backend, unique_id);

                    // Add to list
                    final_list.push(CameraInfo {
                        index: cam_info.index().as_index().unwrap_or(0) as i32,
                        name: name,
                        backend: Some(format!("{:?}", backend)),
                        misc: Some(unique_id),
                        resolution: None, // We could query capabilities here but it's slow
                        fps: None,
                        formats: None,
                    });
                }
            } else {
                // Backend might not be supported or failed
                // println!("[Rust] Backend {:?} not available.", backend);
            }
        }

        if final_list.is_empty() {
            println!("[Rust] No cameras found! Adding fallbacks.");
            final_list.push(CameraInfo {
                index: 0,
                name: "Fallback Camera 0".to_string(),
                backend: Some("Manual".to_string()),
                misc: None,
                resolution: None,
                fps: None,
                formats: None,
            });
        }

        final_list
    }

    pub fn start(&mut self, config: CameraConfig) -> Result<()> {
        println!("[Rust] Starting Camera with Config: {:?}", config);
        
        let width = if config.width > 0 { config.width } else { 640 };
        let height = if config.height > 0 { config.height } else { 480 };

        let index = CameraIndex::Index(config.index);

        // Step 1: Open a probe camera to enumerate supported formats
        let probe = RequestedFormat::new::<RgbFormat>(RequestedFormatType::None);
        let mut probe_cam = Camera::new(index.clone(), probe)
            .map_err(|e| anyhow!("Cannot open camera: {}", e))?;

        // Step 2: Find the best format from what the camera actually supports
        let best_format = if let Ok(formats) = probe_cam.compatible_camera_formats() {
            println!("[Rust] Camera supports {} formats:", formats.len());
            for (i, f) in formats.iter().enumerate().take(20) {
                println!("[Rust]   [{}] {}x{} {:?} @ {}fps", 
                    i, f.resolution().width(), f.resolution().height(),
                    f.format(), f.frame_rate());
            }
            if formats.len() > 20 {
                println!("[Rust]   ... and {} more", formats.len() - 20);
            }

            // Score each format: prioritize matching resolution, then highest fps
            let mut best: Option<CameraFormat> = None;
            let mut best_score: i64 = -1;
            
            for f in &formats {
                let fw = f.resolution().width();
                let fh = f.resolution().height();
                let ffps = f.frame_rate();
                
                // Resolution distance (lower is better, 0 = exact match)
                let res_dist = ((fw as i64 - width as i64).abs() + (fh as i64 - height as i64).abs()) as i64;
                
                // Score: heavily favor matching resolution, then favor high FPS
                let score = (10000 - res_dist.min(10000)) + ffps as i64;
                
                if score > best_score {
                    best_score = score;
                    best = Some(*f);
                }
            }
            best
        } else {
            println!("[Rust] Could not enumerate formats, using defaults");
            None
        };

        // Drop probe camera so we can re-open with correct format
        drop(probe_cam);

        // Step 3: Create a NEW camera requesting the exact best format
        let camera = if let Some(fmt) = best_format {
            println!("[Rust] Requesting exact format: {}x{} {:?} @ {}fps", 
                fmt.resolution().width(), fmt.resolution().height(),
                fmt.format(), fmt.frame_rate());
            
            let exact_request = RequestedFormat::new::<RgbFormat>(
                RequestedFormatType::Exact(fmt)
            );
            match Camera::new(index.clone(), exact_request) {
                Ok(cam) => cam,
                Err(e) => {
                    println!("[Rust] Exact format failed: {}. Falling back to None.", e);
                    Camera::new(index, RequestedFormat::new::<RgbFormat>(RequestedFormatType::None))
                        .map_err(|e2| anyhow!("Fallback also failed: {}", e2))?
                }
            }
        } else {
            Camera::new(index, RequestedFormat::new::<RgbFormat>(RequestedFormatType::None))
                .map_err(|e| anyhow!("Cannot open camera: {}", e))?
        };

        let mut camera = camera;
        camera.open_stream()
            .map_err(|e| anyhow!("Failed to open camera stream: {}", e))?;

        let final_fmt = camera.camera_format();
        println!("[Rust] Camera Started! Final format: {}x{} {:?} @ {}fps", 
            final_fmt.resolution().width(), final_fmt.resolution().height(),
            final_fmt.format(), final_fmt.frame_rate());

        self.camera = Some(camera);
        self.current_config = Some(config);
        Ok(())
    }

    pub fn get_frame(&mut self) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
        if let Some(ref mut camera) = self.camera {
            // Nokhwa handles format conversion to RGB automatically via RgbFormat generic
            let frame = camera.frame()?;
            let buffer = frame.decode_image::<RgbFormat>()?;
            Ok(buffer)
        } else {
            Err(anyhow!("Camera not started"))
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut camera) = self.camera.take() {
            let _ = camera.stop_stream();
            println!("[Rust] Camera Stopped");
        }
    }
}
