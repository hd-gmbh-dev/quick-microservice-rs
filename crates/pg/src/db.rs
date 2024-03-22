use std::sync::Arc;
use crate::config::Config as PgConfig;
use deadpool_diesel::postgres::{Manager, Pool};

struct Inner {
    pool: Pool,
}

#[derive(Clone)]
pub struct DB {
    inner: Arc<Inner>,
}

impl DB {
    pub async fn new(app_name: &str, cfg: &PgConfig) -> anyhow::Result<Self> {
        log::info!(
            "'{app_name}' -> connects to postgresql '{}'",
            cfg.database()
        );
        let manager = Manager::new(
            cfg.address(),
            deadpool_diesel::Runtime::Tokio1,
        );
        let pool = Pool::builder(manager).build()?;
        Ok(Self {
            inner: Arc::new(Inner {
                pool
            })
        })
    }

    pub fn pool(&self) -> &Pool {
        &self.inner.pool
    }
}
