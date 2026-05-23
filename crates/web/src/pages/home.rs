use dioxus::prelude::*;

use crate::models::AdminSessionRecord;
use crate::route::{navigate, Route};
use crate::storage::{load_admin_session, save_admin_session};
use crate::view_helpers::is_enter_key;
use crate::ws::login_admin_socket;

#[component]
pub fn HomePage(route: Signal<Route>) -> Element {
    let mut admin_email = use_signal(String::new);
    let mut admin_password = use_signal(String::new);
    let feedback = use_signal(String::new);
    let has_existing_session = load_admin_session().is_some();

    use_effect(move || {
        if has_existing_session {
            navigate(
                route,
                Route::Admin {
                    queue_id: None,
                    request_id: None,
                },
            );
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

    rsx! {
        if has_existing_session {
            section { class: "empty-stage",
                h1 { "Opening dashboard" }
                p { class: "lede", "Using your saved admin session." }
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
                        onkeydown: move |event| {
                            if is_enter_key(&event) {
                                event.prevent_default();
                                login.call(());
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
                                login.call(());
                            }
                        },
                        placeholder: "Password"
                    }
                }
                div { class: "action-stack",
                    button { class: "button button-primary", onclick: move |_| login.call(()), "Enter dashboard" }
                }
                if !feedback().is_empty() {
                    p { class: "feedback", "{feedback}" }
                }
            }
            }
        }
    }
}
