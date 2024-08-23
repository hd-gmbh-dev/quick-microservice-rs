use qm::keycloak::UserRepresentation;
use qm_example_ctx::Storage;

pub async fn ensure(app: &Storage, cleanup: bool) -> anyhow::Result<UserRepresentation> {
    let realm = app.keycloak().config().realm();
    if cleanup {
        tracing::info!("remove realm: {realm}");
        app.keycloak().remove_realm(realm).await.ok();
    }
    let realms = app.keycloak().realms().await?;
    if !realms.contains(&realm.to_string()) {
        qm::keycloak::realm::create(app.keycloak()).await?;
    }
    qm::keycloak::realm::configure_realm(app.keycloak(), qm_example_auth::roles::groups()).await?;
    let admin_user = qm::keycloak::realm::ensure_admin_user(
        realm,
        app.keycloak(),
        app.keycloak().config().realm_admin_username(),
        app.keycloak().config().realm_admin_password(),
        app.keycloak().config().realm_admin_email(),
        qm_example_auth::roles::admin_group(),
        std::collections::BTreeSet::from_iter(
            qm_example_auth::roles::roles()
                .into_iter()
                .map(|r| r.to_string()),
        ),
    )
    .await?;
    Ok(admin_user)
}
