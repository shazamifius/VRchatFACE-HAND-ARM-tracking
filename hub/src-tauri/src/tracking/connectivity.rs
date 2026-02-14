use anyhow::{Result, anyhow};
use rosc::encoder;
use rosc::{OscMessage, OscPacket, OscType};
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
        // [DEBUG LOG]
        // println!("[OSC] Out -> {} : {:?}", msg.addr, msg.args);
        
        // Clone args if needed or recreate msg, actually msg is moved into Packet.
        // So we printed it before move. But wait, msg.args is Vec which is not Copy.
        // println! borrows msg.
        // OscPacket::Message(msg) moves msg.
        // So print MUST be before move, but we cannot borrow then move?
        // Rust rules:
        // println! borrows.
        // Packet move happens after.
        // This should be fine ONLY if println! doesn't enable a loan that outlasts the statement.
        // But to be safe and avoid headaches:
        
        // [DEBUG LOG]
        // println!("[OSC] Out -> {} : {:?}", addr, args_copy);

        let packet = OscPacket::Message(msg);
        let msg_buf = encoder::encode(&packet)?;
        self.osc_socket.send_to(&msg_buf, &self.osc_target)?;
        Ok(())
    }

    /// Send full tracking data to VRChat
    pub fn send_tracking_data(&self, face_params: Vec<(String, f32)>) -> Result<()> {
        // Aggregate Params for Vectors
        let mut head_pos = Vector3::new(0.0, 0.0, 0.0);
        let mut head_rot = Vector3::new(0.0, 0.0, 0.0); // Pitch, Yaw, Roll
        
        let mut left_hand_pos = Vector3::new(0.0, 0.0, 0.0);
        let mut left_hand_rot_q = (0.0, 0.0, 0.0, 1.0); // x,y,z,w
        let mut right_hand_pos = Vector3::new(0.0, 0.0, 0.0);
        let mut right_hand_rot_q = (0.0, 0.0, 0.0, 1.0);

        let mut has_head = false;
        let mut has_head_pos = false;
        let mut has_left_hand = false;
        let mut has_right_hand = false;

        for (param, value) in face_params {
             // System Rotations handling
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

            // Hand Left Aggregation
            if param == "HandLeftPos_X" { left_hand_pos.x = value; has_left_hand = true; continue; }
            if param == "HandLeftPos_Y" { left_hand_pos.y = value; has_left_hand = true; continue; }
            if param == "HandLeftPos_Z" { left_hand_pos.z = value; has_left_hand = true; continue; }
            if param == "HandLeftRot_X" { left_hand_rot_q.0 = value; has_left_hand = true; continue; }
            if param == "HandLeftRot_Y" { left_hand_rot_q.1 = value; has_left_hand = true; continue; }
            if param == "HandLeftRot_Z" { left_hand_rot_q.2 = value; has_left_hand = true; continue; }
            if param == "HandLeftRot_W" { left_hand_rot_q.3 = value; has_left_hand = true; continue; }

            // Hand Right Aggregation
            if param == "HandRightPos_X" { right_hand_pos.x = value; has_right_hand = true; continue; }
            if param == "HandRightPos_Y" { right_hand_pos.y = value; has_right_hand = true; continue; }
            if param == "HandRightPos_Z" { right_hand_pos.z = value; has_right_hand = true; continue; }
            if param == "HandRightRot_X" { right_hand_rot_q.0 = value; has_right_hand = true; continue; }
            if param == "HandRightRot_Y" { right_hand_rot_q.1 = value; has_right_hand = true; continue; }
            if param == "HandRightRot_Z" { right_hand_rot_q.2 = value; has_right_hand = true; continue; }
            if param == "HandRightRot_W" { right_hand_rot_q.3 = value; has_right_hand = true; continue; }

            // Hand Fingers Mapping
            if param.starts_with("HandLeft") || param.starts_with("HandRight") {
                 let digit = param.replace("HandLeft", "").replace("HandRight", ""); // Index, Middle...
                 let side = if param.contains("Left") { "Left" } else { "Right" };
                 
                 // Skip Pos/Rot if they slipped through (handled above normally)
                 if digit.starts_with("Pos") || digit.starts_with("Rot") { continue; }

                 let g_param = format!("Gesture{}{}", side, digit);
                 self.send_avatar_param(&g_param, value);
                 continue; // Done with this param
            }

            // Standard Params (Face)
            let addr = format!("/avatar/parameters/{}", param);
            let _ = self.send_osc(&addr, vec![OscType::Float(value)]);

            // Aliases
             if param == "JawOpen" {
                self.send_avatar_param("MouthOpen", value);
                self.send_avatar_param("Voice", value); 
                self.send_avatar_param("vrc_MouthOpen", value);
                self.send_avatar_param("Aperture", value);
            }
            if param == "EyeBlinkLeft" {
                 self.send_avatar_param("BlinkLeft", value);
                 self.send_avatar_param("EyeOpenLeft", 1.0 - value); 
                 self.send_avatar_param("EyesClosed", value); 
            }
            if param == "EyeBlinkRight" {
                 self.send_avatar_param("BlinkRight", value);
                 self.send_avatar_param("EyeOpenRight", 1.0 - value); 
            }
        }

        // Send Head Tracker
        if has_head {
             let pitch_deg = head_rot.x * 57.2958; 
             let yaw_deg = head_rot.y * 57.2958;
             let roll_deg = head_rot.z * 57.2958;
             
             let _ = self.send_osc("/tracking/trackers/head/rotation", vec![
                 OscType::Float(pitch_deg),
                 OscType::Float(yaw_deg),
                 OscType::Float(roll_deg)
             ]);
             
             if has_head_pos {
                 let _ = self.send_osc("/tracking/trackers/head/position", vec![
                     OscType::Float(head_pos.x),
                     OscType::Float(head_pos.y),
                     OscType::Float(head_pos.z)
                 ]);
             }

             // Also Generic Head Inputs (for non-FBT)
             let _ = self.send_osc("/input/HeadPitch", vec![OscType::Float(head_rot.x)]);
             let _ = self.send_osc("/input/HeadYaw", vec![OscType::Float(head_rot.y)]);
             let _ = self.send_osc("/input/HeadRoll", vec![OscType::Float(head_rot.z)]);
        }

        // Send Hand Trackers
        if has_left_hand {
             let _ = self.send_osc("/tracking/trackers/1/position", vec![
                 OscType::Float(left_hand_pos.x),
                 OscType::Float(left_hand_pos.y),
                 OscType::Float(left_hand_pos.z)
             ]);
        }
        if has_right_hand {
             let _ = self.send_osc("/tracking/trackers/2/position", vec![
                 OscType::Float(right_hand_pos.x),
                 OscType::Float(right_hand_pos.y),
                 OscType::Float(right_hand_pos.z)
             ]);
        }
        
        Ok(())
    }

    // Helper to send variants
    fn send_avatar_param(&self, param: &str, value: f32) {
         let addr = format!("/avatar/parameters/{}", param);
         let _ = self.send_osc(&addr, vec![OscType::Float(value)]);
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
                            *tunnel_url_store.lock().unwrap() = Some(url);
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
