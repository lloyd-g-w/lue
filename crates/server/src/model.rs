use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use shared::{
    AccountRole, QueueEntryStatus, QueueField, QueueSummary, SiteSettingsView, WeeklySchedule,
};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<Store>>,
    pub updates: broadcast::Sender<Uuid>,
    pub data_path: PathBuf,
    pub microsoft_auth: Option<Arc<MicrosoftAuthConfig>>,
    pub microsoft_auth_requests: Arc<RwLock<HashMap<String, MicrosoftAuthRequest>>>,
}

#[derive(Default)]
pub struct Store {
    pub site_settings: SiteSettings,
    pub accounts: HashMap<Uuid, Account>,
    pub account_email_index: HashMap<String, Uuid>,
    pub admin_sessions: HashMap<String, AdminSession>,
    pub user_sessions: HashMap<String, UserSession>,
    pub queues: HashMap<Uuid, Queue>,
    pub archived_queues: HashMap<Uuid, ArchivedQueue>,
    pub queue_code_index: HashMap<String, Uuid>,
    pub groups: HashMap<Uuid, Group>,
    pub entry_index: HashMap<Uuid, Uuid>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SiteSettings {
    pub site_title: String,
    #[serde(default = "default_true")]
    pub admin_password_sign_in_enabled: bool,
    #[serde(default = "default_true")]
    pub admin_microsoft_sign_in_enabled: bool,
    #[serde(default = "default_true")]
    pub user_password_sign_in_enabled: bool,
    #[serde(default = "default_true")]
    pub user_microsoft_sign_in_enabled: bool,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Account {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    #[serde(alias = "password")]
    pub password_hash: String,
    pub role: AccountRole,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AdminSession {
    pub token: String,
    pub account_id: Uuid,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct UserSession {
    pub token: String,
    pub account_id: Uuid,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Queue {
    pub id: Uuid,
    #[serde(default)]
    pub code: String,
    pub name: String,
    pub allow_guests: bool,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub opens_at: Option<String>,
    #[serde(default)]
    pub weekly_schedule: Option<WeeklySchedule>,
    pub owner_account_id: Uuid,
    pub owner_name: String,
    #[serde(default)]
    pub shared_account_ids: Vec<Uuid>,
    #[serde(default)]
    pub shared_group_ids: Vec<Uuid>,
    pub fields: Vec<QueueField>,
    pub entries: Vec<QueueEntry>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ArchivedQueue {
    pub queue: Queue,
    pub closed_at: String,
    pub closed_by_account_id: Uuid,
    pub closed_by_name: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    pub role: AccountRole,
    pub member_ids: Vec<Uuid>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct QueueEntry {
    pub id: Uuid,
    pub token: String,
    #[serde(default)]
    pub requester_account_id: Option<Uuid>,
    pub requester_label: String,
    pub requester_email: Option<String>,
    pub is_guest: bool,
    pub values: BTreeMap<String, String>,
    pub submitted_at: String,
    #[serde(default)]
    pub left_at: Option<String>,
    pub status: QueueEntryStatus,
    pub claimed_by: Option<String>,
}

#[derive(Default)]
pub struct AdminSubscription {
    pub admin_token: Option<String>,
    pub selected_queue_id: Option<Uuid>,
}

#[derive(Default)]
pub struct QueueSubscription {
    pub queue_id: Option<Uuid>,
    pub entry_token: Option<String>,
    pub user_token: Option<String>,
}

#[derive(Clone)]
pub struct MicrosoftAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub tenant_id: String,
    pub redirect_uri: String,
    pub frontend_base_url: String,
    pub http_client: reqwest::Client,
}

#[derive(Clone)]
pub struct MicrosoftAuthRequest {
    pub kind: MicrosoftAuthKind,
    pub return_to: String,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MicrosoftAuthKind {
    Admin,
    User,
}

impl Account {
    pub fn is_super_admin(&self) -> bool {
        self.role == AccountRole::SuperAdmin
    }

    pub fn can_administer(&self) -> bool {
        matches!(self.role, AccountRole::SuperAdmin | AccountRole::Admin)
    }

    pub fn can_join_queues(&self) -> bool {
        matches!(
            self.role,
            AccountRole::SuperAdmin | AccountRole::Admin | AccountRole::User
        )
    }
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            site_title: "Lue".to_string(),
            admin_password_sign_in_enabled: true,
            admin_microsoft_sign_in_enabled: true,
            user_password_sign_in_enabled: true,
            user_microsoft_sign_in_enabled: true,
        }
    }
}

impl SiteSettings {
    pub fn view(&self) -> SiteSettingsView {
        SiteSettingsView {
            site_title: self.site_title.clone(),
            admin_password_sign_in_enabled: self.admin_password_sign_in_enabled,
            admin_microsoft_sign_in_enabled: self.admin_microsoft_sign_in_enabled,
            user_password_sign_in_enabled: self.user_password_sign_in_enabled,
            user_microsoft_sign_in_enabled: self.user_microsoft_sign_in_enabled,
        }
    }
}

fn default_true() -> bool {
    true
}

impl Queue {
    pub fn normalize_code(value: &str) -> String {
        value
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .map(|character| character.to_ascii_uppercase())
            .collect()
    }

    pub fn new_code(existing_codes: &HashSet<String>) -> String {
        loop {
            let candidate: String = Uuid::new_v4()
                .simple()
                .to_string()
                .chars()
                .take(6)
                .map(|character| character.to_ascii_uppercase())
                .collect();
            if !existing_codes.contains(&candidate) {
                return candidate;
            }
        }
    }

    pub fn waiting_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| matches!(entry.status, QueueEntryStatus::Pending))
            .count()
    }

    pub fn active_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.status,
                    QueueEntryStatus::Pending | QueueEntryStatus::Claimed
                )
            })
            .count()
    }

    pub fn position_for(&self, entry_id: Uuid) -> Option<usize> {
        let mut position = 0usize;

        for entry in &self.entries {
            if entry.status == QueueEntryStatus::Pending {
                position += 1;
            }

            if entry.id == entry_id {
                return if entry.status == QueueEntryStatus::Pending {
                    Some(position)
                } else {
                    None
                };
            }
        }

        None
    }

    pub fn summary(&self) -> QueueSummary {
        QueueSummary {
            id: self.id,
            code: self.code.clone(),
            name: self.name.clone(),
            allow_guests: self.allow_guests,
            is_public: self.is_public,
            opens_at: self.opens_at.clone(),
            weekly_schedule: self.weekly_schedule.clone(),
            waiting_count: self.waiting_count(),
            active_count: self.active_count(),
        }
    }
}
