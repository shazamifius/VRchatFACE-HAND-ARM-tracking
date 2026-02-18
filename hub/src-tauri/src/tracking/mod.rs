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
pub mod bridge;
pub mod solver;
pub mod filter;
pub mod pnp;
pub mod smoothing;
pub mod ik;
pub mod calibration;

#[cfg(test)]
mod tests;

use self::camera::CameraManager;
use self::ai::InferenceEngine;
use self::spatial::SpatialMapper;
use self::connectivity::ConnectivityManager;
use self::web_interface::WebInterface;
use self::types::{TrackingStatus, TrackingData, TrackingCommand}; // [modified]
use self::bridge::OscBridge; // [NEW]


pub struct TrackingEngine {
    pub camera: Arc<Mutex<CameraManager>>,
    ai: Arc<Mutex<InferenceEngine>>,
    spatial: Arc<Mutex<SpatialMapper>>,
    pub connectivity: Arc<Mutex<ConnectivityManager>>,
    pub web_interface: Arc<tokio::sync::Mutex<WebInterface>>,
    pub status: Arc<Mutex<TrackingStatus>>,
    pub data: Arc<Mutex<TrackingData>>, 
    pub commands: Arc<Mutex<Vec<TrackingCommand>>>, // [NEW] Command Queue
    running: Arc<Mutex<bool>>,
}

impl Default for TrackingEngine {
    fn default() -> Self { Self::new() }
}

impl TrackingEngine {
    pub fn new() -> Self {
        Self {
            camera: Arc::new(Mutex::new(CameraManager::new())),
            ai: Arc::new(Mutex::new(InferenceEngine::new())),
            spatial: Arc::new(Mutex::new(SpatialMapper::new())),
            connectivity: Arc::new(Mutex::new(ConnectivityManager::new().unwrap())),
            web_interface: Arc::new(tokio::sync::Mutex::new(WebInterface::new())),
            status: Arc::new(Mutex::new(TrackingStatus::default())),
            data: Arc::new(Mutex::new(TrackingData::default())),
            commands: Arc::new(Mutex::new(Vec::new())), // [NEW]
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn calibrate(&self, command: String) {
        if let Ok(mut cmds) = self.commands.lock() {
            cmds.push(TrackingCommand::Calibrate(command));
        }
    }

    pub fn set_quality(&self, quality: String) {
        if let Ok(mut cmds) = self.commands.lock() {
            cmds.push(TrackingCommand::SetQuality(quality));
        }
    }

    pub fn start(&self, config: crate::tracking::types::CameraConfig) -> Result<()> {
        let running = self.running.clone();
        let camera = self.camera.clone();
        let ai = self.ai.clone();
        let _spatial = self.spatial.clone();
        let web = self.web_interface.clone();
        let status = self.status.clone();
        let data = self.data.clone(); 
        let commands = self.commands.clone();

        let mut running_guard = running.lock().unwrap();
        if *running_guard {
            return Ok(());
        }
        *running_guard = true;
        drop(running_guard); // Release lock before long operations

        // Start OSC Bridge (Receiver)
        let bridge = OscBridge::new(9002).expect("Failed to bind OSC Bridge port 9002");
        let data_clone = data.clone();
        let status_clone = status.clone();
        
        let bridge_running = running.clone();
        bridge.start(bridge_running, Box::new(move |addr, args| {
            // Update TrackingData based on address
            // /tracking/face/landmarks
            // /tracking/hand/{i}/landmarks
            
            if addr == "/tracking/face/landmarks" {
                // Parse 468 points (x, y, z)
                if args.len() == 468 * 3 {
                    let mut points = Vec::with_capacity(468);
                    for chunk in args.chunks(3) {
                         points.push([chunk[0], chunk[1], chunk[2]]);
                    }
                    if let Ok(mut d) = data_clone.lock() {
                        d.face_landmarks = Some(points);
                    }
                    if let Ok(mut s) = status_clone.lock() {
                        s.face_detected = true;
                    }
                }
            } else if addr.starts_with("/tracking/hand/") {
                // Address: /tracking/hand/{i}/landmarks
                // We need to parse 'i' but for now, let's just look at the coordinates to assign Left/Right
                // Heuristic: Center X is 320 (640 width).
                // If X < 320 => User's Right => Avatar Left
                // If X > 320 => User's Left => Avatar Right
                // Note: Mirrors are confusing. Let's start with this.

                if args.len() == 21 * 3 {
                    let mut points = Vec::with_capacity(21);
                    for chunk in args.chunks(3) {
                         points.push([chunk[0], chunk[1], chunk[2]]);
                    }
                    
                    if let Some(wrist) = points.first() {
                         let x = wrist[0];
                         let is_right_hand = x < 320.0; // User Right (Mirror) = Screen Left
                         
                         if let Ok(mut d) = data_clone.lock() {
                             if is_right_hand {
                                 d.right_hand_landmarks = Some(points);
                             } else {
                                 d.left_hand_landmarks = Some(points);
                             }
                         }
                         if let Ok(mut s) = status_clone.lock() {
                             if is_right_hand { s.right_hand_detected = true; }
                             else { s.left_hand_detected = true; }
                         }
                    }
                }
            }
        }));

        // Start Web Server (Moved to lib.rs setup)
        // let web_clone = web.clone();
        // tokio::spawn(async move {
        //     let mut guard = web_clone.lock().await; // [FIX] Async lock
        //     if let Err(e) = guard.start(9001).await {
        //         println!("[Rust] Failed to start web server: {}", e);
        //     }
        // });

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
                 if encoder.encode_image(&preview).is_ok() {
                      web_encoder.lock().await.update_frame(buf);
                 }
            }
        });

