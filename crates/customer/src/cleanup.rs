use futures::stream::FuturesUnordered;

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use strum::AsRefStr;
use tokio::sync::Semaphore;

use qm_entity::ids::CustomerIds;
use qm_entity::ids::InstitutionIds;
use qm_entity::ids::OrganizationIds;
use qm_entity::ids::OrganizationUnitIds;
use qm_keycloak::Keycloak;
use qm_keycloak::KeycloakError;
use sqlx::types::Uuid;

#[derive(
    Default, AsRefStr, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum CleanupTaskType {
    #[strum(serialize = "customers")]
    Customers(CustomerIds),
    #[strum(serialize = "organizations")]
    Organizations(OrganizationIds),
    #[strum(serialize = "institutions")]
    Institutions(InstitutionIds),
    #[strum(serialize = "organization_units")]
    OrganizationUnits(OrganizationUnitIds),
    #[default]
    #[strum(serialize = "none")]
    None,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct CleanupTask {
    pub id: Uuid,
    pub ty: CleanupTaskType,
}

impl CleanupTask {
    pub fn new(ty: CleanupTaskType) -> Self {
        Self {
            id: Uuid::new_v4(),
            ty,
        }
    }
}

async fn remove_users_by_access(
    realm: &str,
    keycloak: &Keycloak,
    role_name: &str,
) -> anyhow::Result<()> {
    let result = keycloak.role_members(realm, role_name).await;
    match result {
        Ok(users) => {
            for user in users {
                keycloak
                    .remove_user(realm, user.id.as_deref().unwrap())
                    .await?;
            }
        }
        Err(err) => match err {
            KeycloakError::HttpFailure { status: 404, .. } => {}
            _ => Err(err)?,
        },
    }
    Ok(())
}

pub async fn cleanup_roles(keycloak: &Keycloak, roles: BTreeSet<String>) -> anyhow::Result<()> {
    if !roles.is_empty() {
        let semaphore = Arc::new(Semaphore::new(4));
        let role_remove_tasks = FuturesUnordered::new();
        for role in roles.clone().into_iter() {
            let keycloak = keycloak.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            role_remove_tasks.push(tokio::spawn(async move {
                let realm = keycloak.config().realm();
                log::debug!("remove users with role from keycloak {role}");
                if remove_users_by_access(realm, &keycloak, &role)
                    .await
                    .is_err()
                {
                    drop(permit);
                    return anyhow::Ok(role);
                }
                log::debug!("remove role from keycloak {role}");
                let result = keycloak.remove_role(realm, &role).await;
                drop(permit);
                match result {
                    Ok(_) => {}
                    Err(err) => match err {
                        KeycloakError::HttpFailure { status: 404, .. } => {}
                        _ => {
                            log::error!("Error: {err:#?}");
                            Err(err)?;
                        }
                    },
                }
                anyhow::Ok(role)
            }));
        }
    }
    Ok(())
}
