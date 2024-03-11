use std::sync::Arc;

use crate::cache::Cache;
use crate::model::UserInput;
use async_graphql::Context;
use async_graphql::{ComplexObject, FieldResult, InputObject, SimpleObject};
use qm_entity::error::{EntityError, EntityResult};
use qm_entity::ids::{CustomerResourceId, EntityId, OrganizationId, ID};
use qm_entity::list::NewList;
use qm_entity::model::Modification;
use qm_entity::{Create, UserId};
use serde::{Deserialize, Serialize};

use super::Customer;

#[derive(Debug, InputObject)]
pub struct CreateOrganizationInput {
    pub name: String,
    pub initial_user: Option<UserInput>,
}

#[derive(Debug, InputObject)]
pub struct UpdateOrganizationInput {
    pub organization: OrganizationId,
    pub name: Option<String>,
}

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
#[graphql(complex)]
pub struct Organization {
    #[graphql(skip)]
    #[serde(flatten)]
    pub id: EntityId,
    pub name: String,
    pub created: Modification,
    pub modified: Option<Modification>,
}

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct OrganizationList {
    pub items: Vec<Organization>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

#[ComplexObject]
impl Organization {
    async fn id(&self) -> FieldResult<OrganizationId> {
        Ok(self.id.clone().try_into()?)
    }

    async fn customer(&self, ctx: &Context<'_>) -> Option<Arc<Customer>> {
        if let Some((cache, id)) = ctx.data::<Cache>().ok().zip(self.id.cid.as_ref()) {
            cache.customer().customer_by_id(id).await
        } else {
            log::warn!("qm::customer::Cache is not installed in schema context");
            None
        }
    }
}

impl AsMut<EntityId> for Organization {
    fn as_mut(&mut self) -> &mut EntityId {
        &mut self.id
    }
}

pub struct OrganizationData(pub ID, pub String);

impl<C> Create<Organization, C> for OrganizationData
where
    C: UserId,
{
    fn create(self, c: &C) -> EntityResult<Organization> {
        let user_id = c.user_id().ok_or(EntityError::Forbidden)?.to_owned();
        Ok(Organization {
            id: EntityId::default().with_customer(self.0),
            name: self.1,
            created: Modification::new(user_id),
            modified: None,
        })
    }
}

impl<'a> TryInto<CustomerResourceId> for &'a Organization {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<CustomerResourceId, Self::Error> {
        Ok(CustomerResourceId {
            cid: self
                .id
                .cid
                .clone()
                .ok_or(anyhow::anyhow!("cid is missing"))?,
            id: self.id.id.clone().ok_or(anyhow::anyhow!("id is missing"))?,
        })
    }
}

impl NewList<Organization> for OrganizationList {
    fn new(
        items: Vec<Organization>,
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
