mod admin;
mod messages;
mod queue;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use admin::{
    AdminEntryView, AdminIdentityView, AdminQueueListItem, AdminQueueView, AdminStateView,
    ArchivedQueueListItem, GroupView,
};
pub use messages::{ClientMessage, ServerMessage};
pub use queue::{QueueEntryStatus, QueueField, QueueSummary, UserEntryView, UserQueueView};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum AccountRole {
    SuperAdmin,
    Admin,
    User,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AccountView {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: AccountRole,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UserIdentityView {
    pub token: String,
    pub name: String,
    pub email: String,
}
