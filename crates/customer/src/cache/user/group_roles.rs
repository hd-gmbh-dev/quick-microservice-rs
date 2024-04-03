use std::{collections::HashSet, sync::Arc};

use qm_pg::DB;

use super::{groups::Groups, roles::Roles};
use crate::{
    cache::{
        update::{Op, Payload},
        GroupRoleMap, GroupRoleMappingUpdate, UserRoleMap,
    },
    query::fetch_group_roles,
};

pub struct GroupRoles {
    group_id_role_map: UserRoleMap,
    role_id_group_map: UserRoleMap,
}

impl GroupRoles {
    pub async fn new(db: &DB, realm: &str) -> anyhow::Result<Self> {
        let (group_id_role_map, role_id_group_map) = fetch_group_roles(db, realm)
            .await?
            .into_iter()
            .filter(|row| row.has_all_fields())
            .fold(
                (GroupRoleMap::default(), GroupRoleMap::default()),
                |mut state, row| {
                    let group_id: Arc<str> = Arc::from(row.group_id.unwrap());
                    let role_id: Arc<str> = Arc::from(row.role_id.unwrap());
                    let e = state.0.entry(group_id.clone()).or_default();
                    e.insert(role_id.clone());
                    let e = state.1.entry(role_id).or_default();
                    e.insert(group_id);
                    state
                },
            );

        Ok(Self {
            group_id_role_map,
            role_id_group_map,
        })
    }

    pub fn by_group_id(&self, group_id: &str) -> Option<&HashSet<Arc<str>>> {
        self.group_id_role_map.get(group_id)
    }

    pub fn update(
        &mut self,
        groups: &Groups,
        roles: &Roles,
        payload: &str,
    ) -> anyhow::Result<bool> {
        let payload: Payload<GroupRoleMappingUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                if groups.contains(&new.group_id) && roles.contains(&new.role_id) {
                    self.group_id_role_map
                        .entry(new.group_id.clone())
                        .or_default()
                        .insert(new.role_id.clone());
                    self.role_id_group_map
                        .entry(new.role_id)
                        .or_default()
                        .insert(new.group_id);
                    return Ok(true);
                }
            }
            (Op::Delete, None, Some(old)) => {
                if groups.contains(&old.group_id) && roles.contains(&old.role_id) {
                    let e = self
                        .group_id_role_map
                        .entry(old.group_id.clone())
                        .or_default();
                    e.remove(&old.role_id);
                    if self.group_id_role_map.is_empty() {
                        self.group_id_role_map.remove(&old.group_id);
                    }
                    self.role_id_group_map
                        .entry(old.role_id.clone())
                        .or_default()
                        .insert(old.group_id);
                    if self.role_id_group_map.is_empty() {
                        self.role_id_group_map.remove(&old.role_id);
                    }
                    return Ok(true);
                }
            }
            _ => {}
        }
        Ok(false)
    }
}
