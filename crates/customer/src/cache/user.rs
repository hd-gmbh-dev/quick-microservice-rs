use super::update::Op;
use super::update::Payload;
use crate::model::*;
use crate::query::*;
use prometheus_client::metrics::gauge::Gauge;
use qm_entity::ids::InfraContext;
use qm_keycloak::RoleRepresentation;
use qm_pg::DB;
use sqlx::postgres::PgListener;
use std::collections::HashMap;
use std::sync::atomic::AtomicI64;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::RwLock;
use tokio::sync::RwLockReadGuard;

fn find_context_from_roles(roles: &[Arc<Role>]) -> Option<InfraContext> {
    roles.as_ref().iter().find_map(|r| {
        if let Some((_, id)) = r.name.rsplit_once("access@") {
            InfraContext::parse(id).ok()
        } else {
            None
        }
    })
}

pub struct UserDB {
    pub realm_name: Arc<str>,
    pub realm_id: RwLock<Option<Arc<str>>>,
    pub users: RwLock<UserMap>,
    pub user_id_map: RwLock<UserMap>,
    pub user_email_map: RwLock<UserMap>,
    pub roles: RwLock<RoleMap>,
    pub role_id_map: RwLock<RoleIdMap>,
    pub groups: RwLock<GroupMap>,
    pub group_id_map: RwLock<GroupIdMap>,
    pub users_total: Gauge<i64, AtomicI64>,
    pub roles_total: Gauge<i64, AtomicI64>,
    pub groups_total: Gauge<i64, AtomicI64>,
}

impl UserDB {
    pub async fn cleanup(db: &DB) -> anyhow::Result<()> {
        let mut migrator = sqlx::migrate!("./migrations/keycloak");
        migrator.set_ignore_missing(true);
        migrator.undo(db.pool(), 0).await?;
        Ok(())
    }

    pub async fn new(db: &DB, realm: &str) -> anyhow::Result<Self> {
        let users_total = Gauge::default();
        let roles_total = Gauge::default();
        let groups_total = Gauge::default();
        let mut migrator = sqlx::migrate!("./migrations/keycloak");
        migrator.set_ignore_missing(true);
        migrator.run(db.pool()).await?;

        let result = Self {
            realm_name: Arc::from(realm.to_string()),
            realm_id: Default::default(),
            users: Default::default(),
            user_id_map: Default::default(),
            user_email_map: Default::default(),
            roles: Default::default(),
            role_id_map: Default::default(),
            groups: Default::default(),
            group_id_map: Default::default(),
            users_total,
            roles_total,
            groups_total,
        };
        Ok(result)
    }

    pub async fn fetch_groups(&self, db: &DB) -> anyhow::Result<()> {
        let group_id_map: GroupIdMap = fetch_groups(db, &self.realm_name).await?.into_iter().fold(
            GroupIdMap::default(),
            |mut state, row| {
                if let Some((id, name)) = row.group_id.zip(row.group_name) {
                    let name: Arc<str> = Arc::from(format!("/{name}"));
                    let id: Arc<str> = Arc::from(id);
                    state
                        .entry(id.clone())
                        .or_insert_with(|| Arc::new(Group { id, name }));
                }
                state
            },
        );
        let groups = GroupMap::from_iter(
            group_id_map
                .clone()
                .into_values()
                .map(|v| (v.name.clone(), v)),
        );
        *self.group_id_map.write().await = group_id_map;
        *self.groups.write().await = groups;
        self.groups_total.set(self.groups.read().await.len() as i64);
        Ok(())
    }

    pub async fn fetch_roles(&self, db: &DB) -> anyhow::Result<()> {
        let role_id_map: RoleIdMap = fetch_roles(db, &self.realm_name).await?.into_iter().fold(
            RoleIdMap::default(),
            |mut state, row| {
                if let Some((id, name)) = row.role_id.zip(row.role_name) {
                    let name: Arc<str> = Arc::from(name);
                    let id: Arc<str> = Arc::from(id);
                    state
                        .entry(id.clone())
                        .or_insert_with(|| Arc::new(Role { id, name }));
                }
                state
            },
        );
        let roles = RoleMap::from_iter(
            role_id_map
                .clone()
                .into_values()
                .map(|v| (v.name.clone(), v)),
        );
        *self.role_id_map.write().await = role_id_map;
        *self.roles.write().await = roles;
        self.roles_total.set(self.roles.read().await.len() as i64);
        Ok(())
    }

