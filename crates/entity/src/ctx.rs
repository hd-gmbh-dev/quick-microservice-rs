use crate::ids::EntityId;
use crate::ids::EntityIds;
use crate::ids::OrganizationUnitId;
use crate::ids::StrictEntityIds;
use crate::ids::StrictInstitutionIds;
use crate::ids::StrictOrganizationIds;
use crate::ids::ID;
use crate::ids::{CustomerResourceId, OrganizationResourceId, StrictEntityId};

use async_graphql::{InputObject, OneofObject};
use qm_mongodb::bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};

/// CustomerFilter is used when filtering from the perspective of a customer using no filter or from the perspective of
/// an admin filtering for a specific customer
#[derive(Default, Debug, Clone, InputObject, Serialize, Deserialize)]
pub struct CustomerFilter {
    pub customer: ID,
}

impl Into<EntityId> for CustomerFilter {
    fn into(self) -> EntityId {
        EntityId {
            cid: Some(self.customer),
            ..Default::default()
        }
    }
}

/// OrganizationFilter is used when filtering from the perspective of a organization using no filter, from the
/// perspective of an admin or a customer filtering for a specific organization
#[derive(Default, Debug, Clone, InputObject, Serialize, Deserialize)]
pub struct OrganizationFilter {
    pub customer: ID,
    pub organization: ID,
}

impl From<OrganizationFilter> for CustomerResourceId {
    fn from(value: OrganizationFilter) -> Self {
        Self {
            cid: value.customer,
            id: value.organization,
        }
    }
}

impl Into<EntityId> for OrganizationFilter {
    fn into(self) -> EntityId {
        EntityId {
            cid: Some(self.customer),
            oid: Some(self.organization),
            ..Default::default()
        }
    }
}

/// OrganizationUnitFilter is used when filtering from the perspective of a organization using no filter, from the
/// perspective of an admin or a customer filtering for a specific organization
#[derive(Default, Debug, Clone, InputObject, Serialize, Deserialize)]
pub struct OrganizationUnitFilter {
    pub customer: ID,
    pub organization: Option<ID>,
    pub organization_unit: ID,
}

impl From<OrganizationUnitFilter> for OrganizationUnitId {
    fn from(value: OrganizationUnitFilter) -> Self {
        if let Some(organization) = value.organization {
            OrganizationUnitId::Organization(OrganizationResourceId {
                cid: value.customer,
                oid: organization,
                id: value.organization_unit,
            })
        } else {
            OrganizationUnitId::Customer(CustomerResourceId {
                cid: value.customer,
                id: value.organization_unit,
            })
        }
    }
}

impl Into<EntityId> for OrganizationUnitFilter {
    fn into(self) -> EntityId {
        EntityId {
            cid: Some(self.customer),
            oid: self.organization,
            iid: Some(self.organization_unit),
            ..Default::default()
        }
    }
}

/// InstitutionFilter is used when filtering from the perspective of a institution using no filter, from the perspective
/// of an admin, a customer or an organization filtering for a specific institution
#[derive(Default, Debug, Clone, InputObject, Serialize, Deserialize)]
pub struct InstitutionFilter {
    pub customer: ID,
    pub organization: ID,
    pub institution: ID,
}

impl From<InstitutionFilter> for OrganizationResourceId {
    fn from(value: InstitutionFilter) -> Self {
        Self {
            cid: value.customer,
            oid: value.organization,
            id: value.institution,
        }
    }
}

impl Into<EntityId> for InstitutionFilter {
    fn into(self) -> EntityId {
        EntityId {
            cid: Some(self.customer),
            oid: Some(self.organization),
            iid: Some(self.institution),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, OneofObject)]
pub enum OrgOrInstFilter {
    Organization(OrganizationFilter),
    Institution(InstitutionFilter),
}

/// Oneof input object in GraphQL
/// Oneof input objects requires have exactly one field
#[derive(Debug, Clone, Serialize, Deserialize, OneofObject)]
pub enum ContextFilterInput {
    #[graphql(name = "customerFilter")]
    Customer(CustomerFilter),
    #[graphql(name = "organizationFilter")]
    Organization(OrganizationFilter),
    #[graphql(name = "organizationUnitFilter")]
    OrganizationUnit(OrganizationUnitFilter),
    #[graphql(name = "institutionFilter")]
    Institution(InstitutionFilter),
}

impl ContextFilterInput {
    pub fn cid(&self) -> &ObjectId {
        match self {
            ContextFilterInput::Customer(v) => v.customer.as_ref(),
            ContextFilterInput::Organization(v) => v.customer.as_ref(),
            ContextFilterInput::OrganizationUnit(v) => v.customer.as_ref(),
            ContextFilterInput::Institution(v) => v.customer.as_ref(),
        }
    }
}

impl<'a> From<&'a StrictEntityId> for MutationContext {
    fn from(value: &StrictEntityId) -> Self {
        MutationContext::Institution(InstitutionFilter {
            customer: value.cid.clone(),
            organization: value.oid.clone(),
            institution: value.iid.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutationContext {
    OptCustomer(Option<CustomerFilter>),
    Customer(CustomerFilter),
    Organization(OrganizationFilter),
    Institution(InstitutionFilter),
    Batch(EntityIds),
    BatchStrict(StrictEntityIds),
    BatchOrganization(StrictOrganizationIds),
    BatchInstitution(StrictInstitutionIds),
}
