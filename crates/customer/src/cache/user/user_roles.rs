use std::{collections::HashSet, sync::Arc};

use qm_pg::DB;

use crate::cache::{
    update::{Op, Payload},
    UserRoleMap, UserRoleMappingUpdate,
};
use crate::query::fetch_user_roles;

use super::{roles::Roles, users::Users};

pub struct UserRoles {
    user_id_role_map: UserRoleMap,
    role_id_user_map: UserRoleMap,
}

impl UserRoles {
    pub async fn new(db: &DB, realm: &str) -> anyhow::Result<Self> {
        let (user_id_role_map, role_id_user_map) = fetch_user_roles(db, realm)
            .await?
            .into_iter()
            .filter(|row| row.has_all_fields())
            .fold(
                (UserRoleMap::default(), UserRoleMap::default()),
                |mut state, row| {
                    let user_id: Arc<str> = Arc::from(row.user_id.unwrap());
                    let role_id: Arc<str> = Arc::from(row.role_id.unwrap());
                    let e = state.0.entry(user_id.clone()).or_default();
                    e.insert(role_id.clone());
                    let e = state.1.entry(role_id).or_default();
                    e.insert(user_id);
                    state
                },
            );

        Ok(Self {
            user_id_role_map,
            role_id_user_map,
        })
    }

    pub fn by_user_id(&self, user_id: &str) -> Option<&HashSet<Arc<str>>> {
        self.user_id_role_map.get(user_id)
    }

    pub fn update(&mut self, users: &Users, roles: &Roles, payload: &str) -> anyhow::Result<bool> {
        let payload: Payload<UserRoleMappingUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                if users.contains(&new.user_id) && roles.contains(&new.role_id) {
                    self.user_id_role_map
                        .entry(new.user_id.clone())
                        .or_default()
                        .insert(new.role_id.clone());
                    self.role_id_user_map
                        .entry(new.role_id)
                        .or_default()
                        .insert(new.user_id);
                    return Ok(true);
                }
            }
            (Op::Delete, None, Some(old)) => {
                if users.contains(&old.user_id) && roles.contains(&old.role_id) {
                    let e = self
                        .user_id_role_map
                        .entry(old.user_id.clone())
                        .or_default();
                    e.remove(&old.role_id);
                    if self.user_id_role_map.is_empty() {
                        self.user_id_role_map.remove(&old.user_id);
                    }
                    self.role_id_user_map
                        .entry(old.role_id.clone())
                        .or_default()
                        .insert(old.user_id);
                    if self.role_id_user_map.is_empty() {
                        self.role_id_user_map.remove(&old.role_id);
                    }
                    return Ok(true);
                }
            }
            _ => {}
        }
        Ok(false)
    }
}
