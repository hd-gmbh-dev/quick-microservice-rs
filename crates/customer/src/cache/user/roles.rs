use std::sync::Arc;

use crate::cache::user::Realm;
use qm_entity::ids::InfraContext;
use qm_keycloak::RoleRepresentation;
use qm_pg::DB;

use crate::{
    cache::{
        update::{Op, Payload},
        KeycloakRoleUpdate, Role, RoleIdMap, RoleMap,
    },
    query::fetch_roles,
};

fn parse_context(name: &str) -> Option<InfraContext> {
    if let Some((_, id)) = name.rsplit_once("access@") {
        return id.parse().ok();
    }
    None
}

pub struct Roles {
    role_name_map: RoleMap,
    role_id_map: RoleIdMap,
}

impl Roles {
    pub async fn new(db: &DB, realm: &str) -> anyhow::Result<Self> {
        let role_id_map = fetch_roles(db, realm).await?.into_iter().fold(
            RoleIdMap::default(),
            |mut state, row| {
                if let Some((id, name)) = row.role_id.zip(row.role_name) {
                    let name: Arc<str> = Arc::from(name);
                    let id: Arc<str> = Arc::from(id);
                    state.entry(id.clone()).or_insert_with(|| {
                        Arc::new(Role {
                            context: parse_context(&name),
                            id,
                            name,
                        })
                    });
                }
                state
            },
        );
        let role_name_map =
            RoleMap::from_iter(role_id_map.values().map(|v| (v.name.clone(), v.clone())));

        Ok(Self {
            role_id_map,
            role_name_map,
        })
    }

    pub fn total(&self) -> i64 {
        self.role_id_map.len() as i64
    }

    pub fn list(&self) -> Arc<[Arc<Role>]> {
        self.role_id_map.values().cloned().collect()
    }

    pub fn new_roles(&mut self, roles: Vec<RoleRepresentation>) {
        for role in roles {
            if let Some((id, name)) = role.id.zip(role.name) {
                let id = Arc::from(id);
                let name = Arc::from(name);
                let role = Arc::new(Role {
                    context: parse_context(&name),
                    id,
                    name,
                });
                self.role_id_map.insert(role.id.clone(), role.clone());
                self.role_name_map.insert(role.name.clone(), role);
            }
        }
    }

    pub fn contains(&self, role_id: &str) -> bool {
        self.role_id_map.contains_key(role_id)
    }

    pub fn get(&self, role_id: &str) -> Option<&Arc<Role>> {
        self.role_id_map.get(role_id)
    }

    pub fn by_name(&self, name: &str) -> Option<&Arc<Role>> {
        self.role_name_map.get(name)
    }

    pub fn update(&mut self, realm: &Realm, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<KeycloakRoleUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                if realm.equals(new.realm_id.as_deref()) {
                    let role = Arc::new(Role {
                        id: new.id.clone(),
                        name: new.name.clone(),
                        context: parse_context(&new.name),
                    });
                    self.role_id_map.insert(role.id.clone(), role.clone());
                    self.role_name_map.insert(role.name.clone(), role);
                }
            }
            (Op::Delete, None, Some(old)) => {
                if realm.equals(old.realm_id.as_deref()) {
                    self.role_id_map.remove(&old.id);
                    self.role_name_map.remove(&old.name);
                }
            }
            _ => {}
        }
        Ok(())
    }
}
