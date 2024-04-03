use crate::model::CreateUserInput;
use async_graphql::{InputObject, SimpleObject};
use qm_entity::ids::{
    CustomerId, InfraId, InstitutionIds, OrganizationId, OrganizationUnitId, PartialEqual,
};
use serde::{Deserialize, Serialize};
use sqlx::types::time::PrimitiveDateTime;
use sqlx::types::uuid::Uuid;
use sqlx::FromRow;

use std::sync::Arc;

#[derive(Debug, InputObject)]
pub struct CreateOrganizationUnitInput {
    pub name: String,
    pub ty: Option<String>,
    pub initial_user: Option<CreateUserInput>,
    pub members: InstitutionIds,
}

pub struct OrganizationUnitData {
    pub cid: InfraId,
    pub oid: Option<InfraId>,
    pub name: String,
    pub ty: Option<String>,
    pub members: InstitutionIds,
}

#[derive(Clone, FromRow)]
pub struct OrganizationUnitMemberQuery {
    pub organization_unit_id: i64,
    pub customer_id: i64,
    pub organization_id: i64,
    pub institution_id: i64,
}

#[derive(FromRow)]
pub struct OrganizationUnitQuery {
    pub id: i64,
    pub customer_id: i64,
    pub organization_id: Option<i64>,
    pub name: Arc<str>,
    pub ty: Arc<str>,
    pub created_by: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_by: Option<Uuid>,
    pub updated_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, SimpleObject, FromRow, Serialize, Deserialize)]
#[graphql(complex)]
pub struct OrganizationUnit {
    #[graphql(skip)]
    pub id: InfraId,
    #[graphql(skip)]
    pub customer_id: InfraId,
    #[graphql(skip)]
    pub organization_id: Option<InfraId>,
    pub name: Arc<str>,
    pub ty: Arc<str>,
    pub created_by: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_by: Option<Uuid>,
    pub updated_at: Option<PrimitiveDateTime>,
    pub members: InstitutionIds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationUnitUpdate {
    pub id: InfraId,
    pub customer_id: InfraId,
    pub organization_id: Option<InfraId>,
    pub name: Arc<str>,
    pub ty: Arc<str>,
    pub created_by: Uuid,
    pub created_at: String,
    pub updated_by: Option<Uuid>,
    pub updated_at: Option<String>,
}

pub struct RemoveOrganizationUnitPayload {
    pub id: InfraId,
    pub customer_id: InfraId,
    pub organization_id: Option<InfraId>,
    pub name: Arc<str>,
}

impl From<OrganizationUnitUpdate> for RemoveOrganizationUnitPayload {
    fn from(value: OrganizationUnitUpdate) -> Self {
        Self {
            id: value.id,
            customer_id: value.customer_id,
            organization_id: value.organization_id,
            name: value.name,
        }
    }
}

impl<'a> From<&'a OrganizationUnit> for RemoveOrganizationUnitPayload {
    fn from(value: &'a OrganizationUnit) -> Self {
        Self {
            id: value.id,
            customer_id: value.customer_id,
            organization_id: value.organization_id,
            name: value.name.clone(),
        }
    }
}

impl OrganizationUnit {
    // pub fn as_id(&self) -> OrganizationUnitId {
    //     if let Some(oid) = self.organization_id.as_ref() {
    //         OrganizationUnitId::Organization(OrganizationResourceId {
    //             cid: self.customer_id,
    //             oid: *oid,
    //             id: self.id,
    //         })
    //     } else {
    //         OrganizationUnitId::Customer(CustomerResourceId {
    //             cid: self.customer_id,
    //             id: self.id,
    //         })
    //     }
    // }

    pub fn with_members(mut self, members: InstitutionIds) -> Self {
        self.members = members;
        self
    }
}

impl From<OrganizationUnitQuery> for OrganizationUnit {
    fn from(value: OrganizationUnitQuery) -> Self {
        Self {
            id: value.id.into(),
            customer_id: value.customer_id.into(),
            organization_id: value.organization_id.map(Into::into),
            name: value.name,
            ty: value.ty,
            created_by: value.created_by,
            created_at: value.created_at,
            updated_by: value.updated_by,
            updated_at: value.updated_at,
            members: Arc::from(vec![]),
        }
    }
}

#[derive(Debug, Clone, SimpleObject)]
pub struct OrganizationUnitList {
    pub items: Arc<[Arc<OrganizationUnit>]>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

impl<'a> From<&'a OrganizationUnit> for OrganizationUnitId {
    fn from(val: &'a OrganizationUnit) -> Self {
        if let Some(organization_id) = val.organization_id {
            let cid: i64 = val.customer_id.into();
            let oid: i64 = organization_id.into();
            let uid: i64 = val.id.into();
            (cid, oid, uid).into()
        } else {
            let cid: i64 = val.customer_id.into();
            let uid: i64 = val.id.into();
            (cid, uid).into()
        }
    }
}

impl PartialEqual<'_, CustomerId> for OrganizationUnit {
    fn partial_equal(&'_ self, r: &'_ CustomerId) -> bool {
        self.customer_id == r.into()
    }
}

impl PartialEqual<'_, OrganizationId> for OrganizationUnit {
    fn partial_equal(&'_ self, r: &'_ OrganizationId) -> bool {
        if let Some(organization_id) = self.organization_id {
            organization_id == r.into()
        } else {
            false
        }
    }
}