    pub async fn fetch_users(&self, db: &DB) -> anyhow::Result<()> {
        let tmp_users = fetch_users(db, &self.realm_name)
            .await?
            .into_iter()
            .filter(|row| row.has_all_fields())
            .fold(TmpUserMap::default(), |mut state, row| {
                let user_id: Arc<str> = Arc::from(row.user_id.unwrap());
                let group_id: Arc<str> = Arc::from(row.group_id.unwrap());
                let role_id: Arc<str> = Arc::from(row.role_id.unwrap());
                let firstname: Arc<str> = Arc::from(row.firstname.unwrap());
                let lastname: Arc<str> = Arc::from(row.lastname.unwrap());
                let username: Arc<str> = Arc::from(row.username.unwrap());
                let email: Arc<str> = Arc::from(row.email.unwrap());
                let e = state.entry(username.clone()).or_insert(TmpUser {
                    id: user_id,
                    username,
                    email,
                    firstname,
                    lastname,
                    groups: HashSet::default(),
                    roles: HashSet::default(),
                    enabled: row.enabled,
                });
                e.groups.insert(group_id);
                e.roles.insert(role_id);
                state
            });
        let group_id_map = self.group_id_map.read().await;
        let role_id_map = self.role_id_map.read().await;
        let users = UserMap::from_iter(tmp_users.into_values().map(|v| {
            let roles: Arc<[Arc<Role>]> = v
                .roles
                .into_iter()
                .filter_map(|role_id| role_id_map.get(&role_id).cloned())
                .collect();
            let context = find_context_from_roles(&roles);
            (
                v.username.clone(),
                Arc::new(User {
                    id: v.id,
                    username: v.username,
                    email: v.email,
                    firstname: v.firstname,
                    lastname: v.lastname,
                    groups: v
                        .groups
                        .into_iter()
                        .filter_map(|group_id| group_id_map.get(&group_id).cloned())
                        .collect(),
                    roles,
                    enabled: v.enabled,
                    context,
                }),
            )
        }));
        let user_id_map = UserMap::from_iter(users.values().map(|v| (v.id.clone(), v.clone())));
        let user_email_map =
            UserMap::from_iter(users.values().map(|v| (v.email.clone(), v.clone())));
        *self.users.write().await = users;
        *self.user_id_map.write().await = user_id_map;
        *self.user_email_map.write().await = user_email_map;
        self.users_total.set(self.users.read().await.len() as i64);
        Ok(())
    }

    pub async fn fetch_realm_info(&self, db: &DB) -> anyhow::Result<()> {
        let realm_query = fetch_realm_info(db, &self.realm_name).await?;
        if let Some(KcRealmQuery { id: Some(realm_id) }) = realm_query {
            self.realm_id.write().await.replace(Arc::from(realm_id));
        }
        Ok(())
    }

    pub async fn reload(&self, db: &DB) -> anyhow::Result<()> {
        self.fetch_realm_info(db).await?;
        self.fetch_groups(db).await?;
        self.fetch_roles(db).await?;
        self.fetch_users(db).await?;
        Ok(())
    }

