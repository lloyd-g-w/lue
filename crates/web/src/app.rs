use dioxus::prelude::*;

use crate::pages::{AdminPage, HomePage, QueuePage};
use crate::route::Route;
use crate::styles::APP_CSS;

#[component]
pub fn App() -> Element {
    let route = use_signal(Route::current);

    rsx! {
        document::Stylesheet { href: "https://fonts.googleapis.com/css2?family=Manrope:wght@400;500;600;700;800&family=IBM+Plex+Mono:wght@400;500&display=swap" }
        style { {APP_CSS} }
        div { class: "shell",
            match route() {
                Route::Home => rsx! { HomePage { route } },
                Route::Admin { queue_id, request_id } => rsx! { AdminPage { route, selected_queue_id: queue_id, selected_request_id: request_id } },
                Route::Queue { queue_id } => rsx! { QueuePage { queue_id } },
            }
        }
    }
}