        // [FIX] Clone data for the tracking thread (to avoid move error)
        let data_tracking = data.clone(); 
        
        // Capture specific value from config for the thread
        let use_remote_cam = config.index == 999;
        let config_clone = config.clone();

        thread::spawn(move || {
            // ... (model init omitted) ...
             if let Ok(mut engine) = ai.lock() {
                 let cwd = std::env::current_dir().unwrap_or_default();
                 let models_dir = if cwd.join("models").exists() { cwd.join("models") } else { cwd.join("../models") };
                 
                 // [REVERTED] Local Inference was disabled, but we need the loop for Phone Mode!
                 if let Err(e) = engine.load_models(models_dir.to_str().unwrap_or(".")) { 
                    println!("[Rust] Failed to load models: {}", e); 
                 } else {
                    println!("[Rust] Local Models Loaded Successfully.");
                 }
                 // println!("[Rust] Local Inference Disabled. Waiting for Python Bridge data...");
                 // return; // [FIX] Removed early exit so Phone Mode loop runs!
             }
             if let Ok(mut cam) = camera.lock() {
                 if !use_remote_cam {
                     if let Err(e) = cam.start(config_clone) { println!("[Rust] Failed to start camera: {}", e); return; }
                 }
             }

            let mut frame_count = 0;
            let mut last_fps_update = std::time::Instant::now();

            while *running.lock().unwrap() {
                // 1. Capture Frame
                let frame_result = if use_remote_cam {
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
                        let (face, left, right) = ai_guard.run_inference(&image_arc).unwrap_or((None, None, None));
                        
                        // BUG-08 FIX: Track what was actually detected this frame
                        let has_face = face.is_some();
                        let has_left = left.is_some();
                        let has_right = right.is_some();

                        // Write to Data (Solver will pick it up)
                        let mut data_for_emit = None;
                        if let Ok(mut d) = data_tracking.lock() {
                             d.face_landmarks = face;
                             d.left_hand_landmarks = left;
                             d.right_hand_landmarks = right;
                             
                             // Clone for emission
                             data_for_emit = Some(d.clone());
                        }

                        // BUG-07/08 FIX: Update detection status based on actual inference
                        if let Ok(mut s) = status.lock() {
                            s.face_detected = has_face;
                            s.left_hand_detected = has_left;
                            s.right_hand_detected = has_right;
                        }

                        // Emit to Frontend (Visualizer)
                        if let Some(data) = data_for_emit {
                            if let Ok(web_guard) = web.try_lock() {
                                web_guard.emit_tracking_data(&data);
                            }
                        }
                        
                        // Update FPS Counter
                        frame_count += 1;
                        if last_fps_update.elapsed().as_secs_f32() >= 1.0 {
                            let fps = frame_count as f32 / last_fps_update.elapsed().as_secs_f32();
                            if let Ok(mut s) = status.lock() {
                                s.fps = fps;
                                s.frame_time_ms = 1000.0 / fps;
                                s.running = true;
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

        // Logic Thread (60 FPS)
        let data_logic = data.clone();
        let connectivity_logic = self.connectivity.clone();
        let running_logic = self.running.clone();
        let commands_logic = commands.clone();
        
        use self::solver::Solver;
        
        thread::spawn(move || {
            let mut solver = Solver::new();
            println!("[Rust] Logic thread started");
            
            let mut last_log = std::time::Instant::now();
            while *running_logic.lock().unwrap() {
                let start = std::time::Instant::now();
                
                
                // 0. Process Commands
                if let Ok(mut cmds) = commands_logic.lock() {
                    for cmd in cmds.drain(..) {
                        match cmd {
                            TrackingCommand::Calibrate(mode) => {
                                println!("[Rust] Received Calibrate Command: {}", mode);
                                let stage = match mode.as_str() {
                                    "Neutral" => crate::tracking::calibration::CalibrationStage::NeutralFace,
                                    "TPose" => crate::tracking::calibration::CalibrationStage::TPose,
                                    "Center" => crate::tracking::calibration::CalibrationStage::CenterGaze,
                                    _ => {
                                       println!("[Rust] Unknown Calibration Mode: {}", mode);
                                       continue;
                                    }
                                };
                                solver.calibration.start(stage);
                            },
                            TrackingCommand::SetQuality(mode) => {
                                println!("[Rust] Set Quality: {}", mode);
                                solver.set_quality(&mode);
                            }
                        }
                    }
                }

                // 1. Get Data
                let tracking_data = {
                    let guard = data_logic.lock().unwrap();
                    guard.clone()
                };

                // 2. Solve
                let output = solver.solve(&tracking_data);

                // 3. Send to VRChat
                let has_data = !output.params.is_empty() || !output.trackers.is_empty();
                
                if has_data {
                    let conn = connectivity_logic.lock().unwrap();
                    
                    // Send Custom Params (Face + Hand Params)
                    if !output.params.is_empty() {
                        if let Err(e) = conn.send_tracking_data(output.params.clone()) {
                             eprintln!("[Rust] OSC Params Error: {}", e); 
                        }
                    }

                    // Send Standard Trackers (Head + Hands)
                    if !output.trackers.is_empty() {
                        if let Err(e) = conn.send_tracker_data(output.trackers.clone()) {
                             eprintln!("[Rust] OSC Trackers Error: {}", e);
                        }
                    }

                    // [DEBUG] Log success every second
                    if last_log.elapsed().as_millis() > 1000 {
                        println!("[Rust] Sent {} params and {} trackers", output.params.len(), output.trackers.len());
                        last_log = std::time::Instant::now();
                    }
                } else {
                     // [DEBUG] Warn if empty?
                     if last_log.elapsed().as_millis() > 1000 {
                         // println!("[Rust] Solver returned 0 params (No Face/Hand?)");
                         // last_log = std::time::Instant::now(); 
                     }
                }

                // 4. Sleep to maintain ~60 FPS
                let elapsed = start.elapsed();
                if elapsed < Duration::from_millis(16) {
                    thread::sleep(Duration::from_millis(16) - elapsed);
                }
            }
            println!("[Rust] Logic thread stopped");
        });

        Ok(())
    }

    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
        self.web_interface.blocking_lock().stop(); // [FIX] Blocking lock
    }
}
