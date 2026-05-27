use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Datelike, Timelike, Utc};
use shared::{
    AccountRole, AccountView, AdminEntryView, AdminIdentityView, AdminQueueListItem,
    AdminQueueView, AdminStateView, ArchivedQueueListItem, GroupView, QueueEntryStatus, QueueField,
    QueueSummary, SiteSettingsView, UserEntryView, UserIdentityView, UserQueueView, WeeklySchedule,
};
use uuid::Uuid;

use crate::model::{
    Account, AdminSession, ArchivedQueue, Group, Queue, QueueEntry, Store, UserSession,
};
use crate::password::{hash_password, verify_password};
use crate::utils::{is_requester_name_key, normalize_email, normalize_fields};

const REJOIN_COOLDOWN_SECS: i64 = 5;

fn weekday_label(weekday: u8) -> &'static str {
    match weekday {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}

fn microsoft_account_name(name: Option<String>, email: &str) -> String {
    let name = name.unwrap_or_default().trim().to_string();
    if !name.is_empty() {
        return name;
    }

    email
        .split_once('@')
        .map(|(local_part, _)| local_part)
        .unwrap_or(email)
        .trim()
        .to_string()
}

impl Store {
    pub fn needs_initial_setup(&self) -> bool {
        !self
            .accounts
            .values()
            .any(|account| account.role == AccountRole::SuperAdmin)
    }

    pub fn setup_initial_super_admin(
        &mut self,
        name: String,
        email: String,
        password: String,
    ) -> Result<AdminIdentityView, String> {
        if !self.needs_initial_setup() {
            return Err("initial setup is already complete".to_string());
        }

        self.bootstrap_super_admin(name, email.clone(), password.clone())?;
        self.login_admin(email, password)
    }

    pub fn bootstrap_super_admin(
        &mut self,
        name: String,
        email: String,
        password: String,
    ) -> Result<(), String> {
        let email = normalize_email(&email)?;
        let name = name.trim().to_string();
        let password = password.trim().to_string();

        if name.is_empty() {
            return Err("super admin name is required".to_string());
        }
        if password.is_empty() {
            return Err("super admin password is required".to_string());
        }

        let account_id = if let Some(existing_id) = self.account_email_index.get(&email).copied() {
            existing_id
        } else {
            let id = Uuid::new_v4();
            self.account_email_index.insert(email.clone(), id);
            id
        };

        self.accounts.insert(
            account_id,
            Account {
                id: account_id,
                name,
                email,
                password_hash: hash_password(&password)?,
                role: AccountRole::SuperAdmin,
            },
        );

        Ok(())
    }

    pub fn login_admin(
        &mut self,
        email: String,
        password: String,
    ) -> Result<AdminIdentityView, String> {
        if !self.site_settings.admin_password_sign_in_enabled {
            return Err("admin password sign-in is disabled".to_string());
        }
        let account = self.authenticate_account(email, password)?;
        if !account.can_administer() {
            return Err("this account does not have admin access".to_string());
        }

        self.create_admin_session(account.id)
    }

    pub fn login_user(
        &mut self,
        email: String,
        password: String,
    ) -> Result<UserIdentityView, String> {
        if !self.site_settings.user_password_sign_in_enabled {
            return Err("user password sign-in is disabled".to_string());
        }
        let account = self.authenticate_account(email, password)?;
        if !account.can_join_queues() {
            return Err("use a user account to join queues".to_string());
        }

        self.create_user_session(account.id)
    }

    pub fn login_admin_with_email(&mut self, email: String) -> Result<AdminIdentityView, String> {
        if !self.site_settings.admin_microsoft_sign_in_enabled {
            return Err("admin Microsoft sign-in is disabled".to_string());
        }
        let account = self.account_by_email(email)?;
        if !account.can_administer() {
            return Err("this account does not have admin access".to_string());
        }

        self.create_admin_session(account.id)
    }

    pub fn login_or_create_user_with_microsoft(
        &mut self,
        email: String,
        name: Option<String>,
    ) -> Result<UserIdentityView, String> {
        if !self.site_settings.user_microsoft_sign_in_enabled {
            return Err("user Microsoft sign-in is disabled".to_string());
        }
        let email = normalize_email(&email)?;
        if let Some(account_id) = self.account_email_index.get(&email).copied() {
            let account = self
                .accounts
                .get(&account_id)
                .ok_or_else(|| "no local account matches this Microsoft email".to_string())?;
            if !account.can_join_queues() {
                return Err("use a user account to join queues".to_string());
            }

            return self.create_user_session(account.id);
        }

        let id = Uuid::new_v4();
        let name = microsoft_account_name(name, &email);
        let password_hash = hash_password(&Uuid::new_v4().to_string())?;
        self.account_email_index.insert(email.clone(), id);
        self.accounts.insert(
            id,
            Account {
                id,
                name,
                email,
                password_hash,
                role: AccountRole::User,
            },
        );

        self.create_user_session(id)
    }

    pub fn create_account(
        &mut self,
        admin_token: &str,
        name: String,
        email: String,
        password: String,
        role: AccountRole,
    ) -> Result<(), String> {
        let admin = self
            .admin_account(admin_token)
            .ok_or_else(|| "unknown admin session".to_string())?;
        if !admin.is_super_admin() {
            return Err("only the super admin can create accounts".to_string());
        }
        let normalized_name = name.trim().to_string();
        let normalized_email = normalize_email(&email)?;
        let normalized_password = password.trim().to_string();

        if normalized_name.is_empty() {
            return Err("account name is required".to_string());
        }
        if normalized_password.len() < 4 {
            return Err("password must be at least 4 characters".to_string());
        }
        if self.account_email_index.contains_key(&normalized_email) {
            return Err("an account with that email already exists".to_string());
        }

        let id = Uuid::new_v4();
        self.account_email_index
            .insert(normalized_email.clone(), id);
        self.accounts.insert(
            id,
            Account {
                id,
                name: normalized_name,
                email: normalized_email,
                password_hash: hash_password(&normalized_password)?,
                role,
            },
        );

        Ok(())
    }

    pub fn update_account(
        &mut self,
        admin_token: &str,
        account_id: Uuid,
        name: String,
        email: String,
        password: Option<String>,
        role: AccountRole,
    ) -> Result<(), String> {
        let admin = self
            .admin_account(admin_token)
            .ok_or_else(|| "unknown admin session".to_string())?;
        if !admin.is_super_admin() {
            return Err("only the super admin can edit accounts".to_string());
        }
        let editing_self = admin.id == account_id;
        if editing_self && role != AccountRole::SuperAdmin {
            return Err("you cannot demote your own super admin account".to_string());
        }

        let normalized_name = name.trim().to_string();
        let normalized_email = normalize_email(&email)?;
        if normalized_name.is_empty() {
            return Err("account name is required".to_string());
        }
        if let Some(existing_id) = self.account_email_index.get(&normalized_email) {
            if *existing_id != account_id {
                return Err("an account with that email already exists".to_string());
            }
        }

        let account = self
            .accounts
            .get_mut(&account_id)
            .ok_or_else(|| "account not found".to_string())?;
        self.account_email_index.remove(&account.email);
        account.name = normalized_name;
        account.email = normalized_email.clone();
        account.role = role;
        if let Some(password) = password {
            let password = password.trim().to_string();
            if !password.is_empty() {
                if password.len() < 4 {
                    return Err("password must be at least 4 characters".to_string());
                }
                account.password_hash = hash_password(&password)?;
            }
        }
        self.account_email_index
            .insert(normalized_email, account_id);
        self.cleanup_account_references();
        Ok(())
    }

    pub fn delete_account(&mut self, admin_token: &str, account_id: Uuid) -> Result<(), String> {
        let admin = self
            .admin_account(admin_token)
            .ok_or_else(|| "unknown admin session".to_string())?;
        if !admin.is_super_admin() {
            return Err("only the super admin can delete accounts".to_string());
        }
        if admin.id == account_id {
            return Err("you cannot delete your own admin account".to_string());
        }
        if self
            .queues
            .values()
            .any(|queue| queue.owner_account_id == account_id)
        {
            return Err("close or reassign this account's queues before deleting it".to_string());
        }
        let account = self
            .accounts
            .remove(&account_id)
            .ok_or_else(|| "account not found".to_string())?;
        self.account_email_index.remove(&account.email);
        self.cleanup_account_references();
        Ok(())
    }

