use async_graphql::{InputObject, SimpleObject};
use qm_entity::ids::OrganizationId;
use qm_entity::ids::{CustomerId, InfraId, InstitutionId};
use serde::{Deserialize, Serialize};
use sqlx::types::time::PrimitiveDateTime;
use sqlx::types::uuid::Uuid;
use sqlx::FromRow;

use std::sync::Arc;

pub struct InstitutionData(
    pub OrganizationId,
    pub String,
    pub Option<String>,
    pub Option<i64>,
);

#[derive(Debug, Clone, SimpleObject)]
pub struct QmInstitutionList {
    pub items: Arc<[Arc<QmInstitution>]>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

#[derive(Debug, Clone, SimpleObject, FromRow, Serialize, Deserialize)]
#[graphql(complex)]
pub struct QmInstitution {
    #[graphql(skip)]
    pub id: InfraId,
    #[graphql(skip)]
    pub customer_id: InfraId,
    #[graphql(skip)]
    pub organization_id: InfraId,
    pub name: Arc<str>,
    pub ty: Arc<str>,
    pub created_by: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_by: Option<Uuid>,
    pub updated_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionUpdate {
    pub id: InfraId,
    pub customer_id: InfraId,
    pub organization_id: InfraId,
    pub name: Arc<str>,
    pub ty: Arc<str>,
    pub created_by: Uuid,
    pub created_at: String,
    pub updated_by: Option<Uuid>,
    pub updated_at: Option<String>,
}

pub struct RemoveInstitutionPayload {
    pub id: InfraId,
    pub customer_id: InfraId,
    pub organization_id: InfraId,
    pub name: Arc<str>,
}

impl From<InstitutionUpdate> for RemoveInstitutionPayload {
    fn from(value: InstitutionUpdate) -> Self {
        Self {
            id: value.id,
            customer_id: value.customer_id,
            organization_id: value.organization_id,
            name: value.name,
        }
    }
}

impl<'a> From<&'a QmInstitution> for RemoveInstitutionPayload {
    fn from(value: &'a QmInstitution) -> Self {
        Self {
            id: value.id,
            customer_id: value.customer_id,
            organization_id: value.organization_id,
            name: value.name.clone(),
        }
    }
}

#[derive(Debug, InputObject)]
pub struct CreateInstitutionInput {
    pub id: Option<i64>,
    pub name: String,
    pub ty: Option<String>,
}

#[derive(Debug, InputObject)]
pub struct UpdateInstitutionInput {
    pub name: String,
}

impl<'a> From<&'a QmInstitution> for InstitutionId {
    fn from(val: &'a QmInstitution) -> Self {
        let cid: i64 = val.customer_id.into();
        let oid: i64 = val.organization_id.into();
        let iid: i64 = val.id.into();
        (cid, oid, iid).into()
    }
}

impl<'a> From<&'a QmInstitution> for OrganizationId {
    fn from(val: &'a QmInstitution) -> Self {
        let cid: i64 = val.customer_id.into();
        let oid: i64 = val.organization_id.into();
        (cid, oid).into()
    }
}

impl<'a> From<&'a QmInstitution> for CustomerId {
    fn from(val: &'a QmInstitution) -> Self {
        let cid: i64 = val.customer_id.into();
        cid.into()
    }
}
