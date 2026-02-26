use std::sync::Mutex;
use std::collections::VecDeque;
use once_cell::sync::Lazy;

// Global Log Buffer (Thread-Safe)
// Keeps last 100 logs
pub static LOG_BUFFER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| {
    Mutex::new(VecDeque::with_capacity(100))
});

pub fn log(msg: &str) {
    // Print to stdout for dev
    println!("{}", msg);
    
    // Store in buffer
    if let Ok(mut mq) = LOG_BUFFER.lock() {
        if mq.len() >= 100 {
            mq.pop_front();
        }
        mq.push_back(msg.to_string());
    }
}

pub fn get_recent_logs() -> Vec<String> {
    if let Ok(mq) = LOG_BUFFER.lock() {
        mq.iter().cloned().collect()
    } else {
        Vec::new()
    }
}
