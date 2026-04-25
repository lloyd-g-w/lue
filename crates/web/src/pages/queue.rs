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
use crate::view_helpers::{status_class_suffix, status_label};
use crate::ws::{connect_socket, login_user_socket, send_ws};

#[component]
pub fn QueuePage(queue_id: String) -> Element {
    let queue_state = use_signal(|| None::<UserQueueView>);
    let your_entry = use_signal(|| None::<UserEntryView>);
    let user_session = use_signal(load_user_session);
    let feedback = use_signal(String::new);
    let auth_feedback = use_signal(String::new);
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
        let mut socket = socket;
        let mut form_values = form_values;
        let queue_id = parsed_queue_id;

        let existing_token = load_entry_token(queue_id);
        let user_token = load_user_session().map(|session| session.token);

        let ws = connect_socket(
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
                        user_token: user_token.clone(),
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

    let login_user = {
        let socket = socket;
        let mut user_session = user_session;
        let auth_email = auth_email;
        let auth_password = auth_password;
        let mut auth_feedback = auth_feedback;
        move |_| {
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
        }
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
        move |_| {
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
        }
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
        if let Some(queue) = snapshot {
            div { class: "queue-page-layout",
                section { class: "queue-hero-panel",
                    p { class: "kicker", "Join Queue" }
                    h1 { "{queue.name}" }
                    p { class: "landing-lede", "{queue.waiting_count} people are currently waiting." }
                    p { class: "hint",
                        if queue.allow_guests {
                            "Guests can join this queue, or you can sign in with a user account."
                        } else {
                            "This queue requires a user account before you can join."
                        }
                    }

                    if let Some(entry) = your_entry() {
                        div { class: "ticket-panel",
                            p { class: "ticket-label", "Current request" }
                            p { class: "status-pill {status_class_suffix(&entry.status)}",
                                "Status: {status_label(&entry.status)}"
                                if let Some(position) = entry.position {
                                    " • position {position}"
                                }
                            }
                            p { class: "hint",
                                "Requester: {entry.requester_label}"
                                if entry.is_guest {
                                    " • guest"
                                }
                            }
                            if let Some(claimed_by) = entry.claimed_by.clone() {
                                p { class: "hint", "Handled by {claimed_by}" }
                            }
                            if matches!(entry.status, QueueEntryStatus::Pending | QueueEntryStatus::Claimed) {
                                button { class: "button danger", onclick: leave_queue, "Leave queue" }
                            } else {
                                p { class: "hint", "This outcome stays visible until you refresh or choose to rejoin." }
                                button { class: "button button-primary", onclick: rejoin_queue, "Rejoin queue" }
                            }
                        }
                    } else {
                        div { class: "ticket-panel muted-ticket",
                            p { class: "ticket-label", "Ready to join" }
                            p { class: "hint",
                                if let Some(user) = user_session() {
                                    "Signed in as {user.email}"
                                } else if queue.allow_guests {
                                    "You can join as a guest or sign in with a user account."
                                } else {
                                    "Sign in with a user account to continue."
                                }
                            }
                        }
                    }
                }

                section { class: "queue-form-panel",
                    if your_entry().is_none() {
                        div { class: "panel-header",
                            div {
                                p { class: "kicker", "Access" }
                                h2 { "Join settings" }
                            }
                        }

                        if let Some(user) = user_session() {
                            div { class: "ticket-panel",
                                p { class: "ticket-label", "Signed in user" }
                                p { class: "hint", "{user.name} • {user.email}" }
                                button { class: "button button-secondary", onclick: sign_out_user, "Sign out" }
                            }
                        } else {
                            div { class: "form-stack",
                                div { class: "input-group",
                                    label { class: "label", "User email" }
                                    input {
                                        class: "input",
                                        value: "{auth_email}",
                                        oninput: move |event| auth_email.set(event.value()),
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
                                        placeholder: "Password"
                                    }
                                }
                                button { class: "button button-secondary", onclick: login_user, "Sign in as user" }
                                if queue.allow_guests {
                                    p { class: "hint", "Skip sign-in and submit the form below as a guest if you prefer." }
                                }
                                if !auth_feedback().is_empty() {
                                    p { class: "feedback", "{auth_feedback}" }
                                }
                            }
                        }

                        if queue.allow_guests || user_session().is_some() {
                            div { class: "panel-header",
                                div {
                                    p { class: "kicker", "Request Form" }
                                    h2 { "Enter the queue" }
                                }
                            }
                            div { class: "form-stack",
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
                                button { class: "button button-primary", onclick: join_queue, "Join queue" }
                            }
                        }
                    } else if let Some(entry) = your_entry() {
                        div { class: "panel-header",
                            div {
                                p { class: "kicker", "Submitted Request" }
                                h2 { "Your details" }
                            }
                        }
                        div { class: "detail-list",
                            div { class: "detail-row",
                                span { class: "detail-key", "Account" }
                                div { class: "detail-value",
                                    "{entry.requester_label}"
                                    if entry.is_guest {
                                        " (guest)"
                                    }
                                }
                            }
                            for field in queue.fields.iter().cloned() {
                                div { class: "detail-row",
                                    span { class: "detail-key", "{field.label}" }
                                    div { class: "detail-value",
                                        "{entry.values.get(&field.key).cloned().unwrap_or_default()}"
                                    }
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
            section { class: "empty-stage",
                h1 { "Connecting to queue..." }
                if !feedback().is_empty() {
                    p { class: "feedback", "{feedback}" }
                }
            }
        }
    }
}
