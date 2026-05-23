use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct QueueField {
    pub key: String,
    pub label: String,
    pub required: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum QueueEntryStatus {
    Pending,
    Claimed,
    Left,
    Resolved,
    Denied,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct QueueSummary {
    pub id: Uuid,
    pub name: String,
    pub allow_guests: bool,
    pub waiting_count: usize,
    pub active_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UserQueueView {
    pub id: Uuid,
    pub name: String,
    pub fields: Vec<QueueField>,
    pub allow_guests: bool,
    pub waiting_count: usize,
    pub closed_at: Option<String>,
    pub closed_by_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UserEntryView {
    pub id: Uuid,
    pub token: String,
    pub status: QueueEntryStatus,
    pub claimed_by: Option<String>,
    pub values: BTreeMap<String, String>,
    pub submitted_at: String,
    pub position: Option<usize>,
    pub requester_label: String,
    pub is_guest: bool,
}
