// HTTP + WebSocket server for Observatory metrics API.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use futures::{sink::SinkExt, stream::StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;

use crate::{AgentStatus, MetricsStore, PipelineTrace, SystemSnapshot};

/// Server config.
pub struct ServerConfig {
    pub port: u16,
}

/// Start metrics server as a background task.
pub fn start(store: Arc<MetricsStore>, cfg: ServerConfig) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let (tx, _) = broadcast::channel::<String>(256);
        store.init_broadcast(tx);

        let app = Router::new()
            .route("/api/agents", get(h_list_agents))
            .route("/api/agents/{id}", get(h_get_agent))
            .route("/api/metrics", get(h_get_metrics))
            .route("/api/pipeline", get(h_get_pipeline))
            .route("/ws", get(h_ws))
            .layer(CorsLayer::permissive())
            .layer(TimeoutLayer::new(Duration::from_secs(30)))
            .with_state(store);

        let addr = SocketAddr::from(([0, 0, 0, 0], cfg.port));
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to bind metrics server on port {}: {}", cfg.port, e);
                return;
            }
        };
        log::info!(
            "Observatory HTTP server listening on http://localhost:{}",
            cfg.port
        );
        axum::serve(listener, app).await.ok();
    })
}

// REST handlers
async fn h_list_agents(State(s): State<Arc<MetricsStore>>) -> Json<Vec<AgentStatus>> {
    Json(s.get_agents())
}

async fn h_get_agent(
    State(s): State<Arc<MetricsStore>>,
    Path(id): Path<String>,
) -> Result<Json<AgentStatus>, StatusCode> {
    s.get_agents()
        .into_iter()
        .find(|a| a.id == id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn h_get_metrics(State(s): State<Arc<MetricsStore>>) -> Json<Option<SystemSnapshot>> {
    Json(s.get_latest_snapshot())
}

async fn h_get_pipeline(State(s): State<Arc<MetricsStore>>) -> Json<Vec<PipelineTrace>> {
    Json(s.get_traces(50))
}

// WebSocket
async fn h_ws(ws: WebSocketUpgrade, State(s): State<Arc<MetricsStore>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| ws_handler(socket, s))
}

async fn ws_handler(socket: WebSocket, store: Arc<MetricsStore>) {
    let Some(mut rx) = store.subscribe() else {
        return;
    };
    let (mut sender, mut receiver): (
        futures::stream::SplitSink<WebSocket, Message>,
        futures::stream::SplitStream<WebSocket>,
    ) = socket.split();

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(json) => {
                        let text_msg: Message = Message::Text(json.into());
                        if sender.send(text_msg).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            incoming = receiver.next() => {
                if incoming.is_none() { break; }
            }
        }
    }
}