    pub fn create_queue(
        &mut self,
        admin_token: &str,
        name: String,
        fields: Vec<QueueField>,
        allow_guests: bool,
        is_public: bool,
        opens_at: Option<String>,
        weekly_schedule: Option<WeeklySchedule>,
    ) -> Result<Uuid, String> {
        let admin = self
            .admin_account(admin_token)
            .ok_or_else(|| "unknown admin session".to_string())?;

        let normalized_name = name.trim().to_string();
        if normalized_name.is_empty() {
            return Err("queue name is required".to_string());
        }

        let fields = normalize_fields(fields)?;
        let opens_at = Self::normalize_opens_at(opens_at)?;
        let weekly_schedule = Self::normalize_weekly_schedule(weekly_schedule)?;

        let id = Uuid::new_v4();
        let existing_codes: HashSet<String> = self
            .queues
            .values()
            .map(|queue| queue.code.clone())
            .chain(
                self.archived_queues
                    .values()
                    .map(|archive| archive.queue.code.clone()),
            )
            .collect();
        let code = Queue::new_code(&existing_codes);
        self.queues.insert(
            id,
            Queue {
                id,
                code: code.clone(),
                name: normalized_name,
                allow_guests,
                is_public,
                opens_at,
                weekly_schedule,
                owner_account_id: admin.id,
                owner_name: admin.name.clone(),
                shared_account_ids: Vec::new(),
                shared_group_ids: Vec::new(),
                fields,
                entries: Vec::new(),
            },
        );
        self.queue_code_index.insert(code, id);
        Ok(id)
    }

    pub fn update_queue_settings(
        &mut self,
        admin_token: &str,
        queue_id: Uuid,
        fields: Vec<QueueField>,
        allow_guests: bool,
        is_public: bool,
        opens_at: Option<String>,
        weekly_schedule: Option<WeeklySchedule>,
    ) -> Result<(), String> {
        let (admin_id, is_super_admin) = self
            .admin_account(admin_token)
            .map(|admin| (admin.id, admin.is_super_admin()))
            .ok_or_else(|| "unknown admin session".to_string())?;
        let fields = normalize_fields(fields)?;
        let opens_at = Self::normalize_opens_at(opens_at)?;
        let weekly_schedule = Self::normalize_weekly_schedule(weekly_schedule)?;
        let queue = self
            .queues
            .get_mut(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;
        if !(is_super_admin || queue.owner_account_id == admin_id) {
            return Err("only the queue owner or super admin can edit this queue".to_string());
        }

        queue.fields = fields;
        queue.allow_guests = allow_guests;
        queue.is_public = is_public;
        queue.opens_at = opens_at;
        queue.weekly_schedule = weekly_schedule;
        Ok(())
    }

    pub fn create_group(
        &mut self,
        admin_token: &str,
        name: String,
        role: AccountRole,
        member_ids: Vec<Uuid>,
    ) -> Result<(), String> {
        let admin = self
            .admin_account(admin_token)
            .ok_or_else(|| "unknown admin session".to_string())?;
        if !admin.is_super_admin() {
            return Err("only the super admin can create groups".to_string());
        }
        if role == AccountRole::SuperAdmin {
            return Err("groups can only be created for admins or users".to_string());
        }

        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("group name is required".to_string());
        }

        let member_ids = self.validated_group_members(role.clone(), member_ids)?;
        let id = Uuid::new_v4();
        self.groups.insert(
            id,
            Group {
                id,
                name,
                role,
                member_ids,
            },
        );
        Ok(())
    }

