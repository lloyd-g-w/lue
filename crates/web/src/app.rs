use std::cell::RefCell;
use std::rc::Rc;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use dioxus::prelude::*;
use serde::Deserialize;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::pages::{AdminPage, HomePage, QueuePage};
use crate::route::{replace_route, Route};
use crate::storage::{load_dark_theme, save_admin_session, save_dark_theme, save_user_session};
use crate::styles::APP_CSS;
use crate::{models::AdminSessionRecord, models::UserSessionRecord};

#[component]
pub fn App() -> Element {
    let mut route = use_signal(Route::current);
    let mut dark_theme = use_signal(|| load_dark_theme().unwrap_or(false));
    let mut toast = use_signal(|| None::<String>);
    let popstate_handler =
        use_hook(|| Rc::new(RefCell::new(None::<Closure<dyn FnMut(web_sys::Event)>>)));

    {
        let popstate_handler = popstate_handler.clone();
        use_effect(move || {
            if popstate_handler.borrow().is_some() {
                return;
            }

            let Some(window) = web_sys::window() else {
                return;
            };

            let handler = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
                route.set(Route::current());
            });
            let _ = window
                .add_event_listener_with_callback("popstate", handler.as_ref().unchecked_ref());
            *popstate_handler.borrow_mut() = Some(handler);
        });
    }

    {
        let popstate_handler = popstate_handler.clone();
        use_drop(move || {
            if let (Some(window), Some(handler)) =
                (web_sys::window(), popstate_handler.borrow_mut().take())
            {
                let _ = window.remove_event_listener_with_callback(
                    "popstate",
                    handler.as_ref().unchecked_ref(),
                );
            }
        });
    }

    use_effect(move || {
        apply_theme(dark_theme());
    });

    let toggle_theme = move |_| {
        let next = !dark_theme();
        dark_theme.set(next);
        save_dark_theme(next);
        apply_theme(next);
    };

    rsx! {
        document::Title { "Lue" }
        document::Stylesheet { href: "https://fonts.googleapis.com/css2?family=Manrope:wght@400;500;600;700;800&family=IBM+Plex+Mono:wght@400;500&display=swap" }
        style { {APP_CSS} }
        button {
            class: if dark_theme() { "theme-toggle theme-toggle-dark" } else { "theme-toggle" },
            title: if dark_theme() { "Use light theme" } else { "Use dark theme" },
            onclick: toggle_theme,
            if dark_theme() {
                SunIcon {}
            } else {
                MoonIcon {}
            }
        }
        if let Some(message) = toast() {
            div { class: "toast-stack",
                div { class: "toast toast-error",
                    p { "{message}" }
                    button {
                        class: "toast-dismiss",
                        title: "Dismiss",
                        onclick: move |_| toast.set(None),
                        "x"
                    }
                }
            }
        }
        div { class: "shell",
            match route() {
                Route::Home => rsx! { HomePage { route } },
                Route::MicrosoftAuthComplete => rsx! {
                    MicrosoftAuthCompletePage {
                        route,
                        on_error: move |message| toast.set(Some(message)),
                    }
                },
                Route::Admin { queue_id, request_id } => rsx! { AdminPage { route, selected_queue_id: queue_id, selected_request_id: request_id } },
                Route::Queue { queue_id } => rsx! { QueuePage { route, queue_id } },
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum MicrosoftAuthSession {
    Admin { session: AdminSessionRecord },
    User { session: UserSessionRecord },
}

#[component]
fn MicrosoftAuthCompletePage(route: Signal<Route>, on_error: EventHandler<String>) -> Element {
    let feedback = use_signal(|| "Finishing Microsoft sign-in...".to_string());

    use_effect(move || {
        let mut feedback = feedback;
        match complete_microsoft_auth(route) {
            Ok(()) => feedback.set("Signed in with Microsoft.".to_string()),
            Err(message) => {
                on_error.call(message);
                replace_route(route, Route::Home);
            }
        }
    });

    rsx! {
        section { class: "queue-page-layout",
            div { class: "ui-panel login-panel",
                p { class: "feedback", "{feedback}" }
            }
        }
    }
}

fn complete_microsoft_auth(route: Signal<Route>) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| "browser window unavailable".to_string())?;
    let search = window
        .location()
        .search()
        .map_err(|_| "failed to read Microsoft sign-in response".to_string())?;
    if let Some(error_payload) = query_value(&search, "error") {
        return Err(decode_url_payload(&error_payload)
            .unwrap_or_else(|| "Microsoft sign-in failed. Please try again.".to_string()));
    }
    let session_payload = query_value(&search, "session")
        .ok_or_else(|| "Microsoft sign-in response did not include a session".to_string())?;
    let return_to_payload = query_value(&search, "return_to").unwrap_or_default();
    let session_json = URL_SAFE_NO_PAD
        .decode(&session_payload)
        .map_err(|error| format!("invalid Microsoft session payload: {error}"))?;
    let session = serde_json::from_slice::<MicrosoftAuthSession>(&session_json)
        .map_err(|error| format!("invalid Microsoft session: {error}"))?;
    let return_to = decode_url_payload(&return_to_payload).unwrap_or_else(|| "/".to_string());

    match session {
        MicrosoftAuthSession::Admin { session } => {
            save_admin_session(&session);
            replace_route(route, Route::from_path(&return_to));
        }
        MicrosoftAuthSession::User { session } => {
            save_user_session(&session);
            replace_route(route, Route::from_path(&return_to));
        }
    }

    Ok(())
}

fn decode_url_payload(payload: &str) -> Option<String> {
    URL_SAFE_NO_PAD
        .decode(payload)
        .ok()
        .and_then(|payload| String::from_utf8(payload).ok())
}

fn query_value(search: &str, name: &str) -> Option<String> {
    search
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| (key == name).then(|| value.to_string()))
}

#[component]
fn MoonIcon() -> Element {
    rsx! {
        svg {
            class: "theme-icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" }
        }
    }
}

#[component]
fn SunIcon() -> Element {
    rsx! {
        svg {
            class: "theme-icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "12", cy: "12", r: "4" }
            path { d: "M12 2v2" }
            path { d: "M12 20v2" }
            path { d: "m4.93 4.93 1.41 1.41" }
            path { d: "m17.66 17.66 1.41 1.41" }
            path { d: "M2 12h2" }
            path { d: "M20 12h2" }
            path { d: "m6.34 17.66-1.41 1.41" }
            path { d: "m19.07 4.93-1.41 1.41" }
        }
    }
}

fn apply_theme(dark_theme: bool) {
    if let Some(root) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
    {
        let _ = root.set_attribute("data-theme", if dark_theme { "dark" } else { "light" });
    }
}
