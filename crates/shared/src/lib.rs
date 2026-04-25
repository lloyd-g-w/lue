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
    Resolved,
    Denied,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct QueueSummary {
    pub id: Uuid,
    pub name: String,
    pub waiting_count: usize,
    pub active_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdminEntryView {
    pub id: Uuid,
    pub status: QueueEntryStatus,
    pub submitted_at: String,
    pub values: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdminQueueView {
    pub summary: QueueSummary,
    pub fields: Vec<QueueField>,
    pub entries: Vec<AdminEntryView>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UserQueueView {
    pub id: Uuid,
    pub name: String,
    pub fields: Vec<QueueField>,
    pub waiting_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UserEntryView {
    pub id: Uuid,
    pub token: String,
    pub status: QueueEntryStatus,
    pub values: BTreeMap<String, String>,
    pub submitted_at: String,
    pub position: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ClientMessage {
    SubscribeAdmin {
        admin_token: String,
    },
    CreateQueue {
        name: String,
        fields: Vec<QueueField>,
    },
    ClaimEntry {
        admin_token: String,
        entry_id: Uuid,
    },
    ResolveEntry {
        admin_token: String,
        entry_id: Uuid,
    },
    DenyEntry {
        admin_token: String,
        entry_id: Uuid,
    },
    SubscribeQueue {
        queue_id: Uuid,
        entry_token: Option<String>,
    },
    JoinQueue {
        queue_id: Uuid,
        values: BTreeMap<String, String>,
    },
    LeaveQueue {
        queue_id: Uuid,
        entry_token: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ServerMessage {
    QueueCreated {
        queue_id: Uuid,
        admin_token: String,
        queue_name: String,
    },
    AdminState {
        queue: AdminQueueView,
    },
    QueueState {
        queue: UserQueueView,
        your_entry: Option<UserEntryView>,
    },
    Info {
        message: String,
    },
    Error {
        message: String,
    },
}
