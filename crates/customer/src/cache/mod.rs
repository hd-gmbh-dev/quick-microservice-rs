use std::sync::Arc;

use qm_keycloak::Keycloak;
use qm_mongodb::DB;

pub mod customer;
pub mod user;

use customer::CustomerCache;
use user::UserCache;

struct Inner {
    customer: CustomerCache,
    user: UserCache,
}

#[derive(Clone)]
pub struct Cache {
    inner: Arc<Inner>,
}

impl Cache {
    pub async fn new(prefix: &str, keycloak: &Keycloak, db: &DB) -> anyhow::Result<Self> {
        Ok(Cache {
            inner: Arc::new(Inner {
                customer: CustomerCache::new(prefix, db).await?,
                user: UserCache::new(prefix, keycloak, db).await?,
            }),
        })
    }

    pub fn customer(&self) -> &CustomerCache {
        &self.inner.customer
    }

    pub fn user(&self) -> &UserCache {
        &self.inner.user
    }

    pub async fn reload_all(
        &self,
        keycloak: &Keycloak,
        db: &DB,
        redis: &deadpool_redis::Pool,
    ) -> anyhow::Result<()> {
        self.customer().reload(db, Some(redis)).await?;
        self.user().reload_users(keycloak, db, Some(redis)).await?;
        self.user().reload_groups(keycloak, Some(redis)).await?;
        self.user().reload_roles(db, Some(redis)).await?;
        Ok(())
    }
}
