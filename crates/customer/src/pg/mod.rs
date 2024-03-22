use deadpool_diesel::postgres::Pool;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations/");
mod service;
mod db_schema;

pub async fn run_migrations(pool: &Pool) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
        .await
        .unwrap()
        .unwrap();
    Ok(())
}
