use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use dioxus::prelude::*;
use shared::{
    ClientMessage, QueueEntryStatus, QueueField, ServerMessage, SiteSettingsView, UserEntryView,
    UserQueueView,
};
use uuid::Uuid;
use web_sys::WebSocket;

use crate::components::{DelayedLoading, UiButton, UiPanel};
use crate::models::UserSessionRecord;
use crate::route::{navigate, Route};
use crate::storage::{
    clear_entry_token, clear_user_session, load_entry_token, load_user_session, save_entry_token,
    save_user_session,
};
use crate::view_helpers::{
    is_enter_key, is_requester_name_key, kebab_case, status_class_suffix, status_label,
};
use crate::ws::{
    connect_reconnecting_socket, login_user_socket, resolve_queue_code_socket, send_ws,
    ReconnectingSocket, SocketStatus,
};

#[component]
pub fn QueuePage(route: Signal<Route>, queue_id: String) -> Element {
    let queue_state = use_signal(|| None::<UserQueueView>);
    let site_settings = use_signal(|| SiteSettingsView {
        site_title: "Lue".to_string(),
        admin_password_sign_in_enabled: true,
        admin_microsoft_sign_in_enabled: true,
        user_password_sign_in_enabled: true,
        user_microsoft_sign_in_enabled: true,
    });
    let mut your_entry = use_signal(|| None::<UserEntryView>);
    let user_session = use_signal(load_user_session);
    let mut feedback = use_signal(String::new);
    let auth_feedback = use_signal(String::new);
    let connection_status = use_signal(|| SocketStatus::Connecting);
    let mut auth_email = use_signal(String::new);
    let mut auth_password = use_signal(String::new);
    let mut form_values = use_signal(BTreeMap::<String, String>::new);
    let socket = use_signal(|| None::<WebSocket>);
    let resolved_queue_id = use_signal(|| Uuid::parse_str(&queue_id).ok());
    let connection_handle = use_hook(|| Rc::new(RefCell::new(None::<ReconnectingSocket>)));
    {
        let connection_handle = connection_handle.clone();
        use_drop(move || {
            if let Some(connection) = connection_handle.borrow_mut().take() {
                connection.close();
            }
        });
    }

    {
        let queue_code = queue_id.clone();
        let mut resolved_queue_id = resolved_queue_id;
        let mut feedback = feedback;
        use_effect(move || {
            if resolved_queue_id().is_some() {
                return;
            }
            resolve_queue_code_socket(
                queue_code.clone(),
                move |queue_id| {
                    resolved_queue_id.set(Some(queue_id));
                    feedback.set(String::new());
                },
                feedback,
            );
        });
    }

    let Some(active_queue_id) = resolved_queue_id() else {
        return rsx! {
            DelayedLoading {
                title: "Finding queue...".to_string(),
                detail: Some("Checking the queue code.".to_string()),
                feedback: feedback(),
            }
        };
    };

    use_effect(move || {
        let mut queue_state = queue_state;
        let mut site_settings = site_settings;
        let mut your_entry = your_entry;
        let mut feedback = feedback;
        let mut connection_status = connection_status;
        let mut socket = socket;
        let mut form_values = form_values;
        let queue_id = active_queue_id;
        let connection_handle = connection_handle.clone();

        let existing_token = load_entry_token(queue_id);
        let user_token = load_user_session().map(|session| session.token);

        let connection = connect_reconnecting_socket(
            move |message| match message {
                ServerMessage::QueueState {
                    queue,
                    your_entry: entry,
                    site_settings: settings,
                } => {
                    site_settings.set(settings);
                    if let Some(entry) = entry.as_ref() {
                        if matches!(
                            entry.status,
                            QueueEntryStatus::Resolved | QueueEntryStatus::Denied
                        ) {
                            clear_entry_token(queue.id);
                        } else {
                            save_entry_token(queue.id, &entry.token);
                        }
                    } else {
                        clear_entry_token(queue.id);
                    }

                    if matches!(
                        entry.as_ref().map(|entry| &entry.status),
                        Some(QueueEntryStatus::Left)
                    ) {
                        let mut next = entry
                            .as_ref()
                            .map(|entry| entry.values.clone())
                            .unwrap_or_default();
                        for field in &queue.fields {
                            next.entry(field.key.clone()).or_default();
                        }
                        form_values.set(next);
                    } else if form_values().is_empty() {
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
        let previous_connection = connection_handle.borrow_mut().replace(connection);
        if let Some(previous_connection) = previous_connection {
            previous_connection.close();
        }
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
                                queue_id: active_queue_id,
                                entry_token: load_entry_token(active_queue_id),
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
    let sign_in_with_microsoft = move |_| {
        if let Some(window) = web_sys::window() {
            let return_to = window
                .location()
                .pathname()
                .unwrap_or_else(|_| "/".to_string());
            let _ = window
                .location()
                .set_href(&crate::ws::backend_http_url(&format!(
                    "/auth/microsoft/start?kind=user&return_to={return_to}"
                )));
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
                            entry_token: load_entry_token(queue.id),
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

    let snapshot = queue_state();

    let entry_resolved_or_denied = matches!(
        your_entry().map(|entry| entry.status),
        Some(QueueEntryStatus::Resolved | QueueEntryStatus::Denied)
    );

    rsx! {
        document::Title { "{site_settings().site_title}" }
        ConnectionStatusStrip { status: connection_status() }
        if let Some(queue) = snapshot {
            {
                let queue_name = kebab_case(&queue.name);
                rsx! {
            div { class: "queue-page-layout",
                div { class: "queue-heading-row",
                    nav { class: "path-nav path-nav-primary mono",
                        button {
                            class: "path-link",
                            onclick: move |_| navigate(route, Route::Home),
                            "~"
                        }
                        span { "/queue/{queue_name}" }
                    }
                }
                section { class: "queue-hero-panel",
                    if let Some(entry) = your_entry() {
                        if matches!(entry.status, QueueEntryStatus::Left) {
                            if queue.closed_at.is_some() {
                                p { class: "hint", "This queue is no longer accepting requests." }
                            }
                        } else {
                            UiPanel { class: "ticket-panel queue-status-block".to_string(),
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
                                    UiButton {
                                        label: "Leave queue".to_string(),
                                        variant: "danger".to_string(),
                                        onclick: leave_queue,
                                    }
                                } else if entry_resolved_or_denied {
                                    UiButton {
                                        label: "Rejoin queue".to_string(),
                                        variant: "primary".to_string(),
                                            onclick: move |_| {
        if let (Some(queue), Some(entry)) = (queue_state(), your_entry()) {
            clear_entry_token(queue.id);

            let mut next = entry.values.clone();
            for field in &queue.fields {
                next.entry(field.key.clone()).or_default();
            }

            form_values.set(next);
            your_entry.set(None);
            feedback.set(String::new());
        }
    },
                                    }
                                } else {
                                    p { class: "hint", "This request is no longer active." }
                                }
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

                if queue.closed_at.is_none() && should_show_join_form(your_entry()) &&  !entry_resolved_or_denied {
                    UiPanel { class: "queue-form-panel".to_string(),
                        div { class: "join-panel-status",
                            span { class: "counter-pill", "{queue.waiting_count} waiting" }
                        }
                        if let Some(user) = user_session() {
                            div { class: "signed-in-strip",
                                span { "Signed in as {user.email}" }
                                UiButton {
                                    label: "Sign out".to_string(),
                                    variant: "secondary".to_string(),
                                    onclick: sign_out_user,
                                }
                            }
                        } else {
                            if site_settings().user_password_sign_in_enabled {
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
                                    UiButton {
                                        label: "Sign in".to_string(),
                                        variant: "secondary".to_string(),
                                        class: "auth-submit".to_string(),
                                        onclick: move |_| login_user.call(()),
                                    }
                                }
                            }
                            if site_settings().user_microsoft_sign_in_enabled {
                                UiButton {
                                    label: "Sign in with Microsoft".to_string(),
                                    variant: "secondary".to_string(),
                                    onclick: sign_in_with_microsoft,
                                }
                            }
                            if !site_settings().user_password_sign_in_enabled && !site_settings().user_microsoft_sign_in_enabled {
                                p { class: "hint", "User sign-in is currently unavailable." }
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
                                    if !(user_session().is_some() && is_requester_name_key(&field.key)) {
                                        QueueFieldInput {
                                            field,
                                            form_values,
                                            join_queue,
                                        }
                                    }
                                }
                                UiButton {
                                    label: join_button_label(user_session()),
                                    variant: "primary".to_string(),
                                    onclick: move |_| join_queue.call(()),
                                }
                            }
                        }

                        if !feedback().is_empty() {
                            p { class: "feedback", "{feedback}" }
                        }
                    }
                } else if !feedback().is_empty() {
                    p { class: "feedback floating-feedback", "{feedback}" }
                }
            }
                }
            }
        } else {
            DelayedLoading {
                title: "Connecting to queue...".to_string(),
                detail: None,
                feedback: feedback(),
            }
        }
    }
}

#[component]
fn QueueFieldInput(
    field: QueueField,
    form_values: Signal<BTreeMap<String, String>>,
    join_queue: EventHandler<()>,
) -> Element {
    let mut form_values = form_values;
    let current_value = form_values().get(&field.key).cloned().unwrap_or_default();

    rsx! {
        div { class: "input-group",
            label { class: "label",
                "{field.label}"
                if !field.required {
                    " optional"
                }
            }
            if field.options.is_empty() {
                input {
                    class: "input",
                    value: "{current_value}",
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
            } else {
                select {
                    class: "input",
                    value: "{current_value}",
                    oninput: move |event| {
                        let mut next = form_values();
                        next.insert(field.key.clone(), event.value());
                        form_values.set(next);
                    },
                    option { value: "", "Select {field.label}" }
                    for option in field.options {
                        option { value: "{option}", "{option}" }
                    }
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

fn should_show_join_form(entry: Option<UserEntryView>) -> bool {
    entry
        .map(|entry| {
            matches!(
                entry.status,
                QueueEntryStatus::Left | QueueEntryStatus::Resolved | QueueEntryStatus::Denied
            )
        })
        .unwrap_or(true)
}

fn join_button_label(user_session: Option<UserSessionRecord>) -> String {
    user_session
        .map(|session| format!("Join queue as {}", session.name))
        .unwrap_or_else(|| "Join queue as a guest".to_string())
}

fn user_status_label(entry: &UserEntryView) -> String {
    match (&entry.status, entry.claimed_by.as_deref()) {
        (QueueEntryStatus::Claimed, Some(name)) => format!("Claimed by {name}"),
        (QueueEntryStatus::Resolved, Some(name)) => format!("Resolved by {name}"),
        (QueueEntryStatus::Denied, Some(name)) => format!("Denied by {name}"),
        _ => status_label(&entry.status).to_string(),
    }
}
