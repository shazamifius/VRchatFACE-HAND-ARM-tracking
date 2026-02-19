use anyhow::{Result, anyhow};
use rosc::encoder;
use rosc::{OscBundle, OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::process::{Command, Child, Stdio};
use std::sync::{Arc, Mutex};
use qrcodegen::{QrCode, QrCodeEcc};
use std::io::{BufRead, BufReader};
use std::thread;
use regex::Regex;
use nalgebra::Vector3;

pub struct ConnectivityManager {
    osc_socket: UdpSocket,
    osc_target: String, // "127.0.0.1:9000"
    cloudflared_process: Arc<Mutex<Option<Child>>>,
    pub tunnel_url: Arc<Mutex<Option<String>>>,
}

impl ConnectivityManager {
    pub fn new() -> Result<Self> {
        let osc_socket = UdpSocket::bind("0.0.0.0:0")?; // Bind to any available port
        osc_socket.set_nonblocking(true).ok(); // Non-blocking for lower latency
        Ok(Self {
            osc_socket,
            osc_target: "127.0.0.1:9000".to_string(), // Default VRChat port
            cloudflared_process: Arc::new(Mutex::new(None)),
            tunnel_url: Arc::new(Mutex::new(None)),
        })
    }

    pub fn set_osc_target(&mut self, ip: &str, port: u16) {
        self.osc_target = format!("{}:{}", ip, port);
    }

    pub fn send_osc(&self, address: &str, args: Vec<OscType>) -> Result<()> {
        let msg = OscMessage {
            addr: address.to_string(),
            args,
        };
        
        // Broadcast to Main Target (VRChat) AND Monitor (Localhost:9005)
        let packet = OscPacket::Message(msg);
        let msg_buf = encoder::encode(&packet)?;
        
        // 1. VRChat
        let _ = self.osc_socket.send_to(&msg_buf, &self.osc_target);
        
        // 2. Monitor (Hardcoded for now as it's internal)
        let _ = self.osc_socket.send_to(&msg_buf, "127.0.0.1:9005");
        
        Ok(())
    }

    /// Send an OSC bundle (multiple messages in 1 UDP packet)
    fn send_osc_bundle(&self, messages: Vec<OscMessage>) -> Result<()> {
        if messages.is_empty() { return Ok(()); }
        let packets: Vec<OscPacket> = messages.into_iter()
            .map(OscPacket::Message)
            .collect();
        let bundle = OscBundle {
            timetag: (0, 1).into(), // Immediate
            content: packets,
        };
        let buf = encoder::encode(&OscPacket::Bundle(bundle))?;
        
        // 1. VRChat
        let _ = self.osc_socket.send_to(&buf, &self.osc_target);
        // 2. Monitor
        let _ = self.osc_socket.send_to(&buf, "127.0.0.1:9005");
        
        Ok(())
    }

    /// Helper: create a float OSC message
    fn make_float_msg(addr: &str, value: f32) -> OscMessage {
        OscMessage { addr: addr.to_string(), args: vec![OscType::Float(value)] }
    }

    /// Helper: create a bool OSC message  
    fn make_bool_msg(addr: &str, value: bool) -> OscMessage {
        OscMessage { addr: addr.to_string(), args: vec![OscType::Bool(value)] }
    }

    /// Helper: create a multi-float OSC message
    fn make_vec3_msg(addr: &str, x: f32, y: f32, z: f32) -> OscMessage {
        OscMessage { addr: addr.to_string(), args: vec![OscType::Float(x), OscType::Float(y), OscType::Float(z)] }
    }

    /// Send full tracking data to VRChat
    /// Send full tracking data to VRChat — batched as a single OSC bundle
    pub fn send_tracking_data(&self, face_params: Vec<(String, f32)>) -> Result<()> {
        let mut msgs: Vec<OscMessage> = Vec::with_capacity(80);

        // Activator Params
        msgs.push(Self::make_bool_msg("/avatar/parameters/FaceTrackingActive", true));
        msgs.push(Self::make_bool_msg("/avatar/parameters/FaceTracking", true));
        msgs.push(Self::make_bool_msg("/avatar/parameters/OSC", true));
        msgs.push(Self::make_bool_msg("/avatar/parameters/PancakeMode", true));
        msgs.push(Self::make_bool_msg("/avatar/parameters/LipTrackingActive", true));
        msgs.push(Self::make_bool_msg("/avatar/parameters/EyeTrackingActive", true));
        msgs.push(Self::make_bool_msg("/avatar/parameters/FTOn", true));
        msgs.push(Self::make_bool_msg("/avatar/parameters/FTOff", false));
        msgs.push(Self::make_bool_msg("/avatar/parameters/FacialExpressionsDisabled", false));
        msgs.push(Self::make_float_msg("/avatar/parameters/FaceTrackingActive", 1.0));
        msgs.push(Self::make_float_msg("/avatar/parameters/FaceTracking", 1.0));
        msgs.push(Self::make_float_msg("/avatar/parameters/OSC", 1.0));
        msgs.push(Self::make_float_msg("/avatar/parameters/LipTracking_GestureControl", 0.0));
        msgs.push(Self::make_float_msg("/avatar/parameters/EyeTracking_GestureControl", 0.0));

        let mut head_pos = Vector3::new(0.0, 0.0, 0.0);
        let mut head_rot = Vector3::new(0.0, 0.0, 0.0);
        let mut left_hand_pos = Vector3::new(0.0, 0.0, 0.0);
        let mut right_hand_pos = Vector3::new(0.0, 0.0, 0.0);
        let mut has_head = false;
        let mut has_head_pos = false;
        let mut has_left_hand = false;
        let mut has_right_hand = false;

        // Smart Eyes: Check Brows first
        let mut brow_max_open = 0.8;
        for (p, v) in &face_params {
            if p == "BrowInnerUp" || p.starts_with("BrowOuterUp") {
                let intent = 0.8 + (0.2 * v);
                if intent > brow_max_open { brow_max_open = intent; }
            }
        }

        // Helper: push avatar param message
        let avatar_msg = |name: &str, val: f32| -> OscMessage {
            Self::make_float_msg(&format!("/avatar/parameters/{}", name), val)
        };

        for (param, value) in face_params {
            // Skip system-internal params
            if param.starts_with("SYS_HEAD_ROT_") { continue; }
            if param.starts_with("HandLeftRot_") { continue; }
            if param.starts_with("HandRightRot_") { continue; }

            // Head Aggregation
            if param == "HeadPos_X" { head_pos.x = value; has_head_pos = true; continue; }
            if param == "HeadPos_Y" { head_pos.y = value; has_head_pos = true; continue; }
            if param == "HeadPos_Z" { head_pos.z = value; has_head_pos = true; continue; }
            if param == "HeadYaw" { head_rot.y = value; has_head = true; }
            if param == "HeadPitch" { head_rot.x = value; has_head = true; }
            if param == "HeadRoll" { head_rot.z = value; has_head = true; }
            if param == "HeadYawCoupling" { head_rot.y += value; has_head = true; continue; }
            if param == "HeadPitchCoupling" { head_rot.x += value; has_head = true; continue; }

            // Hand Aggregation
            if param == "HandLeftPos_X" { left_hand_pos.x = value; has_left_hand = true; continue; }
            if param == "HandLeftPos_Y" { left_hand_pos.y = value; has_left_hand = true; continue; }
            if param == "HandLeftPos_Z" { left_hand_pos.z = value; has_left_hand = true; continue; }
            if param == "HandRightPos_X" { right_hand_pos.x = value; has_right_hand = true; continue; }
            if param == "HandRightPos_Y" { right_hand_pos.y = value; has_right_hand = true; continue; }
            if param == "HandRightPos_Z" { right_hand_pos.z = value; has_right_hand = true; continue; }

            // Hand Fingers
            if param.starts_with("HandLeft") || param.starts_with("HandRight") {
                let digit = param.replace("HandLeft", "").replace("HandRight", "");
                let side = if param.contains("Left") { "Left" } else { "Right" };
                if digit.starts_with("Pos") || digit.starts_with("Rot") { continue; }
                msgs.push(avatar_msg(&format!("Gesture{}{}", side, digit), value));
                continue;
            }

            // Standard Face Param
            msgs.push(Self::make_float_msg(&format!("/avatar/parameters/{}", param), value));

            // Aliases & Unified Expressions
            if param == "JawOpen" {
                msgs.push(avatar_msg("MouthOpen", value));
                msgs.push(avatar_msg("Voice", value));
                msgs.push(avatar_msg("vrc_MouthOpen", value));
                msgs.push(avatar_msg("Aperture", value));
                msgs.push(avatar_msg("jawOpen", value));
                msgs.push(avatar_msg("mouthOpen", value));
                msgs.push(avatar_msg("VRCFaceBlendShape_JawOpen", value));
                msgs.push(avatar_msg("FT/v2/JawOpen", value));
            }
            if param == "CheekPuff" {
                msgs.push(avatar_msg("CheekPuff", value));
                msgs.push(avatar_msg("OSCm/BlendSetRight", value));
                msgs.push(avatar_msg("OSCm/BlendSetLeft", value));
                msgs.push(avatar_msg("FT/v2/CheekPuff", value));
            }
            if param == "EyeBlinkLeft" {
                msgs.push(avatar_msg("BlinkLeft", value));
                msgs.push(avatar_msg("EyeOpenLeft", 1.0 - value));
                msgs.push(avatar_msg("EyesClosed", value));
                msgs.push(avatar_msg("eyeClosedLeft", value));
                msgs.push(avatar_msg("blinkLeft", value));
                msgs.push(avatar_msg("EyeClosedLeft", value));
                msgs.push(avatar_msg("VRCFaceBlendShape_EyeBlinkLeft", value));
                msgs.push(avatar_msg("FT/v2/EyeLidLeft", (1.0 - value) * brow_max_open));
            }
            if param == "EyeBlinkRight" {
                msgs.push(avatar_msg("BlinkRight", value));
                msgs.push(avatar_msg("EyeOpenRight", 1.0 - value));
                msgs.push(avatar_msg("eyeClosedRight", value));
                msgs.push(avatar_msg("blinkRight", value));
                msgs.push(avatar_msg("EyeClosedRight", value));
                msgs.push(avatar_msg("VRCFaceBlendShape_EyeBlinkRight", value));
                msgs.push(avatar_msg("FT/v2/EyeLidRight", (1.0 - value) * brow_max_open));
            }
            if param.starts_with("EyeLook") {
                msgs.push(avatar_msg(&format!("VRCFaceBlendShape_{}", param), value));
                msgs.push(avatar_msg(&format!("FT/v2/{}", param), value));
                let lower = param[0..1].to_lowercase() + &param[1..];
                msgs.push(avatar_msg(&lower, value));
            }
        }

        // Head Tracker messages
        if has_head {
            let pitch_deg = head_rot.x * 57.2958;
            let yaw_deg = head_rot.y * 57.2958;
            let roll_deg = head_rot.z * 57.2958;
            msgs.push(Self::make_vec3_msg("/tracking/trackers/head/rotation", pitch_deg, yaw_deg, roll_deg));
            if has_head_pos {
                msgs.push(Self::make_vec3_msg("/tracking/trackers/head/position", head_pos.x, head_pos.y, head_pos.z));
            }
            msgs.push(Self::make_float_msg("/input/HeadPitch", head_rot.x));
            msgs.push(Self::make_float_msg("/input/HeadYaw", head_rot.y));
            msgs.push(Self::make_float_msg("/input/HeadRoll", head_rot.z));
        }

        // Hand Tracker messages
        if has_left_hand {
            msgs.push(Self::make_vec3_msg("/tracking/trackers/1/position", left_hand_pos.x, left_hand_pos.y, left_hand_pos.z));
        }
        if has_right_hand {
            msgs.push(Self::make_vec3_msg("/tracking/trackers/2/position", right_hand_pos.x, right_hand_pos.y, right_hand_pos.z));
        }

        // Send everything as a single bundle
        self.send_osc_bundle(msgs)
    }

    pub fn send_tracker_data(&self, trackers: Vec<crate::tracking::solver::TrackerData>) -> Result<()> {
        for tracker in trackers {
            let id = tracker.id;
            // VRChat Tracker Endpoints:
            // /tracking/trackers/{id}/position (x, y, z)
            // /tracking/trackers/{id}/rotation (x, y, z) [Euler Degrees]
            
            let pos_addr = if id == 0 {
                "/tracking/trackers/head/position".to_string()
            } else {
                format!("/tracking/trackers/{}/position", id)
            };

            let rot_addr = if id == 0 {
                "/tracking/trackers/head/rotation".to_string()
            } else {
                format!("/tracking/trackers/{}/rotation", id)
            };

            // Send Position
            let _ = self.send_osc(&pos_addr, vec![
                OscType::Float(tracker.position[0]),
                OscType::Float(tracker.position[1]),
                OscType::Float(tracker.position[2]),
            ]);

            // Send Rotation (Convert Quat to Euler Degrees)
            let q = nalgebra::UnitQuaternion::new_normalize(nalgebra::Quaternion::new(
                tracker.rotation[3], // w
                tracker.rotation[0], // x
                tracker.rotation[1], // y
                tracker.rotation[2]  // z
            ));
            let (roll, pitch, yaw) = q.euler_angles();
            // Degrees
            let rx = pitch * 57.2958;
            let ry = yaw * 57.2958;
            let rz = roll * 57.2958;

            let _ = self.send_osc(&rot_addr, vec![
                OscType::Float(rx),
                OscType::Float(ry),
                OscType::Float(rz),
            ]);
        }
        Ok(())
    }



    /// Start Cloudflare Tunnel and scrape the URL
    pub fn start_tunnel(&self, port: u16) -> Result<()> {
        let mut process_guard = self.cloudflared_process.lock().unwrap();
        if process_guard.is_some() {
            return Ok(()); // Already running
        }

        // Try to find cloudflared in current dir or PATH
        let mut exe = "cloudflared".to_string();
        
        let cwd = std::env::current_dir().unwrap_or_default();
        let local_exe = cwd.join("cloudflared.exe");
        if local_exe.exists() {
            exe = local_exe.to_str().unwrap().to_string();
            println!("[Rust] Found cloudflared at: {}", exe);
        } else {
             // Try a level up (if running from debug target)
             let up_exe = cwd.join("../cloudflared.exe");
              if up_exe.exists() {
                exe = up_exe.to_str().unwrap().to_string();
                println!("[Rust] Found cloudflared at: {}", exe);
            }
        }
        
        // On Windows effectively, Command::new needs help if it's not in PATH
        // But if we give full path, it works.

        let mut child = Command::new(&exe)
            .arg("tunnel")
            .arg("--protocol") // [FIX] Force HTTP2 (TCP) for better compatibility
            .arg("http2")
            .arg("--url")
            .arg(format!("http://127.0.0.1:{}", port)) // [FIX] Force IPv4 to avoid [::1] issues
            .stderr(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|e| anyhow!("Failed to start cloudflared at '{}': {}", exe, e))?;

        let stderr = child.stderr.take().ok_or(anyhow!("Failed to capture stderr"))?;
        let tunnel_url_store = self.tunnel_url.clone();

        // Spawn thread to read stderr and find URL
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            let re = Regex::new(r"https://[a-zA-Z0-9-]+\.trycloudflare\.com").unwrap();

            for line in reader.lines() {
                if let Ok(l) = line {
                    println!("[Cloudflare] {}", l);
                    if let Some(caps) = re.captures(&l) {
                        if let Some(match_) = caps.get(0) {
                            let url = match_.as_str().to_string();
                            println!("[Rust] Tunnel URL found: {}", url);
                            *tunnel_url_store.lock().unwrap() = Some(url.clone());
                            
                             // [FIX] Emit event to frontend!
                             // parameters: event name, payload
                             // We need a handle to the app handle or window.
                             // But we are in a thread here. We need the app handle passed to start_tunnel?
                             
                             // Actually, better to use a callback or channel if we can't emit from here easily.
                             // But checking `lib.rs`, `start_cloudflare_tunnel` command calls this.
                             // It doesn't pass the app handle.
                             
                             // WAITING: I need to verify how to emit from here.
                             // Inspecting `mod.rs` might reveal if `ConnectivityManager` has an `app_handle`.
                             // It doesn't seem to have it.
                             
                             // Alternative: Store it in `tunnel_url_store` (which is a Mutex<Option<String>>).
                             // And have a periodic check in frontend? OR have the command loop waiting?
                             
                             // Let's check `lib.rs` command `start_cloudflare_tunnel`.
                        }
                    }
                }
            }
        });

        *process_guard = Some(child);
        Ok(())
    }

    pub fn stop_tunnel(&self) {
        let mut process_guard = self.cloudflared_process.lock().unwrap();
        if let Some(mut child) = process_guard.take() {
            let _ = child.kill();
            println!("[Rust] Cloudflare tunnel stopped");
        }
        *self.tunnel_url.lock().unwrap() = None;
    }

    pub fn generate_qr(&self, content: &str) -> Result<String> {
        let qr = QrCode::encode_text(content, QrCodeEcc::Medium)?;
        
        let border = 4;
        let size = qr.size();
        let view_box_size = size + border * 2;
        
        let mut svg = String::with_capacity(2048);
        svg.push_str(&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" viewBox=\"0 0 {} {}\" stroke=\"none\">", view_box_size, view_box_size));
        svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#FFFFFF\"/>");
        svg.push_str("<path d=\"");
        
        for y in 0..size {
            for x in 0..size {
                if qr.get_module(x, y) {
                    svg.push_str(&format!("M{},{}h1v1h-1z ", x + border, y + border));
                }
            }
        }
        
        svg.push_str("\" fill=\"#000000\"/>");
        svg.push_str("</svg>");
        
        Ok(svg)
    }
}
