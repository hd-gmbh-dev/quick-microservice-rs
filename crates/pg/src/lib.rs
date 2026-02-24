//! PostgreSQL connection and utilities.
//!
//! This crate provides PostgreSQL connection management with automatic database
//! and user creation, connection pooling, and SeaORM integration.
//!
//! ## Features
//!
//! - **Connection Management**: Connect to PostgreSQL with connection pooling
//! - **Auto-provisioning**: Create databases and users if they don't exist
//! - **SeaORM Integration**: Convert pools to SeaORM database connections
//! - **Configuration**: Environment-based configuration with prefix support
//!
//! ## Usage
//!
//! \```ignore
//! use qm_pg::{DbConfig, DB};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = DbConfig::new()?;
//!     let db = DB::new("my-app", &config).await?;
//!     let conn = db.database_connection();
//!     Ok(())
//! }
//! \```
//!
//! ## Environment Variables
//!
//! | Variable | Description | Default |
//! |----------|-------------|---------|
//! | `PG_HOST` | PostgreSQL host | `127.0.0.1` |
//! | `PG_PORT` | PostgreSQL port | `5432` |
//! | `PG_USERNAME` | Database username | (none) |
//! | `PG_PASSWORD` | Database password | (none) |
//! | `PG_DATABASE` | Database name | (none) |
//! | `PG_ROOT_USERNAME` | Admin username | (none) |
//! | `PG_ROOT_PASSWORD` | Admin password | (none) |
//! | `PG_ROOT_DATABASE` | Admin database | (none) |
//! | `PG_MAX_CONNECTIONS` | Max pool connections | `32` |
//! | `PG_MIN_CONNECTIONS` | Min pool connections | `0` |

mod config;
mod db;

use sqlx::Executor;

/// PostgreSQL configuration.
pub use crate::config::Config as DbConfig;
pub use crate::db::DB;

/// Ensure database and user exist, creating them if necessary.
///
/// This function connects as root and creates the database and user
/// if they don't already exist.
pub async fn ensure(app_name: &str, cfgs: &[&DbConfig]) -> anyhow::Result<()> {
    for cfg in cfgs {
        let db = DB::new_root(app_name, cfg).await?;
        let query_args = ::core::result::Result::<_, ::sqlx::error::BoxDynError>::Ok(
            <sqlx::postgres::Postgres as ::sqlx::database::Database>::Arguments::<'_>::default(),
        );
        let databases: Vec<String> =
            sqlx::__query_with_result("SELECT datname FROM pg_database;", query_args)
                .try_map(|v: sqlx::postgres::PgRow| {
                    use ::sqlx::Row as _;
                    v.try_get_unchecked::<String, _>(0usize)
                })
                .fetch_all(db.pool())
                .await?;

        if let Some(database) = cfg.database() {
            if !databases.iter().any(|datname| datname == database) {
                db.pool()
                    .execute(format!("CREATE DATABASE {database}").as_str())
                    .await?;
            }
        }

        if let Some((username, password)) = cfg.username().zip(cfg.password()) {
            let query_args = ::core::result::Result::<_, ::sqlx::error::BoxDynError>::Ok(
                <sqlx::postgres::Postgres as ::sqlx::database::Database>::Arguments::<'_>::default(
                ),
            );
            let users: Vec<Option<String>> =
                sqlx::__query_with_result("SELECT usename FROM pg_catalog.pg_user;", query_args)
                    .try_map(|v: sqlx::postgres::PgRow| {
                        use ::sqlx::Row as _;
                        v.try_get_unchecked::<Option<String>, _>(0usize)
                    })
                    .fetch_all(db.pool())
                    .await?;
            if !users
                .iter()
                .any(|usename| usename.as_deref() == Some(username))
            {
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
