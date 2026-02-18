use axum::{
    extract::State,
    response::{IntoResponse, Response, Html},
    routing::{get, post},
    Router,
    http::{StatusCode, header},
    body::Body, // Re-added Body
};
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use anyhow::Result;
use bytes::Bytes;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use futures::stream::StreamExt; // Use futures for map
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub struct AppState {
    pub latest_frame: Arc<Mutex<Option<Vec<u8>>>>,
    pub input_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    pub tx: broadcast::Sender<Vec<u8>>,
    pub app_handle: Option<AppHandle>, // [NEW]
    pub last_emit: Arc<Mutex<std::time::Instant>>, // [NEW]
}

pub struct WebInterface {
    pub latest_frame: Arc<Mutex<Option<Vec<u8>>>>,
    pub input_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    pub tx: broadcast::Sender<Vec<u8>>,
    pub app_handle: Arc<Mutex<Option<AppHandle>>>, // [NEW]
}

impl WebInterface {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(16);
        Self {
            latest_frame: Arc::new(Mutex::new(None)),
            input_buffer: Arc::new(Mutex::new(None)),
            shutdown_tx: None,
            tx,
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(&mut self, port: u16, app_handle: AppHandle) -> Result<()> {
        // Store handle for emission
        if let Ok(mut guard) = self.app_handle.lock() {
            *guard = Some(app_handle.clone());
        }

        let state = AppState {
            latest_frame: self.latest_frame.clone(),
            input_buffer: self.input_buffer.clone(),
            tx: self.tx.clone(),
            app_handle: Some(app_handle),
            last_emit: Arc::new(Mutex::new(std::time::Instant::now())),
        };

        let app = Router::new()
            .route("/", get(index_handler))
            .route("/snapshot", get(snapshot_handler))
            .route("/stream", get(stream_handler))
            .route("/push", post(push_handler))
            .layer(CorsLayer::permissive())
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = TcpListener::bind(addr).await?;
        println!("[Rust] Web server listening on http://{}", addr);

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(tx);

        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
            {
                println!("[Rust] Server error: {}", e);
            }
        });

        Ok(())
    }

    pub fn update_frame(&self, jpeg_bytes: Vec<u8>) {
        // 1. Update latest helper (legacy)
        if let Ok(mut frame) = self.latest_frame.lock() {
            *frame = Some(jpeg_bytes.clone());
        }
        // 2. Broadcast to streamers
        // We ignore errors (if no receivers, that's fine)
        let _ = self.tx.send(jpeg_bytes);
    }
    
    pub fn get_input_frame(&self) -> Option<Vec<u8>> {
        if let Ok(guard) = self.input_buffer.lock() {
            guard.clone()
        } else {
            None
        }
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    // [NEW] Helper to emit tracking data
    pub fn emit_tracking_data(&self, data: &crate::tracking::types::TrackingData) {
        if let Ok(guard) = self.app_handle.lock() {
            if let Some(app) = &*guard {
                let _ = app.emit("tracking-data", data);
            }
        }
    }
}

async fn index_handler() -> Html<&'static str> {
    const HTML_CONTENT: &str = include_str!("../phone_sender.html");
    Html(HTML_CONTENT)
}

async fn snapshot_handler(State(state): State<AppState>) -> Response {
    let frame_data = {
        let guard = state.latest_frame.lock().unwrap();
        guard.clone()
    };

    match frame_data {
        Some(bytes) => (
            [
                ("Content-Type", "image/jpeg"),
                ("Content-Length", &bytes.len().to_string()),
                ("Access-Control-Allow-Origin", "*"),
            ],
            bytes,
        ).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            "No frame available",
        ).into_response(),
    }
}

// [NEW] MJPEG Stream Handler
async fn stream_handler(State(state): State<AppState>) -> impl IntoResponse {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx);



// ...

    let body_stream = stream.filter_map(|result| async move {
        match result {
            Ok(bytes) => {
                let header = format!(
                    "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                    bytes.len()
                );
                Some(Ok::<_, anyhow::Error>(axum::body::Bytes::from(
                    [header.into_bytes(), bytes, "\r\n".into()].concat()
                )))
            },
            Err(_) => {
                // Lagged or closed, skip silently
                None
            }
        }
    });

    // We must filter out empty bytes if lagged? 
    // Actually, Browsers handle MJPEG streams by boundary.
    // We should allow some yield.
    
    let body = Body::from_stream(body_stream);

    (
        [
            (header::CONTENT_TYPE, "multipart/x-mixed-replace; boundary=frame"),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
        ],
        body,
    )
}

async fn push_handler(State(state): State<AppState>, body: Bytes) -> StatusCode {
    if body.is_empty() {
        return StatusCode::BAD_REQUEST;
    }
    
    // [DEBUG] Log frame reception (Throttle log?)
    // println!("[Rust] Received Phone Frame: {} bytes", body.len());

    let frame_vec = body.to_vec();

    // 1. Update Input Buffer (For Tracking Engine)
    if let Ok(mut guard) = state.input_buffer.lock() {
        *guard = Some(frame_vec.clone());
    }

    // 2. Broadcast to Frontend /stream (For UI Preview)
    let _ = state.tx.send(frame_vec);

    // [NEW] Emit Event to Tauri (Throttled 1s)
    if let Some(app) = &state.app_handle {
        if let Ok(mut last) = state.last_emit.lock() {
            if last.elapsed().as_secs() >= 1 {
                let _ = app.emit("phone-connected", ());
                *last = std::time::Instant::now();
            }
        }
    }
    
    StatusCode::OK
}
