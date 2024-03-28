mod config;
mod db;

use sqlx::Executor;

pub use crate::config::Config as DbConfig;
pub use crate::db::DB;

pub async fn ensure(app_name: &str, cfgs: &[&DbConfig]) -> anyhow::Result<()> {
    for cfg in cfgs {
        let db = DB::new_root(app_name, cfg).await?;
        let databases = sqlx::query!("SELECT datname FROM pg_database;")
            .fetch_all(db.pool())
            .await?;

        if let Some(database) = cfg.database() {
            if !databases.iter().any(|d| d.datname == database) {
                db.pool()
                    .execute(format!("CREATE DATABASE {database}").as_str())
                    .await?;
            }
        }

        if let Some((username, password)) = cfg.username().zip(cfg.password()) {
            let users = sqlx::query!("SELECT usename FROM pg_catalog.pg_user;")
                .fetch_all(db.pool())
                .await?;
            if !users.iter().any(|u| u.usename.as_deref() == Some(username)) {
                db.pool()
                    .execute(
                        format!("CREATE USER {username} WITH SUPERUSER PASSWORD '{password}'")
                            .as_str(),
                    )
                    .await?;
            }
        }
    }
    Ok(())
}
