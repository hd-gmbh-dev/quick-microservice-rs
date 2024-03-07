use qm_keycloak::Keycloak;
use std::sync::Arc;

pub mod customer;
pub mod user;

use customer::CustomerCache;
use user::UserCache;

use self::{customer::CustomerCacheDB, user::UserCacheDB};

pub trait CacheDB: CustomerCacheDB + UserCacheDB {}

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
