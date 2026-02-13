use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub mod camera;
pub mod ai;
pub mod spatial;
pub mod connectivity;
pub mod web_interface;
pub mod types;
pub mod blaze;

use self::camera::CameraManager;
use self::ai::InferenceEngine;
use self::spatial::SpatialMapper;
use self::connectivity::ConnectivityManager;
use self::web_interface::WebInterface;
use self::types::TrackingStatus;


pub struct TrackingEngine {
    pub camera: Arc<Mutex<CameraManager>>, // [FIX] Made public
    ai: Arc<Mutex<InferenceEngine>>,
    spatial: Arc<Mutex<SpatialMapper>>,
    pub connectivity: Arc<Mutex<ConnectivityManager>>,
    web_interface: Arc<tokio::sync::Mutex<WebInterface>>, // [FIX] Async Mutex
    pub status: Arc<Mutex<TrackingStatus>>,
    running: Arc<Mutex<bool>>,
}

impl TrackingEngine {
    pub fn new() -> Self {
        Self {
            camera: Arc::new(Mutex::new(CameraManager::new())),
            ai: Arc::new(Mutex::new(InferenceEngine::new())),
            spatial: Arc::new(Mutex::new(SpatialMapper::new())),
            connectivity: Arc::new(Mutex::new(ConnectivityManager::new().unwrap())),
            web_interface: Arc::new(tokio::sync::Mutex::new(WebInterface::new())), // [FIX] Async Mutex
            status: Arc::new(Mutex::new(TrackingStatus::default())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start(&self, camera_index: usize) -> Result<()> {
        let running = self.running.clone();
        let camera = self.camera.clone();
        let ai = self.ai.clone();
        let spatial = self.spatial.clone();
        let web = self.web_interface.clone();
        let status = self.status.clone(); // [FIX] Shadow self.status for closure

        *running.lock().unwrap() = true;

        // Start Web Server
        let web_clone = web.clone();
        tokio::spawn(async move {
            let mut guard = web_clone.lock().await; // [FIX] Async lock
            if let Err(e) = guard.start(9001).await {
                println!("[Rust] Failed to start web server: {}", e);
            }
        });

        // Channel for JPEG encoding (Drop frame if busy)
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Arc<image::RgbImage>>(1); // [FIX] Arc + Size 1
        
        // JPEG Encoding Thread
        let web_encoder = web.clone();
        tokio::spawn(async move {
            while let Some(img_arc) = rx.recv().await {
                 // Resize for preview (e.g. width 480) to speed up encoding
                 // Use nearest neighbor for max speed
                 let dyn_img = image::DynamicImage::ImageRgb8((*img_arc).clone()); 
                 let preview = dyn_img.resize(640, 480, image::imageops::FilterType::Nearest);

                 let mut buf = Vec::new();
                 let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 40); // [FIX] Quality 40
                 if let Ok(_) = encoder.encode_image(&preview) {
                      web_encoder.lock().await.update_frame(buf);
                 }
            }
        });

        thread::spawn(move || {
            // ... (model init omitted) ...
             if let Ok(mut engine) = ai.lock() {
                 let cwd = std::env::current_dir().unwrap_or_default();
                 let models_dir = if cwd.join("models").exists() { cwd.join("models") } else { cwd.join("../models") };
                 if let Err(e) = engine.load_models(models_dir.to_str().unwrap_or(".")) { println!("[Rust] Failed to load models: {}", e); }
             }
             if let Ok(mut cam) = camera.lock() {
                 if camera_index != 999 {
                     if let Err(e) = cam.start(camera_index) { println!("[Rust] Failed to start camera: {}", e); return; }
                 }
             }

            let mut frame_count = 0;
            let mut last_fps_update = std::time::Instant::now();

            while *running.lock().unwrap() {
                // 1. Capture Frame
                let frame_result = if camera_index == 999 {
                    // Maximum Remote FPS control?
                    // thread::sleep(Duration::from_millis(30)); // ~30 FPS max check
                    
                    // Remote Camera Mode
                    // 1. Get JPEG bytes from Web Interface
                    let jpeg_opt = web.blocking_lock().get_input_frame();
                    
                    match jpeg_opt {
                        Some(data) => {
                            // Phone connected!
                            let mut status_guard = status.lock().unwrap();
                            status_guard.phone_connected = true;
                            status_guard.latency_ms = 50; 
                            drop(status_guard);

                            // 2. Decode JPEG to ImageBuffer
                            match image::load_from_memory(&data) {
                                Ok(dyn_img) => Ok(dyn_img.to_rgb8()),
                                Err(e) => Err(anyhow::anyhow!("Failed to decode remove frame: {}", e)),
                            }
                        },
                        None => {
                            let mut status_guard = status.lock().unwrap();
                            status_guard.phone_connected = false;
                            status_guard.latency_ms = 0;
                            Err(anyhow::anyhow!("No remote frame"))
                        },
                    }
                } else {
                    // Local Camera Mode
                    let mut cam = camera.lock().unwrap();
                    cam.get_frame() // Blocking call typically
                };

                match frame_result {
                    Ok(image) => {
                         let image_arc = Arc::new(image);
                    
                         // Non-blocking send to encoder (Try Send)
                         // If full, we drop the preview frame, but tracking continues!
                         let _ = tx.try_send(image_arc.clone());

                         // 2. Inference
                        let mut ai_guard = ai.lock().unwrap();
                        let _result = ai_guard.run_inference(&image_arc);
                        
                        // 3. Spatial Mapping & Constraints
                        let _spatial_guard = spatial.lock().unwrap();

                        
                        // Update Status
                        frame_count += 1;
                        if last_fps_update.elapsed().as_secs_f32() >= 1.0 {
                            let fps = frame_count as f32 / last_fps_update.elapsed().as_secs_f32();
                            if let Ok(mut s) = status.lock() {
                                s.fps = fps;
                                s.frame_time_ms = 1000.0 / fps;
                                s.running = true;
                                s.face_detected = true; 
                            }
                            frame_count = 0;
                            last_fps_update = std::time::Instant::now();
                        }
                    },
                    Err(_) => {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
                
                // No artificial sleep here to maximize FPS
            }

            // Cleanup
            if let Ok(mut cam) = camera.lock() { cam.stop(); }
            println!("[Rust] Tracking thread stopped");
        });

        Ok(())
    }

    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
        self.web_interface.blocking_lock().stop(); // [FIX] Blocking lock
    }
}
