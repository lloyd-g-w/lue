use dioxus::prelude::*;
use shared::QueueSummary;

use crate::components::DelayedLoading;
use crate::models::UserSessionRecord;
use crate::route::{navigate, Route};
use crate::storage::{clear_user_session, load_user_session, save_user_session};
use crate::view_helpers::{is_enter_key, kebab_case};
use crate::ws::{list_public_queues_socket, login_user_socket};

#[component]
pub fn HomePage(route: Signal<Route>) -> Element {
    let mut user_email = use_signal(String::new);
    let mut user_password = use_signal(String::new);
    let user_session = use_signal(load_user_session);
    let feedback = use_signal(String::new);
    let public_queues = use_signal(|| None::<Vec<QueueSummary>>);

    use_effect(move || {
        if public_queues().is_none() {
            let mut public_queues = public_queues;
            list_public_queues_socket(move |queues| public_queues.set(Some(queues)), feedback);
        }
    });

    let login_user = {
        let user_email = user_email;
        let user_password = user_password;
        let mut user_session = user_session;
        let mut feedback = feedback;
        EventHandler::new(move |_| {
            feedback.set("Signing in...".to_string());
            login_user_socket(
                user_email(),
                user_password(),
                move |user| {
                    let session = UserSessionRecord {
                        token: user.token.clone(),
                        name: user.name,
                        email: user.email,
                    };
                    save_user_session(&session);
                    user_session.set(Some(session));
                    feedback.set(String::new());
                },
                feedback,
            );
        })
    };

    let sign_out_user = {
        let mut user_session = user_session;
        let mut feedback = feedback;
        move |_| {
            clear_user_session();
            user_session.set(None);
            feedback.set(String::new());
        }
    };

    rsx! {
        if public_queues().is_none() {
            DelayedLoading {
                title: "Checking queues".to_string(),
                detail: Some("Connecting to the server.".to_string()),
                feedback: feedback(),
            }
        } else {
            div { class: "home-layout",
            section { class: "home-main",
                div { class: "home-title-row",
                    div {
                        h1 { "Queues" }
                    }
                }
                div { class: "public-queue-list",
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
                }
            }

            section { class: "login-panel",
                div { class: "panel-header",
                    div {
                        p { class: "kicker", "User Access" }
                        h2 { "Sign in" }
                    }
                }
                if let Some(user) = user_session() {
                    div { class: "signed-in-strip",
                        span { "Signed in as {user.email}" }
                        button {
                            class: "button button-secondary",
                            onclick: sign_out_user,
                            "Sign out"
                        }
                    }
                } else {
                    div { class: "input-group",
                        label { class: "label", "Email" }
                        input {
                            class: "input",
                            value: "{user_email}",
                            oninput: move |event| user_email.set(event.value()),
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
                            value: "{user_password}",
                            oninput: move |event| user_password.set(event.value()),
                            onkeydown: move |event| {
                                if is_enter_key(&event) {
                                    event.prevent_default();
                                    login_user.call(());
                                }
                            },
                            placeholder: "Password"
                        }
                    }
                    div { class: "action-stack",
                        button {
                            class: "button button-primary",
                            onclick: move |_| login_user.call(()),
                            "Sign in as user"
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
}

#[component]
fn PublicQueueCard(route: Signal<Route>, queue: QueueSummary) -> Element {
    let queue_id = queue.id.to_string();
    let queue_name = kebab_case(&queue.name);
    rsx! {
        button {
            class: "public-queue-card",
            onclick: move |_| navigate(route, Route::Queue { queue_id: queue_id.clone() }),
            div {
                h3 { "{queue_name}" }
                p { class: "hint", "{queue.waiting_count} waiting" }
            }
            span { class: "counter-pill",
                if queue.allow_guests {
                    "Guest"
                } else {
                    "Sign in"
                }
            }
        }
    }
}