    pub fn update_group(
        &mut self,
        admin_token: &str,
        group_id: Uuid,
        name: String,
        role: AccountRole,
        member_ids: Vec<Uuid>,
    ) -> Result<(), String> {
        let admin = self
            .admin_account(admin_token)
            .ok_or_else(|| "unknown admin session".to_string())?;
        if !admin.is_super_admin() {
            return Err("only the super admin can edit groups".to_string());
        }
        if role == AccountRole::SuperAdmin {
            return Err("groups can only be created for admins or users".to_string());
        }
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("group name is required".to_string());
        }
        let member_ids = self.validated_group_members(role.clone(), member_ids)?;
        let group = self
            .groups
            .get_mut(&group_id)
            .ok_or_else(|| "group not found".to_string())?;
        group.name = name;
        group.role = role;
        group.member_ids = member_ids;
        self.cleanup_group_references();
        Ok(())
    }

    pub fn delete_group(&mut self, admin_token: &str, group_id: Uuid) -> Result<(), String> {
        let admin = self
            .admin_account(admin_token)
            .ok_or_else(|| "unknown admin session".to_string())?;
        if !admin.is_super_admin() {
            return Err("only the super admin can delete groups".to_string());
        }
        self.groups
            .remove(&group_id)
            .ok_or_else(|| "group not found".to_string())?;
        self.cleanup_group_references();
        Ok(())
    }

    pub fn update_site_settings(
        &mut self,
        admin_token: &str,
        site_title: String,
        admin_password_sign_in_enabled: bool,
        admin_microsoft_sign_in_enabled: bool,
        user_password_sign_in_enabled: bool,
        user_microsoft_sign_in_enabled: bool,
    ) -> Result<(), String> {
        let admin = self
            .admin_account(admin_token)
            .ok_or_else(|| "unknown admin session".to_string())?;
        if !admin.is_super_admin() {
            return Err("only the super admin can edit site settings".to_string());
        }

        let site_title = site_title.trim().to_string();
        if site_title.is_empty() {
            return Err("site title is required".to_string());
        }
        if site_title.chars().count() > 80 {
            return Err("site title must be 80 characters or fewer".to_string());
        }

        self.site_settings.site_title = site_title;
        self.site_settings.admin_password_sign_in_enabled = admin_password_sign_in_enabled;
        self.site_settings.admin_microsoft_sign_in_enabled = admin_microsoft_sign_in_enabled;
        self.site_settings.user_password_sign_in_enabled = user_password_sign_in_enabled;
        self.site_settings.user_microsoft_sign_in_enabled = user_microsoft_sign_in_enabled;
        Ok(())
    }

    pub fn share_queue(
        &mut self,
        admin_token: &str,
        queue_id: Uuid,
        account_ids: Vec<Uuid>,
        group_ids: Vec<Uuid>,
    ) -> Result<(), String> {
        let admin = self
            .admin_account(admin_token)
            .ok_or_else(|| "unknown admin session".to_string())?;
        if !admin.is_super_admin() {
            let queue = self
                .queues
                .get(&queue_id)
                .ok_or_else(|| "queue not found".to_string())?;
            if queue.owner_account_id != admin.id {
                return Err("only the queue owner or super admin can share this queue".to_string());
            }
        }

        let shared_account_ids = self.validated_share_accounts(account_ids)?;
        let shared_group_ids = self.validated_share_groups(group_ids)?;
        let queue = self
            .queues
            .get_mut(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;
        queue.shared_account_ids = shared_account_ids;
        queue.shared_group_ids = shared_group_ids;
        Ok(())
    }

    pub fn close_queue(&mut self, admin_token: &str, queue_id: Uuid) -> Result<(), String> {
        let (admin_id, admin_name, is_super_admin) = self
            .admin_account(admin_token)
            .map(|admin| (admin.id, admin.name.clone(), admin.is_super_admin()))
            .ok_or_else(|| "unknown admin session".to_string())?;
        let queue = self
            .queues
            .get(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;
        if !(is_super_admin || queue.owner_account_id == admin_id) {
            return Err("only the queue owner or super admin can close this queue".to_string());
        }

        let queue = self
            .queues
            .remove(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;
        self.queue_code_index
            .remove(&Queue::normalize_code(&queue.code));
        for entry in &queue.entries {
            self.entry_index.remove(&entry.id);
        }
        self.archived_queues.insert(
            queue_id,
            ArchivedQueue {
                queue,
                closed_at: Utc::now().to_rfc3339(),
                closed_by_account_id: admin_id,
                closed_by_name: admin_name,
            },
        );
        Ok(())
    }

    pub fn admin_state(
        &self,
        admin_token: &str,
        selected_queue_id: Option<Uuid>,
    ) -> Option<AdminStateView> {
        let admin = self.admin_identity(admin_token)?;
        let visible_queue_ids = self.visible_queue_ids(admin_token)?;
        let queues: Vec<AdminQueueListItem> = visible_queue_ids
            .iter()
            .filter_map(|queue_id| self.queues.get(queue_id))
            .map(|queue| AdminQueueListItem {
                summary: queue.summary(),
                owner_name: queue.owner_name.clone(),
                shared_account_ids: queue.shared_account_ids.clone(),
                shared_group_ids: queue.shared_group_ids.clone(),
            })
            .collect();

        let fallback_queue_id = queues.first().map(|queue| queue.summary.id);
        let selected_queue = selected_queue_id
            .or(fallback_queue_id)
            .and_then(|queue_id| self.admin_queue_view(admin_token, queue_id));
        let mut archived_queues: Vec<ArchivedQueueListItem> = self
            .archived_queues
            .values()
            .filter(|archive| self.account_can_manage_queue(admin.account_id, &archive.queue))
            .map(|archive| ArchivedQueueListItem {
                summary: archive.queue.summary(),
                owner_name: archive.queue.owner_name.clone(),
                closed_at: archive.closed_at.clone(),
                closed_by_name: archive.closed_by_name.clone(),
                entry_count: archive.queue.entries.len(),
                fields: archive.queue.fields.clone(),
                entries: archive
                    .queue
                    .entries
                    .iter()
                    .map(|entry| AdminEntryView {
                        id: entry.id,
                        status: entry.status.clone(),
                        submitted_at: entry.submitted_at.clone(),
                        claimed_by: entry.claimed_by.clone(),
                        requester_label: entry.requester_label.clone(),
                        requester_email: entry.requester_email.clone(),
                        is_guest: entry.is_guest,
                        values: entry.values.clone(),
                    })
                    .collect(),
            })
            .collect();
        archived_queues.sort_by(|left, right| right.closed_at.cmp(&left.closed_at));

        let accounts = if admin.is_super_admin {
            let mut accounts: Vec<AccountView> = self
                .accounts
                .values()
                .map(|account| AccountView {
                    id: account.id,
                    name: account.name.clone(),
                    email: account.email.clone(),
                    role: account.role.clone(),
                })
                .collect();
            accounts.sort_by(|left, right| left.email.cmp(&right.email));
            accounts
        } else {
            Vec::new()
        };

        let groups = if admin.is_super_admin {
            let mut groups: Vec<GroupView> = self
                .groups
                .values()
                .map(|group| GroupView {
                    id: group.id,
                    name: group.name.clone(),
                    role: group.role.clone(),
                    member_ids: group.member_ids.clone(),
                })
                .collect();
            groups.sort_by(|left, right| left.name.cmp(&right.name));
            groups
        } else {
            Vec::new()
        };

        Some(AdminStateView {
            admin,
            site_settings: self.site_settings_view(),
            queues,
            archived_queues,
            selected_queue,
            accounts,
            groups,
        })
    }

    pub fn admin_queue_view(&self, admin_token: &str, queue_id: Uuid) -> Option<AdminQueueView> {
        if !self.admin_can_see_queue(admin_token, queue_id) {
            return None;
        }

        let queue = self.queues.get(&queue_id)?;
        Some(AdminQueueView {
            summary: queue.summary(),
            owner_name: queue.owner_name.clone(),
            owner_account_id: queue.owner_account_id,
            shared_account_ids: queue.shared_account_ids.clone(),
            shared_group_ids: queue.shared_group_ids.clone(),
            fields: queue.fields.clone(),
            entries: queue
                .entries
                .iter()
                .map(|entry| AdminEntryView {
                    id: entry.id,
                    status: entry.status.clone(),
                    submitted_at: entry.submitted_at.clone(),
                    claimed_by: entry.claimed_by.clone(),
                    requester_label: entry.requester_label.clone(),
                    requester_email: entry.requester_email.clone(),
                    is_guest: entry.is_guest,
                    values: entry.values.clone(),
                })
                .collect(),
        })
    }

    pub fn user_view(
        &self,
        queue_id: Uuid,
        entry_token: Option<&str>,
    ) -> Option<(UserQueueView, Option<UserEntryView>)> {
        if let Some(queue) = self.queues.get(&queue_id) {
            if !Self::queue_is_open(queue) {
                return None;
            }
            return Some(self.user_queue_view(queue, entry_token, None));
        }

        let archive = self.archived_queues.get(&queue_id)?;
        Some(self.user_queue_view(
            &archive.queue,
            entry_token,
            Some((archive.closed_at.clone(), archive.closed_by_name.clone())),
        ))
    }

    fn user_queue_view(
        &self,
        queue: &Queue,
        entry_token: Option<&str>,
        closed: Option<(String, String)>,
    ) -> (UserQueueView, Option<UserEntryView>) {
        let your_entry = entry_token.and_then(|token| {
            queue
                .entries
                .iter()
                .find(|entry| entry.token == token)
                .map(|entry| UserEntryView {
                    id: entry.id,
                    token: entry.token.clone(),
                    status: entry.status.clone(),
                    claimed_by: entry.claimed_by.clone(),
                    values: entry.values.clone(),
                    submitted_at: entry.submitted_at.clone(),
                    left_at: entry.left_at.clone(),
                    rejoin_after: Self::rejoin_after_timestamp(entry.left_at.as_deref()),
                    position: queue.position_for(entry.id),
                    requester_label: entry.requester_label.clone(),
                    is_guest: entry.is_guest,
                })
        });

        let (closed_at, closed_by_name) = closed
            .map(|(closed_at, closed_by_name)| (Some(closed_at), Some(closed_by_name)))
            .unwrap_or((None, None));

        (
            UserQueueView {
                id: queue.id,
                code: queue.code.clone(),
                name: queue.name.clone(),
                fields: queue.fields.clone(),
                allow_guests: queue.allow_guests,
                opens_at: queue.opens_at.clone(),
                weekly_schedule: queue.weekly_schedule.clone(),
                waiting_count: queue.waiting_count(),
                closed_at,
                closed_by_name,
            },
            your_entry,
        )
    }

    fn rejoin_after_timestamp(left_at: Option<&str>) -> Option<String> {
        let left_at = left_at?;
        let left_at = chrono::DateTime::parse_from_rfc3339(left_at)
            .ok()?
            .with_timezone(&Utc);
        Some((left_at + chrono::Duration::seconds(REJOIN_COOLDOWN_SECS)).to_rfc3339())
    }

    fn normalize_opens_at(opens_at: Option<String>) -> Result<Option<String>, String> {
        let Some(opens_at) = opens_at.map(|value| value.trim().to_string()) else {
            return Ok(None);
        };
        if opens_at.is_empty() {
            return Ok(None);
        }

        let opens_at = DateTime::parse_from_rfc3339(&opens_at)
            .map_err(|error| format!("invalid queue opening time: {error}"))?
            .with_timezone(&Utc);
        Ok(Some(opens_at.to_rfc3339()))
    }

    fn normalize_weekly_schedule(
        weekly_schedule: Option<WeeklySchedule>,
    ) -> Result<Option<WeeklySchedule>, String> {
        let Some(schedule) = weekly_schedule else {
            return Ok(None);
        };
        if schedule.weekday > 6 {
            return Err("weekly schedule day is invalid".to_string());
        }
        if schedule.minute_of_day >= 24 * 60 {
            return Err("weekly schedule time is invalid".to_string());
        }
        Ok(Some(schedule))
    }

    fn queue_is_open(queue: &Queue) -> bool {
        let now = Utc::now();
        if let Some(opens_at) = queue.opens_at.as_deref() {
            let is_past_one_time_open = DateTime::parse_from_rfc3339(opens_at)
                .map(|opens_at| now >= opens_at.with_timezone(&Utc))
                .unwrap_or(true);
            if !is_past_one_time_open {
                return false;
            }
        }

        if let Some(schedule) = queue.weekly_schedule.as_ref() {
            let weekday = now.weekday().num_days_from_sunday() as u8;
            let minute_of_day = (now.hour() * 60 + now.minute()) as u16;
            return weekday > schedule.weekday
                || (weekday == schedule.weekday && minute_of_day >= schedule.minute_of_day);
        }

        true
    }

    fn queue_not_open_message(queue: &Queue) -> String {
        if let Some(opens_at) = queue.opens_at.as_deref() {
            if DateTime::parse_from_rfc3339(opens_at)
                .map(|opens_at| Utc::now() < opens_at.with_timezone(&Utc))
                .unwrap_or(false)
            {
                return format!("This queue opens at {opens_at}.");
            }
        }

        if let Some(schedule) = queue.weekly_schedule.as_ref() {
            return format!(
                "This queue opens weekly on {} at {:02}:{:02} UTC.",
                weekday_label(schedule.weekday),
                schedule.minute_of_day / 60,
                schedule.minute_of_day % 60
            );
        }

        "This queue is not open yet.".to_string()
    }

    pub fn join_queue(
        &mut self,
        queue_id: Uuid,
        mut values: BTreeMap<String, String>,
        user_token: Option<&str>,
        entry_token: Option<&str>,
    ) -> Result<String, String> {
        let requester = if let Some(user_token) = user_token {
            let user = self
                .user_account(user_token)
                .ok_or_else(|| "unknown user session".to_string())?;
            Some((
                Some(user.id),
                user.name.clone(),
                Some(user.email.clone()),
                false,
            ))
        } else {
            None
        };

        let queue = self
            .queues
            .get_mut(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;
        if !Self::queue_is_open(queue) {
            return Err(Self::queue_not_open_message(queue));
        }
        let requester_account_id = requester
            .as_ref()
            .and_then(|(account_id, _, _, _)| *account_id);
        let now = Utc::now();
        let active_entry_index = if let Some(account_id) = requester_account_id {
            queue.entries.iter().position(|entry| {
                entry.requester_account_id == Some(account_id)
                    && matches!(
                        entry.status,
                        QueueEntryStatus::Pending | QueueEntryStatus::Claimed
                    )
            })
        } else if let Some(entry_token) = entry_token {
            queue.entries.iter().position(|entry| {
                entry.token == entry_token
                    && matches!(
                        entry.status,
                        QueueEntryStatus::Pending | QueueEntryStatus::Claimed
                    )
            })
        } else {
            None
        };
        if let Some(index) = active_entry_index {
            return Ok(queue.entries[index].token.clone());
        }

        for field in &queue.fields {
            let mut value = values
                .remove(&field.key)
                .unwrap_or_default()
                .trim()
                .to_string();

            if is_requester_name_key(&field.key) && value.is_empty() {
                if let Some((_, requester_name, _, _)) = requester.as_ref() {
                    value = requester_name.clone();
                }
            }

            if field.required && value.is_empty() {
                return Err(format!("{} is required", field.label));
            }
            if !value.is_empty() && !field.options.is_empty() && !field.options.contains(&value) {
                return Err(format!(
                    "{} must be one of the available options",
                    field.label
                ));
            }

            values.insert(field.key.clone(), value);
        }

        let (_, mut requester_label, requester_email, is_guest) = if let Some(requester) = requester
        {
            requester
        } else if queue.allow_guests {
            (None, "Guest".to_string(), None, true)
        } else {
            return Err("this queue requires a user account".to_string());
        };

        for field in &queue.fields {
            if is_requester_name_key(&field.key) {
                if let Some(name) = values
                    .get(&field.key)
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                {
                    requester_label = name.to_string();
                    break;
                }
            }
        }

        let left_entry_index = if let Some(account_id) = requester_account_id {
            queue.entries.iter().rposition(|entry| {
                entry.requester_account_id == Some(account_id)
                    && entry.status == QueueEntryStatus::Left
            })
        } else if let Some(entry_token) = entry_token {
            queue.entries.iter().position(|entry| {
                entry.token == entry_token && entry.status == QueueEntryStatus::Left
            })
        } else {
            None
        };
        if let Some(index) = left_entry_index {
            let entry = queue.entries.get(index).expect("entry index");
            if let Some(left_at) = entry.left_at.as_deref() {
                let left_at = chrono::DateTime::parse_from_rfc3339(left_at)
                    .map(|value| value.with_timezone(&Utc))
                    .map_err(|error| format!("invalid leave timestamp: {error}"))?;
                let rejoin_at = left_at + chrono::Duration::seconds(REJOIN_COOLDOWN_SECS);
                if now < rejoin_at {
                    let remaining = (rejoin_at - now).num_seconds().max(1);
                    return Err(format!(
                        "Please wait {remaining} seconds before attempting to rejoin."
                    ));
                }
            }
        }

        let id = Uuid::new_v4();
        let token = Uuid::new_v4().to_string();
        queue.entries.push(QueueEntry {
            id,
            token: token.clone(),
            requester_account_id,
            requester_label,
            requester_email,
            is_guest,
            values,
            submitted_at: now.to_rfc3339(),
            left_at: None,
            status: QueueEntryStatus::Pending,
            claimed_by: None,
        });
        self.entry_index.insert(id, queue_id);

        Ok(token)
    }

    pub fn leave_queue(&mut self, queue_id: Uuid, entry_token: &str) -> Result<(), String> {
        let queue = self
            .queues
            .get_mut(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;
        let now = Utc::now();

        let Some(entry) = queue
            .entries
            .iter_mut()
            .find(|entry| entry.token == entry_token)
        else {
            return Err("queue entry not found for leave request".to_string());
        };

        match entry.status {
            QueueEntryStatus::Pending | QueueEntryStatus::Claimed => {
                entry.status = QueueEntryStatus::Left;
                entry.claimed_by = None;
                entry.left_at = Some(now.to_rfc3339());
            }
            QueueEntryStatus::Left | QueueEntryStatus::Resolved | QueueEntryStatus::Denied => {
                return Err("queue entry is already closed".to_string());
            }
        }

        Ok(())
    }

    pub fn claim_entry(&mut self, admin_token: &str, entry_id: Uuid) -> Result<Uuid, String> {
        let (admin_id, admin_name, is_super_admin) = self
            .admin_account(admin_token)
            .map(|admin| (admin.id, admin.name.clone(), admin.is_super_admin()))
            .ok_or_else(|| "unknown admin session".to_string())?;
        let queue_id = self
            .entry_index
            .get(&entry_id)
            .copied()
            .ok_or_else(|| "queue entry not found".to_string())?;
        let queue = self
            .queues
            .get_mut(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;
        if !(is_super_admin || queue.owner_account_id == admin_id) {
            return Err("you do not have access to this queue".to_string());
        }

        let entry = queue
            .entries
            .iter_mut()
            .find(|entry| entry.id == entry_id)
            .ok_or_else(|| "queue entry not found".to_string())?;

        match entry.status {
            QueueEntryStatus::Pending => {
                entry.status = QueueEntryStatus::Claimed;
                entry.claimed_by = Some(admin_name);
                Ok(queue_id)
            }
            _ => Err("only pending requests can be claimed".to_string()),
        }
    }

    pub fn unclaim_entry(&mut self, admin_token: &str, entry_id: Uuid) -> Result<Uuid, String> {
        let (admin_id, is_super_admin) = self
            .admin_account(admin_token)
            .map(|admin| (admin.id, admin.is_super_admin()))
            .ok_or_else(|| "unknown admin session".to_string())?;
        let queue_id = self
            .entry_index
            .get(&entry_id)
            .copied()
            .ok_or_else(|| "queue entry not found".to_string())?;
        let queue = self
            .queues
            .get_mut(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;
        if !(is_super_admin || queue.owner_account_id == admin_id) {
            return Err("you do not have access to this queue".to_string());
        }

        let entry = queue
            .entries
            .iter_mut()
            .find(|entry| entry.id == entry_id)
            .ok_or_else(|| "queue entry not found".to_string())?;

        match entry.status {
            QueueEntryStatus::Claimed => {
                entry.status = QueueEntryStatus::Pending;
                entry.claimed_by = None;
                Ok(queue_id)
            }
            _ => Err("only claimed requests can be unclaimed".to_string()),
        }
    }

    pub fn update_entry_status(
        &mut self,
        admin_token: &str,
        entry_id: Uuid,
        next_status: QueueEntryStatus,
    ) -> Result<Uuid, String> {
        let (admin_id, admin_name, is_super_admin) = self
            .admin_account(admin_token)
            .map(|admin| (admin.id, admin.name.clone(), admin.is_super_admin()))
            .ok_or_else(|| "unknown admin session".to_string())?;
        let queue_id = self
            .entry_index
            .get(&entry_id)
            .copied()
            .ok_or_else(|| "queue entry not found".to_string())?;

        let queue = self
            .queues
            .get_mut(&queue_id)
            .ok_or_else(|| "queue not found".to_string())?;
        if !(is_super_admin || queue.owner_account_id == admin_id) {
            return Err("you do not have access to this queue".to_string());
        }

        let entry = queue
            .entries
            .iter_mut()
            .find(|entry| entry.id == entry_id)
            .ok_or_else(|| "queue entry not found".to_string())?;

        match (&entry.status, &next_status) {
            (QueueEntryStatus::Pending, QueueEntryStatus::Resolved)
            | (QueueEntryStatus::Pending, QueueEntryStatus::Denied)
            | (QueueEntryStatus::Claimed, QueueEntryStatus::Resolved)
            | (QueueEntryStatus::Claimed, QueueEntryStatus::Denied) => {
                if entry.claimed_by.is_none() {
                    entry.claimed_by = Some(admin_name);
                }
                entry.status = next_status;
                Ok(queue_id)
            }
            (QueueEntryStatus::Resolved, QueueEntryStatus::Pending)
            | (QueueEntryStatus::Denied, QueueEntryStatus::Pending)
                if entry.left_at.is_none() =>
            {
                entry.status = QueueEntryStatus::Pending;
                entry.claimed_by = None;
                Ok(queue_id)
            }
            _ => Err("invalid status transition".to_string()),
        }
    }

    pub fn admin_can_see_queue(&self, admin_token: &str, queue_id: Uuid) -> bool {
        let Some(admin) = self.admin_account(admin_token) else {
            return false;
        };
        let Some(queue) = self.queues.get(&queue_id) else {
            return false;
        };
        self.account_can_manage_queue(admin.id, queue)
    }

    pub fn visible_queue_ids(&self, admin_token: &str) -> Option<Vec<Uuid>> {
        let admin = self.admin_account(admin_token)?;
        let mut queue_ids: Vec<Uuid> = self
            .queues
            .values()
            .filter(|queue| self.account_can_manage_queue(admin.id, queue))
            .map(|queue| queue.id)
            .collect();
        queue_ids.sort_by_key(|queue_id| self.queues.get(queue_id).map(|queue| queue.name.clone()));
        Some(queue_ids)
    }

    pub fn public_queues(&self) -> Vec<QueueSummary> {
        let mut queues: Vec<QueueSummary> = self
            .queues
            .values()
            .filter(|queue| queue.is_public && Self::queue_is_open(queue))
            .map(Queue::summary)
            .collect();
        queues.sort_by_key(|queue| queue.name.clone());
        queues
    }

    pub fn site_settings_view(&self) -> SiteSettingsView {
        self.site_settings.view()
    }

    pub fn queue_unavailable_message(&self, queue_id: Uuid) -> Option<String> {
        let queue = self.queues.get(&queue_id)?;
        (!Self::queue_is_open(queue)).then(|| Self::queue_not_open_message(queue))
    }

    pub fn queue_id_for_code(&self, code: &str) -> Option<Uuid> {
        self.queue_code_index
            .get(&Queue::normalize_code(code))
            .copied()
    }

    pub fn admin_identity(&self, admin_token: &str) -> Option<AdminIdentityView> {
        let session = self.admin_sessions.get(admin_token)?;
        let admin = self.accounts.get(&session.account_id)?;
        Some(AdminIdentityView {
            token: session.token.clone(),
            account_id: admin.id,
            name: admin.name.clone(),
            email: admin.email.clone(),
            is_super_admin: admin.is_super_admin(),
        })
    }

    pub fn user_identity(&self, user_token: &str) -> Option<UserIdentityView> {
        let session = self.user_sessions.get(user_token)?;
        let user = self.accounts.get(&session.account_id)?;
        Some(UserIdentityView {
            token: session.token.clone(),
            name: user.name.clone(),
            email: user.email.clone(),
        })
    }

    fn authenticate_account(&self, email: String, password: String) -> Result<&Account, String> {
        let email = normalize_email(&email)?;
        let account_id = self
            .account_email_index
            .get(&email)
            .copied()
            .ok_or_else(|| "invalid email or password".to_string())?;
        let account = self
            .accounts
            .get(&account_id)
            .ok_or_else(|| "invalid email or password".to_string())?;
        if !verify_password(password.trim(), &account.password_hash) {
            return Err("invalid email or password".to_string());
        }
        Ok(account)
    }

    fn account_by_email(&self, email: String) -> Result<&Account, String> {
        let email = normalize_email(&email)?;
        let account_id = self
            .account_email_index
            .get(&email)
            .copied()
            .ok_or_else(|| "no local account matches this Microsoft email".to_string())?;
        self.accounts
            .get(&account_id)
            .ok_or_else(|| "no local account matches this Microsoft email".to_string())
    }

    fn create_admin_session(&mut self, account_id: Uuid) -> Result<AdminIdentityView, String> {
        let token = Uuid::new_v4().to_string();
        self.admin_sessions.insert(
            token.clone(),
            AdminSession {
                token: token.clone(),
                account_id,
            },
        );

        self.admin_identity(&token)
            .ok_or_else(|| "failed to create admin session".to_string())
    }

    fn create_user_session(&mut self, account_id: Uuid) -> Result<UserIdentityView, String> {
        let token = Uuid::new_v4().to_string();
        self.user_sessions.insert(
            token.clone(),
            UserSession {
                token: token.clone(),
                account_id,
            },
        );

        self.user_identity(&token)
            .ok_or_else(|| "failed to create user session".to_string())
    }

    fn admin_account(&self, admin_token: &str) -> Option<&Account> {
        let session = self.admin_sessions.get(admin_token)?;
        self.accounts.get(&session.account_id)
    }

    fn user_account(&self, user_token: &str) -> Option<&Account> {
        let session = self.user_sessions.get(user_token)?;
        self.accounts.get(&session.account_id)
    }

    fn account_can_manage_queue(&self, account_id: Uuid, queue: &Queue) -> bool {
        let Some(account) = self.accounts.get(&account_id) else {
            return false;
        };
        account.is_super_admin()
            || queue.owner_account_id == account_id
            || queue.shared_account_ids.contains(&account_id)
            || queue.shared_group_ids.iter().any(|group_id| {
                self.groups.get(group_id).is_some_and(|group| {
                    group.role == AccountRole::Admin && group.member_ids.contains(&account_id)
                })
            })
    }

    fn validated_group_members(
        &self,
        role: AccountRole,
        member_ids: Vec<Uuid>,
    ) -> Result<Vec<Uuid>, String> {
        let mut validated = Vec::new();
        for member_id in member_ids {
            let account = self
                .accounts
                .get(&member_id)
                .ok_or_else(|| "group member account not found".to_string())?;
            let valid = match role {
                AccountRole::Admin => account.can_administer(),
                AccountRole::User => account.can_join_queues(),
                AccountRole::SuperAdmin => false,
            };
            if !valid {
                return Err("group members must match the group role".to_string());
            }
            if !validated.contains(&member_id) {
                validated.push(member_id);
            }
        }
        Ok(validated)
    }

    fn validated_share_accounts(&self, account_ids: Vec<Uuid>) -> Result<Vec<Uuid>, String> {
        let mut validated = Vec::new();
        for account_id in account_ids {
            let account = self
                .accounts
                .get(&account_id)
                .ok_or_else(|| "shared admin account not found".to_string())?;
            if !account.can_administer() {
                return Err("queues can only be shared with admin accounts".to_string());
            }
            if !validated.contains(&account_id) {
                validated.push(account_id);
            }
        }
        Ok(validated)
    }

    fn validated_share_groups(&self, group_ids: Vec<Uuid>) -> Result<Vec<Uuid>, String> {
        let mut validated = Vec::new();
        for group_id in group_ids {
            let group = self
                .groups
                .get(&group_id)
                .ok_or_else(|| "shared admin group not found".to_string())?;
            if group.role != AccountRole::Admin {
                return Err("queues can only be shared with admin groups".to_string());
            }
            if !validated.contains(&group_id) {
                validated.push(group_id);
            }
        }
        Ok(validated)
    }

    fn cleanup_account_references(&mut self) {
        let account_ids: Vec<Uuid> = self.accounts.keys().copied().collect();
        for queue in self.queues.values_mut() {
            queue
                .shared_account_ids
                .retain(|account_id| account_ids.contains(account_id));
        }
        for group in self.groups.values_mut() {
            group
                .member_ids
                .retain(|account_id| account_ids.contains(account_id));
        }
        self.admin_sessions
            .retain(|_, session| self.accounts.contains_key(&session.account_id));
        self.user_sessions
            .retain(|_, session| self.accounts.contains_key(&session.account_id));
    }

    fn cleanup_group_references(&mut self) {
        let admin_group_ids: Vec<Uuid> = self
            .groups
            .values()
            .filter(|group| group.role == AccountRole::Admin)
            .map(|group| group.id)
            .collect();
        for queue in self.queues.values_mut() {
            queue
                .shared_group_ids
                .retain(|group_id| admin_group_ids.contains(group_id));
        }
    }
}

#[cfg(test)]
mod tests {
    use shared::AccountRole;

    use super::*;
    use crate::password::{is_password_hash, verify_password};

    #[test]
    fn created_accounts_store_hashes_and_authenticate_with_plaintext_input() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login super admin");

        store
            .create_account(
                &admin.token,
                "User".to_string(),
                "user@example.com".to_string(),
                "user-pass".to_string(),
                AccountRole::User,
            )
            .expect("create user");
        store
            .create_account(
                &admin.token,
                "Admin".to_string(),
                "admin@example.com".to_string(),
                "admin-pass".to_string(),
                AccountRole::Admin,
            )
            .expect("create admin");
        store
            .create_account(
                &admin.token,
                "Second Super".to_string(),
                "second-super@example.com".to_string(),
                "second-super-pass".to_string(),
                AccountRole::SuperAdmin,
            )
            .expect("create second super admin");

        let user_account = store
            .accounts
            .values()
            .find(|account| account.email == "user@example.com")
            .expect("user account");
        assert!(is_password_hash(&user_account.password_hash));
        assert_ne!(user_account.password_hash, "user-pass");
        assert!(verify_password("user-pass", &user_account.password_hash));

        assert!(store
            .login_user("user@example.com".to_string(), "user-pass".to_string())
            .is_ok());
        assert!(store
            .login_user("admin@example.com".to_string(), "admin-pass".to_string())
            .is_ok());
        assert!(store
            .login_user(
                "second-super@example.com".to_string(),
                "second-super-pass".to_string()
            )
            .is_ok());
        assert!(store
            .login_user("user@example.com".to_string(), "wrong-pass".to_string())
            .is_err());
        assert!(store
            .login_admin(
                "second-super@example.com".to_string(),
                "second-super-pass".to_string()
            )
            .is_ok());
    }

    #[test]
    fn microsoft_user_login_creates_missing_user_account() {
        let mut store = Store::default();

        let user = store
            .login_or_create_user_with_microsoft(
                " Ada@Example.COM ".to_string(),
                Some("Ada Lovelace".to_string()),
            )
            .expect("create user from Microsoft login");

        assert_eq!(user.email, "ada@example.com");
        assert_eq!(user.name, "Ada Lovelace");
        assert!(store.user_identity(&user.token).is_some());

        let account_id = store
            .account_email_index
            .get("ada@example.com")
            .copied()
            .expect("created account id");
        let account = store.accounts.get(&account_id).expect("created account");
        assert_eq!(account.role, AccountRole::User);
        assert_eq!(account.email, "ada@example.com");
        assert_eq!(account.name, "Ada Lovelace");
        assert!(is_password_hash(&account.password_hash));
    }

    #[test]
    fn microsoft_user_login_reuses_existing_account() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login super admin");
        store
            .create_account(
                &admin.token,
                "Existing User".to_string(),
                "user@example.com".to_string(),
                "user-pass".to_string(),
                AccountRole::User,
            )
            .expect("create existing user");

        let user = store
            .login_or_create_user_with_microsoft(
                "user@example.com".to_string(),
                Some("Changed Name".to_string()),
            )
            .expect("login existing user from Microsoft");

        assert_eq!(user.name, "Existing User");
        assert_eq!(store.accounts.len(), 2);
    }

    #[test]
    fn disabled_sign_in_methods_are_rejected() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        store.site_settings.admin_password_sign_in_enabled = false;
        store.site_settings.admin_microsoft_sign_in_enabled = false;
        store.site_settings.user_password_sign_in_enabled = false;
        store.site_settings.user_microsoft_sign_in_enabled = false;

        assert_eq!(
            store
                .login_admin("super@example.com".to_string(), "super-pass".to_string())
                .expect_err("admin password disabled"),
            "admin password sign-in is disabled"
        );
        assert_eq!(
            store
                .login_admin_with_email("super@example.com".to_string())
                .expect_err("admin Microsoft disabled"),
            "admin Microsoft sign-in is disabled"
        );
        assert_eq!(
            store
                .login_user("super@example.com".to_string(), "super-pass".to_string())
                .expect_err("user password disabled"),
            "user password sign-in is disabled"
        );
        assert_eq!(
            store
                .login_or_create_user_with_microsoft(
                    "user@example.com".to_string(),
                    Some("User".to_string())
                )
                .expect_err("user Microsoft disabled"),
            "user Microsoft sign-in is disabled"
        );
    }

    #[test]
    fn queues_get_join_codes_that_resolve_to_queue_ids() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login super admin");

        let queue_id = store
            .create_queue(
                &admin.token,
                "Support".to_string(),
                Vec::new(),
                true,
                false,
                None,
                None,
            )
            .expect("create queue");
        let queue = store.queues.get(&queue_id).expect("queue");

        assert_eq!(queue.code.len(), 6);
        assert_eq!(store.queue_id_for_code(&queue.code), Some(queue_id));
        assert_eq!(
            store.queue_id_for_code(&queue.code.to_ascii_lowercase()),
            Some(queue_id)
        );
        assert_eq!(queue.summary().code, queue.code);
    }

    #[test]
    fn resolved_or_denied_entries_can_be_reopened_if_not_left() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login super admin");
        let queue_id = store
            .create_queue(
                &admin.token,
                "Support".to_string(),
                Vec::new(),
                true,
                false,
                None,
                None,
            )
            .expect("create queue");

        store
            .join_queue(queue_id, BTreeMap::new(), None, None)
            .expect("join queue");
        let entry_id = store.queues[&queue_id].entries[0].id;

        store
            .update_entry_status(&admin.token, entry_id, QueueEntryStatus::Resolved)
            .expect("resolve entry");
        store
            .update_entry_status(&admin.token, entry_id, QueueEntryStatus::Pending)
            .expect("reopen resolved entry");
        assert_eq!(
            store.queues[&queue_id].entries[0].status,
            QueueEntryStatus::Pending
        );
        assert_eq!(store.queues[&queue_id].entries[0].claimed_by, None);

        store
            .update_entry_status(&admin.token, entry_id, QueueEntryStatus::Denied)
            .expect("deny entry");
        store
            .update_entry_status(&admin.token, entry_id, QueueEntryStatus::Pending)
            .expect("reopen denied entry");
        assert_eq!(
            store.queues[&queue_id].entries[0].status,
            QueueEntryStatus::Pending
        );
    }

    #[test]
    fn super_admin_cannot_demote_self() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login super admin");

        assert!(store
            .update_account(
                &admin.token,
                admin.account_id,
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                None,
                AccountRole::Admin,
            )
            .is_err());
        assert_eq!(
            store
                .accounts
                .get(&admin.account_id)
                .map(|account| &account.role),
            Some(&AccountRole::SuperAdmin)
        );
    }

    #[test]
    fn initial_setup_creates_exactly_one_super_admin() {
        let mut store = Store::default();
        assert!(store.needs_initial_setup());

        let admin = store
            .setup_initial_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("setup super admin");

        assert_eq!(admin.email, "super@example.com");
        assert!(admin.is_super_admin);
        assert!(!store.needs_initial_setup());
        assert!(store
            .setup_initial_super_admin(
                "Other Admin".to_string(),
                "other@example.com".to_string(),
                "other-pass".to_string(),
            )
            .is_err());
    }

    #[test]
    fn admin_group_sharing_grants_queue_visibility() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let super_admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login super admin");

        store
            .create_account(
                &super_admin.token,
                "Queue Owner".to_string(),
                "owner@example.com".to_string(),
                "owner-pass".to_string(),
                AccountRole::Admin,
            )
            .expect("create owner");
        store
            .create_account(
                &super_admin.token,
                "Shared Admin".to_string(),
                "shared@example.com".to_string(),
                "shared-pass".to_string(),
                AccountRole::Admin,
            )
            .expect("create shared admin");

        let owner = store
            .login_admin("owner@example.com".to_string(), "owner-pass".to_string())
            .expect("login owner");
        let shared = store
            .login_admin("shared@example.com".to_string(), "shared-pass".to_string())
            .expect("login shared admin");
        let shared_account_id = store
            .account_email_index
            .get("shared@example.com")
            .copied()
            .expect("shared account id");

        let queue_id = store
            .create_queue(
                &owner.token,
                "Support".to_string(),
                vec![QueueField {
                    key: "name".to_string(),
                    label: "Name".to_string(),
                    required: true,
                    options: Vec::new(),
                }],
                true,
                false,
                None,
                None,
            )
            .expect("create queue");

        assert!(!store.admin_can_see_queue(&shared.token, queue_id));

        store
            .create_group(
                &super_admin.token,
                "Support admins".to_string(),
                AccountRole::Admin,
                vec![shared_account_id],
            )
            .expect("create admin group");
        let group_id = store
            .groups
            .values()
            .find(|group| group.name == "Support admins")
            .map(|group| group.id)
            .expect("group id");

        store
            .share_queue(&owner.token, queue_id, Vec::new(), vec![group_id])
            .expect("share queue with group");

        assert!(store.admin_can_see_queue(&shared.token, queue_id));
    }

    #[test]
    fn closing_queue_archives_history_and_removes_active_access() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login admin");

        let queue_id = store
            .create_queue(
                &admin.token,
                "Support".to_string(),
                vec![QueueField {
                    key: "name".to_string(),
                    label: "Name".to_string(),
                    required: true,
                    options: Vec::new(),
                }],
                true,
                false,
                None,
                None,
            )
            .expect("create queue");
        let mut values = BTreeMap::new();
        values.insert("name".to_string(), "Ada".to_string());
        let entry_token = store
            .join_queue(queue_id, values, None, None)
            .expect("join queue");

        store
            .close_queue(&admin.token, queue_id)
            .expect("close queue");

        assert!(!store.queues.contains_key(&queue_id));
        let (closed_queue, closed_entry) = store
            .user_view(queue_id, Some(&entry_token))
            .expect("closed queue remains visible to subscribed users");
        assert_eq!(closed_queue.id, queue_id);
        assert_eq!(closed_queue.closed_by_name.as_deref(), Some("Super Admin"));
        assert!(closed_queue.closed_at.is_some());
        assert_eq!(
            closed_entry.map(|entry| entry.status),
            Some(QueueEntryStatus::Pending)
        );
        assert_eq!(
            store
                .archived_queues
                .get(&queue_id)
                .map(|archive| archive.queue.entries.len()),
            Some(1)
        );
        let admin_state = store
            .admin_state(&admin.token, None)
            .expect("admin archive state");
        let archived_queue = admin_state
            .archived_queues
            .iter()
            .find(|queue| queue.summary.id == queue_id)
            .expect("archived queue view");
        assert_eq!(archived_queue.fields.len(), 1);
        assert_eq!(archived_queue.entries.len(), 1);
        assert_eq!(
            archived_queue.entries[0]
                .values
                .get("name")
                .map(String::as_str),
            Some("Ada")
        );
    }

    #[test]
    fn signed_in_join_infers_optional_name_field() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login admin");
        store
            .create_account(
                &admin.token,
                "Ada Lovelace".to_string(),
                "ada@example.com".to_string(),
                "ada-pass".to_string(),
                AccountRole::User,
            )
            .expect("create user");
        let user = store
            .login_user("ada@example.com".to_string(), "ada-pass".to_string())
            .expect("login user");
        store
            .create_account(
                &admin.token,
                "Grace Hopper".to_string(),
                "grace@example.com".to_string(),
                "grace-pass".to_string(),
                AccountRole::User,
            )
            .expect("create second user");
        let second_user = store
            .login_user("grace@example.com".to_string(), "grace-pass".to_string())
            .expect("login second user");

        let queue_id = store
            .create_queue(
                &admin.token,
                "Support".to_string(),
                vec![
                    QueueField {
                        key: "name".to_string(),
                        label: "Name".to_string(),
                        required: false,
                        options: Vec::new(),
                    },
                    QueueField {
                        key: "subject".to_string(),
                        label: "Subject".to_string(),
                        required: true,
                        options: Vec::new(),
                    },
                ],
                false,
                false,
                None,
                None,
            )
            .expect("create queue");
        let mut values = BTreeMap::new();
        values.insert("subject".to_string(), "Compiler help".to_string());

        store
            .join_queue(queue_id, values, Some(&user.token), None)
            .expect("join queue");

        let entry = store
            .queues
            .get(&queue_id)
            .and_then(|queue| queue.entries.first())
            .expect("queue entry");
        assert_eq!(entry.requester_label, "Ada Lovelace");
        assert_eq!(entry.values.get("name"), Some(&"Ada Lovelace".to_string()));
        assert_eq!(
            entry.values.get("subject"),
            Some(&"Compiler help".to_string())
        );

        let mut values = BTreeMap::new();
        values.insert("name".to_string(), "Grace H.".to_string());
        values.insert("subject".to_string(), "Borrow checker".to_string());
        store
            .join_queue(queue_id, values, Some(&second_user.token), None)
            .expect("join queue with custom name");

        let entry = store
            .queues
            .get(&queue_id)
            .and_then(|queue| queue.entries.get(1))
            .expect("second queue entry");
        assert_eq!(entry.requester_label, "Grace H.");
        assert_eq!(entry.values.get("name"), Some(&"Grace H.".to_string()));
    }

    #[test]
    fn signed_in_join_infers_full_name_field_variations() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login admin");
        store
            .create_account(
                &admin.token,
                "Ada Lovelace".to_string(),
                "ada@example.com".to_string(),
                "ada-pass".to_string(),
                AccountRole::User,
            )
            .expect("create user");
        let user = store
            .login_user("ada@example.com".to_string(), "ada-pass".to_string())
            .expect("login user");
        store
            .create_account(
                &admin.token,
                "Grace Hopper".to_string(),
                "grace@example.com".to_string(),
                "grace-pass".to_string(),
                AccountRole::User,
            )
            .expect("create second user");
        let second_user = store
            .login_user("grace@example.com".to_string(), "grace-pass".to_string())
            .expect("login second user");
        let queue_id = store
            .create_queue(
                &admin.token,
                "Support".to_string(),
                vec![QueueField {
                    key: String::new(),
                    label: "Full Name.".to_string(),
                    required: true,
                    options: Vec::new(),
                }],
                false,
                false,
                None,
                None,
            )
            .expect("create queue");

        store
            .join_queue(queue_id, BTreeMap::new(), Some(&user.token), None)
            .expect("join queue with inferred full name");
        let entry = store
            .queues
            .get(&queue_id)
            .and_then(|queue| queue.entries.first())
            .expect("queue entry");
        assert_eq!(entry.requester_label, "Ada Lovelace");
        assert_eq!(
            entry.values.get("full_name"),
            Some(&"Ada Lovelace".to_string())
        );

        let mut values = BTreeMap::new();
        values.insert("full_name".to_string(), "Grace H.".to_string());
        store
            .join_queue(queue_id, values, Some(&second_user.token), None)
            .expect("join queue with custom full name");
        let entry = store
            .queues
            .get(&queue_id)
            .and_then(|queue| queue.entries.get(1))
            .expect("second queue entry");
        assert_eq!(entry.requester_label, "Grace H.");
    }

    #[test]
    fn queue_can_be_created_and_joined_without_fields() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login admin");
        store
            .create_account(
                &admin.token,
                "Ada Lovelace".to_string(),
                "ada@example.com".to_string(),
                "ada-pass".to_string(),
                AccountRole::User,
            )
            .expect("create user");
        let user = store
            .login_user("ada@example.com".to_string(), "ada-pass".to_string())
            .expect("login user");

        let queue_id = store
            .create_queue(
                &admin.token,
                "Support".to_string(),
                Vec::new(),
                false,
                false,
                None,
                None,
            )
            .expect("create queue");

        store
            .join_queue(queue_id, BTreeMap::new(), Some(&user.token), None)
            .expect("join queue");

        let entry = store
            .queues
            .get(&queue_id)
            .and_then(|queue| queue.entries.first())
            .expect("queue entry");
        assert_eq!(entry.requester_label, "Ada Lovelace");
        assert!(entry.values.is_empty());
    }

    #[test]
    fn live_queue_settings_can_update_fields_and_guest_access() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login admin");
        let queue_id = store
            .create_queue(
                &admin.token,
                "Support".to_string(),
                vec![QueueField {
                    key: "subject".to_string(),
                    label: "Subject".to_string(),
                    required: true,
                    options: Vec::new(),
                }],
                false,
                false,
                None,
                None,
            )
            .expect("create queue");

        store
            .update_queue_settings(
                &admin.token,
                queue_id,
                vec![QueueField {
                    key: "topic".to_string(),
                    label: "Topic".to_string(),
                    required: false,
                    options: Vec::new(),
                }],
                true,
                false,
                None,
                None,
            )
            .expect("update queue settings");

        let (queue_view, _) = store.user_view(queue_id, None).expect("user queue view");
        assert!(queue_view.allow_guests);
        assert_eq!(queue_view.fields[0].key, "topic");
        assert!(!queue_view.fields[0].required);

        store
            .join_queue(queue_id, BTreeMap::new(), None, None)
            .expect("guest can join with optional field empty");
    }

    #[test]
    fn dropdown_field_options_are_normalized_and_enforced() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login admin");
        let queue_id = store
            .create_queue(
                &admin.token,
                "Support".to_string(),
                vec![QueueField {
                    key: "topic".to_string(),
                    label: "Topic".to_string(),
                    required: true,
                    options: vec![
                        "Billing".to_string(),
                        " ".to_string(),
                        "Technical".to_string(),
                        "Billing".to_string(),
                    ],
                }],
                true,
                false,
                None,
                None,
            )
            .expect("create queue");

        let (queue_view, _) = store.user_view(queue_id, None).expect("user queue view");
        assert_eq!(
            queue_view.fields[0].options,
            vec!["Billing".to_string(), "Technical".to_string()]
        );

        let mut values = BTreeMap::new();
        values.insert("topic".to_string(), "Other".to_string());
        let error = store
            .join_queue(queue_id, values, None, None)
            .expect_err("invalid dropdown option is rejected");
        assert!(error.contains("available options"));

        let mut values = BTreeMap::new();
        values.insert("topic".to_string(), "Technical".to_string());
        store
            .join_queue(queue_id, values, None, None)
            .expect("valid dropdown option is accepted");
    }

    #[test]
    fn public_queues_only_returns_public_active_queues() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login admin");

        let private_queue_id = store
            .create_queue(
                &admin.token,
                "Private".to_string(),
                Vec::new(),
                true,
                false,
                None,
                None,
            )
            .expect("create private queue");
        let public_queue_id = store
            .create_queue(
                &admin.token,
                "Public".to_string(),
                Vec::new(),
                true,
                true,
                None,
                None,
            )
            .expect("create public queue");

        let queues = store.public_queues();
        assert_eq!(queues.len(), 1);
        assert_eq!(queues[0].id, public_queue_id);
        assert!(queues[0].is_public);

        store
            .update_queue_settings(
                &admin.token,
                private_queue_id,
                Vec::new(),
                true,
                true,
                None,
                None,
            )
            .expect("make private queue public");
        let queues = store.public_queues();
        assert_eq!(queues.len(), 2);
        assert_eq!(queues[0].name, "Private");
        assert_eq!(queues[1].name, "Public");
    }

    #[test]
    fn scheduled_queue_is_hidden_and_blocked_until_open() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login admin");
        let opens_at = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let queue_id = store
            .create_queue(
                &admin.token,
                "Scheduled".to_string(),
                Vec::new(),
                true,
                true,
                Some(opens_at.clone()),
                None,
            )
            .expect("create scheduled queue");

        assert!(store.public_queues().is_empty());
        assert!(store.user_view(queue_id, None).is_none());
        assert!(store.queue_unavailable_message(queue_id).is_some());
        let error = store
            .join_queue(queue_id, BTreeMap::new(), None, None)
            .expect_err("scheduled queue cannot be joined early");
        assert!(error.contains(&opens_at));

        let past_opens_at = (Utc::now() - chrono::Duration::minutes(1)).to_rfc3339();
        store
            .update_queue_settings(
                &admin.token,
                queue_id,
                Vec::new(),
                true,
                true,
                Some(past_opens_at),
                None,
            )
            .expect("open scheduled queue");

        assert_eq!(store.public_queues().len(), 1);
        assert!(store.user_view(queue_id, None).is_some());
        store
            .join_queue(queue_id, BTreeMap::new(), None, None)
            .expect("opened scheduled queue can be joined");
    }

    #[test]
    fn weekly_scheduled_queue_reopens_on_schedule() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login admin");
        let now = Utc::now();
        let weekday = now.weekday().num_days_from_sunday() as u8;
        let current_minute = (now.hour() * 60 + now.minute()) as u16;
        let future_minute = (current_minute + 1).min(23 * 60 + 59);

        let queue_id = store
            .create_queue(
                &admin.token,
                "Weekly".to_string(),
                Vec::new(),
                true,
                true,
                None,
                Some(WeeklySchedule {
                    weekday,
                    minute_of_day: future_minute,
                }),
            )
            .expect("create weekly scheduled queue");

        if future_minute > current_minute {
            assert!(store.public_queues().is_empty());
            assert!(store.user_view(queue_id, None).is_none());
            let error = store
                .join_queue(queue_id, BTreeMap::new(), None, None)
                .expect_err("weekly queue cannot be joined before opening time");
            assert!(error.contains("weekly"));
        }

        store
            .update_queue_settings(
                &admin.token,
                queue_id,
                Vec::new(),
                true,
                true,
                None,
                Some(WeeklySchedule {
                    weekday,
                    minute_of_day: 0,
                }),
            )
            .expect("move weekly schedule to an already-open time");

        assert_eq!(store.public_queues().len(), 1);
        let (queue_view, _) = store.user_view(queue_id, None).expect("user queue view");
        assert_eq!(
            queue_view.weekly_schedule.expect("weekly schedule").weekday,
            weekday
        );
        store
            .join_queue(queue_id, BTreeMap::new(), None, None)
            .expect("weekly scheduled queue can be joined after opening time");
    }

    #[test]
    fn guest_requester_label_is_always_guest() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login admin");

        let queue_id = store
            .create_queue(
                &admin.token,
                "Support".to_string(),
                vec![QueueField {
                    key: "account".to_string(),
                    label: "Account".to_string(),
                    required: true,
                    options: Vec::new(),
                }],
                true,
                false,
                None,
                None,
            )
            .expect("create queue");
        let mut values = BTreeMap::new();
        values.insert("account".to_string(), "Ada".to_string());

        store
            .join_queue(queue_id, values, None, None)
            .expect("join queue");

        let entry = store
            .queues
            .get(&queue_id)
            .and_then(|queue| queue.entries.first())
            .expect("queue entry");
        assert_eq!(entry.requester_label, "Guest");
        assert_eq!(entry.values.get("account"), Some(&"Ada".to_string()));
    }

    #[test]
    fn guest_rejoin_is_blocked_during_cooldown_then_creates_separate_request() {
        let mut store = Store::default();
        store
            .bootstrap_super_admin(
                "Super Admin".to_string(),
                "super@example.com".to_string(),
                "super-pass".to_string(),
            )
            .expect("bootstrap super admin");
        let admin = store
            .login_admin("super@example.com".to_string(), "super-pass".to_string())
            .expect("login admin");
        let queue_id = store
            .create_queue(
                &admin.token,
                "Support".to_string(),
                Vec::new(),
                true,
                false,
                None,
                None,
            )
            .expect("create queue");

        let entry_token = store
            .join_queue(queue_id, BTreeMap::new(), None, None)
            .expect("guest joins queue");
        store
            .leave_queue(queue_id, &entry_token)
            .expect("guest leaves queue");

        let (_, left_entry) = store
            .user_view(queue_id, Some(&entry_token))
            .expect("queue remains visible");
        let left_entry = left_entry.expect("left entry remains visible by token");
        assert_eq!(left_entry.status, QueueEntryStatus::Left);
        assert!(left_entry.rejoin_after.is_some());

        let error = store
            .join_queue(queue_id, BTreeMap::new(), None, Some(&entry_token))
            .expect_err("guest cannot immediately rejoin");
        assert!(error.contains("Please wait"));
        assert_eq!(
            store.queues.get(&queue_id).map(|queue| queue.entries.len()),
            Some(1)
        );

        let old_left_at = Utc::now() - chrono::Duration::seconds(REJOIN_COOLDOWN_SECS + 1);
        let queue = store.queues.get_mut(&queue_id).expect("queue");
        queue.entries[0].left_at = Some(old_left_at.to_rfc3339());

        let new_token = store
            .join_queue(queue_id, BTreeMap::new(), None, Some(&entry_token))
            .expect("guest can rejoin after cooldown");
        assert_ne!(new_token, entry_token);

        let queue = store.queues.get(&queue_id).expect("queue");
        assert_eq!(queue.entries.len(), 2);
        assert_eq!(queue.entries[0].status, QueueEntryStatus::Left);
        assert_eq!(queue.entries[1].status, QueueEntryStatus::Pending);
    }
}
