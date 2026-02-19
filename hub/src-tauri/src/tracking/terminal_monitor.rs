use std::io::{self, Write};
use crate::tracking::types::TrackingData;

pub struct TerminalMonitor {
    last_draw: std::time::Instant,
}

impl TerminalMonitor {
    pub fn new() -> Self {
        Self {
            last_draw: std::time::Instant::now(),
        }
    }

    pub fn draw(&mut self, data: &TrackingData, fps: f32) {
        // Draw at max 10 FPS to avoid flickering too much
        if self.last_draw.elapsed().as_millis() < 100 {
            return;
        }
        self.last_draw = std::time::Instant::now();

        // Clear Screen & Move Home
        // \x1B[2J = Clear entire screen
        // \x1B[H  = Move cursor to home (top-left)
        // \x1B[?25l = Hide Cursor
        print!("\x1B[2J\x1B[H\x1B[?25l");

        // Header
        println!("\x1B[1;36m================================================================================\x1B[0m");
        println!("\x1B[1;36m VRChat Bridge Hub - TERMINAL MONITOR \x1B[0m");
        println!("\x1B[1;36m================================================================================\x1B[0m");
        println!(" FPS: \x1B[1;32m{:.1}\x1B[0m", fps);
        println!("");

        // Face Data
        if let Some(face) = &data.face_landmarks {
             println!(" Face: \x1B[1;32mCONNECTED\x1B[0m ({} points)", face.len());
        } else {
             println!(" Face: \x1B[1;31mNOT DETECTED\x1B[0m");
        }

        // Hands
        let l_hand = if data.left_hand_landmarks.is_some() { "\x1B[1;32mYES\x1B[0m" } else { "\x1B[1;30mNO \x1B[0m" };
        let r_hand = if data.right_hand_landmarks.is_some() { "\x1B[1;32mYES\x1B[0m" } else { "\x1B[1;30mNO \x1B[0m" };
        println!(" Hands: L [{}]  R [{}]", l_hand, r_hand);
        println!("");
    }
    
    pub fn draw_osc(&mut self, params: &Vec<(String, f32)>, fps: f32) {
        // Draw at max 15 FPS
        if self.last_draw.elapsed().as_millis() < 66 {
            return;
        }
        self.last_draw = std::time::Instant::now();

        // Buffer the output to avoid flickering during print
        let mut buffer = String::new();

        // Clear Screen
        buffer.push_str("\x1B[2J\x1B[H"); 

        buffer.push_str("\x1B[1;36m================================ VRChat Bridge OSC ===============================\x1B[0m\n");
        buffer.push_str(&format!(" FPS: \x1B[1;32m{:.1}\x1B[0m   |   Params sent: {}\n", fps, params.len()));
        buffer.push_str("--------------------------------------------------------------------------------\n");

        if params.is_empty() {
            buffer.push_str("\n  (No parameters sent. Waiting for data...)\n");
        } else {
            // Sort or just take interesting ones?
            // Printing 80 lines is too tall for most CMD.
            // Let's filter key ones + any active ones.
            
            let mut count = 0;
            // params is a Vec, so we can iterate directly.
            // But we might want to prioritize "interesting" ones.
            // For now, just print the non-zero ones first? 
            // Or just print as is, limited to 25.
            
            for (name, value) in params {
                if count > 25 { 
                    buffer.push_str("\x1B[1;30m... (and more)\x1B[0m\n");
                    break; 
                }
                
                // Exclude boring static logic params if 0
                if *value < 0.01 && (name.contains("Right") || name.contains("Left") || name.contains("Cheek")) {
                    continue;
                }

                self.draw_bar(&mut buffer, name, *value);
                count += 1;
            }
        }
        
        buffer.push_str("--------------------------------------------------------------------------------\n");
        buffer.push_str("\x1B[1;30m Press CTRL+C to Exit \x1B[0m\n");

        // Write once
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(buffer.as_bytes());
        let _ = handle.flush();
    }

    fn draw_bar(&self, buffer: &mut String, label: &str, value: f32) {
        // Label (width 25)
        let label_trunc = if label.len() > 24 { &label[0..24] } else { label };
        buffer.push_str(&format!("{:<25} ", label_trunc));

        // Bar (width 20)
        buffer.push_str("[");
        let active = (value * 20.0).round() as usize;
        let active = active.min(20);
        
        // Color based on value
        let color = if value > 0.8 { "\x1B[1;32m" } // Bright Green
                   else if value > 0.4 { "\x1B[0;32m" } // Green
                   else { "\x1B[1;30m" }; // Dark Gray

        buffer.push_str(color);
        for _ in 0..active { buffer.push('#'); }
        buffer.push_str("\x1B[0m"); // Reset
        
        for _ in active..20 { buffer.push('.'); }
        buffer.push_str("] ");

        // Value
        buffer.push_str(&format!("\x1B[1;37m{:.3}\x1B[0m\n", value));
    }
}
