use dioxus::prelude::{Key, KeyboardEvent};
use shared::{QueueEntryStatus, WeeklySchedule};
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

pub fn is_requester_name_key(key: &str) -> bool {
    matches!(key, "name" | "full_name")
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

pub fn local_datetime_to_rfc3339(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let date = js_sys::Date::new(&JsValue::from_str(value));
    if date.get_time().is_nan() {
        return None;
    }

    Some(String::from(date.to_iso_string()))
}

pub fn rfc3339_to_local_datetime(value: Option<&str>) -> String {
    let Some(value) = value else {
        return String::new();
    };

    let date = js_sys::Date::new(&JsValue::from_str(value));
    if date.get_time().is_nan() {
        return String::new();
    }

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date(),
        date.get_hours(),
        date.get_minutes()
    )
}

pub fn local_weekly_to_utc(weekday: u8, time: &str) -> Option<WeeklySchedule> {
    if weekday > 6 {
        return None;
    }
    let mut parts = time.split(':');
    let hour = parts.next()?.parse::<u8>().ok()?;
    let minute = parts.next()?.parse::<u8>().ok()?;
    if hour > 23 || minute > 59 {
        return None;
    }

    let local_day = 4 + weekday;
    let date = js_sys::Date::new(&JsValue::from_str(&format!(
        "2026-01-{local_day:02}T{hour:02}:{minute:02}:00"
    )));
    if date.get_time().is_nan() {
        return None;
    }

    Some(WeeklySchedule {
        weekday: date.get_utc_day() as u8,
        minute_of_day: (date.get_utc_hours() * 60 + date.get_utc_minutes()) as u16,
    })
}

pub fn utc_weekly_to_local(schedule: Option<&WeeklySchedule>) -> (u8, String) {
    let Some(schedule) = schedule else {
        return (1, "09:00".to_string());
    };
    if schedule.weekday > 6 || schedule.minute_of_day >= 24 * 60 {
        return (1, "09:00".to_string());
    }

    let hour = schedule.minute_of_day / 60;
    let minute = schedule.minute_of_day % 60;
    let utc_day = 4 + schedule.weekday;
    let date = js_sys::Date::new(&JsValue::from_str(&format!(
        "2026-01-{utc_day:02}T{hour:02}:{minute:02}:00Z"
    )));
    if date.get_time().is_nan() {
        return (1, "09:00".to_string());
    }

    (
        date.get_day() as u8,
        format!("{:02}:{:02}", date.get_hours(), date.get_minutes()),
    )
}

pub fn weekly_schedule_label(schedule: &WeeklySchedule) -> String {
    let (weekday, time) = utc_weekly_to_local(Some(schedule));
    format!("Weekly on {} at {time}", weekday_name(weekday))
}

pub fn weekday_name(weekday: u8) -> &'static str {
    match weekday {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}
