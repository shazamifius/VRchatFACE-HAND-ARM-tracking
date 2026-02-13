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

#[derive(Clone)]
pub struct AppState {
    pub latest_frame: Arc<Mutex<Option<Vec<u8>>>>,
    pub input_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    pub tx: broadcast::Sender<Vec<u8>>, // [NEW] Broadcast channel
}

pub struct WebInterface {
    pub latest_frame: Arc<Mutex<Option<Vec<u8>>>>,
    pub input_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    pub tx: broadcast::Sender<Vec<u8>>, // [NEW]
}

impl WebInterface {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(16); // Capacity 16 frames
        Self {
            latest_frame: Arc::new(Mutex::new(None)),
            input_buffer: Arc::new(Mutex::new(None)),
            shutdown_tx: None,
            tx,
        }
    }

    pub async fn start(&mut self, port: u16) -> Result<()> {
        let state = AppState {
            latest_frame: self.latest_frame.clone(),
            input_buffer: self.input_buffer.clone(),
            tx: self.tx.clone(),
        };

        let app = Router::new()
            .route("/", get(index_handler))
            .route("/snapshot", get(snapshot_handler))
            .route("/stream", get(stream_handler)) // [NEW]
            .route("/push", post(push_handler))
            .layer(CorsLayer::permissive())
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = TcpListener::bind(addr).await?;
        println!("[Rust] Web server listening on {}", addr);

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

    let body_stream = stream.map(|result| {
        match result {
            Ok(bytes) => {
                let header = format!(
                    "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                    bytes.len()
                );
                Ok::<_, anyhow::Error>(axum::body::Bytes::from(
                    [header.into_bytes(), bytes, "\r\n".into()].concat()
                ))
            },
            Err(_) => {
                // Lagged or closed, skip
                Ok(axum::body::Bytes::new())
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
    
    if let Ok(mut guard) = state.input_buffer.lock() {
        *guard = Some(body.to_vec());
    }
    
    StatusCode::OK
}
