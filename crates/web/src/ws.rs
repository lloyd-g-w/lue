use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use serde_json::from_str;
use shared::{AdminIdentityView, ClientMessage, ServerMessage, UserIdentityView};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{window, MessageEvent, WebSocket};

const WS_BACKEND_PORT: &str = "3000";
const RECONNECT_DELAY_MS: i32 = 1_500;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SocketStatus {
    Connecting,
    Connected,
    Reconnecting,
}

pub fn login_admin_socket(
    email: String,
    password: String,
    mut on_logged_in: impl FnMut(AdminIdentityView) + 'static,
    mut feedback: Signal<String>,
) {
    if email.trim().is_empty() || password.trim().is_empty() {
        feedback.set("Admin email and password are required".to_string());
        return;
    }

    login_socket(
        ClientMessage::LoginAdmin { email, password },
        move |message| match message {
            ServerMessage::AdminLoggedIn { admin } => on_logged_in(admin),
            ServerMessage::Error { message } => feedback.set(message),
            _ => {}
        },
        feedback,
    );
}

pub fn login_user_socket(
    email: String,
    password: String,
    mut on_logged_in: impl FnMut(UserIdentityView) + 'static,
    mut feedback: Signal<String>,
) {
    if email.trim().is_empty() || password.trim().is_empty() {
        feedback.set("User email and password are required".to_string());
        return;
    }

    login_socket(
        ClientMessage::LoginUser { email, password },
        move |message| match message {
            ServerMessage::UserLoggedIn { user } => on_logged_in(user),
            ServerMessage::Error { message } => feedback.set(message),
            _ => {}
        },
        feedback,
    );
}

fn login_socket(
    login_message: ClientMessage,
    mut on_message: impl FnMut(ServerMessage) + 'static,
    mut feedback: Signal<String>,
) {
    let Ok(ws) = WebSocket::new(&backend_ws_url()) else {
        feedback.set("Failed to create websocket".to_string());
        return;
    };

    let ws_for_open = ws.clone();
    let on_open = Closure::<dyn FnMut()>::new(move || {
        let _ = send_ws(&ws_for_open, &login_message);
    });
    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    on_open.forget();

    let on_message_handler = Closure::<dyn FnMut(MessageEvent)>::new(move |event| {
        if let Some(text) = extract_ws_text(event) {
            match from_str::<ServerMessage>(&text) {
                Ok(message) => on_message(message),
                Err(error) => feedback.set(format!("invalid server payload: {error}")),
            }
        }
    });
    ws.set_onmessage(Some(on_message_handler.as_ref().unchecked_ref()));
    on_message_handler.forget();
}

pub fn connect_reconnecting_socket(
    on_message: impl FnMut(ServerMessage) + 'static,
    on_open: impl FnMut(WebSocket) + 'static,
    on_status: impl FnMut(SocketStatus) + 'static,
) {
    let on_message = Rc::new(RefCell::new(
        Box::new(on_message) as Box<dyn FnMut(ServerMessage)>
    ));
    let on_open = Rc::new(RefCell::new(Box::new(on_open) as Box<dyn FnMut(WebSocket)>));
    let on_status = Rc::new(RefCell::new(
        Box::new(on_status) as Box<dyn FnMut(SocketStatus)>
    ));

    connect_reconnecting_attempt(on_message, on_open, on_status);
}

fn connect_reconnecting_attempt(
    on_message: Rc<RefCell<Box<dyn FnMut(ServerMessage)>>>,
    on_open: Rc<RefCell<Box<dyn FnMut(WebSocket)>>>,
    on_status: Rc<RefCell<Box<dyn FnMut(SocketStatus)>>>,
) {
    (on_status.borrow_mut())(SocketStatus::Connecting);

    let Ok(ws) = WebSocket::new(&backend_ws_url()) else {
        (on_status.borrow_mut())(SocketStatus::Reconnecting);
        schedule_reconnect(on_message, on_open, on_status);
        return;
    };

    let on_open_callback = on_open.clone();
    let on_status_callback = on_status.clone();
    let ws_for_open = ws.clone();
    let open_handler = Closure::<dyn FnMut()>::new(move || {
        (on_status_callback.borrow_mut())(SocketStatus::Connected);
        (on_open_callback.borrow_mut())(ws_for_open.clone());
    });
    ws.set_onopen(Some(open_handler.as_ref().unchecked_ref()));
    open_handler.forget();

    let on_message_callback = on_message.clone();
    let message_handler = Closure::<dyn FnMut(MessageEvent)>::new(move |event| {
        if let Some(text) = extract_ws_text(event) {
            if let Ok(message) = serde_json::from_str::<ServerMessage>(&text) {
                (on_message_callback.borrow_mut())(message);
            }
        }
    });
    ws.set_onmessage(Some(message_handler.as_ref().unchecked_ref()));
    message_handler.forget();

    let reconnect_message = on_message.clone();
    let reconnect_open = on_open.clone();
    let reconnect_status = on_status.clone();
    let close_handler = Closure::<dyn FnMut()>::new(move || {
        (reconnect_status.borrow_mut())(SocketStatus::Reconnecting);
        schedule_reconnect(
            reconnect_message.clone(),
            reconnect_open.clone(),
            reconnect_status.clone(),
        );
    });
    ws.set_onclose(Some(close_handler.as_ref().unchecked_ref()));
    close_handler.forget();
}

fn schedule_reconnect(
    on_message: Rc<RefCell<Box<dyn FnMut(ServerMessage)>>>,
    on_open: Rc<RefCell<Box<dyn FnMut(WebSocket)>>>,
    on_status: Rc<RefCell<Box<dyn FnMut(SocketStatus)>>>,
) {
    let reconnect_handler = Closure::<dyn FnMut()>::new(move || {
        connect_reconnecting_attempt(on_message.clone(), on_open.clone(), on_status.clone());
    });

    if let Some(window) = window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            reconnect_handler.as_ref().unchecked_ref(),
            RECONNECT_DELAY_MS,
        );
    }
    reconnect_handler.forget();
}

pub fn send_ws(socket: &WebSocket, message: &ClientMessage) -> Result<(), String> {
    let payload = serde_json::to_string(message).map_err(|error| error.to_string())?;
    socket
        .send_with_str(&payload)
        .map_err(|_| "failed to send websocket message".to_string())
}

fn extract_ws_text(event: MessageEvent) -> Option<String> {
    event.data().as_string().or_else(|| {
        event
            .data()
            .dyn_into::<js_sys::JsString>()
            .ok()
            .map(String::from)
    })
}

fn backend_ws_url() -> String {
    let window = window().expect("browser window");
    let location = window.location();
    let protocol = match location.protocol().ok().as_deref() {
        Some("https:") => "wss",
        _ => "ws",
    };
    let host = location
        .hostname()
        .ok()
        .filter(|host| !host.is_empty())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    format!("{protocol}://{host}:{WS_BACKEND_PORT}/ws")
}
