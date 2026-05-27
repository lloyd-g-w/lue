use dioxus::prelude::*;
use shared::{QueueSummary, SiteSettingsView};

use crate::components::{DelayedLoading, UiButton, UiEmpty, UiHeader, UiModal};
use crate::models::UserSessionRecord;
use crate::route::{navigate, Route};
use crate::storage::{clear_user_session, load_user_session, save_user_session};
use crate::view_helpers::{is_enter_key, kebab_case};
use crate::ws::{
    backend_http_url, list_public_queues_socket, login_user_socket, resolve_queue_code_socket,
};

#[component]
pub fn HomePage(route: Signal<Route>) -> Element {
    let mut user_email = use_signal(String::new);
    let mut user_password = use_signal(String::new);
    let mut queue_code = use_signal(String::new);
    let user_session = use_signal(load_user_session);
    let mut auth_panel_open = use_signal(|| false);
    let feedback = use_signal(String::new);
    let public_queues = use_signal(|| None::<Vec<QueueSummary>>);
    let site_settings = use_signal(|| SiteSettingsView {
        site_title: "Lue".to_string(),
        admin_password_sign_in_enabled: true,
        admin_microsoft_sign_in_enabled: true,
        user_password_sign_in_enabled: true,
        user_microsoft_sign_in_enabled: true,
    });

    use_effect(move || {
        if public_queues().is_none() {
            let mut public_queues = public_queues;
            let mut site_settings = site_settings;
            list_public_queues_socket(
                move |queues, settings| {
                    site_settings.set(settings);
                    public_queues.set(Some(queues));
                },
                feedback,
            );
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
                    auth_panel_open.set(false);
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
            auth_panel_open.set(false);
            feedback.set(String::new());
        }
    };
    let sign_in_with_microsoft = move |_| {
        if let Some(window) = web_sys::window() {
            let return_to = window
                .location()
                .pathname()
                .unwrap_or_else(|_| "/".to_string());
            let _ = window.location().set_href(&backend_http_url(&format!(
                "/auth/microsoft/start?kind=user&return_to={return_to}"
            )));
        }
    };
    let join_by_code = {
        let queue_code = queue_code;
        let mut feedback = feedback;
        EventHandler::new(move |_| {
            let code = queue_code();
            feedback.set("Finding queue...".to_string());
            resolve_queue_code_socket(
                code.clone(),
                move |_| {
                    feedback.set(String::new());
                    navigate(
                        route,
                        Route::Queue {
                            queue_id: normalized_queue_code(&code),
                        },
                    );
                },
                feedback,
            );
        })
    };

    rsx! {
        document::Title { "{site_settings().site_title}" }
        if public_queues().is_none() {
            DelayedLoading {
                title: "Checking queues".to_string(),
                detail: Some("Connecting to the server.".to_string()),
                feedback: feedback(),
            }
        } else {
            div { class: "top-auth-control",
                if user_session().is_some() {
                    UiButton {
                        label: "Sign out".to_string(),
                        variant: "secondary".to_string(),
                        class: "top-auth-button".to_string(),
                        onclick: sign_out_user,
                    }
                } else {
                    UiButton {
                        label: "Sign in".to_string(),
                        variant: "secondary".to_string(),
                        class: "top-auth-button".to_string(),
                        onclick: move |_| auth_panel_open.set(true),
                    }
                }
            }
            div { class: "home-layout",
            section { class: "home-main",
                div { class: "home-title-row",
                    div {
                        h1 { "Queues" }
                    }
                }
                div { class: "join-code-panel",
                    div { class: "input-group",
                        label { class: "label", "Queue code" }
                        input {
                            class: "input code-input",
                            value: "{queue_code}",
                            oninput: move |event| queue_code.set(event.value().to_ascii_uppercase()),
                            onkeydown: move |event| {
                                if is_enter_key(&event) {
                                    event.prevent_default();
                                    join_by_code.call(());
                                }
                            },
                            placeholder: "ABC123"
                        }
                    }
                    UiButton {
                        label: "Join".to_string(),
                        variant: "secondary".to_string(),
                        onclick: move |_| join_by_code.call(()),
                    }
                }
                if !feedback().is_empty() && public_queues().is_some() && !auth_panel_open() {
                    p { class: "feedback", "{feedback}" }
                }
                div { class: "public-queue-list",
                    if let Some(queues) = public_queues() {
                        if queues.is_empty() {
                            UiEmpty { message: "No public queues are open right now.".to_string() }
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
            }
            if auth_panel_open() {
                UiModal { class: "auth-modal".to_string(),
                    div { class: "modal-login-panel",
                        UiHeader {
                            kicker: "User Access".to_string(),
                            title: "Sign in".to_string(),
                            UiButton {
                                label: "Close".to_string(),
                                variant: "secondary".to_string(),
                                onclick: move |_| auth_panel_open.set(false),
                            }
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
                            }
                            div { class: "action-stack",
                                if site_settings().user_password_sign_in_enabled {
                                    UiButton {
                                        label: "Sign in as user".to_string(),
                                        variant: "primary".to_string(),
                                        onclick: move |_| login_user.call(()),
                                    }
                                }
                                if site_settings().user_microsoft_sign_in_enabled {
                                    UiButton {
                                        label: "Sign in with Microsoft".to_string(),
                                        variant: "secondary".to_string(),
                                        onclick: sign_in_with_microsoft,
                                    }
                                }
                            }
                            if !site_settings().user_password_sign_in_enabled && !site_settings().user_microsoft_sign_in_enabled {
                                p { class: "hint", "User sign-in is currently unavailable." }
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
}

fn normalized_queue_code(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_uppercase())
        .collect()
}

#[component]
fn PublicQueueCard(route: Signal<Route>, queue: QueueSummary) -> Element {
    let queue_code = queue.code.clone();
    let queue_name = kebab_case(&queue.name);
    rsx! {
        button {
            class: "public-queue-card",
            onclick: move |_| navigate(route, Route::Queue { queue_id: queue_code.clone() }),
            div {
                h3 { "{queue_name}" }
                p { class: "hint", "Code {queue.code} • {queue.waiting_count} waiting" }
            }
            span { class: "counter-pill",
                if queue.allow_guests {
                    "Guests allowed"
                } else {
                    "Sign in required"
                }
            }
        }
    }
}
