use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AccountView, QueueEntryStatus, QueueField, QueueSummary};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdminIdentityView {
    pub token: String,
    pub name: String,
    pub email: String,
    pub is_super_admin: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdminQueueListItem {
    pub summary: QueueSummary,
    pub owner_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdminEntryView {
    pub id: Uuid,
    pub status: QueueEntryStatus,
    pub submitted_at: String,
    pub claimed_by: Option<String>,
    pub requester_label: String,
    pub requester_email: Option<String>,
    pub is_guest: bool,
    pub values: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdminQueueView {
    pub summary: QueueSummary,
    pub owner_name: String,
    pub fields: Vec<QueueField>,
    pub entries: Vec<AdminEntryView>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdminStateView {
    pub admin: AdminIdentityView,
    pub queues: Vec<AdminQueueListItem>,
    pub selected_queue: Option<AdminQueueView>,
    pub accounts: Vec<AccountView>,
}
