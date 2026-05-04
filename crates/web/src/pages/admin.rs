use dioxus::prelude::*;
use shared::{
    AccountRole, AccountView, AdminEntryView, AdminQueueView, AdminStateView, ClientMessage,
    GroupView, QueueEntryStatus, QueueField, ServerMessage,
};
use uuid::Uuid;
use web_sys::WebSocket;

use crate::models::{AccountDraft, EditableField, GroupDraft};
use crate::route::{frontend_url, navigate, Route};
use crate::storage::{clear_admin_session, load_admin_session};
use crate::view_helpers::{format_timestamp, secondary_field, slugify, status_class, status_label};
use crate::ws::{connect_reconnecting_socket, send_ws, SocketStatus};

#[derive(Clone, Copy, PartialEq, Eq)]
enum AdminSection {
    Queues,
    ClosedQueues,
    NewQueue,
    Accounts,
}

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
    let connection_status = use_signal(|| SocketStatus::Connecting);
    let mut active_section = use_signal(|| AdminSection::Queues);
    let socket = use_signal(|| None::<WebSocket>);
    let queue_name = use_signal(|| "Student Support".to_string());
    let queue_allow_guests = use_signal(|| true);
    let fields = use_signal(|| vec![EditableField::new("Name"), EditableField::new("Subject")]);
    let account_draft = use_signal(AccountDraft::default);
    let group_draft = use_signal(GroupDraft::default);
    let share_account_ids = use_signal(Vec::<Uuid>::new);
    let share_group_ids = use_signal(Vec::<Uuid>::new);

    let admin_token = admin_session.token.clone();
    let admin_label = admin_session.name.clone();
    let admin_email = admin_session.email.clone();
    let is_super_admin = admin_session.is_super_admin;

    use_effect(move || {
        let mut admin_state = admin_state;
        let mut feedback = feedback;
        let mut connection_status = connection_status;
        let mut active_section = active_section;
        let mut share_account_ids = share_account_ids;
        let mut share_group_ids = share_group_ids;
        let mut socket = socket;
        let admin_token = admin_token.clone();

        connect_reconnecting_socket(
            move |message| match message {
                ServerMessage::AdminState { state } => {
                    if let Some(queue) = state.selected_queue.as_ref() {
                        share_account_ids.set(queue.shared_account_ids.clone());
                        share_group_ids.set(queue.shared_group_ids.clone());
                    }
                    admin_state.set(Some(state));
                    if connection_status() == SocketStatus::Connected {
                        feedback.set(String::new());
                    }
                }
                ServerMessage::QueueCreated { queue_id } => {
                    active_section.set(AdminSection::Queues);
                    navigate(
                        route,
                        Route::Admin {
                            queue_id: Some(queue_id.to_string()),
                            request_id: None,
                        },
                    );
                }
                ServerMessage::AccountCreated => feedback.set("Account created".to_string()),
                ServerMessage::AccountUpdated => feedback.set("Account updated".to_string()),
                ServerMessage::AccountDeleted => feedback.set("Account deleted".to_string()),
                ServerMessage::GroupCreated => feedback.set("Group created".to_string()),
                ServerMessage::GroupUpdated => feedback.set("Group updated".to_string()),
                ServerMessage::GroupDeleted => feedback.set("Group deleted".to_string()),
                ServerMessage::QueueSharingUpdated => {
                    feedback.set("Queue access updated".to_string())
                }
                ServerMessage::QueueClosed => feedback.set("Queue closed and archived".to_string()),
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
            move |status| {
                connection_status.set(status);
                if status == SocketStatus::Reconnecting {
                    socket.set(None);
                    feedback.set("Connection dropped. Reconnecting live dashboard...".to_string());
                }
            },
        );
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
                feedback
                    .set("Reconnecting before queue creation. Try again in a moment.".to_string());
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
                feedback.set(
                    "Reconnecting before account creation. Try again in a moment.".to_string(),
                );
            }
        }
    };

    let create_group = {
        let mut group_draft = group_draft;
        let socket = socket;
        let mut feedback = feedback;
        let admin_token = admin_session.token.clone();
        move |_| {
            if let Some(ws) = socket() {
                let draft = group_draft();
                feedback.set("Creating group...".to_string());
                let _ = send_ws(
                    &ws,
                    &ClientMessage::CreateGroup {
                        admin_token: admin_token.clone(),
                        name: draft.name,
                        role: draft.role,
                        member_ids: draft.member_ids,
                    },
                );
                group_draft.set(GroupDraft::default());
            } else {
                feedback
                    .set("Reconnecting before group creation. Try again in a moment.".to_string());
            }
        }
    };

    let update_account = {
        let account_draft = account_draft;
        let socket = socket;
        let mut feedback = feedback;
        let admin_token = admin_session.token.clone();
        move |account_id: Uuid| {
            if let Some(ws) = socket() {
                let draft = account_draft();
                feedback.set("Updating account...".to_string());
                let password = if draft.password.trim().is_empty() {
                    None
                } else {
                    Some(draft.password)
                };
                let _ = send_ws(
                    &ws,
                    &ClientMessage::UpdateAccount {
                        admin_token: admin_token.clone(),
                        account_id,
                        name: draft.name,
                        email: draft.email,
                        password,
                        role: draft.role,
                    },
                );
            }
        }
    };

    let delete_account = {
        let socket = socket;
        let mut feedback = feedback;
        let admin_token = admin_session.token.clone();
        move |account_id: Uuid| {
            if let Some(ws) = socket() {
                feedback.set("Deleting account...".to_string());
                let _ = send_ws(
                    &ws,
                    &ClientMessage::DeleteAccount {
                        admin_token: admin_token.clone(),
                        account_id,
                    },
                );
            }
        }
    };

    let update_group = {
        let group_draft = group_draft;
        let socket = socket;
        let mut feedback = feedback;
        let admin_token = admin_session.token.clone();
        move |group_id: Uuid| {
            if let Some(ws) = socket() {
                let draft = group_draft();
                feedback.set("Updating group...".to_string());
                let _ = send_ws(
                    &ws,
                    &ClientMessage::UpdateGroup {
                        admin_token: admin_token.clone(),
                        group_id,
                        name: draft.name,
                        role: draft.role,
                        member_ids: draft.member_ids,
                    },
                );
            }
        }
    };

    let delete_group = {
        let socket = socket;
        let mut feedback = feedback;
        let admin_token = admin_session.token.clone();
        move |group_id: Uuid| {
            if let Some(ws) = socket() {
                feedback.set("Deleting group...".to_string());
                let _ = send_ws(
                    &ws,
                    &ClientMessage::DeleteGroup {
                        admin_token: admin_token.clone(),
                        group_id,
                    },
                );
            }
        }
    };

    let update_queue_sharing = {
        let socket = socket;
        let share_account_ids = share_account_ids;
        let share_group_ids = share_group_ids;
        let mut feedback = feedback;
        let admin_token = admin_session.token.clone();
        move |queue_id: Uuid| {
            if let Some(ws) = socket() {
                feedback.set("Updating queue access...".to_string());
                let _ = send_ws(
                    &ws,
                    &ClientMessage::ShareQueue {
                        admin_token: admin_token.clone(),
                        queue_id,
                        account_ids: share_account_ids(),
                        group_ids: share_group_ids(),
                    },
                );
            } else {
                feedback
                    .set("Reconnecting before sharing changes. Try again in a moment.".to_string());
            }
        }
    };

    let close_queue = {
        let socket = socket;
        let mut feedback = feedback;
        let admin_token = admin_session.token.clone();
        move |queue_id: Uuid| {
            if let Some(ws) = socket() {
                feedback.set("Closing queue...".to_string());
                let _ = send_ws(
                    &ws,
                    &ClientMessage::CloseQueue {
                        admin_token: admin_token.clone(),
                        queue_id,
                    },
                );
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
    let create_queue = EventHandler::new(create_queue);
    let create_account = EventHandler::new(create_account);
    let create_group = EventHandler::new(create_group);
    let update_account = EventHandler::new(update_account);
    let delete_account = EventHandler::new(delete_account);
    let update_group = EventHandler::new(update_group);
    let delete_group = EventHandler::new(delete_group);
    let update_queue_sharing = EventHandler::new(update_queue_sharing);
    let close_queue = EventHandler::new(close_queue);

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
            ConnectionBanner { status: connection_status() }
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
                            AdminNav {
                                active_section: active_section(),
                                is_super_admin: state.admin.is_super_admin,
                                on_select: move |section| {
                                    active_section.set(section);
                                    navigate(route, Route::Admin { queue_id: None, request_id: None });
                                },
                            }
                        }
                    }

                    main { class: "main-panel page-panel",
                        {render_admin_page(
                            route,
                            &state,
                            active_section(),
                            selected_queue_uuid,
                            selected_request_uuid,
                            selected_queue,
                            selected_entry,
                            queue_name,
                            queue_allow_guests,
                            fields,
                            account_draft,
                            group_draft,
                            share_account_ids,
                            share_group_ids,
                            create_queue,
                            create_account,
                            create_group,
                            update_account,
                            delete_account,
                            update_group,
                            delete_group,
                            update_queue_sharing,
                            close_queue,
                            claim_entry,
                            unclaim_entry,
                            resolve_entry,
                            deny_entry,
                        )}

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
    active_section: AdminSection,
    selected_queue_uuid: Option<Uuid>,
    selected_request_uuid: Option<Uuid>,
    selected_queue: Option<AdminQueueView>,
    selected_entry: Option<AdminEntryView>,
    queue_name: Signal<String>,
    queue_allow_guests: Signal<bool>,
    fields: Signal<Vec<EditableField>>,
    account_draft: Signal<AccountDraft>,
    group_draft: Signal<GroupDraft>,
    share_account_ids: Signal<Vec<Uuid>>,
    share_group_ids: Signal<Vec<Uuid>>,
    create_queue: EventHandler<MouseEvent>,
    create_account: EventHandler<MouseEvent>,
    create_group: EventHandler<MouseEvent>,
    update_account: EventHandler<Uuid>,
    delete_account: EventHandler<Uuid>,
    update_group: EventHandler<Uuid>,
    delete_group: EventHandler<Uuid>,
    update_queue_sharing: EventHandler<Uuid>,
    close_queue: EventHandler<Uuid>,
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
        (None, _, _, _) => match active_section {
            AdminSection::Queues => rsx! { QueueIndexPage { route, state: state.clone() } },
            AdminSection::ClosedQueues => rsx! { ClosedQueueIndexPage { state: state.clone() } },
            AdminSection::NewQueue => rsx! {
                NewQueuePage {
                    queue_name,
                    queue_allow_guests,
                    fields,
                    create_queue,
                }
            },
            AdminSection::Accounts if state.admin.is_super_admin => rsx! {
                AccountsPage {
                    state: state.clone(),
                    account_draft,
                    group_draft,
                    create_account,
                    create_group,
                    update_account,
                    delete_account,
                    update_group,
                    delete_group,
                }
            },
            AdminSection::Accounts => rsx! {
                section { class: "empty-stage",
                    p { class: "kicker", "Accounts" }
                    h2 { "Super admin only" }
                    p { class: "lede", "Account management is only available to super admins." }
                }
            },
        },
        (Some(queue_id), None, Some(queue), _) => rsx! {
            QueueRequestsPage {
                route,
                queue_id,
                queue,
                state: state.clone(),
                share_account_ids,
                share_group_ids,
                update_queue_sharing,
                close_queue,
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
fn ConnectionBanner(status: SocketStatus) -> Element {
    let (label, detail) = match status {
        SocketStatus::Connected => ("Live", "Dashboard updates are streaming."),
        SocketStatus::Connecting => ("Connecting", "Opening the live dashboard channel."),
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

#[component]
fn AdminNav(
    active_section: AdminSection,
    is_super_admin: bool,
    on_select: EventHandler<AdminSection>,
) -> Element {
    rsx! {
        nav { class: "admin-nav",
            button {
                class: nav_button_class(active_section, AdminSection::Queues),
                onclick: move |_| on_select.call(AdminSection::Queues),
                "All queues"
            }
            button {
                class: nav_button_class(active_section, AdminSection::NewQueue),
                onclick: move |_| on_select.call(AdminSection::NewQueue),
                "Create queue"
            }
            button {
                class: nav_button_class(active_section, AdminSection::ClosedQueues),
                onclick: move |_| on_select.call(AdminSection::ClosedQueues),
                "Closed queues"
            }
            if is_super_admin {
                button {
                    class: nav_button_class(active_section, AdminSection::Accounts),
                    onclick: move |_| on_select.call(AdminSection::Accounts),
                    "Account management"
                }
            }
        }
    }
}

#[component]
fn NewQueuePage(
    queue_name: Signal<String>,
    queue_allow_guests: Signal<bool>,
    fields: Signal<Vec<EditableField>>,
    create_queue: EventHandler<MouseEvent>,
) -> Element {
    let mut queue_name = queue_name;
    let mut queue_allow_guests = queue_allow_guests;
    let mut fields = fields;

    rsx! {
        section { class: "table-page-section split-view-section",
            div { class: "panel-header",
                div {
                    p { class: "kicker", "Create Queue" }
                    h2 { "New queue" }
                    p { class: "lede", "Set up the public join form and access mode before sharing a queue link." }
                }
            }
            div { class: "form-stack wide-form",
                div { class: "input-group",
                    label { class: "label", "Queue name" }
                    input {
                        class: "input",
                        value: "{queue_name}",
                        oninput: move |event| queue_name.set(event.value()),
                        placeholder: "Student Support"
                    }
                }
                label { class: "toggle-row ticket-panel",
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
                    button { class: "button button-primary", onclick: move |event| create_queue.call(event), "Create queue" }
                }
            }
        }
    }
}

#[component]
fn AccountsPage(
    state: AdminStateView,
    account_draft: Signal<AccountDraft>,
    group_draft: Signal<GroupDraft>,
    create_account: EventHandler<MouseEvent>,
    create_group: EventHandler<MouseEvent>,
    update_account: EventHandler<Uuid>,
    delete_account: EventHandler<Uuid>,
    update_group: EventHandler<Uuid>,
    delete_group: EventHandler<Uuid>,
) -> Element {
    let mut account_draft = account_draft;
    let mut group_draft = group_draft;
    let mut editing_account_id = use_signal(|| None::<Uuid>);
    let mut editing_group_id = use_signal(|| None::<Uuid>);
    let mut account_modal_open = use_signal(|| false);
    let mut group_modal_open = use_signal(|| false);

    rsx! {
        section { class: "table-page-section split-view-section",
            div { class: "panel-header",
                div {
                    p { class: "kicker", "Accounts" }
                    h2 { "Account management" }
                    p { class: "lede", "Create admins, users, or additional super admins. Passwords are stored as hashes." }
                }
                div { class: "button-row",
                    span { class: "counter-chip", "{state.accounts.len()} accounts" }
                    button {
                        class: "button button-primary",
                        onclick: move |_| {
                            editing_account_id.set(None);
                            account_draft.set(AccountDraft::default());
                            account_modal_open.set(true);
                        },
                        "New account"
                    }
                    button {
                        class: "button button-secondary",
                        onclick: move |_| {
                            editing_group_id.set(None);
                            group_draft.set(GroupDraft::default());
                            group_modal_open.set(true);
                        },
                        "New group"
                    }
                }
            }
            div { class: "account-management-grid",
                div { class: "table-shell",
                    table { class: "data-table accounts-table",
                        thead {
                            tr {
                                th { "Name" }
                                th { "Email" }
                                th { "Role" }
                                th { "Actions" }
                            }
                        }
                        tbody {
                            for account in state.accounts.iter().cloned() {
                                tr {
                                    td { "{account.name}" }
                                    td { class: "mono small-text", "{account.email}" }
                                    td { "{role_label(&account.role)}" }
                                    td {
                                        div { class: "table-actions",
                                            button {
                                                class: "action-button",
                                                onclick: move |_| {
                                                    editing_account_id.set(Some(account.id));
                                                    account_draft.set(AccountDraft {
                                                        name: account.name.clone(),
                                                        email: account.email.clone(),
                                                        password: String::new(),
                                                        role: account.role.clone(),
                                                    });
                                                    account_modal_open.set(true);
                                                },
                                                "Edit"
                                            }
                                            button {
                                                class: "action-button action-danger",
                                                onclick: move |_| delete_account.call(account.id),
                                                "Delete"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "table-shell",
                    table { class: "data-table accounts-table",
                        thead {
                            tr {
                                th { "Group" }
                                th { "Type" }
                                th { "Members" }
                                th { "Actions" }
                            }
                        }
                        tbody {
                            for group in state.groups.iter().cloned() {
                                tr {
                                    td { "{group.name}" }
                                    td { "{role_label(&group.role)}" }
                                    td { "{member_names(&state.accounts, &group.member_ids)}" }
                                    td {
                                        div { class: "table-actions",
                                            button {
                                                class: "action-button",
                                                onclick: move |_| {
                                                    editing_group_id.set(Some(group.id));
                                                    group_draft.set(GroupDraft {
                                                        name: group.name.clone(),
                                                        role: group.role.clone(),
                                                        member_ids: group.member_ids.clone(),
                                                    });
                                                    group_modal_open.set(true);
                                                },
                                                "Edit"
                                            }
                                            button {
                                                class: "action-button action-danger",
                                                onclick: move |_| delete_group.call(group.id),
                                                "Delete"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if account_modal_open() {
                div { class: "modal-backdrop",
                    div { class: "modal-panel form-stack",
                        div { class: "panel-header",
                            div {
                                p { class: "kicker", "Accounts" }
                                h2 {
                                    if editing_account_id().is_some() {
                                        "Edit account"
                                    } else {
                                        "Create account"
                                    }
                                }
                            }
                            button {
                                class: "action-button",
                                onclick: move |_| {
                                    account_modal_open.set(false);
                                    editing_account_id.set(None);
                                    account_draft.set(AccountDraft::default());
                                },
                                "Close"
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
                                placeholder: if editing_account_id().is_some() { "Leave blank to keep current password" } else { "Temporary password" }
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
                                option { value: "super_admin", "Super Admin" }
                            }
                        }
                        div { class: "button-row",
                            if let Some(account_id) = editing_account_id() {
                                button {
                                    class: "button button-primary",
                                    onclick: move |_| {
                                        update_account.call(account_id);
                                        account_modal_open.set(false);
                                    },
                                    "Save account"
                                }
                            } else {
                                button {
                                    class: "button button-primary",
                                    onclick: move |event| {
                                        create_account.call(event);
                                        account_modal_open.set(false);
                                    },
                                    "Create account"
                                }
                            }
                            button {
                                class: "button button-secondary",
                                onclick: move |_| {
                                    account_modal_open.set(false);
                                    editing_account_id.set(None);
                                    account_draft.set(AccountDraft::default());
                                },
                                "Cancel"
                            }
                        }
                    }
                }
            }
            if group_modal_open() {
                div { class: "modal-backdrop",
                    div { class: "modal-panel form-stack",
                        div { class: "panel-header",
                            div {
                                p { class: "kicker", "Groups" }
                                h2 {
                                    if editing_group_id().is_some() {
                                        "Edit group"
                                    } else {
                                        "Create group"
                                    }
                                }
                            }
                            button {
                                class: "action-button",
                                onclick: move |_| {
                                    group_modal_open.set(false);
                                    editing_group_id.set(None);
                                    group_draft.set(GroupDraft::default());
                                },
                                "Close"
                            }
                        }
                        div { class: "input-group",
                            label { class: "label", "Group name" }
                            input {
                                class: "input",
                                value: "{group_draft().name}",
                                oninput: move |event| {
                                    let mut next = group_draft();
                                    next.name = event.value();
                                    group_draft.set(next);
                                },
                                placeholder: "Support admins"
                            }
                        }
                        div { class: "input-group",
                            label { class: "label", "Group type" }
                            select {
                                class: "input",
                                value: "{role_value(&group_draft().role)}",
                                onchange: move |event| {
                                    let mut next = group_draft();
                                    next.role = match event.value().as_str() {
                                        "user" => AccountRole::User,
                                        _ => AccountRole::Admin,
                                    };
                                    next.member_ids.clear();
                                    group_draft.set(next);
                                },
                                option { value: "admin", "Admin group" }
                                option { value: "user", "User group" }
                            }
                        }
                        div { class: "checkbox-list modal-member-list",
                            for account in accounts_for_group(&state.accounts, &group_draft().role) {
                                label { class: "check-row",
                                    input {
                                        r#type: "checkbox",
                                        checked: "{group_draft().member_ids.contains(&account.id)}",
                                        oninput: move |event| {
                                            let mut next = group_draft();
                                            set_uuid_selected(&mut next.member_ids, account.id, event.checked());
                                            group_draft.set(next);
                                        }
                                    }
                                    span { "{account.name} " }
                                    span { class: "mono small-text", "{account.email}" }
                                }
                            }
                        }
                        div { class: "button-row",
                            if let Some(group_id) = editing_group_id() {
                                button {
                                    class: "button button-primary",
                                    onclick: move |_| {
                                        update_group.call(group_id);
                                        group_modal_open.set(false);
                                    },
                                    "Save group"
                                }
                            } else {
                                button {
                                    class: "button button-primary",
                                    onclick: move |event| {
                                        create_group.call(event);
                                        group_modal_open.set(false);
                                    },
                                    "Create group"
                                }
                            }
                            button {
                                class: "button button-secondary",
                                onclick: move |_| {
                                    group_modal_open.set(false);
                                    editing_group_id.set(None);
                                    group_draft.set(GroupDraft::default());
                                },
                                "Cancel"
                            }
                        }
                    }
                }
            }
        }
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
                    p { "No queues yet. Open Create queue from the admin navigation." }
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
                                            class: "action-button",
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
fn ClosedQueueIndexPage(state: AdminStateView) -> Element {
    rsx! {
        section { class: "table-page-section",
            div { class: "panel-header",
                div {
                    p { class: "kicker", "Archive" }
                    h2 { "Closed queues" }
                    p { class: "lede", "Closed queues are removed from active queue lists but retained here with their request history summary." }
                }
                span { class: "counter-chip", "{state.archived_queues.len()} closed" }
            }
            if state.archived_queues.is_empty() {
                div { class: "empty-panel",
                    p { "No queues have been closed yet." }
                }
            } else {
                div { class: "table-shell",
                    table { class: "data-table accounts-table",
                        thead {
                            tr {
                                th { "Queue" }
                                th { "Owner" }
                                th { "Closed by" }
                                th { "Closed" }
                                th { "Entries" }
                            }
                        }
                        tbody {
                            for queue in state.archived_queues.iter().cloned() {
                                tr {
                                    td { "{queue.summary.name}" }
                                    td { "{queue.owner_name}" }
                                    td { "{queue.closed_by_name}" }
                                    td { class: "mono small-text", "{format_timestamp(&queue.closed_at)}" }
                                    td { "{queue.entry_count}" }
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
    state: AdminStateView,
    share_account_ids: Signal<Vec<Uuid>>,
    share_group_ids: Signal<Vec<Uuid>>,
    update_queue_sharing: EventHandler<Uuid>,
    close_queue: EventHandler<Uuid>,
    claim_entry: EventHandler<Uuid>,
    unclaim_entry: EventHandler<Uuid>,
    resolve_entry: EventHandler<Uuid>,
    deny_entry: EventHandler<Uuid>,
) -> Element {
    let mut share_account_ids = share_account_ids;
    let mut share_group_ids = share_group_ids;
    let mut share_modal_open = use_signal(|| false);
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
                        onclick: move |_| {
                            share_account_ids.set(queue.shared_account_ids.clone());
                            share_group_ids.set(queue.shared_group_ids.clone());
                            share_modal_open.set(true);
                        },
                        "Share access"
                    }
                    button {
                        class: "button button-secondary",
                        onclick: move |_| navigate(route, Route::Admin { queue_id: None, request_id: None }),
                        "Back to queues"
                    }
                    button {
                        class: "button danger",
                        onclick: move |_| {
                            close_queue.call(queue_id);
                            navigate(route, Route::Admin { queue_id: None, request_id: None });
                        },
                        "Close queue"
                    }
                }
            }
            if share_modal_open() {
                div { class: "modal-backdrop",
                    div { class: "modal-panel form-stack",
                        div { class: "panel-header",
                            div {
                                p { class: "kicker", "Shared Access" }
                                h2 { "Share queue access" }
                            }
                            button {
                                class: "action-button",
                                onclick: move |_| share_modal_open.set(false),
                                "Close"
                            }
                        }
                        p { class: "hint", "Queue owners and super admins always retain access. Select additional admins or admin groups, then save." }
                        div { class: "share-grid",
                            div { class: "checkbox-list modal-member-list",
                                p { class: "label", "Admin accounts" }
                                for account in admin_accounts_for_share(&state.accounts, queue.owner_account_id) {
                                    label { class: "check-row",
                                        input {
                                            r#type: "checkbox",
                                            checked: "{share_account_ids().contains(&account.id)}",
                                            oninput: move |event| {
                                                let mut next = share_account_ids();
                                                set_uuid_selected(&mut next, account.id, event.checked());
                                                share_account_ids.set(next);
                                            }
                                        }
                                        span { "{account.name} " }
                                        span { class: "mono small-text", "{account.email}" }
                                    }
                                }
                            }
                            div { class: "checkbox-list modal-member-list",
                                p { class: "label", "Admin groups" }
                                for group in admin_groups_for_share(&state.groups) {
                                    label { class: "check-row",
                                        input {
                                            r#type: "checkbox",
                                            checked: "{share_group_ids().contains(&group.id)}",
                                            oninput: move |event| {
                                                let mut next = share_group_ids();
                                                set_uuid_selected(&mut next, group.id, event.checked());
                                                share_group_ids.set(next);
                                            }
                                        }
                                        span { "{group.name} " }
                                        span { class: "mono small-text", "{group.member_ids.len()} members" }
                                    }
                                }
                            }
                        }
                        div { class: "button-row",
                            button {
                                class: "button button-primary",
                                onclick: move |_| {
                                    update_queue_sharing.call(queue_id);
                                    share_modal_open.set(false);
                                },
                                "Save access"
                            }
                            button {
                                class: "button button-secondary",
                                onclick: move |_| share_modal_open.set(false),
                                "Cancel"
                            }
                        }
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
                                                class: "action-button action-strong",
                                                onclick: move |_| claim_entry.call(entry.id),
                                                "Claim"
                                            }
                                        }
                                        if matches!(entry.status, QueueEntryStatus::Claimed) {
                                            button {
                                                class: "action-button",
                                                onclick: move |_| unclaim_entry.call(entry.id),
                                                "Unclaim"
                                            }
                                        }
                                        if matches!(entry.status, QueueEntryStatus::Pending | QueueEntryStatus::Claimed) {
                                            button {
                                                class: "action-button action-success",
                                                onclick: move |_| resolve_entry.call(entry.id),
                                                "Resolve"
                                            }
                                            button {
                                                class: "action-button action-danger",
                                                onclick: move |_| deny_entry.call(entry.id),
                                                "Deny"
                                            }
                                        }
                                        button {
                                            class: "action-button",
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
        "super_admin" => AccountRole::SuperAdmin,
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

fn accounts_for_group(accounts: &[AccountView], role: &AccountRole) -> Vec<AccountView> {
    accounts
        .iter()
        .filter(|account| match role {
            AccountRole::Admin => {
                matches!(account.role, AccountRole::Admin | AccountRole::SuperAdmin)
            }
            AccountRole::User => matches!(
                account.role,
                AccountRole::User | AccountRole::Admin | AccountRole::SuperAdmin
            ),
            AccountRole::SuperAdmin => false,
        })
        .cloned()
        .collect()
}

fn admin_accounts_for_share(accounts: &[AccountView], owner_account_id: Uuid) -> Vec<AccountView> {
    accounts
        .iter()
        .filter(|account| {
            account.id != owner_account_id
                && matches!(account.role, AccountRole::Admin | AccountRole::SuperAdmin)
        })
        .cloned()
        .collect()
}

fn admin_groups_for_share(groups: &[GroupView]) -> Vec<GroupView> {
    groups
        .iter()
        .filter(|group| group.role == AccountRole::Admin)
        .cloned()
        .collect()
}

fn set_uuid_selected(ids: &mut Vec<Uuid>, id: Uuid, selected: bool) {
    if selected {
        if !ids.contains(&id) {
            ids.push(id);
        }
    } else {
        ids.retain(|existing_id| *existing_id != id);
    }
}

fn member_names(accounts: &[AccountView], member_ids: &[Uuid]) -> String {
    let names: Vec<String> = member_ids
        .iter()
        .filter_map(|member_id| accounts.iter().find(|account| account.id == *member_id))
        .map(|account| account.name.clone())
        .collect();
    if names.is_empty() {
        "No members".to_string()
    } else {
        names.join(", ")
    }
}

fn handled_by_label(entry: &AdminEntryView) -> String {
    entry
        .claimed_by
        .clone()
        .unwrap_or_else(|| "Unassigned".to_string())
}

fn nav_button_class(active: AdminSection, target: AdminSection) -> &'static str {
    if active == target {
        "admin-nav-button admin-nav-button-active"
    } else {
        "admin-nav-button"
    }
}

fn connection_class(status: SocketStatus) -> &'static str {
    match status {
        SocketStatus::Connected => "connection-live",
        SocketStatus::Connecting => "connection-connecting",
        SocketStatus::Reconnecting => "connection-reconnecting",
    }
}
