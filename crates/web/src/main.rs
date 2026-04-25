use std::collections::BTreeMap;

use dioxus::prelude::*;
use serde_json::from_str;
use shared::{
    AdminEntryView, AdminQueueView, ClientMessage, QueueEntryStatus, QueueField, ServerMessage,
    UserEntryView, UserQueueView,
};
use uuid::Uuid;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{window, MessageEvent, WebSocket};

const WS_BACKEND_PORT: &str = "3000";

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, PartialEq)]
enum Route {
    Home,
    Admin { token: String },
    Queue { queue_id: String },
}

impl Route {
    fn current() -> Self {
        let path = window()
            .and_then(|browser| browser.location().pathname().ok())
            .unwrap_or_else(|| "/".to_string());

        let parts: Vec<_> = path.trim_matches('/').split('/').collect();
        match parts.as_slice() {
            ["admin", token] if !token.is_empty() => Route::Admin {
                token: token.to_string(),
            },
            ["queue", queue_id] if !queue_id.is_empty() => Route::Queue {
                queue_id: queue_id.to_string(),
            },
            _ => Route::Home,
        }
    }

    fn path(&self) -> String {
        match self {
            Route::Home => "/".to_string(),
            Route::Admin { token } => format!("/admin/{token}"),
            Route::Queue { queue_id } => format!("/queue/{queue_id}"),
        }
    }
}

#[component]
fn App() -> Element {
    let route = use_signal(Route::current);

    rsx! {
        document::Stylesheet { href: "https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;700&display=swap" }
        style { {APP_CSS} }
        div { class: "shell",
            match route() {
                Route::Home => rsx! { HomePage { route } },
                Route::Admin { token } => rsx! { AdminPage { route, token } },
                Route::Queue { queue_id } => rsx! { QueuePage { queue_id } },
            }
        }
    }
}

#[component]
fn HomePage(route: Signal<Route>) -> Element {
    let mut queue_name = use_signal(|| "Student Support".to_string());
    let mut fields = use_signal(|| vec![EditableField::new("Name"), EditableField::new("Subject")]);
    let feedback = use_signal(String::new);

    let create_queue = {
        let queue_name = queue_name;
        let fields = fields;
        let mut feedback = feedback;
        let route = route;
        move |_| {
            let fields_snapshot = fields();
            let outbound_fields: Vec<QueueField> = fields_snapshot
                .iter()
                .map(|field| QueueField {
                    key: slugify(&field.label),
                    label: field.label.clone(),
                    required: true,
                })
                .collect();

            feedback.set("Creating queue...".to_string());
            create_queue_socket(
                queue_name(),
                outbound_fields,
                move |queue_id: Uuid, admin_token: String, _queue_name: String| {
                    feedback.set(String::new());
                    save_last_created_queue(queue_id);
                    navigate(route, Route::Admin { token: admin_token });
                },
                feedback,
            );
        }
    };

    rsx! {
        div { class: "card hero",
            div { class: "eyebrow", "Admin Setup" }
            h1 { "Create a live queue" }
            p { class: "lede",
                "Define the fields users must fill in, then share the generated queue link. Admin and user views update live over WebSockets."
            }
        }
        div { class: "grid two-up",
            div { class: "card",
                h2 { "Queue Settings" }
                label { class: "label", "Queue name" }
                input {
                    class: "input",
                    value: "{queue_name}",
                    oninput: move |event| queue_name.set(event.value()),
                    placeholder: "Student Support"
                }
                div { class: "spacer" }
                h3 { "Required fields" }
                for (index, field) in fields().iter().enumerate() {
                    div { class: "field-row",
                        input {
                            class: "input",
                            value: "{field.label}",
                            oninput: move |event| {
                                let mut next = fields();
                                if let Some(item) = next.get_mut(index) {
                                    item.label = event.value();
                                }
                                fields.set(next);
                            },
                            placeholder: "Field label"
                        }
                        button {
                            class: "ghost-button",
                            onclick: move |_| {
                                let mut next = fields();
                                if next.len() > 1 {
                                    next.remove(index);
                                    fields.set(next);
                                }
                            },
                            "Remove"
                        }
                    }
                }
                div { class: "button-row",
                    button {
                        class: "ghost-button",
                        onclick: move |_| {
                            let mut next = fields();
                            next.push(EditableField::new("Notes"));
                            fields.set(next);
                        },
                        "Add field"
                    }
                    button { class: "button", onclick: create_queue, "Create queue" }
                }
                if !feedback().is_empty() {
                    p { class: "feedback", "{feedback}" }
                }
            }
            div { class: "card",
                h2 { "Queue flow" }
                ol { class: "flow-list",
                    li { "Create the queue and open the admin dashboard." }
                    li { "Send the queue link to users." }
                    li { "Watch entries arrive live, then claim, resolve, or deny them." }
                }
                if let Some(queue_id) = load_last_created_queue() {
                    div { class: "hint",
                        "Last created queue id: "
                        code { "{queue_id}" }
                    }
                }
            }
        }
    }
}

