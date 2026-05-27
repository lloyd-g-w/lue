use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

const LOADING_DELAY_MS: i32 = 180;

#[component]
pub fn DelayedLoading(title: String, detail: Option<String>, feedback: String) -> Element {
    let mut visible = use_signal(|| false);
    let timeout_id = use_hook(|| Rc::new(Cell::new(None::<i32>)));
    let mounted = use_hook(|| Rc::new(Cell::new(true)));

    let timeout_id_for_effect = timeout_id.clone();
    let mounted_for_effect = mounted.clone();
    use_effect(move || {
        if visible() || timeout_id_for_effect.get().is_some() {
            return;
        }

        let Some(window) = web_sys::window() else {
            visible.set(true);
            return;
        };

        let mounted = mounted_for_effect.clone();
        let callback = Closure::<dyn FnMut()>::new(move || {
            if mounted.get() {
                visible.set(true);
            }
        });
        let Ok(next_timeout_id) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            LOADING_DELAY_MS,
        ) else {
            visible.set(true);
            return;
        };
        timeout_id_for_effect.set(Some(next_timeout_id));
        callback.forget();
    });

    let timeout_id_for_drop = timeout_id.clone();
    let mounted_for_drop = mounted.clone();
    use_drop(move || {
        mounted_for_drop.set(false);
        if let (Some(window), Some(timeout_id)) = (web_sys::window(), timeout_id_for_drop.get()) {
            window.clear_timeout_with_handle(timeout_id);
        }
    });

    rsx! {
        if visible() {
            section { class: "loading-stage",
                div { class: "loading-card",
                    div { class: "loading-mark",
                        span {}
                        span {}
                        span {}
                    }
                    div {
                        h1 { "{title}" }
                        if let Some(detail) = detail {
                            p { class: "lede", "{detail}" }
                        }
                    }
                    if !feedback.is_empty() {
                        p { class: "feedback", "{feedback}" }
                    }
                }
            }
        } else {
            div { class: "route-loading-hold" }
        }
    }
}
