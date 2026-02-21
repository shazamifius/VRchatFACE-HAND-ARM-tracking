use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType, ApiBackend, CameraFormat, FrameFormat, Resolution};
use nokhwa::Camera;
use anyhow::{Result, anyhow};
use image::{ImageBuffer, Rgb};
use crate::tracking::types::{CameraInfo, CameraConfig};
use std::collections::HashSet;

pub struct CameraManager {
    pub camera: Option<Camera>,
    pub current_config: Option<CameraConfig>,
}

impl CameraManager {
    pub fn new() -> Self {
        Self {
            camera: None,
            current_config: None,
        }
    }

    /// Find all available cameras using multiple backends
    pub fn get_cameras(&self) -> Vec<CameraInfo> {
        let mut final_list: Vec<CameraInfo> = Vec::new();
        let mut seen_paths = HashSet::new();
 
        // [FIX] Add Python Bridge FIRST so it is default
        final_list.push(CameraInfo {
            index: 999,
            name: "Python Bridge / Remote (RECOMMENDED)".to_string(),
            backend: Some("Virtual".to_string()),
            misc: None,
            resolution: None,
            fps: None,
            formats: None,
        });

        // 1. Define Backends to Query
        #[cfg(target_os = "windows")]
        let backends_to_try = vec![ApiBackend::MediaFoundation, ApiBackend::Auto];
        // Note: DirectShow might be needed for some ASUS cams.

        for backend in backends_to_try {
            if let Ok(cams) = nokhwa::query(backend) {
                println!("[Rust] Querying Backend: {:?} -> Found {}", backend, cams.len());
                for cam_info in cams {
                    // Deduplication logic
                    let unique_id = cam_info.misc().clone();
                    let name = cam_info.human_name();
                    
                    if !unique_id.is_empty() {
                         if seen_paths.contains(&unique_id) { continue; }
                         seen_paths.insert(unique_id.clone());
                    } else {
                        let composite_key = format!("{}_{}", name, cam_info.index());
                        if seen_paths.contains(&composite_key) { continue; }
                        seen_paths.insert(composite_key);
                    }

                    println!("[Rust] Found Camera: {} [{:?}] (Path: {})", name, backend, unique_id);

                    final_list.push(CameraInfo {
                        index: cam_info.index().as_index().unwrap_or(0) as i32,
                        name: name,
                        backend: Some(format!("{:?}", backend)),
                        misc: Some(unique_id),
                        resolution: None,
                        fps: None,
                        formats: None,
                    });
                }
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
        let fps = if config.fps > 0 { config.fps } else { 30 };
        let index = CameraIndex::Index(config.index);

        // Define a helper to convert string format to FrameFormat
        let requested_fmt = config.format.as_deref().and_then(|fmt_str| {
            match fmt_str {
                "MJPEG" => Some(FrameFormat::MJPEG),
                "YUYV" => Some(FrameFormat::YUYV),
                "NV12" => Some(FrameFormat::NV12),
                _ => None
            }
        });

        // ------------------------------------------------------------------
        // NEW LOGIC: If a specific format is requested (from Benchmark), USE IT.
        // ------------------------------------------------------------------
        if let Some(target_fmt) = requested_fmt {
            println!("[Rust] Strict Format Requested: {:?} {}x{} @ {}fps", target_fmt, width, height, fps);
            
            let format = CameraFormat::new(
                Resolution::new(width, height),
                target_fmt,
                fps
            );
            
            // Try EXACT request first
            let req = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(format));
            
            println!("[Rust] Attempting Strict Open...");
            match Camera::new(index.clone(), req) {
                Ok(mut cam) => {
                    if let Ok(_) = cam.open_stream() {
                        let final_fmt = cam.camera_format();
                        println!("[Rust] Strict Open Success! Format: {}x{} {:?} @ {}fps", 
                            final_fmt.resolution().width(), final_fmt.resolution().height(),
                            final_fmt.format(), final_fmt.frame_rate());
                            
                        self.camera = Some(cam);
                        self.current_config = Some(config);
                        return Ok(());
                    } else {
                        println!("[Rust] Strict Open Failed (Stream Init). Falling back to heuristics...");
                    }
                },
                Err(e) => {
                    println!("[Rust] Strict Open Failed (Camera Create): {}. Falling back to heuristics...", e);
                }
            }
        }

        // ------------------------------------------------------------------
        // FALLBACK / AUTO LOGIC (Original Heuristics)
        // ------------------------------------------------------------------
        println!("[Rust] Using Heuristic Camera Selection (MJPEG Priority)...");

        // Step 1: Enumeration to find best "theoretical" format
        // We prioritize MJPEG > YUYV > NV12 because NV12 might be slow to decode or buggy in some backends
        
        #[cfg(target_os = "windows")]
        let probe_backend = ApiBackend::MediaFoundation;
        #[cfg(not(target_os = "windows"))]
        let probe_backend = ApiBackend::Auto;

        let mut probe_cam = Camera::with_backend(index.clone(), RequestedFormat::new::<RgbFormat>(RequestedFormatType::None), probe_backend)
            .map_err(|e| anyhow!("Cannot open camera with backend {:?}: {}", probe_backend, e))?;

        let best_format = if let Ok(formats) = probe_cam.compatible_camera_formats() {
            println!("[Rust] Camera supports {} formats:", formats.len());
            for (i, f) in formats.iter().enumerate() {
                println!("[Rust]   [{}] {}x{} {:?} @ {}fps", 
                    i, f.resolution().width(), f.resolution().height(),
                    f.format(), f.frame_rate());
            }

            // Score: MJPEG > High FPS > Resolution
            let mut best: Option<CameraFormat> = None;
            let mut best_score: i64 = -1;
            
            for f in &formats {
                let fw = f.resolution().width();
                let fh = f.resolution().height();
                let ffps = f.frame_rate();
                let fmt = f.format();
                
                let res_dist = ((fw as i64 - width as i64).abs() + (fh as i64 - height as i64).abs()) as i64;
                let score_res = 10000 - res_dist.min(10000);
                let score_fps = (ffps as i64) * 100;
                let score_fmt = if fmt == FrameFormat::MJPEG { 10000 } else { 0 }; // MJPEG HUGE bonus
                
                let score = score_res + score_fps + score_fmt;
                
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
        drop(probe_cam);

        // Step 2: Open Camera with Retry Logic
        
        let try_open = |camera_idx: CameraIndex, req: RequestedFormat| -> Result<Camera> {
             #[cfg(target_os = "windows")]
             let backend = ApiBackend::MediaFoundation;
             #[cfg(not(target_os = "windows"))]
             let backend = ApiBackend::Auto;

             let mut cam = Camera::with_backend(camera_idx, req, backend)
                .map_err(|e| anyhow!("Camera create failed ({:?}): {}", backend, e))?;
             
             cam.open_stream()
                .map_err(|e| anyhow!("Stream open failed: {}", e))?;
             Ok(cam)
        };

        // Attempt 1: Closest to Best Enumerated (Likely MJPEG if available)
        let mut final_camera = if let Some(fmt) = best_format {
            println!("[Rust] Attempt 1: Closest to enumerated best: {}x{} {:?} @ {}fps", 
                fmt.resolution().width(), fmt.resolution().height(),
                fmt.format(), fmt.frame_rate());
            
            let req = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(fmt));
            match try_open(index.clone(), req) {
                Ok(cam) => Some(cam),
                Err(e) => {
                    println!("[Rust] Attempt 1 failed: {}", e);
                    None
                }
            }
        } else { None };

        // Check FPS
        let mut fps_ok = false;
        if let Some(ref cam) = final_camera {
            let f = cam.camera_format();
            if f.frame_rate() >= 15 { fps_ok = true; }
            else { println!("[Rust] Attempt 1 yielded low FPS: {}", f.frame_rate()); }
        }

        // Attempt 2: Try MJPEG Explicitly at 640x480
        if !fps_ok {
             if let Some(mut cam) = final_camera.take() {
                 let _ = cam.stop_stream(); 
             }
             
             println!("[Rust] Attempt 2: Forcing MJPEG 640x480 @ 30fps...");
             let attempt_fmt = CameraFormat::new(
                 Resolution::new(width, height),
                 FrameFormat::MJPEG,
                 30
             );
             let req = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(attempt_fmt));
             
             match try_open(index.clone(), req) {
                 Ok(cam) => {
                     let f = cam.camera_format();
                     if f.frame_rate() >= 15 { 
                         fps_ok = true; 
                         final_camera = Some(cam);
                         println!("[Rust] Attempt 2 Successful! MJPEG {}fps", f.frame_rate());
                     } else {
                         println!("[Rust] Attempt 2 yielded low FPS: {}", f.frame_rate());
                         final_camera = Some(cam); 
                     }
                 },
                 Err(e) => {
                     println!("[Rust] Attempt 2 (MJPEG) failed: {}", e);
                 }
             }
        }

        // Attempt 3: Lower Resolution (640x360) NV12 (Fallback)
        if !fps_ok {
             if let Some(mut cam) = final_camera.take() {
                 let _ = cam.stop_stream();
             }

             println!("[Rust] Attempt 3: Lower Resolution 640x360 NV12 @ 30fps...");
             let low_res = CameraFormat::new(
                 Resolution::new(640, 360),
                 FrameFormat::NV12,
                 30
             );
             let req = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(low_res));
             
             if let Ok(cam) = try_open(index.clone(), req) {
                  final_camera = Some(cam);
                  println!("[Rust] Attempt 3 Chosen. FPS: {}", final_camera.as_ref().unwrap().camera_format().frame_rate());
             }
        }

        let mut camera = if let Some(cam) = final_camera {
            cam
        } else {
             println!("[Rust] All attempts failed. Falling back to simple request.");
             let fallback_fmt = CameraFormat::new(
                 Resolution::new(width, height),
                 FrameFormat::MJPEG, // Try MJPEG first in fallback
                 30
             );
             try_open(index.clone(), RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(fallback_fmt)))
                 .or_else(|_| try_open(index, RequestedFormat::new::<RgbFormat>(RequestedFormatType::None)))?
        };