#[component]
fn AdminPage(route: Signal<Route>, token: String) -> Element {
    let admin_state = use_signal(|| None::<AdminQueueView>);
    let feedback = use_signal(String::new);
    let socket = use_signal(|| None::<WebSocket>);
    let mut selected_entry = use_signal(|| None::<Uuid>);

    let subscribe_token = token.clone();
    use_effect(move || {
        let mut feedback = feedback;
        let mut admin_state = admin_state;
        let mut socket = socket;
        let mut selected_entry = selected_entry;
        let token = subscribe_token.clone();

        let ws = connect_socket(
            move |message| match message {
                ServerMessage::AdminState { queue } => {
                    if selected_entry().is_some_and(|entry_id| {
                        !queue.entries.iter().any(|entry| entry.id == entry_id)
                    }) {
                        selected_entry.set(None);
                    }
                    admin_state.set(Some(queue));
                    feedback.set(String::new());
                }
                ServerMessage::Error { message } => feedback.set(message),
                ServerMessage::Info { message } => feedback.set(message),
                _ => {}
            },
            move |ws| {
                let _ = send_ws(
                    &ws,
                    &ClientMessage::SubscribeAdmin {
                        admin_token: token.clone(),
                    },
                );
                socket.set(Some(ws));
            },
            move || feedback.set("WebSocket disconnected".to_string()),
        );

        if let Err(message) = ws {
            feedback.set(message);
        }
    });

    let queue_link = admin_state()
        .as_ref()
        .map(|queue| {
            frontend_url(&Route::Queue {
                queue_id: queue.summary.id.to_string(),
            })
        })
        .unwrap_or_default();

    let claim_entry = {
        let token = token.clone();
        let socket = socket;
        move |entry_id: Uuid| {
            if let Some(ws) = socket() {
                let _ = send_ws(
                    &ws,
                    &ClientMessage::ClaimEntry {
                        admin_token: token.clone(),
                        entry_id,
                    },
                );
            }
        }
    };

    let resolve_entry = {
        let token = token.clone();
        let socket = socket;
        move |entry_id: Uuid| {
            if let Some(ws) = socket() {
                let _ = send_ws(
                    &ws,
                    &ClientMessage::ResolveEntry {
                        admin_token: token.clone(),
                        entry_id,
                    },
                );
            }
        }
    };

    let deny_entry = {
        let token = token.clone();
        let socket = socket;
        move |entry_id: Uuid| {
            if let Some(ws) = socket() {
                let _ = send_ws(
                    &ws,
                    &ClientMessage::DenyEntry {
                        admin_token: token.clone(),
                        entry_id,
                    },
                );
            }
        }
    };

    let admin_snapshot = admin_state();
    rsx! {
        div { class: "toolbar",
            button {
                class: "ghost-button",
                onclick: move |_| navigate(route, Route::Home),
                "New queue"
            }
            a {
                class: "ghost-link",
                href: frontend_url(&Route::Admin { token: token.clone() }),
                "Admin link"
            }
            if !queue_link.is_empty() {
                a { class: "button", href: queue_link.clone(), "Open queue link" }
            }
        }

        if let Some(queue) = admin_snapshot {
            div { class: "grid two-up",
                div { class: "card",
                    div { class: "eyebrow", "Admin Dashboard" }
                    h1 { "{queue.summary.name}" }
                    p { class: "lede",
                        "{queue.summary.waiting_count} waiting, {queue.summary.active_count} active"
                    }
                    div { class: "hint",
                        "Queue link: "
                        a { href: queue_link.clone(), "{queue_link}" }
                    }
                    div { class: "entry-list",
                        for entry in queue.entries.iter().cloned() {
                            button {
                                class: if Some(entry.id) == selected_entry() { "entry-card selected" } else { "entry-card" },
                                onclick: move |_| selected_entry.set(Some(entry.id)),
                                div { class: "entry-head",
                                    span { class: status_class(&entry.status), "{status_label(&entry.status)}" }
                                    span { "{entry.submitted_at}" }
                                }
                                p { class: "entry-title",
                                    "{primary_field(queue.fields.as_slice(), &entry)}"
                                }
                                p { class: "entry-subtitle",
                                    "{secondary_field(queue.fields.as_slice(), &entry)}"
                                }
                            }
                        }
                    }
                }
                div { class: "card detail-card",
                    if let Some(selected_id) = selected_entry() {
                        if let Some(entry) = queue.entries.iter().find(|entry| entry.id == selected_id).cloned() {
                            h2 { "Request details" }
                            div { class: "detail-grid",
                                for field in queue.fields.iter().cloned() {
                                    div { class: "detail-item",
                                        span { class: "label", "{field.label}" }
                                        p { "{entry.values.get(&field.key).cloned().unwrap_or_default()}" }
                                    }
                                }
                            }
                            div { class: "button-row",
                                button {
                                    class: "button",
                                    disabled: !matches!(entry.status, QueueEntryStatus::Pending),
                                    onclick: move |_| claim_entry(entry.id),
                                    "Claim"
                                }
                                button {
                                    class: "button success",
                                    disabled: !matches!(entry.status, QueueEntryStatus::Pending | QueueEntryStatus::Claimed),
                                    onclick: move |_| resolve_entry(entry.id),
                                    "Resolve"
                                }
                                button {
                                    class: "button danger",
                                    disabled: !matches!(entry.status, QueueEntryStatus::Pending | QueueEntryStatus::Claimed),
                                    onclick: move |_| deny_entry(entry.id),
                                    "Deny"
                                }
                            }
                        } else {
                            p { class: "lede", "Select an entry to inspect it." }
                        }
                    } else {
                        div { class: "empty-state",
                            h2 { "No entry selected" }
                            p { "Pick a request from the left to inspect the submitted values and act on it." }
                        }
                    }
                }
            }
        } else {
            div { class: "card",
                h1 { "Loading admin dashboard..." }
                if !feedback().is_empty() {
                    p { class: "feedback", "{feedback}" }
                }
            }
        }
    }
}

