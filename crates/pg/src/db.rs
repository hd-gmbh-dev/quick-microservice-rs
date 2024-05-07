use crate::config::Config;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

struct Inner {
    pool: PgPool,
}

#[derive(Clone)]
pub struct DB {
    inner: Arc<Inner>,
}

impl DB {
    pub async fn new(app_name: &str, cfg: &Config) -> anyhow::Result<Self> {
        if let Some(database) = cfg.database() {
            log::info!(
                "'{app_name}' -> connects to postgresql '{database}' with {} max_connections",
                cfg.max_connections(),
            );
        } else {
            log::info!(
                "'{app_name}' -> connects to postgresql with {} max_connections",
                cfg.max_connections(),
            );
        }
        let pool = PgPoolOptions::new()
            .min_connections(cfg.min_connections())
            .max_connections(cfg.max_connections())
            .acquire_timeout(Duration::from_secs(cfg.acquire_timeout()))
            .idle_timeout(Duration::from_secs(cfg.idle_timeout()))
            .max_lifetime(Duration::from_secs(cfg.max_lifetime()))
            .connect(cfg.address())
            .await?;
        Ok(Self {
            inner: Arc::new(Inner { pool }),
        })
    }

    pub async fn new_root(app_name: &str, cfg: &Config) -> anyhow::Result<Self> {
        if let Some(database) = cfg.root_database() {
            log::info!(
                "'{app_name}' -> connects as root to postgresql '{database}' with {} max_connections",
                cfg.max_connections(),
            );
        } else {
            log::info!(
                "'{app_name}' -> connects as root to postgresql with {} max_connections",
                cfg.max_connections(),
            );
        }
        let pool = PgPoolOptions::new()
            .min_connections(1)
            .max_connections(2)
            .acquire_timeout(Duration::from_secs(cfg.acquire_timeout()))
            .connect(cfg.root_address())
            .await?;
        Ok(Self {
            inner: Arc::new(Inner { pool }),
        })
    }

    pub fn database_connection(&self) -> sea_orm::DatabaseConnection {
        sea_orm::SqlxPostgresConnector::from_sqlx_postgres_pool(self.inner.pool.clone())
    }

    pub fn pool(&self) -> &PgPool {
        &self.inner.pool
    }
}
