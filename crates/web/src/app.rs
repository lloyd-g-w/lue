use dioxus::prelude::*;

use crate::pages::{AdminPage, HomePage, QueuePage};
use crate::route::Route;
use crate::storage::{load_dark_theme, save_dark_theme};
use crate::styles::APP_CSS;

#[component]
pub fn App() -> Element {
    let route = use_signal(Route::current);
    let mut dark_theme = use_signal(|| load_dark_theme().unwrap_or(false));

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
        div { class: "shell",
            match route() {
                Route::Home => rsx! { HomePage { route } },
                Route::Admin { queue_id, request_id } => rsx! { AdminPage { route, selected_queue_id: queue_id, selected_request_id: request_id } },
                Route::Queue { queue_id } => rsx! { QueuePage { queue_id } },
            }
        }
    }
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
