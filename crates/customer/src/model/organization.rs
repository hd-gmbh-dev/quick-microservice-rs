use async_graphql::{InputObject, SimpleObject};
use qm_entity::ids::{CustomerId, InfraId, OrganizationId};
use serde::{Deserialize, Serialize};
use sqlx::types::time::PrimitiveDateTime;
use sqlx::types::uuid::Uuid;
use sqlx::FromRow;

use std::sync::Arc;

pub struct OrganizationData(pub InfraId, pub String, pub Option<String>, pub Option<i64>);

#[derive(Debug, InputObject)]
pub struct CreateOrganizationInput {
    pub id: Option<i64>,
    pub name: String,
    pub ty: Option<String>,
}

#[derive(Debug, InputObject)]
pub struct UpdateOrganizationInput {
    pub name: String,
}

#[derive(Debug, Clone, SimpleObject, FromRow, Serialize, Deserialize)]
#[graphql(complex)]
pub struct QmOrganization {
    #[graphql(skip)]
    pub id: InfraId,
    #[graphql(skip)]
    pub customer_id: InfraId,
    pub name: Arc<str>,
    pub ty: Arc<str>,
    pub created_by: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_by: Option<Uuid>,
    pub updated_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationUpdate {
    pub id: InfraId,
    pub customer_id: InfraId,
    pub name: Arc<str>,
    pub ty: Arc<str>,
    pub created_by: Uuid,
    pub created_at: String,
    pub updated_by: Option<Uuid>,
    pub updated_at: Option<String>,
}

pub struct RemoveOrganizationPayload {
    pub id: InfraId,
    pub customer_id: InfraId,
    pub name: Arc<str>,
}

impl From<OrganizationUpdate> for RemoveOrganizationPayload {
    fn from(value: OrganizationUpdate) -> Self {
        Self {
            id: value.id,
            customer_id: value.customer_id,
            name: value.name,
        }
    }
}

impl<'a> From<&'a QmOrganization> for RemoveOrganizationPayload {
    fn from(value: &'a QmOrganization) -> Self {
        Self {
            id: value.id,
            customer_id: value.customer_id,
            name: value.name.clone(),
        }
    }
}

#[derive(Debug, Clone, SimpleObject)]
pub struct QmOrganizationList {
    pub items: Arc<[Arc<QmOrganization>]>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

impl<'a> From<&'a QmOrganization> for OrganizationId {
    fn from(val: &'a QmOrganization) -> Self {
        let cid: i64 = val.customer_id.into();
        let oid: i64 = val.id.into();
        (cid, oid).into()
    }
}

impl<'a> From<&'a QmOrganization> for CustomerId {
    fn from(val: &'a QmOrganization) -> Self {
        let cid: i64 = val.customer_id.into();
        cid.into()
    }
}
