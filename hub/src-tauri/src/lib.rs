// VRChat Bridge Hub - Rust Backend
// Manages tracking via Native Rust Engine

pub mod tracking;

// use std::sync::Mutex;
use tauri::{State, Manager, Emitter};
use std::sync::Arc;

use crate::tracking::TrackingEngine;
use crate::tracking::types::{CameraInfo, TrackingStatus};

/// State managed by Tauri
pub struct TrackingState {
    pub rust_engine: Arc<TrackingEngine>,
}

impl Default for TrackingState {
    fn default() -> Self {
        Self {
            rust_engine: Arc::new(TrackingEngine::new()),
        }
    }
}

/// Get list of available cameras
#[tauri::command]
async fn get_cameras(state: State<'_, TrackingState>) -> Result<Vec<CameraInfo>, String> {
    println!("[Rust] requesting cameras...");
    let camera = state.rust_engine.camera.clone();
    // Spawn on a blocking thread so the UI doesn't freeze
    let cams = tokio::task::spawn_blocking(move || {
        let camera_manager = camera.lock().unwrap();
        camera_manager.get_cameras()
    }).await.map_err(|e| format!("Camera query failed: {}", e))?;
    println!("[Rust] Found {} cameras", cams.len());
    Ok(cams)
}

/// Start the tracking service
#[tauri::command]
async fn start_tracking(
    state: State<'_, TrackingState>,
    camera_index: i32,
    osc_ip: String,
    osc_port: i32,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<u32>,
    format: Option<String>,
) -> Result<bool, String> {
    // 1. Configure OSC
    if let Ok(mut connectivity) = state.rust_engine.connectivity.lock() {
        connectivity.set_osc_target(&osc_ip, osc_port as u16);
    }

    // 2. Start Engine
    let config = crate::tracking::types::CameraConfig {
        index: camera_index as u32,
        width: width.unwrap_or(640),
        height: height.unwrap_or(480),
        fps: fps.unwrap_or(30),
        format: format,
    };
    state.rust_engine.start(config).map_err(|e| e.to_string())?;
    
    Ok(true)
}

/// Stop the tracking service
#[tauri::command]
fn stop_tracking(state: State<TrackingState>) -> Result<bool, String> {
    state.rust_engine.stop();
    Ok(true)
}

/// Get current tracking status
#[tauri::command]
fn get_tracking_status(state: State<TrackingState>) -> TrackingStatus {
    if let Ok(status) = state.rust_engine.status.lock() {
        status.clone()
    } else {
        TrackingStatus::default()
    }
}

/// Get raw tracking data (Landmarks) for Debug View
#[tauri::command]
fn get_tracking_data(state: State<TrackingState>) -> crate::tracking::types::TrackingData {
    if let Ok(data) = state.rust_engine.data.lock() {
        data.clone()
    } else {
        crate::tracking::types::TrackingData::default()
    }
}

/// Start Cloudflare Tunnel
#[tauri::command]
fn start_cloudflare_tunnel(app: tauri::AppHandle, state: State<TrackingState>, port: u16) -> Result<bool, String> {
    let connectivity = state.rust_engine.connectivity.clone();
    
    // Start the process
    {
        let conn = connectivity.lock().map_err(|e| e.to_string())?;
        conn.start_tunnel(port).map_err(|e| e.to_string())?;
    }

    // Spawn a thread to poll for the URL and emit it
    let connectivity_clone = connectivity.clone();
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed().as_secs() > 30 {
                break; // Timeout
            }
            
            let url_opt = {
                if let Ok(conn) = connectivity_clone.lock() {
                     if let Ok(guard) = conn.tunnel_url.lock() {
                         guard.clone()
                     } else { None }
                } else { None }
            };

            if let Some(url) = url_opt {
                println!("[Rust] Emitting URL to frontend: {}", url);
                let _ = app.emit("cloudflare-url", url);
                break;
            }
            
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });

    Ok(true)
}

/// Stop Cloudflare Tunnel
#[tauri::command]
fn stop_cloudflare_tunnel(state: State<TrackingState>) -> Result<bool, String> {
    let connectivity = state.rust_engine.connectivity.clone();
    let conn = connectivity.lock().map_err(|e| e.to_string())?;
    conn.stop_tunnel();
    Ok(true)
}

/// Get Tunnel URL QR Code (SVG)
#[tauri::command]
fn get_tunnel_qr(state: State<TrackingState>) -> Result<Option<String>, String> {
    let connectivity = state.rust_engine.connectivity.clone();
    let conn = connectivity.lock().map_err(|e| e.to_string())?;
    
    let url_guard = conn.tunnel_url.lock().map_err(|e| e.to_string())?;
    if let Some(url) = &*url_guard {
        // [FIX] Use Tunnel URL directly (Rust serves the UI)
        // No query param needed if index_handler serves the UI
        let full_url = url.clone();
        
        // Increase ECC level for better scanning
        let svg = conn.generate_qr(&full_url).map_err(|e| e.to_string())?;
        Ok(Some(svg))
    } else {
        Ok(None)
    }
}

/// Get Local IP Address
#[tauri::command]
fn get_local_ip() -> String {
    use local_ip_address::local_ip;
    match local_ip() {
        Ok(ip) => ip.to_string(),
        Err(_) => "127.0.0.1".to_string(),
    }
}

/// Generate QR Code for arbitrary text
#[tauri::command]
fn generate_qr_code(state: State<TrackingState>, data: String) -> Result<String, String> {
    let connectivity = state.rust_engine.connectivity.clone();
    let conn = connectivity.lock().map_err(|e| e.to_string())?;
    conn.generate_qr(&data).map_err(|e| e.to_string())
}

#[tauri::command]
fn start_calibration(state: State<TrackingState>, mode: String) -> Result<bool, String> {
    state.rust_engine.calibrate(mode);
    Ok(true)
}

#[tauri::command]
fn set_tracking_quality(state: State<TrackingState>, quality: String) -> Result<bool, String> {
    state.rust_engine.set_quality(quality);
    Ok(true)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(TrackingState::default())
        .setup(|app| {
            let state = app.state::<TrackingState>();
            let web_clone = state.rust_engine.web_interface.clone();
            let app_handle = app.handle().clone(); // [NEW] Clone handle
            tauri::async_runtime::spawn(async move {
                let mut guard = web_clone.lock().await;
                // [NEW] Pass app_handle
                if let Err(e) = guard.start(9001, app_handle).await {
                    println!("[Rust] Failed to start web server: {}", e);
                } else {
                    println!("[Rust] Web Server started on port 9001");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_cameras,
            start_tracking,
            stop_tracking,
            get_tracking_status,
            get_tracking_data, // [NEW]
            start_cloudflare_tunnel,
            stop_cloudflare_tunnel,
            get_tunnel_qr,
            get_local_ip,
            generate_qr_code,
            start_calibration,
            set_tracking_quality, // [NEW]
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
