use std::sync::Arc;

use qm_entity::error::{EntityError, EntityResult};
use qm_entity::ids::{EntityId, MemberId, OrganizationUnitId, StrictOrganizationUnitId, ID};

use qm_entity::list::NewList;
use qm_entity::{Create, UserId};

use async_graphql::{ComplexObject, Context, FieldResult, InputObject, SimpleObject};
use serde::{Deserialize, Serialize};

use crate::cache::Cache;
use crate::model::CreateUserInput;
use qm_entity::model::Modification;

use super::{Customer, Organization, Owner};

#[derive(Debug, InputObject)]
pub struct CreateOrganizationUnitInput {
    pub name: String,
    pub initial_user: Option<CreateUserInput>,
    pub members: Vec<MemberId>,
}

#[derive(Debug, InputObject)]
pub struct UpdateOrganizationUnitInput {
    pub organization_unit: StrictOrganizationUnitId,
    pub name: Option<String>,
}

#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
#[graphql(complex)]
pub struct OrganizationUnit {
    #[graphql(skip)]
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    pub name: String,
    #[graphql(skip)]
    pub owner: Owner,
    pub members: Vec<MemberId>,
    pub created: Modification,
    pub modified: Option<Modification>,
}

impl AsMut<Option<ID>> for OrganizationUnit {
    fn as_mut(&mut self) -> &mut Option<ID> {
        &mut self.id
    }
}

pub struct OrganizationUnitData {
    pub cid: ID,
    pub oid: Option<ID>,
    pub name: String,
    pub members: Vec<MemberId>,
}

impl<C> Create<OrganizationUnit, C> for OrganizationUnitData
where
    C: UserId,
{
    fn create(self, c: &C) -> EntityResult<OrganizationUnit> {
        let user_id = c.user_id().ok_or(EntityError::Forbidden)?.to_owned();
        let owner = if let Some(oid) = self.oid {
            Owner::Organization(EntityId {
                cid: Some(self.cid),
                oid: Some(oid),
                ..Default::default()
            })
        } else {
            Owner::Customer(EntityId {
                cid: Some(self.cid),
                ..Default::default()
            })
        };
        Ok(OrganizationUnit {
            id: None,
            owner,
            members: self.members,
            name: self.name,
            created: Modification::new(user_id),
            modified: None,
        })
    }
}

impl OrganizationUnit {
    pub fn as_id(&self) -> OrganizationUnitId {
        match &self.owner {
            Owner::Customer(v) => OrganizationUnitId::Customer(v.clone().into()),
            Owner::Organization(v) => OrganizationUnitId::Organization(v.clone().into()),
            _ => {
                panic!("organization unit '{}' has invalid owner", self.name);
            }
        }
    }
}

#[ComplexObject]
impl OrganizationUnit {
    async fn id(&self) -> FieldResult<OrganizationUnitId> {
        Ok(self.as_id())
    }

    async fn customer(&self, ctx: &Context<'_>) -> Option<Arc<Customer>> {
        let cache = ctx.data::<Cache>().ok();
        if cache.is_none() {
            log::warn!("qm::customer::Cache is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        if let Some(id) = self.owner.customer() {
            cache.customer().customer_by_id(&id.id).await
        } else {
            None
        }
    }

    async fn organization(&self, ctx: &Context<'_>) -> Option<Arc<Organization>> {
        let cache = ctx.data::<Cache>().ok();
        if cache.is_none() {
            log::warn!("qm::customer::Cache is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        if let Some(v) = self.owner.organization() {
            cache.customer().organization_by_id(&v).await
        } else {
            None
        }
    }
}

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct OrganizationUnitList {
    pub items: Vec<OrganizationUnit>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

impl<'a> TryInto<OrganizationUnitId> for &'a OrganizationUnit {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<OrganizationUnitId, Self::Error> {
        Ok(self.as_id())
    }
}

impl NewList<OrganizationUnit> for OrganizationUnitList {
    fn new(
        items: Vec<OrganizationUnit>,
        limit: Option<i64>,
        total: Option<i64>,
        page: Option<i64>,
    ) -> Self {
        Self {
            items,
            limit,
            total,
            page,
        }
    }
}
