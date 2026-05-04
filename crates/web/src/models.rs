use serde::{Deserialize, Serialize};
use shared::AccountRole;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct AdminSessionRecord {
    pub token: String,
    pub name: String,
    pub email: String,
    pub is_super_admin: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSessionRecord {
    pub token: String,
    pub name: String,
    pub email: String,
}

#[derive(Clone, PartialEq)]
pub struct EditableField {
    pub label: String,
}

impl EditableField {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct AccountDraft {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: AccountRole,
}

#[derive(Clone, PartialEq)]
pub struct GroupDraft {
    pub name: String,
    pub role: AccountRole,
    pub member_ids: Vec<uuid::Uuid>,
}

impl Default for AccountDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            email: String::new(),
            password: String::new(),
            role: AccountRole::User,
        }
    }
}

impl Default for GroupDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            role: AccountRole::Admin,
            member_ids: Vec::new(),
        }
    }
}
