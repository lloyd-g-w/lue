use dioxus::prelude::*;
use web_sys::window;

#[derive(Clone, PartialEq)]
pub enum Route {
    Home,
    MicrosoftAuthComplete,
    Admin {
        queue_id: Option<String>,
        request_id: Option<String>,
    },
    Queue {
        queue_id: String,
    },
}

impl Route {
    pub fn current() -> Self {
        let path = window()
            .and_then(|browser| browser.location().pathname().ok())
            .unwrap_or_else(|| "/".to_string());
        Self::from_path(&path)
    }

    pub fn from_path(path: &str) -> Self {
        let parts: Vec<_> = path.trim_matches('/').split('/').collect();
        match parts.as_slice() {
            ["auth", "microsoft", "complete"] => Route::MicrosoftAuthComplete,
            ["admin"] => Route::Admin {
                queue_id: None,
                request_id: None,
            },
            ["admin", "queue", queue_id] if !queue_id.is_empty() => Route::Admin {
                queue_id: Some(queue_id.to_string()),
                request_id: None,
            },
            ["admin", "queue", queue_id, "request", request_id]
                if !queue_id.is_empty() && !request_id.is_empty() =>
            {
                Route::Admin {
                    queue_id: Some(queue_id.to_string()),
                    request_id: Some(request_id.to_string()),
                }
            }
            ["queue", queue_id] if !queue_id.is_empty() => Route::Queue {
                queue_id: queue_id.to_string(),
            },
            _ => Route::Home,
        }
    }

    pub fn path(&self) -> String {
        match self {
            Route::Home => "/".to_string(),
            Route::MicrosoftAuthComplete => "/auth/microsoft/complete".to_string(),
            Route::Admin {
                queue_id: None,
                request_id: None,
            } => "/admin".to_string(),
            Route::Admin {
                queue_id: Some(queue_id),
                request_id: None,
            } => format!("/admin/queue/{queue_id}"),
            Route::Admin {
                queue_id: Some(queue_id),
                request_id: Some(request_id),
            } => format!("/admin/queue/{queue_id}/request/{request_id}"),
            Route::Admin {
                queue_id: None,
                request_id: Some(_),
            } => "/admin".to_string(),
            Route::Queue { queue_id } => format!("/queue/{queue_id}"),
        }
    }
}

pub fn navigate(mut route_signal: Signal<Route>, route: Route) {
    if let Some(browser) = window() {
        let _ = browser.history().and_then(|history| {
            history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&route.path()))
        });
    }
    route_signal.set(route);
}

pub fn replace_route(mut route_signal: Signal<Route>, route: Route) {
    if let Some(browser) = window() {
        let _ = browser.history().and_then(|history| {
            history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&route.path()))
        });
    }
    route_signal.set(route);
}

pub fn frontend_url(route: &Route) -> String {
    let location = window().expect("browser window").location();
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let host = location
        .host()
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    format!("{protocol}//{host}{}", route.path())
}
