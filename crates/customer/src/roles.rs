use qm_keycloak::Keycloak;
use qm_keycloak::KeycloakError;
use qm_keycloak::RoleRepresentation;

// pub const DEFAULT_COLLECTION: &str = "roles";

// pub trait RoleDB: AsRef<DB> {
//     fn collection(&self) -> &str {
//         DEFAULT_COLLECTION
//     }
//     fn roles(&self) -> qm_entity::Collection<RoleRepresentation> {
//         let collection = self.collection();
//         qm_entity::Collection(
//             self.as_ref()
//                 .get()
//                 .collection::<RoleRepresentation>(collection),
//         )
//     }
// }

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
