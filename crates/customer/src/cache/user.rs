use futures::stream::TryStreamExt;
use prometheus_client::metrics::gauge::Gauge;
use qm_keycloak::GroupRepresentation;
use qm_keycloak::Keycloak;
use qm_keycloak::KeycloakError;
use qm_keycloak::RoleRepresentation;
use qm_keycloak::UserRepresentation;
use qm_mongodb::bson::doc;
use qm_mongodb::bson::Uuid;
use qm_mongodb::DB;
use qm_redis::redis::{AsyncCommands, Msg};
use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::model::User;
use crate::model::UserDetails;

pub type DbUserMap = BTreeMap<Arc<Uuid>, Arc<User>>;
pub type KeycloakUserMap = BTreeMap<Arc<Uuid>, Arc<UserDetails>>;
pub type KeycloakGroupMap = BTreeMap<Arc<str>, (Arc<str>, GroupRepresentation)>;
pub type KeycloakRoleMap = BTreeMap<Arc<str>, (Arc<str>, RoleRepresentation)>;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum UserCacheEventType {
    NewUser(Arc<UserDetails>, Arc<User>),
    DeleteUser(Arc<User>),
    NewRoles(Vec<RoleRepresentation>),
    ReloadUsers,
    ReloadRoles,
    ReloadGroups,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UserCacheEvent {
    publisher: Arc<Uuid>,
    event: UserCacheEventType,
}

struct UserCacheInner {
    realm: Arc<str>,
    channel: Arc<str>,
    id: Arc<Uuid>,
    db_users: Arc<RwLock<DbUserMap>>,
    keycloak_users: Arc<RwLock<KeycloakUserMap>>,
    users_total: Gauge<f64, AtomicU64>,
    roles: Arc<RwLock<KeycloakRoleMap>>,
    roles_total: Gauge<f64, AtomicU64>,
    groups: Arc<RwLock<KeycloakGroupMap>>,
    groups_total: Gauge<f64, AtomicU64>,
}

#[derive(Clone)]
pub struct UserCache {
    inner: Arc<UserCacheInner>,
}

fn ignore_404<T>(result: Result<T, KeycloakError>) -> anyhow::Result<T>
where
    T: Default,
{
    match result {
        Ok(t) => Ok(t),
        Err(KeycloakError::HttpFailure { status: 404, .. }) => Ok(T::default()),
        Err(e) => Err(e)?,
    }
}

async fn load_users(db: &DB) -> anyhow::Result<Vec<User>> {
    Ok(db
        .get()
        .collection(crate::schema::user::DEFAULT_COLLECTION)
        .find(doc! {}, None)
        .await?
        .try_collect()
        .await?)
}

async fn load_keycloak_users(
    realm: &str,
    keycloak: &Keycloak,
) -> anyhow::Result<Vec<UserRepresentation>> {
    ignore_404(keycloak.users(realm, None, None, None).await)
}

async fn load_user_maps(
    realm: &str,
    keycloak: &Keycloak,
    db: &DB,
) -> anyhow::Result<(DbUserMap, KeycloakUserMap)> {
    let db_users: DbUserMap = BTreeMap::from_iter(
        load_users(db)
            .await?
            .into_iter()
            .map(|u| (u.details.user_id.clone(), Arc::new(u))),
    );
    let keycloak_users = load_keycloak_users(realm, keycloak).await?;
    let keycloak_user_map: KeycloakUserMap =
        BTreeMap::from_iter(keycloak_users.into_iter().filter_map(|u| {
            let details: Option<UserDetails> = u.try_into().ok();
            details.map(|details| (details.user_id.clone(), Arc::new(details)))
        }));
    Ok((db_users, keycloak_user_map))
}

type ParsedRole = (Arc<str>, (Arc<str>, RoleRepresentation));

fn parse_role(r: &RoleRepresentation) -> Option<ParsedRole> {
    r.name.as_ref().zip(r.id.as_ref()).map(|(path, uid)| {
        (
            Arc::from(path.to_string()),
            (Arc::from(uid.to_string()), r.clone()),
        )
    })
}

async fn load_roles(db: &DB) -> anyhow::Result<KeycloakRoleMap> {
    let roles: Vec<RoleRepresentation> = db
        .get()
        .collection("roles")
        .find(doc! {}, None)
        .await?
        .try_collect()
        .await?;
    Ok(KeycloakRoleMap::from_iter(
        roles.iter().filter_map(parse_role),
    ))
}

impl UserCache {
    pub async fn new(prefix: &str, keycloak: &Keycloak, db: &DB) -> anyhow::Result<Self> {
        let realm = keycloak.config().realm();
        let (db_users, keycloak_users) = load_user_maps(realm, keycloak, db).await?;
        log::info!("loaded {} users", db_users.len());
        let users_total = Gauge::<f64, AtomicU64>::default();
        users_total.set(db_users.len() as f64);
        let groups = load_groups(realm, keycloak).await?;
        log::info!("loaded {} groups", groups.len());
        let groups_total = Gauge::<f64, AtomicU64>::default();
        groups_total.set(groups.len() as f64);
        let roles = load_roles(db).await?;
        log::info!("loaded {} roles", roles.len());
        let roles_total = Gauge::<f64, AtomicU64>::default();
        roles_total.set(roles.len() as f64);
        Ok(Self {
            inner: Arc::new(UserCacheInner {
                id: Arc::new(Uuid::new()),
                channel: Arc::from(format!("{prefix}_users")),
                realm: Arc::from(realm.to_string()),
                db_users: Arc::new(RwLock::new(db_users)),
                keycloak_users: Arc::new(RwLock::new(keycloak_users)),
                users_total,
                groups: Arc::new(RwLock::new(groups)),
                groups_total,
                roles: Arc::new(RwLock::new(roles)),
                roles_total,
            }),
        })
    }

    pub fn users_total(&self) -> &Gauge<f64, AtomicU64> {
        &self.inner.users_total
    }

    pub async fn db_user_by_id(&self, id: &Uuid) -> anyhow::Result<Option<Arc<User>>> {
        Ok(self.inner.db_users.read().await.get(id).cloned())
    }

    pub async fn db_user_by_uid(&self, id: &Uuid) -> Option<Arc<User>> {
        self.inner.db_users.read().await.get(id).cloned()
    }

    pub async fn user_by_uid(&self, id: &Uuid) -> Option<Arc<UserDetails>> {
        self.inner.keycloak_users.read().await.get(id).cloned()
    }

    pub async fn get_group_id(&self, path: &str) -> Option<Arc<str>> {
        self.inner
            .groups
            .read()
            .await
            .get(path)
            .map(|(id, _)| id)
            .cloned()
    }

    pub async fn reload_users(
        &self,
        keycloak: &Keycloak,
        db: &DB,
        redis: Option<&deadpool_redis::Pool>,
    ) -> anyhow::Result<()> {
        let (db_users, keycloak_users) =
            load_user_maps(self.inner.realm.as_ref(), keycloak, db).await?;
        self.inner.users_total.set(db_users.len() as f64);
        *self.inner.db_users.write().await = db_users;
        *self.inner.keycloak_users.write().await = keycloak_users;

        if let Some(redis) = redis {
            let publisher = self.inner.id.clone();
            let mut con = redis.get().await?;
            con.publish(
                self.inner.channel.as_ref(),
                serde_json::to_string(&UserCacheEvent {
                    publisher,
                    event: UserCacheEventType::ReloadUsers,
                })?,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn reload_groups(
        &self,
        keycloak: &Keycloak,
        redis: Option<&deadpool_redis::Pool>,
    ) -> anyhow::Result<()> {
        let next_items = load_groups(self.inner.realm.as_ref(), keycloak).await?;
        self.inner.groups_total.set(next_items.len() as f64);
        *self.inner.groups.write().await = next_items;

        if let Some(redis) = redis {
            let publisher = self.inner.id.clone();
            let mut con = redis.get().await?;
            con.publish(
                self.inner.channel.as_ref(),
                serde_json::to_string(&UserCacheEvent {
                    publisher,
                    event: UserCacheEventType::ReloadGroups,
                })?,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn reload_roles(
        &self,
        db: &DB,
        redis: Option<&deadpool_redis::Pool>,
    ) -> anyhow::Result<()> {
        let next_items = load_roles(db).await?;
        self.inner.roles_total.set(next_items.len() as f64);
        *self.inner.roles.write().await = next_items;
        if let Some(redis) = redis {
            let publisher = self.inner.id.clone();
            let mut con = redis.get().await?;
            con.publish(
                self.inner.channel.as_ref(),
                serde_json::to_string(&UserCacheEvent {
                    publisher,
                    event: UserCacheEventType::ReloadRoles,
                })?,
            )
            .await?;
        }
        Ok(())
    }

    async fn load_user(&self, user_details: Arc<UserDetails>, db_user: Arc<User>) {
        self.inner
            .db_users
            .write()
            .await
            .insert(db_user.details.user_id.clone(), db_user.clone());
        self.inner
            .keycloak_users
            .write()
            .await
            .insert(db_user.details.user_id.clone(), user_details);
        self.inner
            .users_total
            .set(self.inner.db_users.read().await.len() as f64);
    }

    async fn unload_user(&self, db_user: Arc<User>) {
        self.inner
            .db_users
            .write()
            .await
            .remove(&db_user.details.user_id);
        self.inner
            .keycloak_users
            .write()
            .await
            .remove(&db_user.details.user_id);
        self.inner
            .users_total
            .set(self.inner.db_users.read().await.len() as f64);
    }

    pub async fn new_user(
        &self,
        redis: &deadpool_redis::Pool,
        keycloak_user: UserRepresentation,
        db_user: Arc<User>,
    ) -> anyhow::Result<()> {
        let user_details: UserDetails = keycloak_user.try_into()?;
        // user_details.id = db_user.id.clone();
        let user_details = Arc::new(user_details);
        self.load_user(user_details.clone(), db_user.clone()).await;
        let publisher = self.inner.id.clone();
        let mut con = redis.get().await?;
        con.publish(
            self.inner.channel.as_ref(),
            serde_json::to_string(&UserCacheEvent {
                publisher,
                event: UserCacheEventType::NewUser(user_details, db_user),
            })?,
        )
        .await?;
        Ok(())
    }

    pub async fn delete_user(
        &self,
        redis: &deadpool_redis::Pool,
        db_user: Arc<User>,
    ) -> anyhow::Result<()> {
        let publisher = self.inner.id.clone();
        let mut con = redis.get().await?;
        con.publish(
            self.inner.channel.as_ref(),
            serde_json::to_string(&UserCacheEvent {
                publisher,
                event: UserCacheEventType::DeleteUser(db_user),
            })?,
        )
        .await?;
        Ok(())
    }

    async fn load_roles(&self, roles: &[RoleRepresentation]) {
        self.inner
            .roles
            .write()
            .await
            .extend(roles.iter().filter_map(parse_role));
        self.inner
            .roles_total
            .set(self.inner.roles.read().await.len() as f64);
    }

    pub async fn process_event(
        &self,
        keycloak: &Keycloak,
        db: &DB,
        msg: Msg,
        /*cache: &Cache,*/
    ) -> anyhow::Result<()> {
        let UserCacheEvent { publisher, event } = serde_json::from_slice(msg.get_payload_bytes())?;
        match event {
            UserCacheEventType::NewUser(user_details, db_user) => {
                if publisher != self.inner.id {
                    log::debug!("new user with id: {:#?}", db_user.details.user_id);
                    self.load_user(user_details, db_user).await;
                }
                // qm_entity::cache::invalidate::<User>(cache).await;
            }
            UserCacheEventType::DeleteUser(db_user) => {
                if publisher != self.inner.id {
                    log::debug!("delete user with id: {:#?}", db_user.details.user_id);
                    self.unload_user(db_user).await;
                } else {
                    // qm_entity::cache::invalidate::<User>(cache).await;
                }
            }
            UserCacheEventType::ReloadUsers => {
                self.reload_users(keycloak, db, None).await?;
            }
            UserCacheEventType::NewRoles(roles) => {
                log::debug!("new roles: {:#?}", roles);
                self.load_roles(&roles).await;
            }
            UserCacheEventType::ReloadGroups => {
                self.reload_groups(keycloak, None).await?;
            }
            UserCacheEventType::ReloadRoles => {
                self.reload_roles(db, None).await?;
            }
        }
        Ok(())
    }
}

type ParsedGroup = Option<(Arc<str>, (Arc<str>, GroupRepresentation))>;
fn parse_group(r: &GroupRepresentation) -> ParsedGroup {
    r.path.as_ref().zip(r.id.as_ref()).map(|(path, uid)| {
        (
            Arc::from(path.to_string()),
            (Arc::from(uid.to_string()), r.clone()),
        )
    })
}

async fn load_groups(realm: &str, keycloak: &Keycloak) -> anyhow::Result<KeycloakGroupMap> {
    let groups = ignore_404(keycloak.groups_with_subgroups(realm).await)?;
    Ok(KeycloakGroupMap::from_iter(
        groups.iter().filter_map(parse_group),
    ))
}
