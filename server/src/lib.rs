//! Health endpoints and a JSON WebSocket session gateway (KET-23, KET-24).

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::{IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    sessions: Arc<Mutex<HashSet<u64>>>,
    next_id: Arc<AtomicU64>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashSet::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }
}

impl AppState {
    pub async fn session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/ws", get(ws_upgrade))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn ready() -> Json<Value> {
    Json(json!({
        "status": "ready",
        "core": dab_core::crate_name(),
    }))
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

#[derive(Deserialize)]
struct ClientMsg {
    #[serde(rename = "type")]
    kind: String,
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let session_id = state.next_id.fetch_add(1, Ordering::Relaxed);
    state.sessions.lock().await.insert(session_id);
    tracing::info!(session_id, "ws connected");

    let hello = json!({ "type": "hello", "sessionId": session_id.to_string() });
    if socket
        .send(Message::Text(hello.to_string().into()))
        .await
        .is_err()
    {
        state.sessions.lock().await.remove(&session_id);
        return;
    }

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                let Ok(parsed) = serde_json::from_str::<ClientMsg>(text.as_str()) else {
                    continue;
                };
                if parsed.kind == "ping" {
                    let pong = json!({ "type": "pong" });
                    if socket
                        .send(Message::Text(pong.to_string().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
            Message::Ping(payload) => {
                if socket.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    state.sessions.lock().await.remove(&session_id);
    tracing::info!(session_id, "ws disconnected");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use futures_util::{SinkExt, StreamExt};
    use http_body_util::BodyExt;
    use tokio_tungstenite::tungstenite::Message as WsMessage;
    use tower::ServiceExt;

    #[test]
    fn core_game_constructs() {
        let geom = dab_core::BoardGeom::new(3, 3).expect("geom");
        let game = dab_core::Game::new(geom);
        assert!(!game.is_terminal());
        assert_eq!(dab_core::crate_name(), "dab-core");
    }

    #[tokio::test]
    async fn health_ok() {
        let app = router(AppState::default());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn ready_mentions_core() {
        let app = router(AppState::default());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["status"], "ready");
        assert_eq!(v["core"], "dab-core");
    }

    #[tokio::test]
    async fn ws_hello_ping_close_clears_registry() {
        let state = AppState::default();
        let app = router(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });

        let url = format!("ws://{addr}/ws");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("connect");

        let hello = ws.next().await.expect("hello frame").expect("hello ok");
        let WsMessage::Text(text) = hello else {
            panic!("expected text hello, got {hello:?}");
        };
        let parsed: Value = serde_json::from_str(text.as_str()).unwrap();
        assert_eq!(parsed["type"], "hello");
        assert!(parsed["sessionId"].as_str().is_some());
        assert_eq!(state.session_count().await, 1);

        ws.send(WsMessage::Text(r#"{"type":"ping"}"#.into()))
            .await
            .expect("send ping");
        let pong = ws.next().await.expect("pong frame").expect("pong ok");
        let WsMessage::Text(text) = pong else {
            panic!("expected text pong, got {pong:?}");
        };
        let parsed: Value = serde_json::from_str(text.as_str()).unwrap();
        assert_eq!(parsed["type"], "pong");

        ws.close(None).await.expect("close");
        while ws.next().await.is_some() {}

        let empty = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if state.session_count().await == 0 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(empty.is_ok(), "registry still held the session after close");
    }
}
