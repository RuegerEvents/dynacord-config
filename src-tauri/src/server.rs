//! HTTP server: static UI, device discovery API and a WebSocket→TCP bridge for OCP.1.
//!
//! The browser speaks AES70 (OCP.1) over WebSocket via aes70.js; the bridge pipes the
//! bytes unchanged to the amp's OCA TCP port. aes70.js reassembles split PDUs itself.

use crate::discovery::Registry;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(RustEmbed)]
#[folder = "../dist"]
struct Assets;

/// 127.0.0.1:7070 by default. Set DYNACORD_LAN=1 to serve phones/tablets on the network,
/// DYNACORD_PORT to change the port.
pub fn bind_addr() -> SocketAddr {
    let port = std::env::var("DYNACORD_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7070);
    let ip = if std::env::var("DYNACORD_LAN").is_ok_and(|v| v == "1") {
        [0, 0, 0, 0]
    } else {
        [127, 0, 0, 1]
    };
    SocketAddr::from((ip, port))
}

pub async fn serve(listener: tokio::net::TcpListener) {
    let registry = Registry::default();
    registry.start();

    let app = Router::new()
        .route("/api/devices", get(devices))
        .route("/api/probe", get(probe))
        .route("/ws", get(ws_bridge))
        .fallback(static_file)
        .with_state(registry);

    axum::serve(listener, app).await.expect("web server");
}

async fn devices(State(reg): State<Registry>) -> impl IntoResponse {
    Json(reg.list())
}

#[derive(Deserialize)]
struct Target {
    host: String,
    port: Option<u16>,
}

async fn connect_tcp(t: &Target) -> Result<TcpStream, (StatusCode, String)> {
    let addr = format!("{}:{}", t.host, t.port.unwrap_or(55555));
    match tokio::time::timeout(std::time::Duration::from_secs(3), TcpStream::connect(&addr)).await {
        Ok(Ok(s)) => Ok(s),
        Ok(Err(e)) => Err((StatusCode::BAD_GATEWAY, format!("{addr}: {e}"))),
        Err(_) => Err((StatusCode::GATEWAY_TIMEOUT, format!("{addr}: timed out"))),
    }
}

/// Browsers hide why a WebSocket failed, so the UI asks here for the actual TCP error.
async fn probe(Query(t): Query<Target>) -> Response {
    match connect_tcp(&t).await {
        Ok(_) => "ok".into_response(),
        Err(e) => e.into_response(),
    }
}

async fn ws_bridge(ws: WebSocketUpgrade, Query(t): Query<Target>) -> Response {
    let tcp = match connect_tcp(&t).await {
        Ok(s) => s,
        Err(e) => return e.into_response(),
    };
    let _ = tcp.set_nodelay(true);
    ws.on_upgrade(move |socket| pipe(socket, tcp))
}

async fn pipe(socket: WebSocket, tcp: TcpStream) {
    let (mut tcp_rx, mut tcp_tx) = tcp.into_split();
    let (mut ws_tx, mut ws_rx) = socket.split();

    let up = async {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Binary(b) => {
                    if tcp_tx.write_all(&b).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    };
    let down = async {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match tcp_rx.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if ws_tx
                        .send(Message::Binary(buf[..n].to_vec().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
        let _ = ws_tx.close().await;
    };
    tokio::select! { _ = up => {}, _ = down => {} }
}

async fn static_file(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path).or_else(|| Assets::get("index.html")) {
        Some(f) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], f.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "UI not built; run `npm run build`").into_response(),
    }
}
