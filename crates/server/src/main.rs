mod auth;
mod model;
mod password;
mod persistence;
mod store;
mod utils;
mod ws;

use auth::{microsoft_auth_config, microsoft_callback_handler, microsoft_start_handler};
use axum::routing::get;
use axum::Router;
use model::{AppState, Store};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use ws::ws_handler;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let data_path = env::var("DATA_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/store.json"));

    let (updates, _) = broadcast::channel(128);
    let store = Store::load_from_disk(&data_path).expect("load persistent store from disk");
    let state = AppState {
        store: Arc::new(RwLock::new(store)),
        updates,
        data_path,
        microsoft_auth: microsoft_auth_config(),
        microsoft_auth_requests: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
        .route("/auth/microsoft/start", get(microsoft_start_handler))
        .route("/auth/microsoft/callback", get(microsoft_callback_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let listener = TcpListener::bind(&server_addr)
        .await
        .expect("bind backend listener");

    println!("server listening on http://{server_addr}");
    axum::serve(listener, app)
        .await
        .expect("serve axum application");
}
