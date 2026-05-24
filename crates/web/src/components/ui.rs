use dioxus::prelude::*;

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
pub fn UiModal(class: Option<String>, children: Element) -> Element {
    let class = join_class("ui-modal", class);

    rsx! {
        div { class: "ui-modal-backdrop",
            div { class: "{class}", {children} }
        }
    }
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
