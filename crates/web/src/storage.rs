use uuid::Uuid;
use web_sys::window;

use crate::models::{AdminSessionRecord, UserSessionRecord};

const ADMIN_SESSION_KEY: &str = "admin-session";
const USER_SESSION_KEY: &str = "user-session";
const DARK_THEME_KEY: &str = "dark-theme";

fn storage_key(queue_id: Uuid) -> String {
    format!("queue-entry-token:{queue_id}")
}

pub fn save_entry_token(queue_id: Uuid, token: &str) {
    if let Some(storage) = window().and_then(|browser| browser.local_storage().ok().flatten()) {
        let _ = storage.set_item(&storage_key(queue_id), token);
    }
}

pub fn load_entry_token(queue_id: Uuid) -> Option<String> {
    window()
        .and_then(|browser| browser.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(&storage_key(queue_id)).ok().flatten())
}

pub fn clear_entry_token(queue_id: Uuid) {
    if let Some(storage) = window().and_then(|browser| browser.local_storage().ok().flatten()) {
        let _ = storage.remove_item(&storage_key(queue_id));
    }
}

pub fn save_admin_session(session: &AdminSessionRecord) {
    if let Some(storage) = window().and_then(|browser| browser.local_storage().ok().flatten()) {
        let _ = storage.set_item(
            ADMIN_SESSION_KEY,
            &serde_json::to_string(session).unwrap_or_default(),
        );
    }
}

pub fn load_admin_session() -> Option<AdminSessionRecord> {
    window()
        .and_then(|browser| browser.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(ADMIN_SESSION_KEY).ok().flatten())
        .and_then(|payload| serde_json::from_str(&payload).ok())
}

pub fn clear_admin_session() {
    if let Some(storage) = window().and_then(|browser| browser.local_storage().ok().flatten()) {
        let _ = storage.remove_item(ADMIN_SESSION_KEY);
    }
}

pub fn save_user_session(session: &UserSessionRecord) {
    if let Some(storage) = window().and_then(|browser| browser.local_storage().ok().flatten()) {
        let _ = storage.set_item(
            USER_SESSION_KEY,
            &serde_json::to_string(session).unwrap_or_default(),
        );
    }
}

pub fn load_user_session() -> Option<UserSessionRecord> {
    window()
        .and_then(|browser| browser.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(USER_SESSION_KEY).ok().flatten())
        .and_then(|payload| serde_json::from_str(&payload).ok())
}

pub fn clear_user_session() {
    if let Some(storage) = window().and_then(|browser| browser.local_storage().ok().flatten()) {
        let _ = storage.remove_item(USER_SESSION_KEY);
    }
}

pub fn save_dark_theme(enabled: bool) {
    if let Some(storage) = window().and_then(|browser| browser.local_storage().ok().flatten()) {
        let _ = storage.set_item(DARK_THEME_KEY, if enabled { "true" } else { "false" });
    }
}

pub fn load_dark_theme() -> Option<bool> {
    window()
        .and_then(|browser| browser.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(DARK_THEME_KEY).ok().flatten())
        .and_then(|value| match value.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        })
}
