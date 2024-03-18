use qm_keycloak::Keycloak;
use std::sync::Arc;
use tokio::{runtime::Builder, task::LocalSet};

pub mod customer;
pub mod user;

use customer::CustomerCache;
use user::UserCache;

use crate::context::RelatedStorage;

use self::{customer::CustomerCacheDB, user::UserCacheDB};

pub trait CacheDB: Clone + CustomerCacheDB + UserCacheDB {}

struct Inner {
    customer: CustomerCache,
    user: UserCache,
}

#[derive(Clone)]
pub struct Cache {
    inner: Arc<Inner>,
}

impl Cache {
    pub async fn new(prefix: &str, realm: &str) -> anyhow::Result<Self> {
        Ok(Cache {
            inner: Arc::new(Inner {
                customer: CustomerCache::new(prefix).await?,
                user: UserCache::new(prefix, realm).await?,
            }),
        })
    }

    pub fn customer(&self) -> &CustomerCache {
        &self.inner.customer
    }

    pub fn user(&self) -> &UserCache {
        &self.inner.user
    }

    pub async fn reload_all(&self, keycloak: &Keycloak, db: &impl CacheDB) -> anyhow::Result<()> {
        self.customer().reload(db, None).await?;
        self.user().reload_users(keycloak, db, None).await?;
        self.user().reload_groups(keycloak, None).await?;
        self.user().reload_roles(db, None).await?;
        Ok(())
    }
}

#[inline]
pub fn subscribe<T>(t: T)
where
    T: RelatedStorage + Send + Sync + 'static,
{
    let worker_client = t.redis().client().clone();
    let worker_cache = t.cache().clone();
    let cache_db = t.clone();
    std::thread::spawn(move || {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let local = LocalSet::new();
        local.spawn_local(async move {
            let mut con = match worker_client.get_connection() {
                Ok(con) => con,
                Err(err) => {
                    log::error!("{err}");
                    std::process::exit(1);
                }
            };
            let mut pubsub = con.as_pubsub();
            if let Err(err) = pubsub.subscribe(worker_cache.customer().channel()) {
                log::error!("{err}");
                std::process::exit(1);
            }
            loop {
                let msg = pubsub.get_message();
                if let Err(err) = &msg {
                    log::error!("{err}");
                    std::process::exit(1);
                } else if let Err(err) = worker_cache
                    .customer()
                    .process_event(&cache_db, msg.unwrap())
                    .await
                {
                    log::error!("{err}");
                    std::process::exit(1);
                }
            }
        });
        rt.block_on(local);
    });
    let worker_client = t.redis().client().clone();
    let worker_keycloak = t.keycloak().clone();
    let worker_cache = t.cache().clone();
    let cache_db = t.clone();
    std::thread::spawn(move || {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let local = LocalSet::new();
        local.spawn_local(async move {
            let mut con = match worker_client.get_connection() {
                Ok(con) => con,
                Err(err) => {
                    log::error!("{err}");
                    std::process::exit(1);
                }
            };
            let mut pubsub = con.as_pubsub();
            if let Err(err) = pubsub.subscribe(worker_cache.user().channel()) {
                log::error!("{err}");
                std::process::exit(1);
            }
            loop {
                let msg = pubsub.get_message();
                if let Err(err) = &msg {
                    log::error!("{err}");
                    std::process::exit(1);
                } else if let Err(err) = worker_cache
                    .user()
                    .process_event(&worker_keycloak, &cache_db, msg.unwrap())
                    .await
                {
                    log::error!("{err}");
                    std::process::exit(1);
                }
            }
        });
        rt.block_on(local);
    });
}
