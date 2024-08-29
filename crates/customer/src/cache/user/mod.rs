use std::sync::{atomic::AtomicI64, Arc};

use prometheus_client::metrics::gauge::Gauge;
use qm_keycloak::RoleRepresentation;
use sqlx::postgres::PgListener;
use tokio::sync::RwLock;

use qm_pg::DB;

use self::{
    group_attributes::GroupAttributes, group_roles::GroupRoles, groups::Groups, realm::Realm,
    roles::Roles, user_groups::UserGroups, user_roles::UserRoles, users::Users,
};

use super::{Group, GroupDetail, User};

pub mod group_attributes;
pub mod group_roles;
pub mod groups;
pub mod realm;
pub mod roles;
pub mod user_groups;
pub mod user_roles;
pub mod users;

pub struct UserDB {
    pub realm: RwLock<Realm>,
    pub roles: RwLock<Roles>,
    pub groups: RwLock<Groups>,
    pub group_attributes: RwLock<GroupAttributes>,
    pub user_groups: RwLock<UserGroups>,
    pub user_roles: RwLock<UserRoles>,
    pub group_roles: RwLock<GroupRoles>,
    pub users: RwLock<Users>,
    pub users_total: Gauge<i64, AtomicI64>,
    pub groups_total: Gauge<i64, AtomicI64>,
    pub roles_total: Gauge<i64, AtomicI64>,
}

impl UserDB {
    pub async fn new(
        db: &DB,
        realm_name: &str,
        realm_admin_username: &str,
    ) -> anyhow::Result<Self> {
        let mut migrator = sqlx::migrate!("./migrations/keycloak");
        migrator.set_ignore_missing(true);
        migrator.run(db.pool()).await?;
        let realm = RwLock::new(Realm::new(db, realm_name).await?);
        let roles = RwLock::new(Roles::new(db, realm_name).await?);
        let groups = RwLock::new(Groups::new(db, realm_name).await?);
        let group_attributes = RwLock::new(GroupAttributes::new(db, realm_name).await?);
        let user_groups = RwLock::new(UserGroups::new(db, realm_name).await?);
        let user_roles = RwLock::new(UserRoles::new(db, realm_name).await?);
        let group_roles = RwLock::new(GroupRoles::new(db, realm_name).await?);
        let users = RwLock::new(Users::new(db, realm_name, realm_admin_username).await?);
        let users_total = Gauge::default();
        users_total.set(users.read().await.total());
        let groups_total = Gauge::default();
        groups_total.set(groups.read().await.total());
        let roles_total = Gauge::default();
        roles_total.set(roles.read().await.total());
        Ok(Self {
            realm,
            roles,
            groups,
            group_attributes,
            user_groups,
            user_roles,
            group_roles,
            users,
            users_total,
            groups_total,
            roles_total,
        })
    }

    pub async fn new_roles(&self, roles: Vec<RoleRepresentation>) {
        self.roles.write().await.new_roles(roles);
        self.roles_total.set(self.roles.read().await.total());
    }

    pub async fn new_group(
        &self,
        group: Arc<Group>,
        parent_name: Arc<str>,
        group_detail: Arc<GroupDetail>,
    ) {
        self.group_attributes
            .write()
            .await
            .new_group(group.id.clone(), group_detail);
        self.groups.write().await.new_group(group, parent_name);
        self.groups_total.set(self.groups.read().await.total());
    }

    pub async fn new_user(&self, user: Arc<User>) {
        self.users.write().await.new_user(user);
        self.users_total.set(self.users.read().await.total());
    }

    pub async fn cleanup(db: &DB) -> anyhow::Result<()> {
        let mut migrator = sqlx::migrate!("./migrations/keycloak");
        migrator.set_ignore_missing(true);
        migrator.undo(db.pool(), 0).await?;
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
                "group_attribute_update",
            ])
            .await?;

        while let Some(notification) = listener.try_recv().await? {
            match notification.channel() {
                "realm_update" => {
                    self.realm.write().await.update(notification.payload())?;
                }
                "user_entity_update" => {
                    let realm = self.realm.read().await;
                    self.users
                        .write()
                        .await
                        .update(&realm, notification.payload())?;
                    self.users_total.set(self.users.read().await.total());
                }
                "keycloak_role_update" => {
                    let realm = self.realm.read().await;
                    self.roles
                        .write()
                        .await
                        .update(&realm, notification.payload())?;
                    self.roles_total.set(self.roles.read().await.total());
                }
                "keycloak_group_update" => {
                    let realm = self.realm.read().await;
                    self.groups
                        .write()
                        .await
                        .update(&realm, notification.payload())?;
                    self.groups_total.set(self.groups.read().await.total());
                }
                "group_attribute_update" => {
                    let groups = self.groups.read().await;
                    self.group_attributes
                        .write()
                        .await
                        .update(&groups, notification.payload())?;
                }
                "user_role_mapping_update" => {
                    let users = self.users.read().await;
                    let roles = self.roles.read().await;
                    self.user_roles
                        .write()
                        .await
                        .update(&users, &roles, notification.payload())?;
                }
                "user_group_membership_update" => {
                    let users = self.users.read().await;
                    let groups = self.groups.read().await;
                    self.user_groups.write().await.update(
                        &users,
                        &groups,
                        notification.payload(),
                    )?;
                }
                "group_role_mapping_update" => {
                    let roles = self.roles.read().await;
                    let groups = self.groups.read().await;
                    self.group_roles.write().await.update(
                        &groups,
                        &roles,
                        notification.payload(),
                    )?;
                }
                _ => {}
            }
        }
        tracing::error!("postgresql listener disconnected");
        std::process::exit(1);
    }
}
