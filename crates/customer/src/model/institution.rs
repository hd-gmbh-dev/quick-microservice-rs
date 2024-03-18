use std::sync::Arc;

use async_graphql::{ComplexObject, Context, FieldResult, InputObject, SimpleObject};
use qm_entity::list::NewList;
use serde::{Deserialize, Serialize};

use crate::cache::Cache;
use crate::model::CreateUserInput;
use qm_entity::error::{EntityError, EntityResult};
use qm_entity::ids::{
    EntityId, InstitutionId, OrganizationId, OrganizationResourceId, StrictInstitutionId, ID,
};
use qm_entity::model::Modification;
use qm_entity::{Create, UserId};

use super::{Customer, Organization, Owner};

#[derive(Debug, InputObject)]
pub struct CreateInstitutionInput {
    pub name: String,
    pub initial_user: Option<CreateUserInput>,
}

#[derive(Debug, InputObject)]
pub struct UpdateInstitutionInput {
    pub institution: InstitutionId,
    pub name: Option<String>,
}

#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
#[graphql(complex)]
pub struct Institution {
    #[graphql(skip)]
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    pub name: String,
    #[graphql(skip)]
    pub owner: Owner,
    pub created: Modification,
    pub modified: Option<Modification>,
}

impl TryInto<StrictInstitutionId> for Institution {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<StrictInstitutionId, Self::Error> {
        let rid = self.as_id();
        Ok((rid.cid, rid.oid, rid.id).into())
    }
}

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct InstitutionList {
    pub items: Vec<Institution>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

impl Institution {
    pub fn as_id(&self) -> InstitutionId {
        self.owner
            .organization()
            .zip(self.id.clone())
            .map(|(rid, id)| InstitutionId {
                cid: rid.cid,
                oid: rid.id,
                id,
            })
            .unwrap_or_else(|| panic!("institution '{}' is invalid", &self.name))
    }
}

#[ComplexObject]
impl Institution {
    async fn id(&self) -> FieldResult<InstitutionId> {
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

impl AsMut<Option<ID>> for Institution {
    fn as_mut(&mut self) -> &mut Option<ID> {
        &mut self.id
    }
}

pub struct InstitutionData(pub OrganizationId, pub String);

impl<C> Create<Institution, C> for InstitutionData
where
    C: UserId,
{
    fn create(self, c: &C) -> EntityResult<Institution> {
        let user_id = c.user_id().ok_or(EntityError::Forbidden)?.to_owned();
        Ok(Institution {
            id: None,
            owner: Owner::Organization(EntityId {
                cid: Some(self.0.cid),
                oid: Some(self.0.id),
                ..Default::default()
            }),
            name: self.1,
            created: Modification::new(user_id),
            modified: None,
        })
    }
}

impl<'a> TryInto<OrganizationResourceId> for &'a Institution {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<OrganizationResourceId, Self::Error> {
        Ok(self.as_id())
    }
}

impl NewList<Institution> for InstitutionList {
    fn new(
        items: Vec<Institution>,
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
