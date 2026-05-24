use dioxus::prelude::*;
use shared::QueueSummary;

use crate::models::AdminSessionRecord;
use crate::route::{navigate, Route};
use crate::storage::{load_admin_session, save_admin_session};
use crate::view_helpers::is_enter_key;
use crate::ws::{
    check_setup_socket, list_public_queues_socket, login_admin_socket, setup_super_admin_socket,
};

#[component]
pub fn HomePage(route: Signal<Route>) -> Element {
    let mut setup_name = use_signal(String::new);
    let mut admin_email = use_signal(String::new);
    let mut admin_password = use_signal(String::new);
    let feedback = use_signal(String::new);
    let setup_required = use_signal(|| None::<bool>);
    let public_queues = use_signal(|| None::<Vec<QueueSummary>>);
    let has_existing_session = load_admin_session().is_some();

    use_effect(move || {
        if setup_required().is_none() {
            let mut setup_required = setup_required;
            check_setup_socket(
                move |needs_setup| setup_required.set(Some(needs_setup)),
                feedback,
            );
        }
        if public_queues().is_none() {
            let mut public_queues = public_queues;
            list_public_queues_socket(move |queues| public_queues.set(Some(queues)), feedback);
        }
    });

    let login = {
        let admin_email = admin_email;
        let admin_password = admin_password;
        let mut feedback = feedback;
        let route = route;
        EventHandler::new(move |_| {
            feedback.set("Signing in...".to_string());
            login_admin_socket(
                admin_email(),
                admin_password(),
                move |admin| {
                    save_admin_session(&AdminSessionRecord {
                        token: admin.token.clone(),
                        name: admin.name,
                        email: admin.email,
                        is_super_admin: admin.is_super_admin,
                    });
                    navigate(
                        route,
                        Route::Admin {
                            queue_id: None,
                            request_id: None,
                        },
                    );
                },
                feedback,
            );
        })
    };

    let setup = {
        let setup_name = setup_name;
        let admin_email = admin_email;
        let admin_password = admin_password;
        let mut feedback = feedback;
        let route = route;
        EventHandler::new(move |_| {
            feedback.set("Creating super admin...".to_string());
            setup_super_admin_socket(
                setup_name(),
                admin_email(),
                admin_password(),
                move |admin| {
                    save_admin_session(&AdminSessionRecord {
                        token: admin.token.clone(),
                        name: admin.name,
                        email: admin.email,
                        is_super_admin: admin.is_super_admin,
                    });
                    navigate(
                        route,
                        Route::Admin {
                            queue_id: None,
                            request_id: None,
                        },
                    );
                },
                feedback,
            );
        })
    };

    rsx! {
        if setup_required().is_none() || public_queues().is_none() {
            section { class: "empty-stage",
                h1 { "Checking setup" }
                p { class: "lede", "Connecting to the server." }
                if !feedback().is_empty() {
                    p { class: "feedback", "{feedback}" }
                }
            }
        } else {
            div { class: "landing-layout",
            section { class: "landing-copy",
                p { class: "kicker", "Queue System" }
                h1 { "A cleaner way to run live queues." }
                p { class: "landing-lede",
                    "Named admins, explicit queue ownership, request history, and real-time updates without the usual dashboard clutter."
                }
                div { class: "point-list",
                    if let Some(queues) = public_queues() {
                        if queues.is_empty() {
                            div { class: "empty-panel",
                                p { class: "hint", "No public queues are open right now." }
                            }
                        } else {
                            div { class: "public-queue-list",
                                for queue in queues {
                                    PublicQueueCard { route, queue }
                                }
                            }
                        }
                    }
                    if has_existing_session {
                        button {
                            class: "button button-secondary",
                            onclick: move |_| navigate(route, Route::Admin { queue_id: None, request_id: None }),
                            "Open admin dashboard"
                        }
                    }
                }
            }

            section { class: "login-panel",
                div { class: "panel-header",
                    div {
                        p { class: "kicker", "Admin Access" }
                        if setup_required() == Some(true) {
                            h2 { "Create super admin" }
                        } else {
                            h2 { "Sign in" }
                        }
                    }
                }
                if setup_required() == Some(true) {
                    div { class: "input-group",
                        label { class: "label", "Name" }
                        input {
                            class: "input",
                            value: "{setup_name}",
                            oninput: move |event| setup_name.set(event.value()),
                            onkeydown: move |event| {
                                if is_enter_key(&event) {
                                    event.prevent_default();
                                    setup.call(());
                                }
                            },
                            placeholder: "Super Admin"
                        }
                    }
                }
                div { class: "input-group",
                    label { class: "label", "Email" }
                    input {
                        class: "input",
                        value: "{admin_email}",
                        oninput: move |event| admin_email.set(event.value()),
                        onkeydown: move |event| {
                            if is_enter_key(&event) {
                                event.prevent_default();
                                if setup_required() == Some(true) {
                                    setup.call(());
                                } else {
                                    login.call(());
                                }
                            }
                        },
                        placeholder: "admin@example.com"
                    }
                }
                div { class: "input-group",
                    label { class: "label", "Password" }
                    input {
                        class: "input",
                        r#type: "password",
                        value: "{admin_password}",
                        oninput: move |event| admin_password.set(event.value()),
                        onkeydown: move |event| {
                            if is_enter_key(&event) {
                                event.prevent_default();
                                if setup_required() == Some(true) {
                                    setup.call(());
                                } else {
                                    login.call(());
                                }
                            }
                        },
                        placeholder: "Password"
                    }
                }
                div { class: "action-stack",
                    if setup_required() == Some(true) {
                        button { class: "button button-primary", onclick: move |_| setup.call(()), "Create account" }
                    } else {
                        button { class: "button button-primary", onclick: move |_| login.call(()), "Enter dashboard" }
                    }
                }
                if !feedback().is_empty() {
                    p { class: "feedback", "{feedback}" }
                }
            }
            }
        }
    }
}

#[component]
fn PublicQueueCard(route: Signal<Route>, queue: QueueSummary) -> Element {
    let queue_id = queue.id.to_string();
    rsx! {
        button {
            class: "public-queue-card",
            onclick: move |_| navigate(route, Route::Queue { queue_id: queue_id.clone() }),
            div {
                h3 { "{queue.name}" }
                p { class: "hint",
                    "{queue.waiting_count} waiting"
                    if queue.allow_guests {
                        " • guests allowed"
                    } else {
                        " • sign-in required"
                    }
                }
            }
            span { class: "counter-pill", "{queue.active_count} active" }
        }
    }
}
