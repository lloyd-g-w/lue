use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use shared::{ClientMessage, QueueEntryStatus, ServerMessage};
use uuid::Uuid;

use crate::model::{AdminSubscription, AppState, QueueSubscription};

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut updates = state.updates.subscribe();
    let mut admin_subscription = AdminSubscription::default();
    let mut queue_subscription = QueueSubscription::default();

    loop {
        tokio::select! {
            message = receiver.next() => {
                let Some(Ok(message)) = message else {
                    break;
                };

                if let Message::Text(text) = message {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(command) => {
                            match process_command(
                                &state,
                                command,
                                &mut admin_subscription,
                                &mut queue_subscription,
                                &mut sender,
                            ).await {
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
                            let _ = send_message(
                                &mut sender,
                                &ServerMessage::Error {
                                    message: format!("invalid websocket message: {error}"),
                                },
                            )
                            .await;
                        }
                    }
                }
            }
            updated = updates.recv() => {
                let Ok(queue_id) = updated else {
                    continue;
                };

                if let Some(admin_token) = admin_subscription.admin_token.as_deref() {
                    let store = state.store.read().await;
                    if store.admin_can_see_queue(admin_token, queue_id) {
                        if let Some(state_view) = store.admin_state(admin_token, admin_subscription.selected_queue_id) {
                            if send_message(&mut sender, &ServerMessage::AdminState { state: state_view }).await.is_err() {
                                break;
                            }
                        }
                    }
                }

                if queue_subscription.queue_id == Some(queue_id) {
                    let store = state.store.read().await;
                    if let Some((queue, your_entry)) =
                        store.user_view(queue_id, queue_subscription.entry_token.as_deref())
                    {
                        if send_message(
                            &mut sender,
                            &ServerMessage::QueueState {
                                queue,
                                your_entry,
                                site_settings: store.site_settings_view(),
                            },
                        )
                        .await
                        .is_err()
                        {
                            break;
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
    admin_subscription: &mut AdminSubscription,
    queue_subscription: &mut QueueSubscription,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> Result<Option<Uuid>, String> {
    match command {
        ClientMessage::CheckSetup => {
            let store = state.store.read().await;
            send_message(
                sender,
                &ServerMessage::SetupState {
                    needs_setup: store.needs_initial_setup(),
                },
            )
            .await
            .map_err(|error| error.to_string())?;
            Ok(None)
        }
        ClientMessage::ListPublicQueues => {
            let store = state.store.read().await;
            send_message(
                sender,
                &ServerMessage::PublicQueues {
                    queues: store.public_queues(),
                    site_settings: store.site_settings_view(),
                },
            )
            .await
            .map_err(|error| error.to_string())?;
            Ok(None)
        }
        ClientMessage::SetupSuperAdmin {
            name,
            email,
            password,
        } => {
            let mut store = state.store.write().await;
            let admin = store.setup_initial_super_admin(name, email, password)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            admin_subscription.admin_token = Some(admin.token.clone());

            send_message(
                sender,
                &ServerMessage::AdminLoggedIn {
                    admin: admin.clone(),
                },
            )
            .await
            .map_err(|error| error.to_string())?;

            if let Some(state_view) = store.admin_state(&admin.token, None) {
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }

            Ok(None)
        }
        ClientMessage::LoginAdmin { email, password } => {
            let mut store = state.store.write().await;
            let admin = store.login_admin(email, password)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            admin_subscription.admin_token = Some(admin.token.clone());

            send_message(
                sender,
                &ServerMessage::AdminLoggedIn {
                    admin: admin.clone(),
                },
            )
            .await
            .map_err(|error| error.to_string())?;

            if let Some(state_view) = store.admin_state(&admin.token, None) {
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }

            Ok(None)
        }
        ClientMessage::LoginUser { email, password } => {
            let mut store = state.store.write().await;
            let user = store.login_user(email, password)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            queue_subscription.user_token = Some(user.token.clone());

            send_message(sender, &ServerMessage::UserLoggedIn { user })
                .await
                .map_err(|error| error.to_string())?;
            Ok(None)
        }
        ClientMessage::SubscribeAdmin {
            admin_token,
            selected_queue_id,
        } => {
            let store = state.store.read().await;
            let Some(state_view) = store.admin_state(&admin_token, selected_queue_id) else {
                return Err("unknown admin session".to_string());
            };

            admin_subscription.admin_token = Some(admin_token);
            admin_subscription.selected_queue_id = state_view
                .selected_queue
                .as_ref()
                .map(|queue| queue.summary.id);

            send_message(sender, &ServerMessage::AdminState { state: state_view })
                .await
                .map_err(|error| error.to_string())?;
            Ok(None)
        }
        ClientMessage::CreateQueue {
            admin_token,
            name,
            fields,
            allow_guests,
            is_public,
            opens_at,
            weekly_schedule,
        } => {
            let mut store = state.store.write().await;
            let queue_id = store.create_queue(
                &admin_token,
                name,
                fields,
                allow_guests,
                is_public,
                opens_at,
                weekly_schedule,
            )?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            admin_subscription.admin_token = Some(admin_token.clone());
            admin_subscription.selected_queue_id = Some(queue_id);

            send_message(sender, &ServerMessage::QueueCreated { queue_id })
                .await
                .map_err(|error| error.to_string())?;

            if let Some(state_view) = store.admin_state(&admin_token, Some(queue_id)) {
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }

            Ok(Some(queue_id))
        }
        ClientMessage::UpdateQueueSettings {
            admin_token,
            queue_id,
            fields,
            allow_guests,
            is_public,
            opens_at,
            weekly_schedule,
        } => {
            let mut store = state.store.write().await;
            store.update_queue_settings(
                &admin_token,
                queue_id,
                fields,
                allow_guests,
                is_public,
                opens_at,
                weekly_schedule,
            )?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            if let Some(state_view) = store.admin_state(&admin_token, Some(queue_id)) {
                send_message(sender, &ServerMessage::QueueSettingsUpdated)
                    .await
                    .map_err(|error| error.to_string())?;
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }

            Ok(Some(queue_id))
        }
        ClientMessage::CreateAccount {
            admin_token,
            name,
            email,
            password,
            role,
        } => {
            let mut store = state.store.write().await;
            store.create_account(&admin_token, name, email, password, role)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            if let Some(state_view) =
                store.admin_state(&admin_token, admin_subscription.selected_queue_id)
            {
                send_message(sender, &ServerMessage::AccountCreated)
                    .await
                    .map_err(|error| error.to_string())?;
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(None)
        }
        ClientMessage::UpdateAccount {
            admin_token,
            account_id,
            name,
            email,
            password,
            role,
        } => {
            let mut store = state.store.write().await;
            store.update_account(&admin_token, account_id, name, email, password, role)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            if let Some(state_view) =
                store.admin_state(&admin_token, admin_subscription.selected_queue_id)
            {
                send_message(sender, &ServerMessage::AccountUpdated)
                    .await
                    .map_err(|error| error.to_string())?;
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(None)
        }
        ClientMessage::DeleteAccount {
            admin_token,
            account_id,
        } => {
            let mut store = state.store.write().await;
            store.delete_account(&admin_token, account_id)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            if let Some(state_view) =
                store.admin_state(&admin_token, admin_subscription.selected_queue_id)
            {
                send_message(sender, &ServerMessage::AccountDeleted)
                    .await
                    .map_err(|error| error.to_string())?;
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(None)
        }
        ClientMessage::CreateGroup {
            admin_token,
            name,
            role,
            member_ids,
        } => {
            let mut store = state.store.write().await;
            store.create_group(&admin_token, name, role, member_ids)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            if let Some(state_view) =
                store.admin_state(&admin_token, admin_subscription.selected_queue_id)
            {
                send_message(sender, &ServerMessage::GroupCreated)
                    .await
                    .map_err(|error| error.to_string())?;
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(None)
        }
        ClientMessage::UpdateGroup {
            admin_token,
            group_id,
            name,
            role,
            member_ids,
        } => {
            let mut store = state.store.write().await;
            store.update_group(&admin_token, group_id, name, role, member_ids)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            if let Some(state_view) =
                store.admin_state(&admin_token, admin_subscription.selected_queue_id)
            {
                send_message(sender, &ServerMessage::GroupUpdated)
                    .await
                    .map_err(|error| error.to_string())?;
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(None)
        }
        ClientMessage::DeleteGroup {
            admin_token,
            group_id,
        } => {
            let mut store = state.store.write().await;
            store.delete_group(&admin_token, group_id)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            if let Some(state_view) =
                store.admin_state(&admin_token, admin_subscription.selected_queue_id)
            {
                send_message(sender, &ServerMessage::GroupDeleted)
                    .await
                    .map_err(|error| error.to_string())?;
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(None)
        }
        ClientMessage::UpdateSiteSettings {
            admin_token,
            site_title,
        } => {
            let mut store = state.store.write().await;
            store.update_site_settings(&admin_token, site_title)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            if let Some(state_view) =
                store.admin_state(&admin_token, admin_subscription.selected_queue_id)
            {
                send_message(sender, &ServerMessage::SiteSettingsUpdated)
                    .await
                    .map_err(|error| error.to_string())?;
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(None)
        }
        ClientMessage::ShareQueue {
            admin_token,
            queue_id,
            account_ids,
            group_ids,
        } => {
            let mut store = state.store.write().await;
            store.share_queue(&admin_token, queue_id, account_ids, group_ids)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            if let Some(state_view) = store.admin_state(&admin_token, Some(queue_id)) {
                send_message(sender, &ServerMessage::QueueSharingUpdated)
                    .await
                    .map_err(|error| error.to_string())?;
                send_message(sender, &ServerMessage::AdminState { state: state_view })
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(Some(queue_id))
        }
        ClientMessage::CloseQueue {
            admin_token,
            queue_id,
        } => {
            let mut store = state.store.write().await;
            store.close_queue(&admin_token, queue_id)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            admin_subscription.selected_queue_id = None;
            if let Some(state_view) = store.admin_state(&admin_token, None) {
                send_message(sender, &ServerMessage::QueueClosed)
                    .await
                    .map_err(|error| error.to_string())?;
                send_message(sender, &ServerMessage::AdminState { state: state_view })
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
            let queue_id = store.claim_entry(&admin_token, entry_id)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            Ok(Some(queue_id))
        }
        ClientMessage::UnclaimEntry {
            admin_token,
            entry_id,
        } => {
            let mut store = state.store.write().await;
            let queue_id = store.unclaim_entry(&admin_token, entry_id)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            Ok(Some(queue_id))
        }
        ClientMessage::ResolveEntry {
            admin_token,
            entry_id,
        } => {
            let mut store = state.store.write().await;
            let queue_id =
                store.update_entry_status(&admin_token, entry_id, QueueEntryStatus::Resolved)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            Ok(Some(queue_id))
        }
        ClientMessage::DenyEntry {
            admin_token,
            entry_id,
        } => {
            let mut store = state.store.write().await;
            let queue_id =
                store.update_entry_status(&admin_token, entry_id, QueueEntryStatus::Denied)?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            Ok(Some(queue_id))
        }
        ClientMessage::SubscribeQueue {
            queue_id,
            entry_token,
            user_token,
        } => {
            let store = state.store.read().await;
            if let Some(message) = store.queue_unavailable_message(queue_id) {
                return Err(message);
            }
            let Some((queue, your_entry)) = store.user_view(queue_id, entry_token.as_deref())
            else {
                return Err("unknown queue".to_string());
            };

            queue_subscription.queue_id = Some(queue_id);
            queue_subscription.entry_token = entry_token;
            queue_subscription.user_token = user_token;
            send_message(
                sender,
                &ServerMessage::QueueState {
                    queue,
                    your_entry,
                    site_settings: store.site_settings_view(),
                },
            )
            .await
            .map_err(|error| error.to_string())?;
            Ok(None)
        }
        ClientMessage::JoinQueue {
            queue_id,
            values,
            user_token,
            entry_token,
        } => {
            let mut store = state.store.write().await;
            let token = store.join_queue(
                queue_id,
                values,
                user_token.as_deref(),
                entry_token.as_deref(),
            )?;
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            queue_subscription.queue_id = Some(queue_id);
            queue_subscription.entry_token = Some(token.clone());
            queue_subscription.user_token = user_token;

            if let Some((queue, your_entry)) =
                store.user_view(queue_id, queue_subscription.entry_token.as_deref())
            {
                send_message(
                    sender,
                    &ServerMessage::QueueState {
                        queue,
                        your_entry,
                        site_settings: store.site_settings_view(),
                    },
                )
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
            store
                .save_to_disk(&state.data_path)
                .map_err(|error| format!("failed to save store: {error}"))?;
            queue_subscription.queue_id = Some(queue_id);
            queue_subscription.entry_token = Some(entry_token);

            if let Some((queue, your_entry)) =
                store.user_view(queue_id, queue_subscription.entry_token.as_deref())
            {
                send_message(
                    sender,
                    &ServerMessage::QueueState {
                        queue,
                        your_entry,
                        site_settings: store.site_settings_view(),
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
            }

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
