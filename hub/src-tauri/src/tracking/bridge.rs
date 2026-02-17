use anyhow::Result;
use rosc::{OscPacket, OscType, OscMessage};
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;


pub struct OscBridge {
    socket: UdpSocket,
}

impl OscBridge {
    pub fn new(port: u16) -> Result<Self> {
        let addr = format!("0.0.0.0:{}", port);
        let socket = UdpSocket::bind(&addr)?;
        socket.set_nonblocking(false)?;
        println!("[Rust] Bridge bound to {}", addr);
        Ok(Self { socket })
    }

    pub fn start(&self, running: Arc<Mutex<bool>>, data_callback: Box<dyn Fn(String, Vec<f32>) + Send + Sync + 'static>) {
        let socket = self.socket.try_clone().expect("Failed to clone socket");
        socket.set_read_timeout(Some(std::time::Duration::from_millis(100))).expect("Failed to set read timeout");
        
        thread::spawn(move || {
            let mut buf = [0u8; 65535]; 
            println!("[Rust] Bridge listener started");

            while *running.lock().unwrap() {
                match socket.recv_from(&mut buf) {
                    Ok((size, _addr)) => {
                        if let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                            handle_packet(packet, &data_callback);
                        }
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::WouldBlock && e.kind() != std::io::ErrorKind::TimedOut {
                            println!("[Rust] Bridge Error: {}", e);
                            thread::sleep(std::time::Duration::from_millis(100));
                        }
                        // If timed out, just loop again and check running flag
                    }
                }
            }
            println!("[Rust] Bridge listener stopped");
        });
    }
}

fn handle_packet(packet: OscPacket, callback: &Box<dyn Fn(String, Vec<f32>) + Send + Sync>) {
    match packet {
        OscPacket::Message(msg) => {
            handle_message(msg, callback);
        }
        OscPacket::Bundle(bundle) => {
            for packet in bundle.content {
                handle_packet(packet, callback);
            }
        }
    }
}

fn handle_message(msg: OscMessage, callback: &Box<dyn Fn(String, Vec<f32>) + Send + Sync>) {
    // We expect addresses like /tracking/face/landmarks
    // And args to be a list of Floats
    
    let mut float_args = Vec::with_capacity(msg.args.len());
    
    for arg in msg.args {
        if let OscType::Float(f) = arg {
            float_args.push(f);
        }
    }

    if !float_args.is_empty() {
        callback(msg.addr, float_args);
    }
}