        // --- DISABLE AUTO EXPOSURE (Classic 1 FPS Fix) ---
        println!("[Rust] Attempting to disable Auto-Exposure for performance...");
        use nokhwa::utils::{KnownCameraControl, ControlValueSetter};
        if let Err(e) = camera.set_camera_control(KnownCameraControl::Exposure, ControlValueSetter::Boolean(false)) {
            println!("[Rust] Warning: Failed to set AutoExposure to False: {}", e);
        } else {
            println!("[Rust] AutoExposure set to False (Manual)");
        }
        
        // Backend specific exposure tuning
        let backend = camera.backend();
        println!("[Rust] Camera Backend Detect: {:?}", backend);
        
        let exposure_val = match backend {
            nokhwa::utils::ApiBackend::MediaFoundation => ControlValueSetter::Integer(-8), 
            _ => {
                // Heuristic: If we are on Windows and it's Auto, it's likely MF
                if cfg!(target_os = "windows") {
                    ControlValueSetter::Integer(-8)
                } else {
                    ControlValueSetter::Integer(2500)
                }
            }
        };

        let exposure_desc = format!("{:?}", exposure_val);
        if let Err(e) = camera.set_camera_control(KnownCameraControl::Exposure, exposure_val) {
             println!("[Rust] Note: Exposure manual tuning skipped/failed: {}", e);
        } else {
             println!("[Rust] Exposure tuned for performance ({}).", exposure_desc);
        }
        
