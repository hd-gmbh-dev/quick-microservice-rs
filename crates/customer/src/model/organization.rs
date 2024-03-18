use std::sync::Arc;

use crate::cache::Cache;
use crate::model::CreateUserInput;
use async_graphql::Context;
use async_graphql::{ComplexObject, FieldResult, InputObject, SimpleObject};
use qm_entity::error::{EntityError, EntityResult};
use qm_entity::ids::{CustomerResourceId, EntityId, OrganizationId, ID};
use qm_entity::list::NewList;
use qm_entity::model::Modification;
use qm_entity::{Create, UserId};
use serde::{Deserialize, Serialize};

use super::{Customer, Owner};

#[derive(Debug, InputObject)]
pub struct CreateOrganizationInput {
    pub name: String,
    pub initial_user: Option<CreateUserInput>,
}

#[derive(Debug, InputObject)]
pub struct UpdateOrganizationInput {
    pub organization: OrganizationId,
    pub name: Option<String>,
}

#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
#[graphql(complex)]
pub struct Organization {
    #[graphql(skip)]
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    #[graphql(skip)]
    pub owner: Owner,
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

impl Organization {
    pub fn as_id(&self) -> OrganizationId {
        self.owner
            .customer()
            .zip(self.id.clone())
            .map(|(customer_id, id)| OrganizationId {
                cid: customer_id.id,
                id,
            })
            .unwrap_or_else(|| panic!("organization '{}' is invalid", &self.name))
    }
}

#[ComplexObject]
impl Organization {
    pub async fn id(&self) -> FieldResult<OrganizationId> {
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
            cache.customer().customer_by_id(id.as_ref()).await
        } else {
            None
        }
    }
}

impl AsMut<Option<ID>> for Organization {
    fn as_mut(&mut self) -> &mut Option<ID> {
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
            id: None,
            owner: Owner::Customer(EntityId::default().with_customer(self.0)),
            name: self.1,
            created: Modification::new(user_id),
            modified: None,
        })
    }
}

impl<'a> TryInto<CustomerResourceId> for &'a Organization {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<CustomerResourceId, Self::Error> {
        Ok(self.as_id())
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