    pub async fn roles(&self) -> RwLockReadGuard<'_, RoleMap> {
        self.roles.read().await
    }

    pub async fn new_user(&self, user: Arc<User>) {
        self.users
            .write()
            .await
            .insert(user.username.clone(), user.clone());
        self.user_id_map
            .write()
            .await
            .insert(user.id.clone(), user.clone());
        if !user.email.is_empty() {
            self.user_email_map
                .write()
                .await
                .insert(user.email.clone(), user.clone());
        }
    }

    pub async fn new_roles(&self, roles: Vec<RoleRepresentation>) -> anyhow::Result<()> {
        for role in roles {
            if let Some((name, id)) = role.name.zip(role.id) {
                let role = Arc::new(Role {
                    name: Arc::from(name),
                    id: Arc::from(id),
                });
                self.role_id_map
                    .write()
                    .await
                    .insert(role.id.clone(), role.clone());
                self.roles.write().await.insert(role.name.clone(), role);
            }
        }
        Ok(())
    }

    pub async fn listen(&self, db: &DB) -> anyhow::Result<()> {
        let mut listener = PgListener::connect_with(db.pool()).await?;
        listener
            .listen_all([
                "realm_update",
                "user_entity_update",
                "keycloak_role_update",
                "keycloak_group_update",
                "user_role_mapping_update",
                "user_group_membership_update",
            ])
            .await?;

        while let Some(notification) = listener.try_recv().await? {
            match notification.channel() {
                "realm_update" => {
                    self.realm_update(notification.payload()).await?;
                }
                "user_entity_update" => {
                    self.user_entity_update(notification.payload(), db).await?;
                }
                "keycloak_role_update" => {
                    self.keycloak_role_update(notification.payload()).await?;
                }
                "keycloak_group_update" => {
                    self.keycloak_group_update(notification.payload()).await?;
                }
                "user_role_mapping_update" => {
                    self.user_role_mapping_update(notification.payload())
                        .await?;
                }
                "user_group_membership_update" => {
                    self.user_group_membership_update(notification.payload())
                        .await?;
                }
                _ => {}
            }
        }
        log::error!("postgresql listener disconnected");
        std::process::exit(1);
    }

    async fn realm_update(&self, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<RealmUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                self.realm_id.write().await.replace(new.id);
            }
            (Op::Delete, None, Some(old)) => {
                if old.name.as_ref() == self.realm_name.as_ref() {
                    self.realm_id.write().await.take();
                    self.users.write().await.clear();
                    self.roles.write().await.clear();
                    self.role_id_map.write().await.clear();
                    self.groups.write().await.clear();
                    self.group_id_map.write().await.clear();
                }
            }
            _ => {}
        };
        Ok(())
    }

    async fn user_entity_update(&self, payload: &str, db: &DB) -> anyhow::Result<()> {
        log::info!("user_entity: {payload:#?}");
        let payload: Payload<UserEntityUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Update, Some(new), _) | (Op::Insert, Some(new), _) => {
                let is_same_realm = async {
                    self.realm_id
                        .read()
                        .await
                        .as_ref()
                        .zip(new.realm_id)
                        .map(|(my_realm_id, realm_id)| my_realm_id.as_ref() == realm_id.as_ref())
                        .unwrap_or(false)
                }
                .await;
                if is_same_realm {
                    let user_roles = fetch_user_roles(db, &new.id).await?;
                    let roles = async {
                        let existing_roles = self.role_id_map.read().await;
                        let roles: Arc<[Arc<Role>]> = user_roles
                            .into_iter()
                            .filter_map(|role| {
                                if let Some(role_id) = role.role_id.map(Arc::from).as_ref() {
                                    existing_roles.get(role_id).cloned()
                                } else {
                                    None
                                }
                            })
                            .collect();
                        roles
                    }
                    .await;
                    let user_groups = fetch_user_groups(db, &new.id).await?;
                    let groups = async {
                        let existing_groups = self.group_id_map.read().await;
                        let groups: Arc<[Arc<Group>]> = user_groups
                            .into_iter()
                            .filter_map(|group| {
                                if let Some(group_id) = group.group_id.map(Arc::from).as_ref() {
                                    existing_groups.get(group_id).cloned()
                                } else {
                                    None
                                }
                            })
                            .collect();
                        groups
                    }
                    .await;
                    let context = find_context_from_roles(&roles);
                    let user = Arc::new(User {
                        id: new.id.clone(),
                        username: new.username.clone(),
                        email: new.email.unwrap_or_else(|| Arc::from("")),
                        firstname: new.first_name.unwrap_or_else(|| Arc::from("")),
                        lastname: new.last_name.unwrap_or_else(|| Arc::from("")),
                        groups,
                        roles,
                        enabled: new.enabled,
                        context,
                    });
                    self.new_user(user).await;
                }
            }
            (Op::Delete, None, Some(old)) => {
                self.user_id_map.write().await.remove(&old.id);
                self.users.write().await.remove(&old.username);
                if let Some(email) = old.email.as_ref() {
                    self.user_email_map.write().await.remove(email);
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn keycloak_role_update(&self, payload: &str) -> anyhow::Result<()> {
        log::info!("keycloak_role_update: {payload}");
        let payload: Payload<KeycloakRoleUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                let is_same_realm = async {
                    self.realm_id
                        .read()
                        .await
                        .as_ref()
                        .zip(new.realm_id)
                        .map(|(my_realm_id, realm_id)| my_realm_id.as_ref() == realm_id.as_ref())
                        .unwrap_or(false)
                }
                .await;
                if is_same_realm {
                    let role = Arc::new(Role {
                        id: new.id,
                        name: new.name,
                    });
                    self.roles
                        .write()
                        .await
                        .insert(role.name.clone(), role.clone());
                    self.role_id_map.write().await.insert(role.id.clone(), role);
                }
            }
            (Op::Delete, None, Some(old)) => {
                let is_same_realm = async {
                    self.realm_id
                        .read()
                        .await
                        .as_ref()
                        .zip(old.realm_id)
                        .map(|(my_realm_id, realm_id)| my_realm_id.as_ref() == realm_id.as_ref())
                        .unwrap_or(false)
                }
                .await;
                if is_same_realm {
                    self.roles.write().await.remove(&old.name);
                    self.role_id_map.write().await.remove(&old.id);
                    let updated_users = async {
                        let mut updated_users = Vec::default();
                        let users = self.users.read().await;
                        let outdated_users = users
                            .values()
                            .filter(|user| user.roles.iter().any(|r| r.name == old.name));
                        for user in outdated_users {
                            let next_roles: Arc<[Arc<Role>]> = user
                                .roles
                                .iter()
                                .filter(|r| r.name != old.name)
                                .cloned()
                                .collect();
                            let context = find_context_from_roles(&next_roles);
                            updated_users.push(Arc::new(User {
                                id: user.id.clone(),
                                username: user.username.clone(),
                                email: user.email.clone(),
                                firstname: user.firstname.clone(),
                                lastname: user.lastname.clone(),
                                groups: user.groups.clone(),
                                roles: next_roles,
                                enabled: user.enabled,
                                context,
                            }));
                        }
                        updated_users
                    }
                    .await;
                    for updated_user in updated_users {
                        self.new_user(updated_user).await;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn keycloak_group_update(&self, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<KeycloakGroupUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                let is_same_realm = async {
                    self.realm_id
                        .read()
                        .await
                        .as_ref()
                        .zip(new.realm_id)
                        .map(|(my_realm_id, realm_id)| my_realm_id.as_ref() == realm_id.as_ref())
                        .unwrap_or(false)
                }
                .await;
                if is_same_realm {
                    let group = Arc::new(Group {
                        id: new.id,
                        name: Arc::from(format!("/{}", new.name)),
                    });
                    self.groups
                        .write()
                        .await
                        .insert(group.name.clone(), group.clone());
                    self.group_id_map
                        .write()
                        .await
                        .insert(group.id.clone(), group);
                }
            }
            (Op::Delete, None, Some(old)) => {
                let is_same_realm = async {
                    self.realm_id
                        .read()
                        .await
                        .as_ref()
                        .zip(old.realm_id)
                        .map(|(my_realm_id, realm_id)| my_realm_id.as_ref() == realm_id.as_ref())
                        .unwrap_or(false)
                }
                .await;
                if is_same_realm {
                    let old_name: Arc<str> = Arc::from(format!("/{}", old.name));
                    self.groups.write().await.remove(&old_name);
                    self.group_id_map.write().await.remove(&old.id);
                    let updated_users = async {
                        let mut updated_users = Vec::default();
                        let users = self.users.read().await;
                        let outdated_users = users
                            .values()
                            .filter(|user| user.groups.iter().any(|r| r.name == old_name));
                        for user in outdated_users {
                            let next_groups = user
                                .groups
                                .iter()
                                .filter(|r| r.name != old_name)
                                .cloned()
                                .collect();
                            updated_users.push(Arc::new(User {
                                id: user.id.clone(),
                                username: user.username.clone(),
                                email: user.email.clone(),
                                firstname: user.firstname.clone(),
                                lastname: user.lastname.clone(),
                                roles: user.roles.clone(),
                                groups: next_groups,
                                enabled: user.enabled,
                                context: user.context,
                            }));
                        }
                        updated_users
                    }
                    .await;
                    for updated_user in updated_users {
                        self.new_user(updated_user).await;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn user_role_mapping_update(&self, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<UserRoleMappingUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Delete, None, Some(old)) => {
                let updated_users = async {
                    let mut updated_users = Vec::default();
                    let users = self.users.read().await;
                    let outdated_users = users
                        .values()
                        .filter(|user| user.id.as_ref() == old.user_id.as_ref());
                    for user in outdated_users {
                        let next_roles: Arc<[Arc<Role>]> = user
                            .roles
                            .iter()
                            .filter(|r| r.id != old.role_id)
                            .cloned()
                            .collect();
                        let context = find_context_from_roles(&next_roles);
                        updated_users.push(Arc::new(User {
                            id: user.id.clone(),
                            username: user.username.clone(),
                            email: user.email.clone(),
                            firstname: user.firstname.clone(),
                            lastname: user.lastname.clone(),
                            groups: user.groups.clone(),
                            roles: next_roles,
                            enabled: user.enabled,
                            context,
                        }));
                    }
                    updated_users
                }
                .await;
                for updated_user in updated_users {
                    self.new_user(updated_user).await;
                }
            }
            (Op::Insert, Some(new), None) => {
                let updated_users = async {
                    let mut updated_users = Vec::default();
                    let users = self.users.read().await;
                    let outdated_users = users
                        .values()
                        .filter(|user| user.id.as_ref() == new.user_id.as_ref());
                    for user in outdated_users {
                        let mut next_roles: HashMap<Arc<str>, Arc<Role>> = HashMap::from_iter(
                            user.roles.iter().map(|r| (r.id.clone(), r.clone())),
                        );
                        if let Some(role) = self.role_id_map.read().await.get(&new.role_id) {
                            next_roles.insert(role.id.clone(), role.clone());
                        }
                        let next_roles: Arc<[Arc<Role>]> = next_roles.values().cloned().collect();
                        let context = find_context_from_roles(&next_roles);
                        updated_users.push(Arc::new(User {
                            id: user.id.clone(),
                            username: user.username.clone(),
                            email: user.email.clone(),
                            firstname: user.firstname.clone(),
                            lastname: user.lastname.clone(),
                            groups: user.groups.clone(),
                            roles: next_roles,
                            enabled: user.enabled,
                            context,
                        }));
                    }
                    updated_users
                }
                .await;
                for updated_user in updated_users {
                    self.new_user(updated_user).await;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn user_group_membership_update(&self, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<UserGroupMembershipUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Delete, None, Some(old)) => {
                let updated_users = async {
                    let mut updated_users = Vec::default();
                    let users = self.users.read().await;
                    let outdated_users = users
                        .values()
                        .filter(|user| user.id.as_ref() == old.user_id.as_ref());
                    for user in outdated_users {
                        let next_groups: Arc<[Arc<Group>]> = user
                            .groups
                            .iter()
                            .filter(|r| r.id != old.group_id)
                            .cloned()
                            .collect();
                        let context = find_context_from_roles(&user.roles);
                        updated_users.push(Arc::new(User {
                            id: user.id.clone(),
                            username: user.username.clone(),
                            email: user.email.clone(),
                            firstname: user.firstname.clone(),
                            lastname: user.lastname.clone(),
                            groups: next_groups,
                            roles: user.roles.clone(),
                            enabled: user.enabled,
                            context,
                        }));
                    }
                    updated_users
                }
                .await;
                for updated_user in updated_users {
                    self.new_user(updated_user).await;
                }
            }
            (Op::Insert, Some(new), None) => {
                let updated_users = async {
                    let mut updated_users = Vec::default();
                    let users = self.users.read().await;
                    let outdated_users = users
                        .values()
                        .filter(|user| user.id.as_ref() == new.user_id.as_ref());
                    for user in outdated_users {
                        let mut next_groups: HashMap<Arc<str>, Arc<Group>> = HashMap::from_iter(
                            user.groups.iter().map(|r| (r.id.clone(), r.clone())),
                        );
                        if let Some(group) = self.group_id_map.read().await.get(&new.group_id) {
                            next_groups.insert(group.id.clone(), group.clone());
                        }
                        let next_groups: Arc<[Arc<Group>]> =
                            next_groups.values().cloned().collect();
                        updated_users.push(Arc::new(User {
                            id: user.id.clone(),
                            username: user.username.clone(),
                            email: user.email.clone(),
                            firstname: user.firstname.clone(),
                            lastname: user.lastname.clone(),
                            groups: next_groups,
                            roles: user.roles.clone(),
                            enabled: user.enabled,
                            context: user.context,
                        }));
                    }
                    updated_users
                }
                .await;
                for updated_user in updated_users {
                    self.new_user(updated_user).await;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
