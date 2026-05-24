use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AccountRole, AccountView, QueueEntryStatus, QueueField, QueueSummary};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SiteSettingsView {
    pub site_title: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdminIdentityView {
    pub token: String,
    pub account_id: Uuid,
    pub name: String,
    pub email: String,
    pub is_super_admin: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdminQueueListItem {
    pub summary: QueueSummary,
    pub owner_name: String,
    pub shared_account_ids: Vec<Uuid>,
    pub shared_group_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ArchivedQueueListItem {
    pub summary: QueueSummary,
    pub owner_name: String,
    pub closed_at: String,
    pub closed_by_name: String,
    pub entry_count: usize,
    pub fields: Vec<QueueField>,
    pub entries: Vec<AdminEntryView>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GroupView {
    pub id: Uuid,
    pub name: String,
    pub role: AccountRole,
    pub member_ids: Vec<Uuid>,
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
    pub owner_account_id: Uuid,
    pub shared_account_ids: Vec<Uuid>,
    pub shared_group_ids: Vec<Uuid>,
    pub fields: Vec<QueueField>,
    pub entries: Vec<AdminEntryView>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdminStateView {
    pub admin: AdminIdentityView,
    pub site_settings: SiteSettingsView,
    pub queues: Vec<AdminQueueListItem>,
    pub archived_queues: Vec<ArchivedQueueListItem>,
    pub selected_queue: Option<AdminQueueView>,
    pub accounts: Vec<AccountView>,
    pub groups: Vec<GroupView>,
}
