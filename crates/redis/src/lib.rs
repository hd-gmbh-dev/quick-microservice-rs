pub use deadpool_redis::redis;
use deadpool_redis::Runtime;
use std::sync::Arc;
mod config;
pub mod lock;

pub use crate::config::Config as RedisConfig;
use crate::lock::Lock;

pub struct Inner {
    config: RedisConfig,
    client: redis::Client,
    pool: deadpool_redis::Pool,
}

#[derive(Clone)]
pub struct Redis {
    inner: Arc<Inner>,
}

impl AsRef<deadpool_redis::Pool> for Redis {
    fn as_ref(&self) -> &deadpool_redis::Pool {
        &self.inner.pool
    }
}

impl Redis {
    pub fn new() -> anyhow::Result<Self> {
        let config = RedisConfig::builder().build()?;
        let client = redis::Client::open(config.address())?;
        let redis_cfg = deadpool_redis::Config::from_url(config.address());
        let pool = redis_cfg.create_pool(Some(Runtime::Tokio1))?;
        Ok(Self {
            inner: Arc::new(Inner {
                config,
                client,
                pool,
            }),
        })
    }

    pub fn config(&self) -> &RedisConfig {
        &self.inner.config
    }

    pub fn client(&self) -> &redis::Client {
        &self.inner.client
    }

    pub async fn connect(&self) -> Result<deadpool_redis::Connection, deadpool_redis::PoolError> {
        self.inner.pool.get().await
    }

    pub async fn lock(
        &self,
        key: &str,
        ttl: usize,
        retry_count: u32,
        retry_delay: u32,
    ) -> Result<Lock, lock::Error> {
        let mut con = self.connect().await?;
        lock::lock(&mut con, key, ttl, retry_count, retry_delay).await
    }

    pub async fn unlock(&self, key: &str, lock_id: &str) -> Result<i64, lock::Error> {
        let mut con = self.connect().await?;
        lock::unlock(&mut con, key, lock_id).await
    }
}

#[macro_export]
macro_rules! redis {
    ($storage:ty) => {
        impl AsRef<qm::redis::Redis> for $storage {
            fn as_ref(&self) -> &qm::redis::Redis {
                &self.inner.redis
            }
        }
    };
}