#[component]
fn QueuePage(queue_id: String) -> Element {
    let queue_state = use_signal(|| None::<UserQueueView>);
    let your_entry = use_signal(|| None::<UserEntryView>);
    let feedback = use_signal(String::new);
    let mut form_values = use_signal(BTreeMap::<String, String>::new);
    let socket = use_signal(|| None::<WebSocket>);

    let parsed_queue_id = Uuid::parse_str(&queue_id).ok();
    if parsed_queue_id.is_none() {
        return rsx! {
            div { class: "card",
                h1 { "Invalid queue link" }
                p { "The queue id in the URL is not valid." }
            }
        };
    }
    let parsed_queue_id = parsed_queue_id.expect("validated queue id");

    use_effect(move || {
        let mut queue_state = queue_state;
        let mut your_entry = your_entry;
        let mut feedback = feedback;
        let mut socket = socket;
        let mut form_values = form_values;
        let queue_id = parsed_queue_id;

        let existing_token = load_entry_token(queue_id);

        let ws = connect_socket(
            move |message| match message {
                ServerMessage::QueueState {
                    queue,
                    your_entry: entry,
                } => {
                    if let Some(entry) = entry.as_ref() {
                        save_entry_token(queue.id, &entry.token);
                    } else {
                        clear_entry_token(queue.id);
                    }

                    if form_values().is_empty() {
                        let mut initial = BTreeMap::new();
                        for field in &queue.fields {
                            initial.insert(field.key.clone(), String::new());
                        }
                        form_values.set(initial);
                    }

                    queue_state.set(Some(queue));
                    your_entry.set(entry);
                    feedback.set(String::new());
                }
                ServerMessage::Error { message } => feedback.set(message),
                ServerMessage::Info { message } => feedback.set(message),
                _ => {}
            },
            move |ws| {
                let _ = send_ws(
                    &ws,
                    &ClientMessage::SubscribeQueue {
                        queue_id,
                        entry_token: existing_token.clone(),
                    },
                );
                socket.set(Some(ws));
            },
            move || feedback.set("WebSocket disconnected".to_string()),
        );

        if let Err(message) = ws {
            feedback.set(message);
        }
    });

    let join_queue = {
        let queue_state = queue_state;
        let form_values = form_values;
        let socket = socket;
        move |_| {
            if let Some(queue) = queue_state() {
                if let Some(ws) = socket() {
                    let _ = send_ws(
                        &ws,
                        &ClientMessage::JoinQueue {
                            queue_id: queue.id,
                            values: form_values(),
                        },
                    );
                }
            }
        }
    };

    let leave_queue = {
        let queue_state = queue_state;
        let your_entry = your_entry;
        let socket = socket;
        move |_| {
            if let (Some(queue), Some(entry), Some(ws)) = (queue_state(), your_entry(), socket()) {
                let _ = send_ws(
                    &ws,
                    &ClientMessage::LeaveQueue {
                        queue_id: queue.id,
                        entry_token: entry.token,
                    },
                );
            }
        }
    };

    let user_queue_snapshot = queue_state();
    rsx! {
        if let Some(queue) = user_queue_snapshot {
            div { class: "grid two-up",
                div { class: "card",
                    div { class: "eyebrow", "Queue" }
                    h1 { "{queue.name}" }
                    p { class: "lede", "{queue.waiting_count} people waiting" }
                    if let Some(entry) = your_entry() {
                        p { class: "status-pill",
                            "Your status: {status_label(&entry.status)}"
                            if let Some(position) = entry.position {
                                " (position {position})"
                            }
                        }
                        button { class: "button danger", onclick: leave_queue, "Leave queue" }
                    } else {
                        p { class: "hint", "Fill in your details to join the queue." }
                    }
                }
                div { class: "card",
                    if your_entry().is_none() {
                        h2 { "Join the queue" }
                        for field in queue.fields.iter().cloned() {
                            div { class: "input-group",
                                label { class: "label", "{field.label}" }
                                input {
                                    class: "input",
                                    value: "{form_values().get(&field.key).cloned().unwrap_or_default()}",
                                    oninput: move |event| {
                                        let mut next = form_values();
                                        next.insert(field.key.clone(), event.value());
                                        form_values.set(next);
                                    },
                                    placeholder: "{field.label}"
                                }
                            }
                        }
                        button { class: "button", onclick: join_queue, "Join queue" }
                    } else if let Some(entry) = your_entry() {
                        h2 { "Submitted details" }
                        div { class: "detail-grid",
                            for field in queue.fields.iter().cloned() {
                                div { class: "detail-item",
                                    span { class: "label", "{field.label}" }
                                    p { "{entry.values.get(&field.key).cloned().unwrap_or_default()}" }
                                }
                            }
                        }
                    }
                    if !feedback().is_empty() {
                        p { class: "feedback", "{feedback}" }
                    }
                }
            }
        } else {
            div { class: "card",
                h1 { "Connecting to queue..." }
                if !feedback().is_empty() {
                    p { class: "feedback", "{feedback}" }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct EditableField {
    label: String,
}

impl EditableField {
    fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}

fn create_queue_socket(
    name: String,
    fields: Vec<QueueField>,
    mut on_created: impl FnMut(Uuid, String, String) + 'static,
    mut feedback: Signal<String>,
) {
    if name.trim().is_empty() {
        feedback.set("Queue name is required".to_string());
        return;
    }

    if fields.iter().any(|field| field.label.trim().is_empty()) {
        feedback.set("Every field needs a label".to_string());
        return;
    }

    let Ok(ws) = WebSocket::new(&backend_ws_url()) else {
        feedback.set("Failed to create websocket".to_string());
        return;
    };

    let mut feedback_for_open = feedback;
    let fields_for_open = fields.clone();
    let name_for_open = name.clone();
    let ws_for_open = ws.clone();
    let on_open = Closure::<dyn FnMut()>::new(move || {
        feedback_for_open.set("Queue created. Redirecting...".to_string());
        let _ = send_ws(
            &ws_for_open,
            &ClientMessage::CreateQueue {
                name: name_for_open.clone(),
                fields: fields_for_open.clone(),
            },
        );
    });
    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    on_open.forget();

    let mut feedback_for_message = feedback;
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event| {
        if let Some(text) = extract_ws_text(event) {
            match from_str::<ServerMessage>(&text) {
                Ok(ServerMessage::QueueCreated {
                    queue_id,
                    admin_token,
                    queue_name,
                }) => on_created(queue_id, admin_token, queue_name),
                Ok(ServerMessage::Error { message }) => feedback_for_message.set(message),
                Ok(_) => {}
                Err(error) => feedback_for_message.set(format!("invalid server payload: {error}")),
            }
        }
    });
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
}

fn connect_socket(
    mut on_message: impl FnMut(ServerMessage) + 'static,
    mut on_open: impl FnMut(WebSocket) + 'static,
    on_close: impl FnMut() + 'static,
) -> Result<(), String> {
    let ws =
        WebSocket::new(&backend_ws_url()).map_err(|_| "Failed to create websocket".to_string())?;

    let ws_for_open = ws.clone();
    let open_handler = Closure::<dyn FnMut()>::new(move || on_open(ws_for_open.clone()));
    ws.set_onopen(Some(open_handler.as_ref().unchecked_ref()));
    open_handler.forget();

    let message_handler = Closure::<dyn FnMut(MessageEvent)>::new(move |event| {
        if let Some(text) = extract_ws_text(event) {
            if let Ok(message) = serde_json::from_str::<ServerMessage>(&text) {
                on_message(message);
            }
        }
    });
    ws.set_onmessage(Some(message_handler.as_ref().unchecked_ref()));
    message_handler.forget();

    let close_handler = Closure::<dyn FnMut()>::new(on_close);
    ws.set_onclose(Some(close_handler.as_ref().unchecked_ref()));
    close_handler.forget();

    Ok(())
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

fn send_ws(socket: &WebSocket, message: &ClientMessage) -> Result<(), String> {
    let payload = serde_json::to_string(message).map_err(|error| error.to_string())?;
    socket
        .send_with_str(&payload)
        .map_err(|_| "failed to send websocket message".to_string())
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

fn frontend_url(route: &Route) -> String {
    let location = window().expect("browser window").location();
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let host = location
        .host()
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    format!("{protocol}//{host}{}", route.path())
}

fn navigate(mut route_signal: Signal<Route>, route: Route) {
    if let Some(browser) = window() {
        let _ = browser.history().and_then(|history| {
            history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&route.path()))
        });
    }
    route_signal.set(route);
}

fn slugify(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .replace("__", "_")
}

fn status_label(status: &QueueEntryStatus) -> &'static str {
    match status {
        QueueEntryStatus::Pending => "Pending",
        QueueEntryStatus::Claimed => "Claimed",
        QueueEntryStatus::Resolved => "Resolved",
        QueueEntryStatus::Denied => "Denied",
    }
}

fn status_class(status: &QueueEntryStatus) -> &'static str {
    match status {
        QueueEntryStatus::Pending => "badge badge-pending",
        QueueEntryStatus::Claimed => "badge badge-claimed",
        QueueEntryStatus::Resolved => "badge badge-resolved",
        QueueEntryStatus::Denied => "badge badge-denied",
    }
}

fn primary_field(fields: &[QueueField], entry: &AdminEntryView) -> String {
    fields
        .first()
        .and_then(|field| entry.values.get(&field.key))
        .cloned()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Unnamed request".to_string())
}

fn secondary_field(fields: &[QueueField], entry: &AdminEntryView) -> String {
    fields
        .get(1)
        .and_then(|field| entry.values.get(&field.key))
        .cloned()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "No extra subject".to_string())
}

fn storage_key(queue_id: Uuid) -> String {
    format!("queue-entry-token:{queue_id}")
}

fn save_entry_token(queue_id: Uuid, token: &str) {
    if let Some(storage) = window().and_then(|browser| browser.local_storage().ok().flatten()) {
        let _ = storage.set_item(&storage_key(queue_id), token);
    }
}

fn load_entry_token(queue_id: Uuid) -> Option<String> {
    window()
        .and_then(|browser| browser.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(&storage_key(queue_id)).ok().flatten())
}

fn clear_entry_token(queue_id: Uuid) {
    if let Some(storage) = window().and_then(|browser| browser.local_storage().ok().flatten()) {
        let _ = storage.remove_item(&storage_key(queue_id));
    }
}

fn save_last_created_queue(queue_id: Uuid) {
    if let Some(storage) = window().and_then(|browser| browser.local_storage().ok().flatten()) {
        let _ = storage.set_item("last-created-queue", &queue_id.to_string());
    }
}

fn load_last_created_queue() -> Option<String> {
    window()
        .and_then(|browser| browser.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item("last-created-queue").ok().flatten())
}

const APP_CSS: &str = r#"
:root {
  --bg: #f3eee4;
  --surface: rgba(255, 250, 244, 0.86);
  --surface-strong: #fff8ef;
  --ink: #1d1b18;
  --muted: #695f55;
  --line: rgba(29, 27, 24, 0.12);
  --accent: #0f766e;
  --accent-soft: rgba(15, 118, 110, 0.14);
  --success: #2b8a3e;
  --danger: #b02a37;
  --shadow: 0 20px 50px rgba(80, 54, 28, 0.14);
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  font-family: "Space Grotesk", sans-serif;
  color: var(--ink);
  background:
    radial-gradient(circle at top left, rgba(15, 118, 110, 0.18), transparent 30%),
    radial-gradient(circle at top right, rgba(176, 42, 55, 0.12), transparent 35%),
    linear-gradient(180deg, #f6f0e7 0%, #ede4d6 100%);
}

a {
  color: inherit;
}

.shell {
  max-width: 1200px;
  margin: 0 auto;
  padding: 32px 20px 64px;
}

.grid {
  display: grid;
  gap: 20px;
}

.two-up {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.card {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 24px;
  padding: 24px;
  box-shadow: var(--shadow);
  backdrop-filter: blur(10px);
}

.hero {
  margin-bottom: 20px;
}

.toolbar,
.button-row,
.field-row {
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
}

.toolbar {
  justify-content: space-between;
  margin-bottom: 20px;
}

.eyebrow,
.label {
  display: inline-block;
  font-size: 0.82rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.lede,
.feedback,
.hint {
  color: var(--muted);
  line-height: 1.6;
}

.input-group {
  margin-bottom: 14px;
}

.input,
.button,
.ghost-button,
.ghost-link {
  font: inherit;
}

.input {
  width: 100%;
  padding: 14px 16px;
  border-radius: 14px;
  border: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.9);
}

.button,
.ghost-button,
.ghost-link,
.entry-card {
  border: none;
  border-radius: 16px;
  padding: 12px 16px;
  text-decoration: none;
  cursor: pointer;
  transition: transform 120ms ease, background 120ms ease;
}

.button:hover,
.ghost-button:hover,
.entry-card:hover {
  transform: translateY(-1px);
}

.button {
  background: var(--accent);
  color: white;
}

.button.success {
  background: var(--success);
}

.button.danger {
  background: var(--danger);
}

.ghost-button,
.ghost-link {
  background: var(--accent-soft);
  color: var(--accent);
}

.entry-list {
  display: grid;
  gap: 12px;
  margin-top: 20px;
}

.entry-card {
  width: 100%;
  text-align: left;
  background: rgba(255, 255, 255, 0.72);
  border: 1px solid transparent;
}

.entry-card.selected {
  border-color: var(--accent);
  background: white;
}

.entry-head {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
  color: var(--muted);
  font-size: 0.9rem;
}

.entry-title {
  margin: 0 0 6px;
  font-size: 1.08rem;
  font-weight: 700;
}

.entry-subtitle {
  margin: 0;
  color: var(--muted);
}

.badge,
.status-pill {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 7px 12px;
  border-radius: 999px;
  font-size: 0.84rem;
  font-weight: 700;
}

.badge-pending,
.status-pill {
  background: rgba(180, 108, 16, 0.14);
  color: #8f5f0e;
}

.badge-claimed {
  background: rgba(15, 118, 110, 0.14);
  color: var(--accent);
}

.badge-resolved {
  background: rgba(43, 138, 62, 0.16);
  color: var(--success);
}

.badge-denied {
  background: rgba(176, 42, 55, 0.16);
  color: var(--danger);
}

.detail-grid {
  display: grid;
  gap: 12px;
  margin: 20px 0;
}

.detail-item {
  padding: 14px 16px;
  border-radius: 16px;
  background: var(--surface-strong);
  border: 1px solid var(--line);
}

.detail-item p {
  margin: 8px 0 0;
}

.flow-list {
  margin: 0;
  padding-left: 18px;
  line-height: 1.8;
}

.empty-state,
.detail-card {
  min-height: 320px;
}

.spacer {
  height: 16px;
}

@media (max-width: 900px) {
  .two-up {
    grid-template-columns: 1fr;
  }
}
"#;
