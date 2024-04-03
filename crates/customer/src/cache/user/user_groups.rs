use std::{collections::HashSet, sync::Arc};

use qm_pg::DB;

use crate::cache::{
    update::{Op, Payload},
    UserGroupMap, UserGroupMembershipUpdate,
};
use crate::query::fetch_user_groups;

use super::{groups::Groups, users::Users};

pub struct UserGroups {
    user_id_group_map: UserGroupMap,
    group_id_user_map: UserGroupMap,
}

impl UserGroups {
    pub async fn new(db: &DB, realm: &str) -> anyhow::Result<Self> {
        let (user_id_group_map, group_id_user_map) = fetch_user_groups(db, realm)
            .await?
            .into_iter()
            .filter(|row| row.has_all_fields())
            .fold(
                (UserGroupMap::default(), UserGroupMap::default()),
                |mut state, row| {
                    let user_id: Arc<str> = Arc::from(row.user_id.unwrap());
                    let group_id: Arc<str> = Arc::from(row.group_id.unwrap());
                    let e = state.0.entry(user_id.clone()).or_default();
                    e.insert(group_id.clone());
                    let e = state.1.entry(group_id).or_default();
                    e.insert(user_id);
                    state
                },
            );

        Ok(Self {
            user_id_group_map,
            group_id_user_map,
        })
    }

    pub fn by_user_id(&self, user_id: &str) -> Option<&HashSet<Arc<str>>> {
        self.user_id_group_map.get(user_id)
    }

    pub fn update(
        &mut self,
        users: &Users,
        groups: &Groups,
        payload: &str,
    ) -> anyhow::Result<bool> {
        let payload: Payload<UserGroupMembershipUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                if users.contains(&new.user_id) && groups.contains(&new.group_id) {
                    self.user_id_group_map
                        .entry(new.user_id.clone())
                        .or_default()
                        .insert(new.group_id.clone());
                    self.group_id_user_map
                        .entry(new.group_id)
                        .or_default()
                        .insert(new.user_id);
                    return Ok(true);
                }
            }
            (Op::Delete, None, Some(old)) => {
                if users.contains(&old.user_id) && groups.contains(&old.group_id) {
                    let e = self
                        .user_id_group_map
                        .entry(old.user_id.clone())
                        .or_default();
                    e.remove(&old.group_id);
                    if self.user_id_group_map.is_empty() {
                        self.user_id_group_map.remove(&old.user_id);
                    }
                    self.group_id_user_map
                        .entry(old.group_id.clone())
                        .or_default()
                        .insert(old.user_id);
                    if self.group_id_user_map.is_empty() {
                        self.group_id_user_map.remove(&old.group_id);
                    }
                    return Ok(true);
                }
            }
            _ => {}
        }
        Ok(false)
    }
}
