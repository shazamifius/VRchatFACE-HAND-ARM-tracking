// VRChat Bridge Hub - Rust Backend
// Manages tracking via Native Rust Engine

pub mod tracking;

// use std::sync::Mutex;
use tauri::State;
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
fn get_cameras(state: State<TrackingState>) -> Vec<CameraInfo> {
    let camera_manager = state.rust_engine.camera.lock().unwrap();
    camera_manager.get_cameras()
}

/// Start the tracking service
#[tauri::command]
async fn start_tracking(
    state: State<'_, TrackingState>,
    camera_index: i32,
    osc_ip: String,
    osc_port: i32,
) -> Result<bool, String> {
    // 1. Configure OSC
    if let Ok(mut connectivity) = state.rust_engine.connectivity.lock() {
        connectivity.set_osc_target(&osc_ip, osc_port as u16);
    }

    // 2. Start Engine
    state.rust_engine.start(camera_index as usize).map_err(|e| e.to_string())?;
    
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

/// Start Cloudflare Tunnel
#[tauri::command]
fn start_cloudflare_tunnel(state: State<TrackingState>, port: u16) -> Result<bool, String> {
    let connectivity = state.rust_engine.connectivity.clone();
    let conn = connectivity.lock().map_err(|e| e.to_string())?;
    conn.start_tunnel(port).map_err(|e| e.to_string())?;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(TrackingState::default())
        .invoke_handler(tauri::generate_handler![
            get_cameras,
            start_tracking,
            stop_tracking,
            get_tracking_status,
            start_cloudflare_tunnel,
            stop_cloudflare_tunnel,
            get_tunnel_qr,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
