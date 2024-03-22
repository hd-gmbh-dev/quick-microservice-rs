//! # configure command
//!
//! This command configures MongoDB, Keycloak and S3.
//!
use crate::commands::ConfigureCommand;
use std::collections::BTreeSet;

async fn configure_keycloak() -> anyhow::Result<()> {
    let keycloak = qm::keycloak::Keycloak::builder()
        .with_no_refresh()
        .build()
        .await?;
    let realms = keycloak.realms().await?;
    if !realms.contains(&keycloak.config().realm().into()) {
        qm::keycloak::realm::create(&keycloak).await?;
    }
    qm::keycloak::realm::configure_realm(&keycloak, qm_example_auth::roles::groups()).await?;
    let keycloak_config = keycloak.config();
    qm::keycloak::realm::ensure_admin_user(
        keycloak_config.realm(),
        &keycloak,
        keycloak_config.realm_admin_username(),
        keycloak_config.realm_admin_password(),
        keycloak_config.realm_admin_email(),
        qm_example_auth::roles::admin_group(),
        BTreeSet::from_iter(
            qm_example_auth::roles::roles()
                .into_iter()
                .map(|r| r.to_string()),
        ),
    )
    .await?;
    Ok(())
}

async fn configure_posgres() -> anyhow::Result<()> {
    let db = qm::pg::DB::new("qm_example", &qm::pg::DbConfig::new()?).await?;
    // qm::customer::pg::run_migrations(&db).await?;
    Ok(())
}

impl ConfigureCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        match self.resource {
            super::Resource::All => {
                configure_keycloak().await?;
                configure_posgres().await?;
            }
            super::Resource::KeycloakRealm => {
                configure_keycloak().await?;
            }
            _ => {
                unimplemented!()
            }
        }
        Ok(())
    }
}
