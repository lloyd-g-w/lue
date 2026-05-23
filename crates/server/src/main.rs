mod model;
mod password;
mod persistence;
mod store;
mod utils;
mod ws;

use axum::routing::get;
use axum::Router;
use model::{AppState, Store};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use ws::ws_handler;

#[tokio::main]
async fn main() {
    let data_path = env::var("DATA_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/store.json"));

    let (updates, _) = broadcast::channel(128);
    let store = Store::load_from_disk(&data_path).expect("load persistent store from disk");
    let state = AppState {
        store: Arc::new(RwLock::new(store)),
        updates,
        data_path,
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind backend listener");

    println!("server listening on http://127.0.0.1:3000");
    axum::serve(listener, app)
        .await
        .expect("serve axum application");
}
