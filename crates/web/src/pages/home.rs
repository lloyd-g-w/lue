use dioxus::prelude::*;

use crate::models::AdminSessionRecord;
use crate::route::{navigate, Route};
use crate::storage::{load_admin_session, save_admin_session};
use crate::ws::login_admin_socket;

#[component]
pub fn HomePage(route: Signal<Route>) -> Element {
    let mut admin_email = use_signal(String::new);
    let mut admin_password = use_signal(String::new);
    let feedback = use_signal(String::new);
    let existing_session = load_admin_session();

    let login = {
        let admin_email = admin_email;
        let admin_password = admin_password;
        let mut feedback = feedback;
        let route = route;
        move |_| {
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
        }
    };

    rsx! {
        div { class: "landing-layout",
            section { class: "landing-copy",
                p { class: "kicker", "Queue System" }
                h1 { "A cleaner way to run live queues." }
                p { class: "landing-lede",
                    "Named admins, explicit queue ownership, request history, and real-time updates without the usual dashboard clutter."
                }
                div { class: "point-list",
                    div { class: "point-row",
                        span { class: "point-badge", "01" }
                        div {
                            h3 { "Named actions" }
                            p { class: "lede", "Claims and outcomes carry the admin name through to the user view." }
                        }
                    }
                    div { class: "point-row",
                        span { class: "point-badge", "02" }
                        div {
                            h3 { "Queues as a real list" }
                            p { class: "lede", "Browse queues as rows, open one, then drill into a request from its own list." }
                        }
                    }
                    div { class: "point-row",
                        span { class: "point-badge", "03" }
                        div {
                            h3 { "Live, but understandable" }
                            p { class: "lede", "Users get status updates immediately while admins keep a visible audit trail." }
                        }
                    }
                }
            }

            section { class: "login-panel",
                div { class: "panel-header",
                    div {
                        p { class: "kicker", "Admin Access" }
                        h2 { "Sign in" }
                    }
                }
                div { class: "input-group",
                    label { class: "label", "Email" }
                    input {
                        class: "input",
                        value: "{admin_email}",
                        oninput: move |event| admin_email.set(event.value()),
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
                        placeholder: "Password"
                    }
                }
                div { class: "action-stack",
                    button { class: "button button-primary", onclick: login, "Enter dashboard" }
                    if existing_session.is_some() {
                        button {
                            class: "button button-secondary",
                            onclick: move |_| navigate(route, Route::Admin { queue_id: None, request_id: None }),
                            "Resume saved session"
                        }
                    }
                }
                if let Some(session) = existing_session {
                    p { class: "hint",
                        "Saved session: "
                        strong { "{session.email}" }
                        if session.is_super_admin {
                            " (super admin)"
                        }
                    }
                }
                if !feedback().is_empty() {
                    p { class: "feedback", "{feedback}" }
                }
            }
        }
    }
}
