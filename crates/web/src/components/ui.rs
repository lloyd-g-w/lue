use dioxus::prelude::*;

const MIN_TABLE_COLUMN_PERCENT: f64 = 6.0;

#[derive(Clone)]
struct UiTableDrag {
    index: usize,
    start_x: f64,
    start_widths: Vec<f64>,
}

#[derive(Clone)]
struct UiTableResizeContext {
    table_id: Option<String>,
    widths: Signal<Vec<f64>>,
    drag: Signal<Option<UiTableDrag>>,
    default_widths: Vec<f64>,
}

fn join_class(base: &str, extra: Option<String>) -> String {
    match extra {
        Some(extra) if !extra.trim().is_empty() => format!("{base} {extra}"),
        _ => base.to_string(),
    }
}

#[component]
pub fn UiPanel(class: Option<String>, children: Element) -> Element {
    let class = join_class("ui-panel", class);

    rsx! {
        section { class: "{class}", {children} }
    }
}

#[component]
pub fn UiHeader(
    kicker: String,
    title: String,
    lede: Option<String>,
    class: Option<String>,
    children: Element,
) -> Element {
    let class = join_class("ui-header", class);

    rsx! {
        div { class: "{class}",
            div { class: "ui-header-copy",
                p { class: "ui-kicker", "{kicker}" }
                h2 { "{title}" }
                if let Some(lede) = lede {
                    p { class: "ui-lede", "{lede}" }
                }
            }
            {children}
        }
    }
}

#[component]
pub fn UiButton(
    label: String,
    variant: Option<String>,
    class: Option<String>,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let variant = variant.unwrap_or_else(|| "secondary".to_string());
    let class = join_class(&format!("ui-button ui-button-{variant}"), class);

    rsx! {
        button {
            class: "{class}",
            onclick: move |event| onclick.call(event),
            "{label}"
        }
    }
}

#[component]
pub fn UiSwitch(label: String, checked: bool, onchange: EventHandler<bool>) -> Element {
    rsx! {
        label { class: "ui-switch",
            input {
                r#type: "checkbox",
                checked,
                oninput: move |event| onchange.call(event.checked())
            }
            span { class: "ui-switch-box" }
            span { class: "ui-switch-label", "{label}" }
        }
    }
}

#[component]
pub fn UiEmpty(message: String) -> Element {
    rsx! {
        div { class: "ui-empty",
            p { class: "hint", "{message}" }
        }
    }
}

#[component]
pub fn UiTable(
    id: Option<String>,
    class: Option<String>,
    columns: Option<Vec<String>>,
    children: Element,
) -> Element {
    let table_class = join_class("data-table ui-table", class);
    let configured_widths = columns
        .unwrap_or_default()
        .iter()
        .filter_map(|width| percent_width_from_config(width))
        .collect::<Vec<_>>();
    let default_widths = normalize_widths(configured_widths);
    let table_id = id.clone();
    let mut widths = use_signal({
        let table_id = table_id.clone();
        let default_widths = default_widths.clone();
        move || {
            table_id
                .as_deref()
                .and_then(|id| load_table_widths(id, default_widths.len()))
                .unwrap_or_else(|| default_widths.clone())
        }
    });
    let mut drag = use_signal(|| None::<UiTableDrag>);

    use_context_provider(|| UiTableResizeContext {
        table_id: table_id.clone(),
        widths,
        drag,
        default_widths: default_widths.clone(),
    });

    let shell_class = if drag().is_some() {
        "table-shell ui-table-shell ui-table-resizing"
    } else {
        "table-shell ui-table-shell"
    };
    let shell_dom_id = table_id.as_deref().map(table_dom_id).unwrap_or_default();
    let table_id_for_overlay_move = table_id.clone();
    let table_id_for_overlay_up = table_id.clone();

    rsx! {
        div {
            class: "{shell_class}",
            id: "{shell_dom_id}",
            table {
                class: "{table_class}",
                if !widths().is_empty() {
                    colgroup {
                        for width in widths() {
                            col { style: "width: {width:.3}%;" }
                        }
                    }
                }
                {children}
            }
            if drag().is_some() {
                div {
                    class: "ui-table-resize-capture",
                    onmousemove: move |event| {
                        let Some(active_drag) = drag() else {
                            return;
                        };
                        let table_width = table_container_width(table_id_for_overlay_move.as_deref());
                        widths.set(resized_widths(
                            &active_drag,
                            event.client_coordinates().x,
                            table_width,
                        ));
                    },
                    onmouseup: move |_| {
                        if let Some(table_id) = table_id_for_overlay_up.as_deref() {
                            save_table_widths(table_id, &widths());
                        }
                        drag.set(None);
                    }
                }
            }
        }
    }
}

