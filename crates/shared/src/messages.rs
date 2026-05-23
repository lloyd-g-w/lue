use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AccountRole, AdminIdentityView, AdminStateView, QueueField, UserEntryView, UserIdentityView,
    UserQueueView,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ClientMessage {
    CheckSetup,
    SetupSuperAdmin {
        name: String,
        email: String,
        password: String,
    },
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
    UpdateAccount {
        admin_token: String,
        account_id: Uuid,
        name: String,
        email: String,
        password: Option<String>,
        role: AccountRole,
    },
    DeleteAccount {
        admin_token: String,
        account_id: Uuid,
    },
    CreateGroup {
        admin_token: String,
        name: String,
        role: AccountRole,
        member_ids: Vec<Uuid>,
    },
    UpdateGroup {
        admin_token: String,
        group_id: Uuid,
        name: String,
        role: AccountRole,
        member_ids: Vec<Uuid>,
    },
    DeleteGroup {
        admin_token: String,
        group_id: Uuid,
    },
    ShareQueue {
        admin_token: String,
        queue_id: Uuid,
        account_ids: Vec<Uuid>,
        group_ids: Vec<Uuid>,
    },
    CloseQueue {
        admin_token: String,
        queue_id: Uuid,
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
    SetupState {
        needs_setup: bool,
    },
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
    AccountUpdated,
    AccountDeleted,
    GroupCreated,
    GroupUpdated,
    GroupDeleted,
    QueueSharingUpdated,
    QueueClosed,
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
