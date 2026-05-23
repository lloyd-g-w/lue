use std::collections::BTreeMap;

use dioxus::prelude::*;
use shared::{ClientMessage, QueueEntryStatus, ServerMessage, UserEntryView, UserQueueView};
use uuid::Uuid;
use web_sys::WebSocket;

use crate::models::UserSessionRecord;
use crate::storage::{
    clear_entry_token, clear_user_session, load_entry_token, load_user_session, save_entry_token,
    save_user_session,
};
use crate::view_helpers::{is_enter_key, status_class_suffix, status_label};
use crate::ws::{connect_reconnecting_socket, login_user_socket, send_ws, SocketStatus};

#[component]
pub fn QueuePage(queue_id: String) -> Element {
    let queue_state = use_signal(|| None::<UserQueueView>);
    let your_entry = use_signal(|| None::<UserEntryView>);
    let user_session = use_signal(load_user_session);
    let feedback = use_signal(String::new);
    let auth_feedback = use_signal(String::new);
    let connection_status = use_signal(|| SocketStatus::Connecting);
    let mut auth_email = use_signal(String::new);
    let mut auth_password = use_signal(String::new);
    let mut form_values = use_signal(BTreeMap::<String, String>::new);
    let socket = use_signal(|| None::<WebSocket>);

    let parsed_queue_id = Uuid::parse_str(&queue_id).ok();
    if parsed_queue_id.is_none() {
        return rsx! {
            section { class: "empty-stage",
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
        let mut connection_status = connection_status;
        let mut socket = socket;
        let mut form_values = form_values;
        let queue_id = parsed_queue_id;

        let existing_token = load_entry_token(queue_id);
        let user_token = load_user_session().map(|session| session.token);

        connect_reconnecting_socket(
            move |message| match message {
                ServerMessage::QueueState {
                    queue,
                    your_entry: entry,
                } => {
                    if let Some(entry) = entry.as_ref() {
                        if matches!(
                            entry.status,
                            QueueEntryStatus::Left
                                | QueueEntryStatus::Resolved
                                | QueueEntryStatus::Denied
                        ) {
                            clear_entry_token(queue.id);
                        } else {
                            save_entry_token(queue.id, &entry.token);
                        }
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
                    if connection_status() == SocketStatus::Connected {
                        feedback.set(String::new());
                    }
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
                        user_token: user_token.clone(),
                    },
                );
                socket.set(Some(ws));
            },
            move |status| {
                connection_status.set(status);
                if status == SocketStatus::Reconnecting {
                    socket.set(None);
                    feedback.set("Live updates disconnected. Reconnecting...".to_string());
                }
            },
        );
    });

    let login_user = {
        let socket = socket;
        let mut user_session = user_session;
        let auth_email = auth_email;
        let auth_password = auth_password;
        let mut auth_feedback = auth_feedback;
        EventHandler::new(move |_| {
            auth_feedback.set("Signing in...".to_string());
            login_user_socket(
                auth_email(),
                auth_password(),
                move |user| {
                    let session = UserSessionRecord {
                        token: user.token.clone(),
                        name: user.name,
                        email: user.email,
                    };
                    save_user_session(&session);
                    user_session.set(Some(session.clone()));
                    auth_feedback.set(String::new());

                    if let Some(ws) = socket() {
                        let _ = send_ws(
                            &ws,
                            &ClientMessage::SubscribeQueue {
                                queue_id: parsed_queue_id,
                                entry_token: load_entry_token(parsed_queue_id),
                                user_token: Some(session.token),
                            },
                        );
                    }
                },
                auth_feedback,
            );
        })
    };

    let sign_out_user = {
        let mut user_session = user_session;
        move |_| {
            clear_user_session();
            user_session.set(None);
        }
    };

    let join_queue = {
        let queue_state = queue_state;
        let form_values = form_values;
        let user_session = user_session;
        let socket = socket;
        EventHandler::new(move |_| {
            if let Some(queue) = queue_state() {
                if let Some(ws) = socket() {
                    let _ = send_ws(
                        &ws,
                        &ClientMessage::JoinQueue {
                            queue_id: queue.id,
                            values: form_values(),
                            user_token: user_session().map(|session| session.token),
                        },
                    );
                }
            }
        })
    };

    let leave_queue = {
        let queue_state = queue_state;
        let your_entry = your_entry;
        let socket = socket;
        move |_| {
            if let (Some(queue), Some(entry), Some(ws)) = (queue_state(), your_entry(), socket()) {
                clear_entry_token(queue.id);
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

    let rejoin_queue = {
        let queue_state = queue_state;
        let mut your_entry = your_entry;
        let mut form_values = form_values;
        move |_| {
            if let Some(queue) = queue_state() {
                clear_entry_token(queue.id);
            }
            if let Some(entry) = your_entry() {
                form_values.set(entry.values.clone());
            }
            your_entry.set(None);
        }
    };

    let snapshot = queue_state();
    rsx! {
        ConnectionStatusStrip { status: connection_status() }
        if let Some(queue) = snapshot {
            div { class: "queue-page-layout",
                section { class: "queue-hero-panel",
                    p { class: "kicker", "Queue" }
                    h1 { "{queue.name}" }
                    div { class: "queue-meta-line",
                        if queue.closed_at.is_some() {
                            span { class: "status-pill status-left", "Closed" }
                        } else {
                            span { "{queue.waiting_count} waiting" }
                        }
                    }

                    if let Some(entry) = your_entry() {
                        div { class: "ticket-panel queue-status-block",
                            p { class: "ticket-label", "Your request" }
                            p { class: "status-pill {status_class_suffix(&entry.status)}",
                                "{user_status_label(&entry)}"
                                if let Some(position) = entry.position {
                                    " • position {position}"
                                }
                            }
                            if queue.closed_at.is_some() {
                                p { class: "feedback", "This queue has been closed." }
                            } else if matches!(entry.status, QueueEntryStatus::Pending | QueueEntryStatus::Claimed) {
                                button { class: "button danger", onclick: leave_queue, "Leave queue" }
                            } else {
                                button { class: "button button-primary", onclick: rejoin_queue, "Rejoin queue" }
                            }
                        }
                    } else if queue.closed_at.is_some() {
                        p { class: "hint", "This queue is no longer accepting requests." }
                    } else if queue.allow_guests {
                        p { class: "hint", "Join as a guest, or sign in first." }
                    } else {
                        p { class: "hint", "Sign in to join this queue." }
                    }
                }

                if queue.closed_at.is_none() && your_entry().is_none() {
                    section { class: "queue-form-panel",
                        if let Some(user) = user_session() {
                            div { class: "signed-in-strip",
                                span { "Signed in as {user.email}" }
                                button { class: "button button-secondary", onclick: sign_out_user, "Sign out" }
                            }
                        } else {
                            div { class: "auth-inline-grid",
                                div { class: "input-group",
                                    label { class: "label", "Email" }
                                    input {
                                        class: "input",
                                        value: "{auth_email}",
                                        oninput: move |event| auth_email.set(event.value()),
                                        onkeydown: move |event| {
                                            if is_enter_key(&event) {
                                                event.prevent_default();
                                                login_user.call(());
                                            }
                                        },
                                        placeholder: "user@example.com"
                                    }
                                }
                                div { class: "input-group",
                                    label { class: "label", "Password" }
                                    input {
                                        class: "input",
                                        r#type: "password",
                                        value: "{auth_password}",
                                        oninput: move |event| auth_password.set(event.value()),
                                        onkeydown: move |event| {
                                            if is_enter_key(&event) {
                                                event.prevent_default();
                                                login_user.call(());
                                            }
                                        },
                                        placeholder: "Password"
                                    }
                                }
                                button { class: "button button-secondary auth-submit", onclick: move |_| login_user.call(()), "Sign in" }
                            }
                            if !queue.allow_guests {
                                p { class: "hint", "An account is required for this queue." }
                            }
                            if !auth_feedback().is_empty() {
                                p { class: "feedback", "{auth_feedback}" }
                            }
                        }

                        if queue.allow_guests || user_session().is_some() {
                            div { class: "form-stack",
                                for field in queue.fields.iter().cloned() {
                                    if !(user_session().is_some() && field.key == "name" && !field.required) {
                                        div { class: "input-group",
                                            label { class: "label",
                                                "{field.label}"
                                                if !field.required {
                                                    " optional"
                                                }
                                            }
                                            input {
                                                class: "input",
                                                value: "{form_values().get(&field.key).cloned().unwrap_or_default()}",
                                                oninput: move |event| {
                                                    let mut next = form_values();
                                                    next.insert(field.key.clone(), event.value());
                                                    form_values.set(next);
                                                },
                                                onkeydown: move |event| {
                                                    if is_enter_key(&event) {
                                                        event.prevent_default();
                                                        join_queue.call(());
                                                    }
                                                },
                                                placeholder: "{field.label}"
                                            }
                                        }
                                    }
                                }
                                button { class: "button button-primary", onclick: move |_| join_queue.call(()), "Join queue" }
                            }
                        } else {
                            p { class: "hint", "Sign in to unlock the form." }
                        }

                        if !feedback().is_empty() {
                            p { class: "feedback", "{feedback}" }
                        }
                    }
                } else if !feedback().is_empty() {
                    p { class: "feedback floating-feedback", "{feedback}" }
                }
            }
        } else {
            section { class: "empty-stage",
                h1 { "Connecting to queue..." }
                if !feedback().is_empty() {
                    p { class: "feedback", "{feedback}" }
                }
            }
        }
    }
}

#[component]
fn ConnectionStatusStrip(status: SocketStatus) -> Element {
    let (label, detail) = match status {
        SocketStatus::Connected => ("Live", "Queue updates are streaming."),
        SocketStatus::Connecting => ("Connecting", "Opening the queue channel."),
        SocketStatus::Reconnecting => (
            "Reconnecting",
            "Connection dropped. Retrying automatically.",
        ),
    };

    rsx! {
        div { class: "connection-banner {connection_class(status)}",
            span { class: "connection-orb" }
            div {
                strong { "{label}" }
                p { "{detail}" }
            }
        }
    }
}

fn connection_class(status: SocketStatus) -> &'static str {
    match status {
        SocketStatus::Connected => "connection-live",
        SocketStatus::Connecting => "connection-connecting",
        SocketStatus::Reconnecting => "connection-reconnecting",
    }
}

fn user_status_label(entry: &UserEntryView) -> String {
    match (&entry.status, entry.claimed_by.as_deref()) {
        (QueueEntryStatus::Claimed, Some(name)) => format!("Claimed by {name}"),
        (QueueEntryStatus::Resolved, Some(name)) => format!("Resolved by {name}"),
        (QueueEntryStatus::Denied, Some(name)) => format!("Denied by {name}"),
        _ => status_label(&entry.status).to_string(),
    }
}
