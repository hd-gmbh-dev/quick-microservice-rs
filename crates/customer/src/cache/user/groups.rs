use std::collections::HashMap;

use std::sync::Arc;

use crate::cache::user::Realm;
use qm_pg::DB;

use crate::cache::update::{Op, Payload};
use crate::cache::{Group, GroupIdMap, GroupMap, KeycloakGroupUpdate};
use crate::query::fetch_groups;

#[derive(Default)]
pub struct Groups {
    group_id_map: GroupIdMap,
    group_name_map: GroupMap,
}

impl Groups {
    pub async fn new(db: &DB, realm: &str) -> anyhow::Result<Self> {
        let group_id_map: GroupIdMap = fetch_groups(db, realm).await?.into_iter().fold(
            GroupIdMap::default(),
            |mut state, row| {
                if let Some((id, name)) = row.id.zip(row.name) {
                    let name: Arc<str> = Arc::from(name);
                    let id: Arc<str> = Arc::from(id);
                    state.entry(id.clone()).or_insert_with(|| {
                        Arc::new(Group {
                            id,
                            parent_group: row.parent_group.map(Arc::from),
                            name,
                        })
                    });
                }
                state
            },
        );
        let group_name_map =
            group_id_map
                .values()
                .fold(GroupMap::default(), |mut state, current| {
                    if let Some(parent) = current
                        .parent_group
                        .as_ref()
                        .and_then(|id| group_id_map.get(id))
                    {
                        let e = state.entry(parent.name.clone()).or_default();
                        e.entry(current.name.clone()).or_insert(current.clone());
                    } else {
                        state.entry(current.name.clone()).or_default();
                    }
                    state
                });
        Ok(Self {
            group_id_map,
            group_name_map,
        })
    }

    pub fn contains(&self, group_id: &str) -> bool {
        self.group_id_map.contains_key(group_id)
    }

    pub fn get(&self, group_id: &str) -> Option<&Arc<Group>> {
        self.group_id_map.get(group_id)
    }

    pub fn by_parent(&self, name: &str) -> Option<&HashMap<Arc<str>, Arc<Group>>> {
        self.group_name_map.get(name)
    }

    pub fn total(&self) -> i64 {
        self.group_id_map.len() as i64
    }

    pub fn update(&mut self, realm: &Realm, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<KeycloakGroupUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                if realm.equals(new.realm_id.as_deref()) {
                    let group = Arc::new(Group {
                        id: new.id,
                        parent_group: new.parent_group,
                        name: new.name,
                    });
                    self.group_id_map.insert(group.id.clone(), group.clone());
                    if let Some(parent) = group
                        .parent_group
                        .as_ref()
                        .and_then(|id| self.group_id_map.get(id))
                    {
                        let e = self.group_name_map.entry(parent.name.clone()).or_default();
                        e.insert(group.name.clone(), group);
                    } else if group.parent_group.is_none() {
                        self.group_name_map
                            .insert(group.name.clone(), HashMap::default());
                    }
                }
            }
            (Op::Delete, None, Some(old)) => {
                if realm.equals(old.realm_id.as_deref()) {
                    if let Some(parent) = old
                        .parent_group
                        .as_ref()
                        .and_then(|id| self.group_id_map.get(id))
                    {
                        let e = self.group_name_map.entry(parent.name.clone()).or_default();
                        e.remove(&old.name);
                        if e.is_empty() {
                            self.group_name_map.remove(&parent.name);
                        }
                    } else if old.parent_group.is_none() {
                        self.group_name_map.remove(&old.name);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