#[component]
pub fn UiTh(index: usize, children: Element) -> Element {
    let context = try_consume_context::<UiTableResizeContext>();
    let can_resize = context
        .as_ref()
        .is_some_and(|context| index + 1 < (context.widths)().len());
    let context_for_mouse = context.clone();
    let context_for_double = context.clone();

    rsx! {
        th { class: "ui-table-heading",
            span { class: "ui-table-heading-content", {children} }
            if can_resize {
                button {
                    class: "ui-table-resize-handle",
                    aria_label: "Resize column",
                    onmousedown: move |event| {
                        event.prevent_default();
                        if let Some(mut context) = context_for_mouse.clone() {
                            context.drag.set(Some(UiTableDrag {
                                index,
                                start_x: event.client_coordinates().x,
                                start_widths: (context.widths)(),
                            }));
                        }
                    },
                    ondoubleclick: move |_| {
                        if let Some(mut context) = context_for_double.clone() {
                            context.widths.set(context.default_widths.clone());
                            if let Some(table_id) = context.table_id.as_deref() {
                                if let Some(storage) = web_sys::window()
                                    .and_then(|window| window.local_storage().ok().flatten())
                                {
                                    let _ = storage.remove_item(&table_storage_key(table_id));
                                }
                            }
                        }
                    },
                    span { class: "ui-table-resize-grip" }
                }
            }
        }
    }
}

#[component]
pub fn UiModal(class: Option<String>, children: Element) -> Element {
    let class = join_class("ui-modal", class);

    rsx! {
        div { class: "ui-modal-backdrop",
            div { class: "{class}", {children} }
        }
    }
}

fn percent_width_from_config(width: &str) -> Option<f64> {
    width
        .trim()
        .trim_end_matches('%')
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn normalize_widths(widths: Vec<f64>) -> Vec<f64> {
    if widths.is_empty() {
        return widths;
    }
    let total = widths.iter().sum::<f64>();
    if total <= 0.0 {
        let width = 100.0 / widths.len() as f64;
        return vec![width; widths.len()];
    }
    widths
        .into_iter()
        .map(|width| (width / total) * 100.0)
        .collect()
}

fn resized_widths(active_drag: &UiTableDrag, client_x: f64, table_width: f64) -> Vec<f64> {
    let mut next = active_drag.start_widths.clone();
    if active_drag.index + 1 >= next.len() {
        return next;
    }

    let delta_percent = ((client_x - active_drag.start_x) / table_width.max(1.0)) * 100.0;
    let left_start = active_drag.start_widths[active_drag.index];
    let right_start = active_drag.start_widths[active_drag.index + 1];
    let min_delta = MIN_TABLE_COLUMN_PERCENT - left_start;
    let max_delta = right_start - MIN_TABLE_COLUMN_PERCENT;
    let clamped_delta = delta_percent.clamp(min_delta, max_delta);

    next[active_drag.index] = left_start + clamped_delta;
    next[active_drag.index + 1] = right_start - clamped_delta;
    next
}

fn table_storage_key(table_id: &str) -> String {
    format!("lue.table-widths.v3.{table_id}")
}

fn table_dom_id(table_id: &str) -> String {
    let sanitized = table_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("lue-table-{sanitized}")
}

fn load_table_widths(table_id: &str, expected_len: usize) -> Option<Vec<f64>> {
    if expected_len == 0 {
        return None;
    }
    let value = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| {
            storage
                .get_item(&table_storage_key(table_id))
                .ok()
                .flatten()
        })?;
    let widths = value
        .split(',')
        .filter_map(|part| part.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= MIN_TABLE_COLUMN_PERCENT)
        .collect::<Vec<_>>();
    if widths.len() == expected_len {
        Some(normalize_widths(widths))
    } else {
        None
    }
}

fn save_table_widths(table_id: &str, widths: &[f64]) {
    let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    else {
        return;
    };
    let value = widths
        .iter()
        .map(|width| format!("{width:.3}"))
        .collect::<Vec<_>>()
        .join(",");
    let _ = storage.set_item(&table_storage_key(table_id), &value);
}

fn table_container_width(table_id: Option<&str>) -> f64 {
    let Some(table_id) = table_id else {
        return fallback_table_width();
    };
    let dom_id = table_dom_id(table_id);
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(&dom_id))
        .map(|element| element.client_width() as f64)
        .filter(|width| *width > 0.0)
        .unwrap_or_else(fallback_table_width)
}

fn fallback_table_width() -> f64 {
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|value| value.as_f64())
        .filter(|width| *width > 0.0)
        .unwrap_or(1200.0)
}

#[component]
pub fn UiScheduleOption(
    label: String,
    value: String,
    selected: bool,
    onchange: EventHandler<String>,
) -> Element {
    let class = if selected {
        "ui-option ui-option-active"
    } else {
        "ui-option"
    };

    rsx! {
        label { class,
            input {
                r#type: "radio",
                name: "schedule-mode",
                checked: selected,
                oninput: move |_| onchange.call(value.clone())
            }
            span { "{label}" }
        }
    }
}
