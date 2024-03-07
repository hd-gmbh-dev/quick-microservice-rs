use qm_keycloak::Keycloak;
use qm_keycloak::KeycloakError;
use qm_keycloak::RoleRepresentation;

pub async fn ensure(
    keycloak: &Keycloak,
    role_set: impl Iterator<Item = String>,
) -> anyhow::Result<Vec<RoleRepresentation>> {
    let realm = keycloak.config().realm();
    let mut roles = vec![];
    for role in role_set {
        let result = keycloak
            .create_role(
                realm,
                RoleRepresentation {
                    name: Some(role.clone()),
                    ..RoleRepresentation::default()
                },
            )
            .await;
        match result {
            Ok(_) => {
                roles.push(keycloak.realm_role_by_name(realm, &role).await?);
            }
            Err(err) => match err {
                KeycloakError::HttpFailure { status: 409, .. } => {
                    roles.push(keycloak.realm_role_by_name(realm, &role).await?);
                }
                _ => {
                    log::error!("{err:#?}");
                    Err(err)?
                }
            },
        }
    }
    Ok(roles)
}
