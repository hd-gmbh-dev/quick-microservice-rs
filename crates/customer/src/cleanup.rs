use futures::stream::FuturesUnordered;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use strum::AsRefStr;
use tokio::sync::Semaphore;

use futures::StreamExt;
use qm_entity::ids::{
    CustomerId, StrictInstitutionIds, StrictOrganizationIds, StrictOrganizationUnitIds,
};
use qm_keycloak::Keycloak;
use qm_keycloak::KeycloakError;
use qm_mongodb::bson::doc;
use qm_mongodb::bson::Uuid;
use qm_mongodb::ClientSession;

use crate::cache::user::UserCache;
use crate::roles::RoleDB;

pub type CustomerIds = Arc<[CustomerId]>;
#[derive(
    Default, AsRefStr, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum CleanupTaskType {
    #[strum(serialize = "customers")]
    Customers(CustomerIds),
    #[strum(serialize = "organizations")]
    Organizations(StrictOrganizationIds),
    #[strum(serialize = "institutions")]
    Institutions(StrictInstitutionIds),
    #[strum(serialize = "organization_units")]
    OrganizationUnits(StrictOrganizationUnitIds),
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
            id: Uuid::new(),
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

pub async fn cleanup_roles(
    db: &impl RoleDB,
    redis: &deadpool_redis::Pool,
    keycloak: &Keycloak,
    user_cache: &UserCache,
    roles: BTreeSet<String>,
    session: &mut ClientSession,
) -> anyhow::Result<()> {
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
        role_remove_tasks.collect::<Vec<_>>().await;
        let roles: Vec<&String> = roles.iter().collect();
        let result = db
            .roles()
            .as_ref()
            .delete_many_with_session(doc! { "name": { "$in": &roles }}, None, session)
            .await?;
        if result.deleted_count != 0 {
            user_cache.reload_roles(db, Some(redis)).await?;
        }
    }
    Ok(())
}
