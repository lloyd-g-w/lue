use std::collections::BTreeMap;

use chrono::Utc;
use shared::{
    AccountRole, AccountView, AdminEntryView, AdminIdentityView, AdminQueueListItem,
    AdminQueueView, AdminStateView, ArchivedQueueListItem, GroupView, QueueEntryStatus, QueueField,
    UserEntryView, UserIdentityView, UserQueueView,
};
use uuid::Uuid;

use crate::model::{
    Account, AdminSession, ArchivedQueue, Group, Queue, QueueEntry, Store, UserSession,
};
use crate::password::{hash_password, verify_password};
use crate::utils::{display_label_from_values, normalize_email, normalize_fields};

impl Store {
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
            return Err("SUPER_ADMIN_NAME cannot be empty".to_string());
        }
        if password.is_empty() {
            return Err("SUPER_ADMIN_PASSWORD cannot be empty".to_string());
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
        let account = self.authenticate_account(email, password)?;
        if !account.can_administer() {
            return Err("this account does not have admin access".to_string());
        }

        let token = Uuid::new_v4().to_string();
        self.admin_sessions.insert(
            token.clone(),
            AdminSession {
                token: token.clone(),
                account_id: account.id,
            },
        );

        self.admin_identity(&token)
            .ok_or_else(|| "failed to create admin session".to_string())
    }

    pub fn login_user(
        &mut self,
        email: String,
        password: String,
    ) -> Result<UserIdentityView, String> {
        let account = self.authenticate_account(email, password)?;
        if !account.can_join_queues() {
            return Err("use a user account to join queues".to_string());
        }

        let token = Uuid::new_v4().to_string();
        self.user_sessions.insert(
            token.clone(),
            UserSession {
                token: token.clone(),
                account_id: account.id,
            },
        );

        self.user_identity(&token)
            .ok_or_else(|| "failed to create user session".to_string())
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
    ) -> Result<Uuid, String> {
        let admin = self
            .admin_account(admin_token)
            .ok_or_else(|| "unknown admin session".to_string())?;

        let normalized_name = name.trim().to_string();
        if normalized_name.is_empty() {
            return Err("queue name is required".to_string());
        }

        let fields = normalize_fields(fields)?;
        if fields.is_empty() {
            return Err("at least one queue field is required".to_string());
        }

        let id = Uuid::new_v4();
        self.queues.insert(
            id,
            Queue {
                id,
                name: normalized_name,
                allow_guests,
                owner_account_id: admin.id,
                owner_name: admin.name.clone(),
                shared_account_ids: Vec::new(),
                shared_group_ids: Vec::new(),
                fields,
                entries: Vec::new(),
            },
        );
        Ok(id)
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
        let queue = self.queues.get(&queue_id)?;
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
                    position: queue.position_for(entry.id),
                    requester_label: entry.requester_label.clone(),
                    is_guest: entry.is_guest,
                })
        });

        Some((
            UserQueueView {
                id: queue.id,
                name: queue.name.clone(),
                fields: queue.fields.clone(),
                allow_guests: queue.allow_guests,
                waiting_count: queue.waiting_count(),
            },
            your_entry,
        ))
    }

    pub fn join_queue(
        &mut self,
        queue_id: Uuid,
        mut values: BTreeMap<String, String>,
        user_token: Option<&str>,
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

        for field in &queue.fields {
            let value = values
                .remove(&field.key)
                .unwrap_or_default()
                .trim()
                .to_string();

            if field.required && value.is_empty() {
                return Err(format!("{} is required", field.label));
            }

            values.insert(field.key.clone(), value);
        }

        let (_account_id, requester_label, requester_email, is_guest) =
            if let Some(requester) = requester {
                requester
            } else if queue.allow_guests {
                (None, display_label_from_values(&values), None, true)
            } else {
                return Err("this queue requires a user account".to_string());
            };

        let id = Uuid::new_v4();
        let token = Uuid::new_v4().to_string();
        queue.entries.push(QueueEntry {
            id,
            token: token.clone(),
            requester_label,
            requester_email,
            is_guest,
            values,
            submitted_at: Utc::now().to_rfc3339(),
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
                }],
                true,
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
                }],
                true,
            )
            .expect("create queue");
        let mut values = BTreeMap::new();
        values.insert("name".to_string(), "Ada".to_string());
        store
            .join_queue(queue_id, values, None)
            .expect("join queue");

        store
            .close_queue(&admin.token, queue_id)
            .expect("close queue");

        assert!(!store.queues.contains_key(&queue_id));
        assert!(store.user_view(queue_id, None).is_none());
        assert_eq!(
            store
                .archived_queues
                .get(&queue_id)
                .map(|archive| archive.queue.entries.len()),
            Some(1)
        );
    }
}
