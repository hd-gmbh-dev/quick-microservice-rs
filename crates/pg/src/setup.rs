use deadpool_diesel::postgres::{Manager, Pool};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;

use crate::DbConfig as PgConfig;

pub async fn run(cfg: &PgConfig) -> anyhow::Result<()> {
    log::info!("connect to {}", cfg.root_address());
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(cfg.root_address())
        .await?;
    let query = format!("CREATE DATABASE {}", cfg.database());
    let results = sqlx::query(&query).fetch_all(&pool).await;
    match results {
        Ok(r) => {
            for r in r {
                log::info!("{:#?}", r.columns());
            }
        }
        Err(err) => {
            log::error!("{err:#?}");
        }
    }
    let query = format!(
        "CREATE USER {} with encrypted password '{}'",
        cfg.username(),
        cfg.password()
    );
    let results = sqlx::query(&query).fetch_all(&pool).await;
    match results {
        Ok(r) => {
            for r in r {
                log::info!("{:#?}", r.columns());
            }
        }
        Err(err) => {
            log::error!("{err:#?}");
        }
    }
    let root_address = format!("{}{}", cfg.root_address(), cfg.database());
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&root_address)
        .await?;
    log::info!("connect to {root_address}");
    // let query = format!(
    //     "GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {}",
    //     cfg.username()
    // );
    // let results = sqlx::query(&query).fetch_all(&pool).await;
    // match results {
    //     Ok(r) => {
    //         for r in r {
    //             log::info!("{:#?}", r.columns());
    //         }
    //     }
    //     Err(err) => {
    //         log::error!("{err:#?}");
    //     }
    // }
    let query = format!(
        "ALTER USER {} WITH SUPERUSER",
        cfg.username()
    );
    let results = sqlx::query(&query).fetch_all(&pool).await;
    match results {
        Ok(r) => {
            for r in r {
                log::info!("{:#?}", r.columns());
            }
        }
        Err(err) => {
            log::error!("{err:#?}");
        }
    }
    Ok(())
}
