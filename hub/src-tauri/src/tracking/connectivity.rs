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
        let packet = OscPacket::Message(msg);
        let msg_buf = encoder::encode(&packet)?;
        self.osc_socket.send_to(&msg_buf, &self.osc_target)?;
        Ok(())
    }

    /// Send full tracking data to VRChat
    pub fn send_tracking_data(&self, face_params: Vec<(String, f32)>) -> Result<()> {
        // VRChat OSC Schema: /avatar/parameters/[Name] [Float/Int/Bool]
        
        for (param, value) in face_params {
            let addr = format!("/avatar/parameters/{}", param);
            let _ = self.send_osc(&addr, vec![OscType::Float(value)]);
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
