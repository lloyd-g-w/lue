use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::{Account, AdminSession, ArchivedQueue, Group, Queue, Store, UserSession};
use crate::password::{hash_password, is_password_hash};

#[derive(Default, Deserialize, Serialize)]
struct StoreSnapshot {
    accounts: HashMap<Uuid, Account>,
    queues: HashMap<Uuid, Queue>,
    #[serde(default)]
    archived_queues: HashMap<Uuid, ArchivedQueue>,
    #[serde(default)]
    groups: HashMap<Uuid, Group>,
    #[serde(default)]
    admin_sessions: HashMap<String, AdminSession>,
    #[serde(default)]
    user_sessions: HashMap<String, UserSession>,
}

impl Store {
    pub fn load_from_disk(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(contents) => {
                let snapshot: StoreSnapshot = serde_json::from_str(&contents).map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("failed to parse store snapshot: {error}"),
                    )
                })?;
                Self::from_snapshot(snapshot)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    pub fn save_to_disk(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let snapshot = StoreSnapshot {
            accounts: self.accounts.clone(),
            queues: self.queues.clone(),
            archived_queues: self.archived_queues.clone(),
            groups: self.groups.clone(),
            admin_sessions: self.admin_sessions.clone(),
            user_sessions: self.user_sessions.clone(),
        };
        let contents = serde_json::to_string_pretty(&snapshot)
            .map_err(|error| io::Error::other(format!("failed to serialize store: {error}")))?;
        let temp_path = path.with_extension("json.tmp");
        fs::write(&temp_path, contents)?;
        fs::rename(temp_path, path)?;
        Ok(())
    }

    fn from_snapshot(snapshot: StoreSnapshot) -> io::Result<Self> {
        let mut store = Self {
            accounts: snapshot.accounts,
            queues: snapshot.queues,
            archived_queues: snapshot.archived_queues,
            groups: snapshot.groups,
            admin_sessions: snapshot.admin_sessions,
            user_sessions: snapshot.user_sessions,
            ..Self::default()
        };
        store.prune_invalid_sessions();
        store.hash_legacy_plaintext_passwords()?;
        store.rebuild_indexes();
        Ok(store)
    }

    fn prune_invalid_sessions(&mut self) {
        self.admin_sessions
            .retain(|_, session| self.accounts.contains_key(&session.account_id));
        self.user_sessions
            .retain(|_, session| self.accounts.contains_key(&session.account_id));
    }

    fn hash_legacy_plaintext_passwords(&mut self) -> io::Result<()> {
        for account in self.accounts.values_mut() {
            if !is_password_hash(&account.password_hash) {
                account.password_hash = hash_password(&account.password_hash).map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("failed to migrate plaintext password: {error}"),
                    )
                })?;
            }
        }
        Ok(())
    }

    fn rebuild_indexes(&mut self) {
        self.account_email_index = self
            .accounts
            .iter()
            .map(|(id, account)| (account.email.clone(), *id))
            .collect();

        self.entry_index.clear();
        for (queue_id, queue) in &self.queues {
            for entry in &queue.entries {
                self.entry_index.insert(entry.id, *queue_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::fs;

    use shared::{AccountRole, QueueEntryStatus, QueueField};

    use super::*;
    use crate::model::{AdminSession, QueueEntry};

    #[test]
    fn store_round_trips_persisted_data_and_rebuilds_indexes() {
        let temp_dir = std::env::temp_dir().join(format!("lue-store-test-{}", Uuid::new_v4()));
        let data_path = temp_dir.join("store.json");

        let account_id = Uuid::new_v4();
        let queue_id = Uuid::new_v4();
        let entry_id = Uuid::new_v4();
        let mut values = BTreeMap::new();
        values.insert("name".to_string(), "Ada".to_string());

        let mut store = Store {
            accounts: HashMap::from([(
                account_id,
                Account {
                    id: account_id,
                    name: "Admin".to_string(),
                    email: "admin@example.com".to_string(),
                    password_hash: "pass".to_string(),
                    role: AccountRole::Admin,
                },
            )]),
            queues: HashMap::from([(
                queue_id,
                Queue {
                    id: queue_id,
                    name: "Support".to_string(),
                    allow_guests: true,
                    is_public: false,
                    opens_at: None,
                    weekly_schedule: None,
                    owner_account_id: account_id,
                    owner_name: "Admin".to_string(),
                    shared_account_ids: Vec::new(),
                    shared_group_ids: Vec::new(),
                    fields: vec![QueueField {
                        key: "name".to_string(),
                        label: "Name".to_string(),
                        required: true,
                        options: Vec::new(),
                    }],
                    entries: vec![QueueEntry {
                        id: entry_id,
                        token: "entry-token".to_string(),
                        requester_account_id: None,
                        requester_label: "Ada".to_string(),
                        requester_email: None,
                        is_guest: true,
                        values,
                        submitted_at: "2026-05-04T00:00:00Z".to_string(),
                        left_at: None,
                        status: QueueEntryStatus::Resolved,
                        claimed_by: Some("Admin".to_string()),
                    }],
                },
            )]),
            admin_sessions: HashMap::from([(
                "admin-token".to_string(),
                AdminSession {
                    token: "admin-token".to_string(),
                    account_id,
                },
            )]),
            ..Store::default()
        };
        store.account_email_index.clear();
        store.entry_index.clear();

        store.save_to_disk(&data_path).expect("save store");
        let loaded = Store::load_from_disk(&data_path).expect("load store");

        assert_eq!(
            loaded.account_email_index.get("admin@example.com"),
            Some(&account_id)
        );
        assert_eq!(loaded.entry_index.get(&entry_id), Some(&queue_id));
        assert_eq!(
            loaded
                .queues
                .get(&queue_id)
                .and_then(|queue| queue.entries.first())
                .map(|entry| &entry.status),
            Some(&QueueEntryStatus::Resolved)
        );
        assert_ne!(
            loaded
                .accounts
                .get(&account_id)
                .map(|account| account.password_hash.as_str()),
            Some("pass")
        );
        assert!(loaded
            .accounts
            .get(&account_id)
            .is_some_and(|account| is_password_hash(&account.password_hash)));
        assert!(loaded.admin_state("admin-token", Some(queue_id)).is_some());

        let _ = fs::remove_dir_all(temp_dir);
    }
}