        // Also try to set a reasonable exposure time if it's manual now?
        // Some cameras need a small exposure value to hit 30fps.
        // camera.set_camera_control(KnownCameraControl::Exposure, ControlValueSetter::Integer(5000)); 

        let final_fmt = camera.camera_format();
        println!("[Rust] Camera Started! Final format: {}x{} {:?} @ {}fps", 
            final_fmt.resolution().width(), final_fmt.resolution().height(),
            final_fmt.format(), final_fmt.frame_rate());
            
        if final_fmt.frame_rate() < 15 {
             println!("[Rust] WARNING: Still low FPS. Tracking WILL suffer.");
             println!("[Rust] TIP: Try another USB port or check Windows Privacy Settings.");
        }

        self.camera = Some(camera);
        self.current_config = Some(config);
        Ok(())
    }

    pub fn get_frame(&mut self) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
        if let Some(ref mut camera) = self.camera {
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

    pub fn test_all_cameras(&self) -> Vec<crate::tracking::types::CameraBenchmarkResult> {
        use crate::tracking::types::CameraBenchmarkResult;
        let cameras = self.get_cameras();
        let mut results = Vec::new();

        println!("[Rust] Starting Camera Benchmark ({} cameras)...", cameras.len());

        for cam_info in cameras {
            // Skip "virtual" or duplicate entries if needed, but for now test all physical ones.
            if cam_info.index == 999 { continue; } 

            let index = CameraIndex::Index(cam_info.index as u32);
            let mut result = CameraBenchmarkResult {
                index: cam_info.index,
                name: cam_info.name.clone(),
                format: "Unknown".to_string(),
                width: 0,
                height: 0,
                fps_expected: 30,
                fps_actual: 0.0,
                success: false,
                error: None,
            };

            println!("[Rust] Benchmarking Camera {}: {}", cam_info.index, cam_info.name);

            // Attempt: Open with high compatibility settings (Auto)
            // We want to see if we can get ANY stream.
            let req = RequestedFormat::new::<RgbFormat>(RequestedFormatType::None);
            
            match Camera::new(index.clone(), req) {
                Ok(mut cam) => {
                    match cam.open_stream() {
                        Ok(_) => {
                            let fmt = cam.camera_format();
                            result.format = format!("{:?}", fmt.format());
                            result.width = fmt.resolution().width();
                            result.height = fmt.resolution().height();
                            
                            // Measure FPS over 2 seconds
                            let start = std::time::Instant::now();
                            let mut frames = 0;
                            let mut success_read = false;
                            
                            loop {
                                if start.elapsed().as_secs_f32() > 2.0 { break; }
                                match cam.frame() {
                                    Ok(_) => {
                                        frames += 1;
                                        success_read = true;
                                    },
                                    Err(_) => {
                                        // std::thread::sleep(std::time::Duration::from_millis(5));
                                    }
                                }
                            }
                            
                            let fps = frames as f32 / start.elapsed().as_secs_f32();
                            result.fps_actual = fps;
                            result.success = fps > 1.0 && success_read; // At least SOME frames
                            
                            println!("[Rust]   Result: {:.1} FPS (Format: {} {}x{})", fps, result.format, result.width, result.height);
                            let _ = cam.stop_stream();
                        },
                        Err(e) => {
                            println!("[Rust]   Stream Open Failed: {}", e);
                            result.error = Some(e.to_string());
                        }
                    }
                },
                Err(e) => {
                    println!("[Rust]   Camera Access Failed: {}", e);
                    result.error = Some(e.to_string());
                }
            }
            results.push(result);
        }
        println!("[Rust] Benchmark Complete. Returning {} results.", results.len());
        results
    }
}
