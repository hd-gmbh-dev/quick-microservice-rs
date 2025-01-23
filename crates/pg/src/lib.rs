mod config;
mod db;

use sqlx::Executor;

pub use crate::config::Config as DbConfig;
pub use crate::db::DB;

pub async fn ensure(app_name: &str, cfgs: &[&DbConfig]) -> anyhow::Result<()> {
    for cfg in cfgs {
        let db = DB::new_root(app_name, cfg).await?;
        let query_args =  ::core::result::Result:: <_, ::sqlx::error::BoxDynError> ::Ok(<sqlx::postgres::Postgres as ::sqlx::database::Database> ::Arguments:: <'_> ::default());
        let databases: Vec<String> = sqlx::__query_with_result("SELECT datname FROM pg_database;", query_args).try_map(|v: sqlx::postgres::PgRow| {
            use ::sqlx::Row as _;
            Ok(v.try_get_unchecked::<String, _>(0usize)?.into())
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
            let query_args =  ::core::result::Result:: <_, ::sqlx::error::BoxDynError> ::Ok(<sqlx::postgres::Postgres as ::sqlx::database::Database> ::Arguments:: <'_> ::default());
            let users: Vec<Option<String>> = sqlx::__query_with_result("SELECT usename FROM pg_catalog.pg_user;", query_args)
                .try_map(|v: sqlx::postgres::PgRow| {
                    use ::sqlx::Row as _;
                    Ok(v.try_get_unchecked::<Option<String>, _>(0usize)?.into())
                })
                .fetch_all(db.pool())
                .await?;
            if !users.iter().any(|usename| usename.as_deref() == Some(username)) {
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
