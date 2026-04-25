use dioxus::prelude::*;
use shared::{AdminQueueView, QueueEntryStatus};

use crate::route::{frontend_url, Route};
use crate::view_helpers::{secondary_field, status_class, status_label};

#[component]
pub fn AdminQueueDetail(
    on_back: EventHandler<()>,
    queue: AdminQueueView,
    selected_entry: Signal<Option<uuid::Uuid>>,
    claim_entry: EventHandler<uuid::Uuid>,
    unclaim_entry: EventHandler<uuid::Uuid>,
    resolve_entry: EventHandler<uuid::Uuid>,
    deny_entry: EventHandler<uuid::Uuid>,
) -> Element {
    let queue_link = frontend_url(&Route::Queue {
        queue_id: queue.summary.id.to_string(),
    });

    rsx! {
        div { class: "queue-workspace",
            section { class: "workspace-header",
                div { class: "detail-header",
                    div {
                        p { class: "kicker", "Queue Detail" }
                        h2 { "{queue.summary.name}" }
                        p { class: "lede",
                            "Owned by {queue.owner_name} • {queue.summary.waiting_count} waiting • {queue.summary.active_count} active"
                            if queue.summary.allow_guests {
                                " • guests allowed"
                            } else {
                                " • account required"
                            }
                        }
                    }
                    div { class: "button-row",
                        a { class: "button button-primary", href: queue_link.clone(), "Open user link" }
                        button {
                            class: "button button-secondary",
                            onclick: move |_| on_back.call(()),
                            "Back to queue list"
                        }
                    }
                }
            }

            div { class: "workspace-columns",
                section { class: "request-list-panel",
                    div { class: "panel-header",
                        h3 { "Requests" }
                        span { class: "counter-chip", "{queue.entries.len()}" }
                    }
                    div { class: "list-shell request-list-shell",
                        for entry in queue.entries.iter().cloned() {
                            button {
                                class: if selected_entry() == Some(entry.id) { "request-row request-row-active" } else { "request-row" },
                                onclick: move |_| selected_entry.set(Some(entry.id)),
                                div { class: "request-row-top",
                                    div {
                                        p { class: "request-name", "{entry.requester_label}" }
                                        p { class: "request-subline", "{secondary_field(queue.fields.as_slice(), &entry)}" }
                                    }
                                    span { class: status_class(&entry.status), "{status_label(&entry.status)}" }
                                }
                                div { class: "request-row-meta",
                                    span { class: "mono small-text", "{entry.submitted_at}" }
                                    if entry.is_guest {
                                        span { class: "row-meta", "Guest" }
                                    } else if let Some(email) = entry.requester_email.clone() {
                                        span { class: "row-meta", "{email}" }
                                    }
                                    if let Some(claimed_by) = entry.claimed_by.clone() {
                                        span { class: "row-meta", "Handled by {claimed_by}" }
                                    }
                                }
                            }
                        }
                        if queue.entries.is_empty() {
                            div { class: "empty-panel",
                                p { "No one has joined this queue yet." }
                            }
                        }
                    }
                }

                section { class: "request-detail-panel",
                    if let Some(selected_id) = selected_entry() {
                        if let Some(entry) = queue.entries.iter().find(|entry| entry.id == selected_id).cloned() {
                            div { class: "panel-header",
                                div {
                                    p { class: "kicker", "Request Detail" }
                                    h3 { "{entry.requester_label}" }
                                }
                                span { class: status_class(&entry.status), "{status_label(&entry.status)}" }
                            }
                            div { class: "request-meta-strip",
                                span { class: "mono small-text", "{entry.submitted_at}" }
                                if entry.is_guest {
                                    span { class: "row-meta", "Guest" }
                                } else if let Some(email) = entry.requester_email.clone() {
                                    span { class: "row-meta", "{email}" }
                                }
                                if let Some(claimed_by) = entry.claimed_by.clone() {
                                    span { class: "row-meta", "Handled by {claimed_by}" }
                                }
                            }
                            div { class: "detail-list",
                                for field in queue.fields.iter().cloned() {
                                    div { class: "detail-row",
                                        span { class: "detail-key", "{field.label}" }
                                        div { class: "detail-value",
                                            "{entry.values.get(&field.key).cloned().unwrap_or_default()}"
                                        }
                                    }
                                }
                            }
                            div { class: "action-bar",
                                if matches!(entry.status, QueueEntryStatus::Pending) {
                                    button {
                                        class: "button button-primary",
                                        onclick: move |_| claim_entry.call(entry.id),
                                        "Claim"
                                    }
                                }
                                if matches!(entry.status, QueueEntryStatus::Claimed) {
                                    button {
                                        class: "button button-secondary",
                                        onclick: move |_| unclaim_entry.call(entry.id),
                                        "Unclaim"
                                    }
                                }
                                if matches!(entry.status, QueueEntryStatus::Pending | QueueEntryStatus::Claimed) {
                                    button {
                                        class: "button success",
                                        onclick: move |_| resolve_entry.call(entry.id),
                                        "Resolve"
                                    }
                                }
                                if matches!(entry.status, QueueEntryStatus::Pending | QueueEntryStatus::Claimed) {
                                    button {
                                        class: "button danger",
                                        onclick: move |_| deny_entry.call(entry.id),
                                        "Deny"
                                    }
                                }
                            }
                            if matches!(entry.status, QueueEntryStatus::Left | QueueEntryStatus::Resolved | QueueEntryStatus::Denied) {
                                p { class: "hint inspector-note", "No further actions are available for this request." }
                            }
                        } else {
                            section { class: "empty-stage",
                                h3 { "Select a request" }
                                p { class: "lede", "Choose an item from the request list to open its details." }
                            }
                        }
                    } else {
                        section { class: "empty-stage",
                            p { class: "kicker", "Request Detail" }
                            h3 { "Open a request from the list." }
                            p { class: "lede", "The inspector shows submitted values and available actions for the selected request." }
                        }
                    }
                }
            }
        }
    }
}
