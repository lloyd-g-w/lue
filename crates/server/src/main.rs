use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use shared::{
    AdminEntryView, AdminQueueView, ClientMessage, QueueEntryStatus, QueueField, QueueSummary,
    ServerMessage, UserEntryView, UserQueueView,
};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    store: Arc<RwLock<Store>>,
    updates: broadcast::Sender<Uuid>,
}

#[derive(Default)]
struct Store {
    queues: HashMap<Uuid, Queue>,
    admin_index: HashMap<String, Uuid>,
}

struct Queue {
    id: Uuid,
    name: String,
    admin_token: String,
    fields: Vec<QueueField>,
    entries: Vec<QueueEntry>,
}

struct QueueEntry {
    id: Uuid,
    token: String,
    values: BTreeMap<String, String>,
    submitted_at: String,
    status: QueueEntryStatus,
}

#[tokio::main]
async fn main() {
    let (updates, _) = broadcast::channel(128);
    let state = AppState {
        store: Arc::new(RwLock::new(Store::default())),
        updates,
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

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut updates = state.updates.subscribe();
    let mut admin_subscription: Option<String> = None;
    let mut queue_subscription: Option<(Uuid, Option<String>)> = None;

    loop {
        tokio::select! {
            message = receiver.next() => {
                let Some(Ok(message)) = message else {
                    break;
                };

                if let Message::Text(text) = message {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(command) => {
                            match process_command(&state, command, &mut admin_subscription, &mut queue_subscription, &mut sender).await {
                                Ok(Some(queue_id)) => {
                                    let _ = state.updates.send(queue_id);
                                }
                                Ok(None) => {}
                                Err(message) => {
                                    if send_message(&mut sender, &ServerMessage::Error { message }).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            let _ = send_message(&mut sender, &ServerMessage::Error {
                                message: format!("invalid websocket message: {error}"),
                            })
                            .await;
                        }
                    }
                }
            }
            updated = updates.recv() => {
                let Ok(queue_id) = updated else {
                    continue;
                };

                if let Some(admin_token) = admin_subscription.as_deref() {
                    let store = state.store.read().await;
                    if store.queue_id_for_admin(admin_token) == Some(queue_id) {
                        if let Some(view) = store.admin_view(admin_token) {
                            if send_message(&mut sender, &ServerMessage::AdminState { queue: view }).await.is_err() {
                                break;
                            }
                        }
                    }
                }

                if let Some((subscribed_queue_id, entry_token)) = queue_subscription.as_ref() {
                    if *subscribed_queue_id == queue_id {
                        let store = state.store.read().await;
                        if let Some((queue, your_entry)) =
                            store.user_view(*subscribed_queue_id, entry_token.as_deref())
                        {
                            if send_message(&mut sender, &ServerMessage::QueueState { queue, your_entry }).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn process_command(
    state: &AppState,
    command: ClientMessage,
    admin_subscription: &mut Option<String>,
    queue_subscription: &mut Option<(Uuid, Option<String>)>,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> Result<Option<Uuid>, String> {
    match command {
        ClientMessage::CreateQueue { name, fields } => {
            let mut store = state.store.write().await;
            let (queue_id, queue_name, admin_token) = store.create_queue(name, fields)?;
            *admin_subscription = Some(admin_token.clone());

            send_message(
                sender,
                &ServerMessage::QueueCreated {
                    queue_id,
                    admin_token: admin_token.clone(),
                    queue_name,
                },
            )
            .await
            .map_err(|error| error.to_string())?;

            if let Some(view) = store.admin_view(&admin_token) {
                send_message(sender, &ServerMessage::AdminState { queue: view })
                    .await
                    .map_err(|error| error.to_string())?;
            }

            Ok(Some(queue_id))
        }
        ClientMessage::SubscribeAdmin { admin_token } => {
            let store = state.store.read().await;
            let Some(view) = store.admin_view(&admin_token) else {
                return Err("unknown admin token".to_string());
            };

            *admin_subscription = Some(admin_token);
            send_message(sender, &ServerMessage::AdminState { queue: view })
                .await
                .map_err(|error| error.to_string())?;
            Ok(None)
        }
        ClientMessage::SubscribeQueue {
            queue_id,
            entry_token,
        } => {
            let store = state.store.read().await;
            let Some((queue, your_entry)) = store.user_view(queue_id, entry_token.as_deref())
            else {
                return Err("unknown queue".to_string());
            };

            *queue_subscription = Some((queue_id, entry_token));
            send_message(sender, &ServerMessage::QueueState { queue, your_entry })
                .await
                .map_err(|error| error.to_string())?;
            Ok(None)
        }
        ClientMessage::JoinQueue { queue_id, values } => {
            let mut store = state.store.write().await;
            let token = store.join_queue(queue_id, values)?;
            *queue_subscription = Some((queue_id, Some(token)));

            if let Some((queue, your_entry)) = store.user_view(
                queue_id,
                queue_subscription
                    .as_ref()
                    .and_then(|(_, entry_token)| entry_token.as_deref()),
            ) {
                send_message(sender, &ServerMessage::QueueState { queue, your_entry })
                    .await
                    .map_err(|error| error.to_string())?;
            }

            Ok(Some(queue_id))
        }
        ClientMessage::LeaveQueue {
            queue_id,
            entry_token,
        } => {
            let mut store = state.store.write().await;
            store.leave_queue(queue_id, &entry_token)?;
            *queue_subscription = Some((queue_id, None));

            if let Some((queue, your_entry)) = store.user_view(queue_id, None) {
                send_message(sender, &ServerMessage::QueueState { queue, your_entry })
                    .await
                    .map_err(|error| error.to_string())?;
            }

            Ok(Some(queue_id))
        }
        ClientMessage::ClaimEntry {
            admin_token,
            entry_id,
        } => {
            let mut store = state.store.write().await;
            let queue_id =
                store.update_entry_status(&admin_token, entry_id, QueueEntryStatus::Claimed)?;
            Ok(Some(queue_id))
        }
        ClientMessage::ResolveEntry {
            admin_token,
            entry_id,
        } => {
            let mut store = state.store.write().await;
            let queue_id =
                store.update_entry_status(&admin_token, entry_id, QueueEntryStatus::Resolved)?;
            Ok(Some(queue_id))
        }
        ClientMessage::DenyEntry {
            admin_token,
            entry_id,
        } => {
            let mut store = state.store.write().await;
            let queue_id =
                store.update_entry_status(&admin_token, entry_id, QueueEntryStatus::Denied)?;
            Ok(Some(queue_id))
        }
    }
}

async fn send_message(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    message: &ServerMessage,
) -> Result<(), axum::Error> {
    sender
        .send(Message::Text(
            serde_json::to_string(message)
                .expect("serialize server message")
                .into(),
        ))
        .await
}

impl Store {
    fn create_queue(
        &mut self,
        name: String,
        fields: Vec<QueueField>,
    ) -> Result<(Uuid, String, String), String> {
        let normalized_name = name.trim().to_string();
        if normalized_name.is_empty() {
            return Err("queue name is required".to_string());
        }

        let fields = normalize_fields(fields)?;
        if fields.is_empty() {
            return Err("at least one queue field is required".to_string());
        }

        let id = Uuid::new_v4();
        let admin_token = Uuid::new_v4().to_string();
        let queue = Queue {
            id,
            name: normalized_name,
            admin_token: admin_token.clone(),
            fields,
            entries: Vec::new(),
        };

        self.admin_index.insert(admin_token, id);
        self.queues.insert(id, queue);

        let queue = self
            .queues
            .get(&id)
            .ok_or_else(|| "failed to read queue after creation".to_string())?;
        Ok((queue.id, queue.name.clone(), queue.admin_token.clone()))
    }

    fn queue_id_for_admin(&self, admin_token: &str) -> Option<Uuid> {
        self.admin_index.get(admin_token).copied()
    }

    fn admin_view(&self, admin_token: &str) -> Option<AdminQueueView> {
        let queue_id = self.queue_id_for_admin(admin_token)?;
        let queue = self.queues.get(&queue_id)?;

        Some(AdminQueueView {
            summary: queue.summary(),
            fields: queue.fields.clone(),
            entries: queue
                .entries
                .iter()
                .map(|entry| AdminEntryView {
                    id: entry.id,
                    status: entry.status.clone(),
                    submitted_at: entry.submitted_at.clone(),
                    values: entry.values.clone(),
                })
                .collect(),
        })
    }

    fn user_view(
        &self,
        queue_id: Uuid,
        entry_token: Option<&str>,
    ) -> Option<(UserQueueView, Option<UserEntryView>)> {
        let queue = self.queues.get(&queue_id)?;
        let your_entry = entry_token.and_then(|token| {
            queue
                .entries
                .iter()
                .find(|entry| entry.token == token)
                .map(|entry| UserEntryView {
                    id: entry.id,
                    token: entry.token.clone(),
                    status: entry.status.clone(),
                    values: entry.values.clone(),
                    submitted_at: entry.submitted_at.clone(),
                    position: queue.position_for(entry.id),
                })
        });

        Some((
            UserQueueView {
                id: queue.id,
                name: queue.name.clone(),
                fields: queue.fields.clone(),
                waiting_count: queue.waiting_count(),
            },
            your_entry,
        ))
    }

    fn join_queue(
        &mut self,
        queue_id: Uuid,
        mut values: BTreeMap<String, String>,
    ) -> Result<String, String> {
        let queue = self
            .queues
            .get_mut(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;

        for field in &queue.fields {
            let value = values
                .remove(&field.key)
                .unwrap_or_default()
                .trim()
                .to_string();

            if field.required && value.is_empty() {
                return Err(format!("{} is required", field.label));
            }

            values.insert(field.key.clone(), value);
        }

        let token = Uuid::new_v4().to_string();
        queue.entries.push(QueueEntry {
            id: Uuid::new_v4(),
            token: token.clone(),
            values,
            submitted_at: Utc::now().to_rfc3339(),
            status: QueueEntryStatus::Pending,
        });

        Ok(token)
    }

    fn leave_queue(&mut self, queue_id: Uuid, entry_token: &str) -> Result<(), String> {
        let queue = self
            .queues
            .get_mut(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;

        let original_len = queue.entries.len();
        queue.entries.retain(|entry| {
            if entry.token != entry_token {
                return true;
            }

            matches!(
                entry.status,
                QueueEntryStatus::Resolved | QueueEntryStatus::Denied
            )
        });

        if queue.entries.len() == original_len {
            return Err("active queue entry not found for leave request".to_string());
        }

        Ok(())
    }

    fn update_entry_status(
        &mut self,
        admin_token: &str,
        entry_id: Uuid,
        next_status: QueueEntryStatus,
    ) -> Result<Uuid, String> {
        let queue_id = self
            .queue_id_for_admin(admin_token)
            .ok_or_else(|| "unknown admin token".to_string())?;

        let queue = self
            .queues
            .get_mut(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;

        let entry = queue
            .entries
            .iter_mut()
            .find(|entry| entry.id == entry_id)
            .ok_or_else(|| "queue entry not found".to_string())?;

        match (&entry.status, &next_status) {
            (QueueEntryStatus::Pending, QueueEntryStatus::Claimed)
            | (QueueEntryStatus::Pending, QueueEntryStatus::Resolved)
            | (QueueEntryStatus::Pending, QueueEntryStatus::Denied)
            | (QueueEntryStatus::Claimed, QueueEntryStatus::Resolved)
            | (QueueEntryStatus::Claimed, QueueEntryStatus::Denied) => {
                entry.status = next_status;
                Ok(queue_id)
            }
            _ => Err("invalid status transition".to_string()),
        }
    }
}

impl Queue {
    fn waiting_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| matches!(entry.status, QueueEntryStatus::Pending))
            .count()
    }

    fn active_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.status,
                    QueueEntryStatus::Pending | QueueEntryStatus::Claimed
                )
            })
            .count()
    }

    fn position_for(&self, entry_id: Uuid) -> Option<usize> {
        let mut position = 0usize;

        for entry in &self.entries {
            if entry.status == QueueEntryStatus::Pending {
                position += 1;
            }

            if entry.id == entry_id {
                return if entry.status == QueueEntryStatus::Pending {
                    Some(position)
                } else {
                    None
                };
            }
        }

        None
    }

    fn summary(&self) -> QueueSummary {
        QueueSummary {
            id: self.id,
            name: self.name.clone(),
            waiting_count: self.waiting_count(),
            active_count: self.active_count(),
        }
    }
}

fn normalize_fields(fields: Vec<QueueField>) -> Result<Vec<QueueField>, String> {
    let mut normalized = Vec::new();
    let mut seen = HashMap::new();

    for field in fields {
        let label = field.label.trim().to_string();
        if label.is_empty() {
            return Err("field labels cannot be empty".to_string());
        }

        let key = if field.key.trim().is_empty() {
            slugify(&label)
        } else {
            slugify(field.key.trim())
        };

        if key.is_empty() {
            return Err(format!("field label '{}' produced an empty key", label));
        }

        if seen.insert(key.clone(), true).is_some() {
            return Err(format!("duplicate field key '{}'", key));
        }

        normalized.push(QueueField {
            key,
            label,
            required: field.required,
        });
    }

    Ok(normalized)
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
