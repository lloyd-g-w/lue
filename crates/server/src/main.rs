mod model;
mod password;
mod persistence;
mod store;
mod utils;
mod ws;

use axum::routing::get;
use axum::Router;
use dotenvy::dotenv;
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
    dotenv().ok();

    let super_admin_name =
        env::var("SUPER_ADMIN_NAME").unwrap_or_else(|_| "Super Admin".to_string());
    let super_admin_email =
        env::var("SUPER_ADMIN_EMAIL").expect("SUPER_ADMIN_EMAIL must be set in .env");
    let super_admin_password =
        env::var("SUPER_ADMIN_PASSWORD").expect("SUPER_ADMIN_PASSWORD must be set in .env");
    let data_path = env::var("DATA_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/store.json"));

    let (updates, _) = broadcast::channel(128);
    let mut store = Store::load_from_disk(&data_path).expect("load persistent store from disk");
    store
        .bootstrap_super_admin(super_admin_name, super_admin_email, super_admin_password)
        .expect("bootstrap super admin from .env");
    store
        .save_to_disk(&data_path)
        .expect("save persistent store to disk");
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
