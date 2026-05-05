use dioxus::prelude::{Key, KeyboardEvent};
use shared::QueueEntryStatus;
use wasm_bindgen::JsValue;

pub fn slugify(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .replace("__", "_")
}

pub fn status_label(status: &QueueEntryStatus) -> &'static str {
    match status {
        QueueEntryStatus::Pending => "Pending",
        QueueEntryStatus::Claimed => "Claimed",
        QueueEntryStatus::Left => "Left",
        QueueEntryStatus::Resolved => "Resolved",
        QueueEntryStatus::Denied => "Denied",
    }
}

pub fn status_class(status: &QueueEntryStatus) -> &'static str {
    match status {
        QueueEntryStatus::Pending => "badge badge-pending",
        QueueEntryStatus::Claimed => "badge badge-claimed",
        QueueEntryStatus::Left => "badge badge-left",
        QueueEntryStatus::Resolved => "badge badge-resolved",
        QueueEntryStatus::Denied => "badge badge-denied",
    }
}

pub fn status_class_suffix(status: &QueueEntryStatus) -> &'static str {
    match status {
        QueueEntryStatus::Pending => "pending-bg",
        QueueEntryStatus::Claimed => "claimed-bg",
        QueueEntryStatus::Left => "left-bg",
        QueueEntryStatus::Resolved => "resolved-bg",
        QueueEntryStatus::Denied => "denied-bg",
    }
}

pub fn is_enter_key(event: &KeyboardEvent) -> bool {
    event.key() == Key::Enter
}

pub fn format_timestamp(value: &str) -> String {
    let date = js_sys::Date::new(&JsValue::from_str(value));
    if date.get_time().is_nan() {
        return value.to_string();
    }

    date.to_locale_string("en-AU", &JsValue::UNDEFINED).into()
}
