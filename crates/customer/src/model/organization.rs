use async_graphql::{ComplexObject, InputObject, SimpleObject};
use serde::{Deserialize, Serialize};

use crate::model::UserInput;
use qm_entity::error::{EntityError, EntityResult};
use qm_entity::ids::{CustomerResourceId, EntityId, OrganizationId, ID};
use qm_entity::model::Modification;
use qm_entity::{Create, UserId};

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
    pub async fn cid(&self) -> Option<ID> {
        self.id.cid.clone()
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
