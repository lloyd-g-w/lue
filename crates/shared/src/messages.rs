use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AccountRole, AdminIdentityView, AdminStateView, QueueField, UserEntryView, UserIdentityView,
    UserQueueView,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ClientMessage {
    LoginAdmin {
        email: String,
        password: String,
    },
    LoginUser {
        email: String,
        password: String,
    },
    SubscribeAdmin {
        admin_token: String,
        selected_queue_id: Option<Uuid>,
    },
    CreateQueue {
        admin_token: String,
        name: String,
        fields: Vec<QueueField>,
        allow_guests: bool,
    },
    CreateAccount {
        admin_token: String,
        name: String,
        email: String,
        password: String,
        role: AccountRole,
    },
    ClaimEntry {
        admin_token: String,
        entry_id: Uuid,
    },
    UnclaimEntry {
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
        user_token: Option<String>,
    },
    JoinQueue {
        queue_id: Uuid,
        values: BTreeMap<String, String>,
        user_token: Option<String>,
    },
    LeaveQueue {
        queue_id: Uuid,
        entry_token: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ServerMessage {
    AdminLoggedIn {
        admin: AdminIdentityView,
    },
    UserLoggedIn {
        user: UserIdentityView,
    },
    QueueCreated {
        queue_id: Uuid,
    },
    AccountCreated,
    AdminState {
        state: AdminStateView,
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
