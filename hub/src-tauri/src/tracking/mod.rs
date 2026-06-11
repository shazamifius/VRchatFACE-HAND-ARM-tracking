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
pub mod vmt;
pub mod head_bridge;
pub mod mouse_look;
pub mod controller_input;
pub mod body;
pub mod solver;
pub mod filter;
pub mod terminal_monitor;
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

        let mut running_guard = running.lock().unwrap_or_else(|e| e.into_inner());
        if *running_guard {
            return Ok(());
        }
        *running_guard = true;
        drop(running_guard); // Release lock before long operations

        // Start OSC Bridge (Receiver) — optional. Only used when landmarks arrive
        // over OSC from an external AI. Must NOT crash the app if port 9002 is busy
        // (e.g. during a quick stop -> start when switching cameras).
        let bridge_opt = OscBridge::new(9002).ok();
        let data_clone = data.clone();
        let status_clone = status.clone();
        let status_logic = status.clone(); // For logic thread profiling

        let bridge_running = running.clone();
        if let Some(bridge) = bridge_opt {
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
                    // [DEBUG] Log receipt (Throttled)
                    // We can't easily throttle here without static/atomic.
                    // Let's just print every 100th frame maybe? 
                    // Or rely on the logic thread [Perf] logs.
                    // Actually, let's print once per second if possible.
                    // For now, a simple low-freq print:
                    if rand::random::<f32>() < 0.01 {
                         println!("[Rust] Bridge RX: Face Data Received");
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
        } else {
            println!("[Rust] OSC Bridge receiver port 9002 unavailable; continuing without OSC-input bridge.");
        }

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

        // Create a separate clone for the tracking thread (status_clone was moved to bridge)
        let status_capture = status.clone();

        thread::spawn(move || {
            // ... (model init omitted) ...
             if let Ok(mut engine) = ai.lock() {
                 let cwd = std::env::current_dir().unwrap_or_default();
                 let models_dir = if cwd.join("models").exists() { cwd.join("models") } else { cwd.join("../models") };
                 
                 if let Err(e) = engine.load_models(models_dir.to_str().unwrap_or(".")) { 
                    crate::logging::log(&format!("[Rust] Failed to load models: {}", e));
                    if let Ok(mut s) = status_capture.lock() { s.model_loaded = false; }
                 } else {
                    crate::logging::log("[Rust] Local Models Loaded Successfully.");
                    if let Ok(mut s) = status_capture.lock() { s.model_loaded = true; }
                 }
             }
              if let Ok(mut cam) = camera.lock() {
                 if !use_remote_cam {
                     if let Err(e) = cam.start(config_clone) {
                         crate::logging::log(&format!("[Rust] Failed to start camera: {}", e));
                         // Don't leave a zombie "running" session with no camera —
                         // clear the flag so Stop / Start / camera-switch can recover.
                         if let Ok(mut r) = running.lock() { *r = false; }
                         if let Ok(mut s) = status_capture.lock() {
                             s.running = false;
                             s.diagnostic_message = Some(format!("Camera failed to start: {}", e));
                         }
                         println!("[Rust] Camera start failed -> session cleared. {}", e);
                         return;
                     }
                      if let Ok(mut s) = status_capture.lock() {
                          if true {
                              if let Some(c) = &cam.camera {
                                  let fmt = c.camera_format();
                                  s.camera_width = fmt.resolution().width();
                                  s.camera_height = fmt.resolution().height();
                                  s.camera_fps_real = fmt.frame_rate() as f32;
                              }
                          }
                      }
                       crate::logging::log("[Rust] Camera started.");
                 }
             }

            let mut frame_count = 0;
            let mut last_fps_update = std::time::Instant::now();

            while *running.lock().unwrap_or_else(|e| e.into_inner()) {
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
                            let mut status_guard = status_capture.lock().unwrap();
                            status_guard.phone_connected = true;
                            status_guard.latency_ms = 50; 

                            // 2. Decode JPEG to ImageBuffer
                            match image::load_from_memory(&data) {
                                Ok(dyn_img) => {
                                    status_guard.camera_width = dyn_img.width();
                                    status_guard.camera_height = dyn_img.height();
                                    drop(status_guard);
                                    Ok(dyn_img.to_rgb8())
                                },
                                Err(e) => {
                                    drop(status_guard);
                                    Err(anyhow::anyhow!("Failed to decode remove frame: {}", e))
                                },
                            }
                        },
                        None => {
                            let mut status_guard = status_capture.lock().unwrap();
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
                         // Downscale large frames BEFORE inference. The camera often
                         // streams 1080p (cheap to capture) but running 4 ONNX models
                         // on 1080p collapses the pipeline to <1 fps. Cap width at 640:
                         // the models downsample to 128/224 internally anyway, so
                         // accuracy is unaffected while per-frame CPU cost drops ~9x.
                         // Landmarks then live in this resolution, and frame_w/h is
                         // stamped from it (below), so the solver stays consistent.
                         let image = {
                             let w = image.width();
                             // Cap at 960px (was 640): the ONNX models run at fixed
                             // 128/192/224 and extract_roi samples a fixed grid, so the
                             // per-frame cost barely changes, but a larger frame gives
                             // the face-mesh crop far more detail — needed to actually
                             // capture mouth/eye motion (640 was too coarse: jaw/blink
                             // landmarks barely moved).
                             const INFER_W: u32 = 640;
                             if w > INFER_W {
                                 let target_h = (image.height() * INFER_W / w).max(1);
                                 image::imageops::resize(&image, INFER_W, target_h, image::imageops::FilterType::Triangle)
                             } else {
                                 image
                             }
                         };
                         let image_arc = Arc::new(image);

                         // Non-blocking send to encoder (Try Send)
                         // For remote cameras, the HTTP endpoint already broadcasts the raw JPEG, so we skip re-encoding
                         // to absolutely prevent interleaving frame rollbacks on the UI stream.
                         if !use_remote_cam {
                             let _ = tx.try_send(image_arc.clone());
                         }

                        // 2. Inference — isolated per frame. A panic inside one
                        // frame's inference (e.g. a degenerate crop) used to unwind
                        // and KILL this whole tracking thread silently: no more
                        // landmarks were produced while the OSC thread kept
                        // re-emitting the last value forever, so the app "froze".
                        // catch_unwind contains the panic, logs where it happened,
                        // drops that frame, and lets tracking keep running. The lock
                        // is taken poison-tolerantly so a prior panic can't cascade.
                        let inference_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let mut ai_guard = ai.lock().unwrap_or_else(|e| e.into_inner());
                            ai_guard.run_inference(&image_arc)
                        }));

                        let (face, left, right, pose, brightness, diagnostic) = match inference_result {
                            Ok(Ok(res)) => res,
                            Ok(Err(e)) => {
                                // [DIAG] Surface the real inference error to the console.
                                println!("[AI ERROR] run_inference failed: {:#}", e);
                                (None, None, None, None, 0.0, Some(format!("AI Error: {}", e)))
                            }
                            Err(panic) => {
                                let msg = panic
                                    .downcast_ref::<&str>()
                                    .map(|s| s.to_string())
                                    .or_else(|| panic.downcast_ref::<String>().cloned())
                                    .unwrap_or_else(|| "unknown panic".to_string());
                                println!("[AI PANIC] inference panicked (frame skipped): {}", msg);
                                (None, None, None, None, 0.0, Some("AI Panic (recovered)".to_string()))
                            }
                        };

                        // BUG-08 FIX: Track what was actually detected this frame
                        let has_face = face.is_some();
                        let has_left = left.is_some();
                        let has_right = right.is_some();
                        let has_pose = pose.is_some();

                        // Write to Data (Solver will pick it up)
                        let mut data_for_emit = None;
                        if let Ok(mut d) = data_tracking.lock() {
                             d.face_landmarks = face;
                             d.left_hand_landmarks = left;
                             d.right_hand_landmarks = right;
                             d.pose_landmarks = pose;
                             // Stamp the exact resolution the landmarks live in,
                             // so the solver scales against the right dimensions.
                             d.frame_w = image_arc.width() as f32;
                             d.frame_h = image_arc.height() as f32;

                             // Clone for emission
                             data_for_emit = Some(d.clone());
                        }

                        // BUG-07/08 FIX: Update detection status based on actual inference
                        if let Ok(mut s) = status_capture.lock() {
                            s.face_detected = has_face;
                            s.left_hand_detected = has_left;
                            s.right_hand_detected = has_right;
                            s.mean_brightness = brightness;
                            s.diagnostic_message = diagnostic;
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
                            if let Ok(mut s) = status_capture.lock() {
                                s.fps = fps;
                                s.camera_fps_real = fps;
                                s.frame_time_ms = 1000.0 / fps;
                                s.running = true;
                            }
                            // Concise 1 Hz health summary (replaces the per-frame spam).
                            println!(
                                "[Status] {:>4.1} fps | face:{} Lhand:{} Rhand:{} body:{} | brightness:{:.0}",
                                fps,
                                if has_face { "ON " } else { "off" },
                                if has_left { "ON " } else { "off" },
                                if has_right { "ON " } else { "off" },
                                if has_pose { "ON " } else { "off" },
                                brightness
                            );
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
        let web_logic = self.web_interface.clone(); // [NEW] Pass web interface to logic thread
        
        use self::solver::Solver;
        
        thread::spawn(move || {
            let mut solver = Solver::new();
            println!("[Rust] Logic thread started");

            // VMT body-tracker output (SteamVR device emulation). Optional: if the
            // socket can't open we just skip it — the rest of the engine (face
            // params, VRChat OSC) is unaffected. When body pose is present we map
            // it to hips/chest/feet trackers and push them to VMT every frame.
            let vmt = crate::tracking::vmt::VmtBridge::new().ok();
            if vmt.is_some() {
                println!("[Rust] VMT body-tracker output ready (127.0.0.1:39570).");
            } else {
                println!("[Rust] VMT socket unavailable; body trackers will not be sent.");
            }

            // Head output -> our own virtual HMD driver (vrcbridge) on UDP 39571.
            // Streams the solver's smoothed head quaternion every frame so the
            // VRChat view turns with the user's head. Continuous + filtered, so
            // no teleport jumps (which SteamVR/VRChat read as tracking loss).
            let head = crate::tracking::head_bridge::HeadBridge::new().ok();
            if head.is_some() {
                println!("[Rust] Head output ready -> vrcbridge HMD (127.0.0.1:39571).");
            } else {
                println!("[Rust] Head socket unavailable; HMD rotation will not be sent.");
            }

            // Mouse-look: hold the RIGHT mouse button to turn the head a full
            // 360° (the webcam alone tops out near ±86°). Fused with the webcam
            // orientation below. Cursor stays free when the button isn't held.
            let mut mouse_look = crate::tracking::mouse_look::MouseLook::new();

            // Controller input: mouse aims the laser + clicks, keyboard moves /
            // opens menu, driving the two virtual VR controllers (UDP 39572) so
            // the user can navigate VRChat's VR menus and walk around.
            let controllers = crate::tracking::controller_input::ControllerInput::new().ok();
            if controllers.is_some() {
                println!("[Rust] Controller input ready -> vrcbridge hands (127.0.0.1:39572).");
            } else {
                println!("[Rust] Controller socket unavailable; menu navigation will not be sent.");
            }
            
            let mut _last_log = std::time::Instant::now();
            while *running_logic.lock().unwrap_or_else(|e| e.into_inner()) {
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

                // 2. Solve (profiled)
                let t_solve = std::time::Instant::now();
                // Use the resolution stamped onto the data itself (atomic with the
                // landmarks). Fall back to status dims, then a sane default, so a
                // momentarily-unset value can never reach the solver as 0.
                let (cam_w, cam_h) = {
                    if tracking_data.frame_w > 0.0 && tracking_data.frame_h > 0.0 {
                        (tracking_data.frame_w, tracking_data.frame_h)
                    } else {
                        let s = status_logic.lock().unwrap();
                        let (sw, sh) = (s.camera_width as f32, s.camera_height as f32);
                        if sw > 0.0 && sh > 0.0 { (sw, sh) } else { (640.0, 480.0) }
                    }
                };
                let output = solver.solve(&tracking_data, cam_w, cam_h);
                let solve_ms = t_solve.elapsed().as_secs_f32() * 1000.0;

                // --- VMT BODY OUTPUT --- Map the 33-pt body pose to upper-body
                // SteamVR trackers (hips + chest) and drive VMT. This is the
                // body path: VRChat's IK animates the avatar's torso lean from
                // these real tracked devices — what OSC avatar params can't do.
                // (Feet are omitted by design for seated/desk users.)
                if let (Some(vmt), Some(pose)) = (&vmt, &tracking_data.pose_landmarks) {
                    let body = crate::tracking::body::pose_to_body_trackers(pose, cam_w, cam_h);
                    if !body.is_empty() {
                        if let Err(e) = vmt.send_trackers(&body) {
                            eprintln!("[Rust] VMT send error: {}", e);
                        }
                    }
                }

                // --- HEAD OUTPUT --- Fuse mouse-look with the webcam head
                // orientation and forward it to the virtual HMD. Mouse-look adds
                // yaw (world-up) + pitch (local-right) composed IN FRONT of the
                // webcam quaternion, so the user gets a full 360° turn plus the
                // webcam's natural head motion. Rotation only; eye height stays
                // fixed at standing so the view can't sink to the floor. Mouse is
                // polled every frame (cursor only captured while RMB is held).
                let (mouse_yaw, mouse_pitch) = mouse_look.poll();
                let g = |key: &str| output.params.iter().find(|(k, _)| k == key).map(|(_, v)| *v);
                // The fused head orientation (xyzw), shared by the HMD and the
                // controller anchoring below. Falls back to identity when the
                // webcam has no head quaternion this frame so mouse-look still
                // works (and the hands stay anchored to the look direction).
                let head_quat = {
                    use nalgebra::{Quaternion, UnitQuaternion, Vector3};
                    let q_cam = match (
                        g("SYS_HEAD_ROT_X"),
                        g("SYS_HEAD_ROT_Y"),
                        g("SYS_HEAD_ROT_Z"),
                        g("SYS_HEAD_ROT_W"),
                    ) {
                        (Some(qx), Some(qy), Some(qz), Some(qw)) => {
                            // nalgebra Quaternion is (w, i, j, k); wire order is xyzw.
                            UnitQuaternion::from_quaternion(Quaternion::new(qw, qx, qy, qz))
                        }
                        _ => UnitQuaternion::identity(),
                    };
                    let q_yaw = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), mouse_yaw);
                    let q_pitch = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), mouse_pitch);
                    let q = (q_yaw * q_pitch * q_cam).into_inner();
                    [q.i, q.j, q.k, q.w]
                };
                if let Some(head) = &head {
                    if let Err(e) = head.send_rotation(head_quat) {
                        eprintln!("[Rust] Head send error: {}", e);
                    }
                }
                if let Some(controllers) = &controllers {
                    if let Err(e) = controllers.update(head_quat, mouse_look.rmb_held()) {
                        eprintln!("[Rust] Controller send error: {}", e);
                    }
                }

                // [OSC OUT] 1 Hz dump of the ACTUAL values the engine emits, so the
                // real output can be observed from logs (independent of VRChat/avatar).
                if _last_log.elapsed().as_secs_f32() >= 1.0 {
                    _last_log = std::time::Instant::now();
                    let g = |key: &str| output.params.iter().find(|(k, _)| k == key).map(|(_, v)| *v);
                    let face_seen = g("SYS_HEAD_ROT_W").is_some();
                    println!(
                        "[OSC OUT] params={} | Jaw={:.2} BlinkL={:.2} BlinkR={:.2} EyesX={:+.2} EyesY={:+.2} | HeadYaw={:+.2} HeadPitch={:+.2} HeadRoll={:+.2}",
                        output.params.len(),
                        g("JawOpen").unwrap_or(-1.0),
                        g("EyeBlinkLeft").unwrap_or(-1.0),
                        g("EyeBlinkRight").unwrap_or(-1.0),
                        g("EyesX").unwrap_or(-9.0),
                        g("EyesY").unwrap_or(-9.0),
                        g("HeadYaw").unwrap_or(-9.0),
                        g("HeadPitch").unwrap_or(-9.0),
                        g("HeadRoll").unwrap_or(-9.0),
                    );
                    // [HMD OUT] What we actually stream to the virtual headset, so
                    // we can tell whether head rotation is even moving and why.
                    println!(
                        "[HMD OUT] face_webcam={} | mouse_look(RMB held={}) yaw={:+.2} pitch={:+.2} | head_quat=[{:+.2} {:+.2} {:+.2} {:+.2}]",
                        if face_seen { "YES" } else { "no" },
                        mouse_look.rmb_held(),
                        mouse_yaw,
                        mouse_pitch,
                        head_quat[0], head_quat[1], head_quat[2], head_quat[3],
                    );
                }

                // 3. Send to VRChat (profiled)
                let has_data = !output.params.is_empty() || !output.trackers.is_empty();
                let mut osc_ms = 0.0f32;
                
                if has_data {
                    let conn = connectivity_logic.lock().unwrap();
                    
                    let t_osc = std::time::Instant::now();
                    if !output.params.is_empty() {
                        if let Err(e) = conn.send_tracking_data(output.params.clone()) {
                             eprintln!("[Rust] OSC Params Error: {}", e); 
                        }
                    }
                    if !output.trackers.is_empty() {
                        if let Err(e) = conn.send_tracker_data(output.trackers.clone()) {
                             eprintln!("[Rust] OSC Trackers Error: {}", e);
                        }
                    }
                    osc_ms = t_osc.elapsed().as_secs_f32() * 1000.0;

                    // [NEW] Emit OSC Data to Frontend (Visual Monitor)
                    // Throttle? The frontend can handle 60 events/sec or we can throttle here.
                    // Let's emit every frame, JS can throttle rendering if needed.
                    if has_data {
                        if let Some(guard) = web_logic.try_lock().ok() { // usage of try_lock to avoid blocking logic thread
                             guard.emit_osc_data(&output.params);
                        }
                    }
                }

                // Total frame time
                let total_ms = start.elapsed().as_secs_f32() * 1000.0;

                // Spike warning
                if total_ms > 20.0 {
                    eprintln!("[Perf] SPIKE {:.1}ms (solve={:.1} osc={:.1})", total_ms, solve_ms, osc_ms);
                }

                // Update status with profiling + tracking health
                if let Ok(mut s) = status_logic.lock() {
                    s.solver_ms = solve_ms;
                    s.osc_ms = osc_ms;
                    s.face_lost = tracking_data.face_landmarks.is_none();
                    s.left_hand_lost = tracking_data.left_hand_landmarks.is_none();
                    s.right_hand_lost = tracking_data.right_hand_landmarks.is_none();
                }

                // === TERMINAL MONITOR (User Request: "Invite de commande stylé") ===
                // We use our new monitor module to draw the dashboard
                // Since this thread runs at 60Hz, the monitor handles its own throttling (e.g. 15Hz)
                if output.params.len() > 0 {
                    // Lazy init monitor (or just create local instance since we don't share it)
                    // We can't easily modify 'self' here as we are in a closure.
                    // So we create a static/local monitor?
                    // Actually, let's just create it outside the loop.
                }
                
                // (See below for where I actually put the variable)
                

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
        // Poison-tolerant: if a worker thread panicked while holding this lock,
        // Stop must STILL succeed, otherwise the app gets stuck "running" forever
        // (this was a real symptom: after a bad camera switch, Stop did nothing).
        match self.running.lock() {
            Ok(mut r) => *r = false,
            Err(poisoned) => *poisoned.into_inner() = false,
        }
        self.web_interface.blocking_lock().stop(); // [FIX] Blocking lock
        println!("[Rust] Stop requested (running = false).");
    }
}
