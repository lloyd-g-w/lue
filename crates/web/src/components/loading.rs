use dioxus::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

const LOADING_DELAY_MS: i32 = 180;

#[component]
pub fn DelayedLoading(title: String, detail: Option<String>, feedback: String) -> Element {
    let mut visible = use_signal(|| false);

    use_effect(move || {
        if visible() {
            return;
        }

        let Some(window) = web_sys::window() else {
            visible.set(true);
            return;
        };

        let callback = Closure::<dyn FnMut()>::new(move || {
            visible.set(true);
        });
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            LOADING_DELAY_MS,
        );
        callback.forget();
    });

    rsx! {
        if visible() {
            section { class: "empty-stage",
                h1 { "{title}" }
                if let Some(detail) = detail {
                    p { class: "lede", "{detail}" }
                }
                if !feedback.is_empty() {
                    p { class: "feedback", "{feedback}" }
                }
            }
        } else {
            div { class: "route-loading-hold" }
        }
    }
}
