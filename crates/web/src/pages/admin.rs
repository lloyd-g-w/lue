use dioxus::prelude::*;
use shared::{
    AccountRole, AdminEntryView, AdminQueueView, AdminStateView, ClientMessage, QueueEntryStatus,
    QueueField, ServerMessage,
};
use uuid::Uuid;
use web_sys::WebSocket;

use crate::models::{AccountDraft, EditableField};
use crate::route::{frontend_url, navigate, Route};
use crate::storage::{clear_admin_session, load_admin_session};
use crate::view_helpers::{format_timestamp, secondary_field, slugify, status_class, status_label};
use crate::ws::{connect_socket, send_ws};

#[component]
pub fn AdminPage(
    route: Signal<Route>,
    selected_queue_id: Option<String>,
    selected_request_id: Option<String>,
) -> Element {
    let Some(admin_session) = load_admin_session() else {
        return rsx! {
            section { class: "empty-stage",
                h1 { "No admin session" }
                p { class: "lede", "Sign in first to create or manage queues." }
                button {
                    class: "button button-primary",
                    onclick: move |_| navigate(route, Route::Home),
                    "Go to login"
                }
            }
        };
    };

    let selected_queue_uuid = selected_queue_id
        .as_deref()
        .and_then(|queue_id| Uuid::parse_str(queue_id).ok());
    let selected_request_uuid = selected_request_id
        .as_deref()
        .and_then(|request_id| Uuid::parse_str(request_id).ok());

    let admin_state = use_signal(|| None::<AdminStateView>);
    let feedback = use_signal(String::new);
    let socket = use_signal(|| None::<WebSocket>);
    let mut queue_name = use_signal(|| "Student Support".to_string());
    let mut queue_allow_guests = use_signal(|| true);
    let mut fields = use_signal(|| vec![EditableField::new("Name"), EditableField::new("Subject")]);
    let mut account_draft = use_signal(AccountDraft::default);

    let admin_token = admin_session.token.clone();
    let admin_label = admin_session.name.clone();
    let admin_email = admin_session.email.clone();
    let is_super_admin = admin_session.is_super_admin;

    use_effect(move || {
        let mut admin_state = admin_state;
        let mut feedback = feedback;
        let mut socket = socket;
        let admin_token = admin_token.clone();

        let ws = connect_socket(
            move |message| match message {
                ServerMessage::AdminState { state } => {
                    admin_state.set(Some(state));
                    feedback.set(String::new());
                }
                ServerMessage::QueueCreated { queue_id } => {
                    navigate(
                        route,
                        Route::Admin {
                            queue_id: Some(queue_id.to_string()),
                            request_id: None,
                        },
                    );
                }
                ServerMessage::AccountCreated => feedback.set("Account created".to_string()),
                ServerMessage::Error { message } => feedback.set(message),
                ServerMessage::Info { message } => feedback.set(message),
                _ => {}
            },
            move |ws| {
                let _ = send_ws(
                    &ws,
                    &ClientMessage::SubscribeAdmin {
                        admin_token: admin_token.clone(),
                        selected_queue_id: selected_queue_uuid,
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

    let create_queue = {
        let queue_name = queue_name;
        let fields = fields;
        let queue_allow_guests = queue_allow_guests;
        let socket = socket;
        let mut feedback = feedback;
        let admin_token = admin_session.token.clone();
        move |_| {
            let field_values: Vec<QueueField> = fields()
                .iter()
                .map(|field| QueueField {
                    key: slugify(&field.label),
                    label: field.label.clone(),
                    required: true,
                })
                .collect();

            if let Some(ws) = socket() {
                feedback.set("Creating queue...".to_string());
                let _ = send_ws(
                    &ws,
                    &ClientMessage::CreateQueue {
                        admin_token: admin_token.clone(),
                        name: queue_name(),
                        fields: field_values,
                        allow_guests: queue_allow_guests(),
                    },
                );
            } else {
                feedback.set("Admin socket is not connected".to_string());
            }
        }
    };

    let create_account = {
        let mut account_draft = account_draft;
        let socket = socket;
        let mut feedback = feedback;
        let admin_token = admin_session.token.clone();
        move |_| {
            if let Some(ws) = socket() {
                let draft = account_draft();
                feedback.set("Creating account...".to_string());
                let _ = send_ws(
                    &ws,
                    &ClientMessage::CreateAccount {
                        admin_token: admin_token.clone(),
                        name: draft.name,
                        email: draft.email,
                        password: draft.password,
                        role: draft.role,
                    },
                );
                account_draft.set(AccountDraft::default());
            } else {
                feedback.set("Admin socket is not connected".to_string());
            }
        }
    };

    let claim_entry = {
        let socket = socket;
        let admin_token = admin_session.token.clone();
        move |entry_id: Uuid| {
            if let Some(ws) = socket() {
                let _ = send_ws(
                    &ws,
                    &ClientMessage::ClaimEntry {
                        admin_token: admin_token.clone(),
                        entry_id,
                    },
                );
            }
        }
    };

    let resolve_entry = {
        let socket = socket;
        let admin_token = admin_session.token.clone();
        move |entry_id: Uuid| {
            if let Some(ws) = socket() {
                let _ = send_ws(
                    &ws,
                    &ClientMessage::ResolveEntry {
                        admin_token: admin_token.clone(),
                        entry_id,
                    },
                );
            }
        }
    };

    let unclaim_entry = {
        let socket = socket;
        let admin_token = admin_session.token.clone();
        move |entry_id: Uuid| {
            if let Some(ws) = socket() {
                let _ = send_ws(
                    &ws,
                    &ClientMessage::UnclaimEntry {
                        admin_token: admin_token.clone(),
                        entry_id,
                    },
                );
            }
        }
    };

    let deny_entry = {
        let socket = socket;
        let admin_token = admin_session.token.clone();
        move |entry_id: Uuid| {
            if let Some(ws) = socket() {
                let _ = send_ws(
                    &ws,
                    &ClientMessage::DenyEntry {
                        admin_token: admin_token.clone(),
                        entry_id,
                    },
                );
            }
        }
    };

    let sign_out = {
        let route = route;
        move |_| {
            clear_admin_session();
            navigate(route, Route::Home);
        }
    };

    let claim_entry = EventHandler::new(claim_entry);
    let resolve_entry = EventHandler::new(resolve_entry);
    let unclaim_entry = EventHandler::new(unclaim_entry);
    let deny_entry = EventHandler::new(deny_entry);

    let snapshot = admin_state();
    let selected_queue = snapshot
        .as_ref()
        .and_then(|state| state.selected_queue.clone());
    let selected_entry = selected_queue.as_ref().and_then(|queue| {
        selected_request_uuid.and_then(|request_id| {
            queue
                .entries
                .iter()
                .find(|entry| entry.id == request_id)
                .cloned()
        })
    });

    rsx! {
        div { class: "admin-shell",
            header { class: "admin-header",
                div {
                    p { class: "kicker", "Admin Workspace" }
                    h1 { class: "page-title", "Queue Operations" }
                    p { class: "lede",
                        strong { "{admin_label}" }
                        " • {admin_email}"
                        if is_super_admin {
                            " • super admin"
                        }
                    }
                }
                div { class: "button-row",
                    button {
                        class: "button button-secondary",
                        onclick: move |_| navigate(route, Route::Home),
                        "Home"
                    }
                    button { class: "button button-secondary", onclick: sign_out, "Sign out" }
                }
            }

            if let Some(state) = snapshot {
                div { class: "admin-grid",
                    aside { class: "sidebar-panel",
                        section { class: "sidebar-block",
                            div { class: "panel-header",
                                h2 { "Overview" }
                                span { class: "counter-chip", "{state.queues.len()} queues" }
                            }
                            p { class: "hint",
                                if state.admin.is_super_admin {
                                    "You can see every queue and create accounts for the whole system."
                                } else {
                                    "You can manage queues you created."
                                }
                            }
                            div { class: "detail-list compact-list",
                                div { class: "detail-row",
                                    span { class: "detail-key", "Queues" }
                                    div { class: "detail-value", "{state.queues.len()}" }
                                }
                                div { class: "detail-row",
                                    span { class: "detail-key", "Waiting" }
                                    div { class: "detail-value", "{total_waiting(&state)}" }
                                }
                                div { class: "detail-row",
                                    span { class: "detail-key", "Active" }
                                    div { class: "detail-value", "{total_active(&state)}" }
                                }
                            }
                            button {
                                class: "button button-secondary",
                                onclick: move |_| navigate(route, Route::Admin { queue_id: None, request_id: None }),
                                "Open queue list"
                            }
                        }

                        section { class: "sidebar-block",
                            div { class: "panel-header",
                                div {
                                    p { class: "kicker", "Create Queue" }
                                    h2 { "New queue" }
                                }
                            }
                            div { class: "input-group",
                                label { class: "label", "Queue name" }
                                input {
                                    class: "input",
                                    value: "{queue_name}",
                                    oninput: move |event| queue_name.set(event.value()),
                                    placeholder: "Student Support"
                                }
                            }
                            label { class: "toggle-row",
                                input {
                                    r#type: "checkbox",
                                    checked: "{queue_allow_guests}",
                                    oninput: move |event| queue_allow_guests.set(event.checked())
                                }
                                span { "Allow guests to join without a user account" }
                            }
                            div { class: "field-list",
                                for (index, field) in fields().iter().enumerate() {
                                    div { class: "field-editor-row",
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
                                            class: "button button-secondary",
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
                            }
                            div { class: "button-row",
                                button {
                                    class: "button button-secondary",
                                    onclick: move |_| {
                                        let mut next = fields();
                                        next.push(EditableField::new("New field"));
                                        fields.set(next);
                                    },
                                    "Add field"
                                }
                                button { class: "button button-primary", onclick: create_queue, "Create queue" }
                            }
                        }

                        if state.admin.is_super_admin {
                            section { class: "sidebar-block",
                                div { class: "panel-header",
                                    div {
                                        p { class: "kicker", "Accounts" }
                                        h2 { "Create account" }
                                    }
                                }
                                div { class: "input-group",
                                    label { class: "label", "Name" }
                                    input {
                                        class: "input",
                                        value: "{account_draft().name}",
                                        oninput: move |event| {
                                            let mut next = account_draft();
                                            next.name = event.value();
                                            account_draft.set(next);
                                        },
                                        placeholder: "Jordan Lee"
                                    }
                                }
                                div { class: "input-group",
                                    label { class: "label", "Email" }
                                    input {
                                        class: "input",
                                        value: "{account_draft().email}",
                                        oninput: move |event| {
                                            let mut next = account_draft();
                                            next.email = event.value();
                                            account_draft.set(next);
                                        },
                                        placeholder: "person@example.com"
                                    }
                                }
                                div { class: "input-group",
                                    label { class: "label", "Password" }
                                    input {
                                        class: "input",
                                        r#type: "password",
                                        value: "{account_draft().password}",
                                        oninput: move |event| {
                                            let mut next = account_draft();
                                            next.password = event.value();
                                            account_draft.set(next);
                                        },
                                        placeholder: "Temporary password"
                                    }
                                }
                                div { class: "input-group",
                                    label { class: "label", "Role" }
                                    select {
                                        class: "input",
                                        value: "{role_value(&account_draft().role)}",
                                        onchange: move |event| {
                                            let mut next = account_draft();
                                            next.role = role_from_value(&event.value());
                                            account_draft.set(next);
                                        },
                                        option { value: "user", "User" }
                                        option { value: "admin", "Admin" }
                                    }
                                }
                                button { class: "button button-primary", onclick: create_account, "Create account" }
                            }
                        }
                    }

                    main { class: "main-panel page-panel",
                        {render_admin_page(
                            route,
                            &state,
                            selected_queue_uuid,
                            selected_request_uuid,
                            selected_queue,
                            selected_entry,
                            claim_entry,
                            unclaim_entry,
                            resolve_entry,
                            deny_entry,
                        )}

                        if state.admin.is_super_admin && !state.accounts.is_empty() && selected_queue_uuid.is_none() {
                            section { class: "table-page-section",
                                div { class: "panel-header",
                                    div {
                                        p { class: "kicker", "Accounts" }
                                        h2 { "All accounts" }
                                    }
                                    span { class: "counter-chip", "{state.accounts.len()}" }
                                }
                                div { class: "table-shell",
                                    table { class: "data-table",
                                        thead {
                                            tr {
                                                th { "Name" }
                                                th { "Email" }
                                                th { "Role" }
                                            }
                                        }
                                        tbody {
                                            for account in state.accounts.iter().cloned() {
                                                tr {
                                                    td { "{account.name}" }
                                                    td { class: "mono small-text", "{account.email}" }
                                                    td { "{role_label(&account.role)}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if !feedback().is_empty() {
                            p { class: "feedback floating-feedback", "{feedback}" }
                        }
                    }
                }
            } else {
                section { class: "empty-stage",
                    h1 { "Connecting to dashboard..." }
                    if !feedback().is_empty() {
                        p { class: "feedback", "{feedback}" }
                    }
                }
            }
        }
    }
}

fn render_admin_page(
    route: Signal<Route>,
    state: &AdminStateView,
    selected_queue_uuid: Option<Uuid>,
    selected_request_uuid: Option<Uuid>,
    selected_queue: Option<AdminQueueView>,
    selected_entry: Option<AdminEntryView>,
    claim_entry: EventHandler<Uuid>,
    unclaim_entry: EventHandler<Uuid>,
    resolve_entry: EventHandler<Uuid>,
    deny_entry: EventHandler<Uuid>,
) -> Element {
    match (
        selected_queue_uuid,
        selected_request_uuid,
        selected_queue,
        selected_entry,
    ) {
        (None, _, _, _) => rsx! { QueueIndexPage { route, state: state.clone() } },
        (Some(queue_id), None, Some(queue), _) => rsx! {
            QueueRequestsPage {
                route,
                queue_id,
                queue,
                claim_entry,
                unclaim_entry,
                resolve_entry,
                deny_entry,
            }
        },
        (Some(queue_id), Some(_), Some(queue), Some(entry)) => rsx! {
            RequestDetailPage {
                route,
                queue_id,
                queue,
                entry,
                claim_entry,
                unclaim_entry,
                resolve_entry,
                deny_entry,
            }
        },
        (Some(_), Some(_), Some(queue), None) => rsx! {
            section { class: "empty-stage",
                p { class: "kicker", "Request" }
                h2 { "Request not found" }
                p { class: "lede", "That request is no longer available in this queue." }
                button {
                    class: "button button-secondary",
                    onclick: move |_| navigate(route, Route::Admin {
                        queue_id: Some(queue.summary.id.to_string()),
                        request_id: None,
                    }),
                    "Back to requests"
                }
            }
        },
        _ => rsx! {
            section { class: "empty-stage",
                p { class: "kicker", "Queue" }
                h2 { "Queue not found" }
                p { class: "lede", "That queue is unavailable or you no longer have access to it." }
                button {
                    class: "button button-secondary",
                    onclick: move |_| navigate(route, Route::Admin { queue_id: None, request_id: None }),
                    "Back to queues"
                }
            }
        },
    }
}

#[component]
fn QueueIndexPage(route: Signal<Route>, state: AdminStateView) -> Element {
    rsx! {
        section { class: "table-page-section",
            div { class: "panel-header",
                div {
                    p { class: "kicker", "Queues" }
                    h2 { "All queues" }
                }
                span { class: "counter-chip", "{state.queues.len()}" }
            }
            p { class: "lede", "Each queue has its own page of requests. Open one to work request-by-request." }
            if state.queues.is_empty() {
                div { class: "empty-panel",
                    p { "No queues yet. Create one from the sidebar." }
                }
            } else {
                div { class: "table-shell",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Queue" }
                                th { "Owner" }
                                th { "Access" }
                                th { "Waiting" }
                                th { "Active" }
                                th { "Open" }
                            }
                        }
                        tbody {
                            for queue in state.queues.iter().cloned() {
                                tr {
                                    td { "{queue.summary.name}" }
                                    td { "{queue.owner_name}" }
                                    td {
                                        if queue.summary.allow_guests {
                                            "Guests allowed"
                                        } else {
                                            "Accounts only"
                                        }
                                    }
                                    td { "{queue.summary.waiting_count}" }
                                    td { "{queue.summary.active_count}" }
                                    td {
                                        button {
                                            class: "button button-secondary table-action",
                                            onclick: move |_| navigate(route, Route::Admin {
                                                queue_id: Some(queue.summary.id.to_string()),
                                                request_id: None,
                                            }),
                                            "View requests"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn QueueRequestsPage(
    route: Signal<Route>,
    queue_id: Uuid,
    queue: AdminQueueView,
    claim_entry: EventHandler<Uuid>,
    unclaim_entry: EventHandler<Uuid>,
    resolve_entry: EventHandler<Uuid>,
    deny_entry: EventHandler<Uuid>,
) -> Element {
    let queue_link = frontend_url(&Route::Queue {
        queue_id: queue.summary.id.to_string(),
    });
    let queue_name = queue.summary.name.clone();

    rsx! {
        section { class: "table-page-section",
            div { class: "page-breadcrumbs mono small-text",
                button {
                    class: "breadcrumb-link",
                    onclick: move |_| navigate(route, Route::Admin { queue_id: None, request_id: None }),
                    "Queues"
                }
                span { "/" }
                span { "{queue_name}" }
            }
            div { class: "panel-header",
                div {
                    p { class: "kicker", "Requests" }
                    h2 { "{queue.summary.name}" }
                    p { class: "lede",
                        "Owned by {queue.owner_name} • {queue.summary.waiting_count} waiting • {queue.summary.active_count} active"
                    }
                }
                div { class: "button-row",
                    a { class: "button button-primary", href: queue_link, "Open user link" }
                    button {
                        class: "button button-secondary",
                        onclick: move |_| navigate(route, Route::Admin { queue_id: None, request_id: None }),
                        "Back to queues"
                    }
                }
            }
            div { class: "table-shell",
                table { class: "data-table data-table-clickable",
                    thead {
                        tr {
                            th { "Requester" }
                            th { "Subject" }
                            th { "Status" }
                            th { "Submitted" }
                            th { "Claimed by" }
                            th { "Actions" }
                        }
                    }
                    tbody {
                        for entry in queue.entries.iter().cloned() {
                            tr {
                                td {
                                    div { class: "table-primary",
                                        "{entry.requester_label}"
                                        if entry.is_guest {
                                            span { class: "table-inline-note", "Guest" }
                                        }
                                    }
                                    if let Some(email) = entry.requester_email.clone() {
                                        div { class: "mono small-text row-meta", "{email}" }
                                    }
                                }
                                td { "{secondary_field(queue.fields.as_slice(), &entry)}" }
                                td { span { class: status_class(&entry.status), "{status_label(&entry.status)}" } }
                                td { class: "mono small-text", "{format_timestamp(&entry.submitted_at)}" }
                                td { "{handled_by_label(&entry)}" }
                                td {
                                    div { class: "table-actions",
                                        if matches!(entry.status, QueueEntryStatus::Pending) {
                                            button {
                                                class: "button button-primary table-action",
                                                onclick: move |_| claim_entry.call(entry.id),
                                                "Claim"
                                            }
                                        }
                                        if matches!(entry.status, QueueEntryStatus::Claimed) {
                                            button {
                                                class: "button button-secondary table-action",
                                                onclick: move |_| unclaim_entry.call(entry.id),
                                                "Unclaim"
                                            }
                                        }
                                        if matches!(entry.status, QueueEntryStatus::Pending | QueueEntryStatus::Claimed) {
                                            button {
                                                class: "button success table-action",
                                                onclick: move |_| resolve_entry.call(entry.id),
                                                "Resolve"
                                            }
                                            button {
                                                class: "button danger table-action",
                                                onclick: move |_| deny_entry.call(entry.id),
                                                "Deny"
                                            }
                                        }
                                        button {
                                            class: "button button-secondary table-action",
                                            onclick: move |_| navigate(route, Route::Admin {
                                                queue_id: Some(queue_id.to_string()),
                                                request_id: Some(entry.id.to_string()),
                                            }),
                                            "Details"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if queue.entries.is_empty() {
                div { class: "empty-panel",
                    p { "No one has joined this queue yet." }
                }
            }
        }
    }
}

#[component]
fn RequestDetailPage(
    route: Signal<Route>,
    queue_id: Uuid,
    queue: AdminQueueView,
    entry: AdminEntryView,
    claim_entry: EventHandler<Uuid>,
    unclaim_entry: EventHandler<Uuid>,
    resolve_entry: EventHandler<Uuid>,
    deny_entry: EventHandler<Uuid>,
) -> Element {
    let queue_name = queue.summary.name.clone();
    let handled_by = entry
        .claimed_by
        .clone()
        .unwrap_or_else(|| "Unassigned".to_string());

    rsx! {
        section { class: "table-page-section",
            div { class: "page-breadcrumbs mono small-text",
                button {
                    class: "breadcrumb-link",
                    onclick: move |_| navigate(route, Route::Admin { queue_id: None, request_id: None }),
                    "Queues"
                }
                span { "/" }
                button {
                    class: "breadcrumb-link",
                    onclick: move |_| navigate(route, Route::Admin {
                        queue_id: Some(queue_id.to_string()),
                        request_id: None,
                    }),
                    "{queue_name}"
                }
                span { "/" }
                span { "{entry.requester_label}" }
            }

            div { class: "panel-header",
                div {
                    p { class: "kicker", "Request Detail" }
                    h2 { "{entry.requester_label}" }
                    p { class: "lede",
                        "Submitted {format_timestamp(&entry.submitted_at)}"
                        if let Some(email) = entry.requester_email.clone() {
                            " • {email}"
                        }
                        if entry.is_guest {
                            " • guest"
                        }
                    }
                }
                div { class: "button-row",
                    span { class: status_class(&entry.status), "{status_label(&entry.status)}" }
                    button {
                        class: "button button-secondary",
                        onclick: move |_| navigate(route, Route::Admin {
                            queue_id: Some(queue_id.to_string()),
                            request_id: None,
                        }),
                        "Back to requests"
                    }
                }
            }

            div { class: "detail-list",
                div { class: "detail-row",
                    span { class: "detail-key", "Claimed by" }
                    div { class: "detail-value", "{handled_by}" }
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

            div { class: "action-bar" ,
                if matches!(entry.status, QueueEntryStatus::Pending) {
                    button {
                        class: "button button-primary",
                        onclick: move |_| claim_entry.call(entry.id),
                        "Claim"
                    }
                }
                if matches!(entry.status, QueueEntryStatus::Claimed) {
                    button {
                        class: "button button-secondary",
                        onclick: move |_| unclaim_entry.call(entry.id),
                        "Unclaim"
                    }
                }
                if matches!(entry.status, QueueEntryStatus::Pending | QueueEntryStatus::Claimed) {
                    button {
                        class: "button success",
                        onclick: move |_| resolve_entry.call(entry.id),
                        "Resolve"
                    }
                    button {
                        class: "button danger",
                        onclick: move |_| deny_entry.call(entry.id),
                        "Deny"
                    }
                }
            }
            if matches!(entry.status, QueueEntryStatus::Left | QueueEntryStatus::Resolved | QueueEntryStatus::Denied) {
                p { class: "hint inspector-note", "No further actions are available for this request." }
            }
        }
    }
}

fn total_waiting(state: &AdminStateView) -> usize {
    state
        .queues
        .iter()
        .map(|queue| queue.summary.waiting_count)
        .sum()
}

fn total_active(state: &AdminStateView) -> usize {
    state
        .queues
        .iter()
        .map(|queue| queue.summary.active_count)
        .sum()
}

fn role_value(role: &AccountRole) -> &'static str {
    match role {
        AccountRole::SuperAdmin => "super_admin",
        AccountRole::Admin => "admin",
        AccountRole::User => "user",
    }
}

fn role_from_value(value: &str) -> AccountRole {
    match value {
        "admin" => AccountRole::Admin,
        _ => AccountRole::User,
    }
}

fn role_label(role: &AccountRole) -> &'static str {
    match role {
        AccountRole::SuperAdmin => "Super Admin",
        AccountRole::Admin => "Admin",
        AccountRole::User => "User",
    }
}

fn handled_by_label(entry: &AdminEntryView) -> String {
    entry
        .claimed_by
        .clone()
        .unwrap_or_else(|| "Unassigned".to_string())
}
