use async_graphql::{ComplexObject, InputObject, SimpleObject};
use serde::{Deserialize, Serialize};

use qm_entity::error::{EntityError, EntityResult};
use qm_entity::ids::CustomerId;
use qm_entity::ids::EntityId;
use qm_entity::model::Modification;
use qm_entity::{Create, UserId};

use super::UserInput;

pub struct CustomerData(pub String);

#[derive(Debug, InputObject)]
pub struct CreateCustomerInput {
    pub name: String,
    pub initial_user: Option<UserInput>,
}

#[derive(Debug, InputObject)]
pub struct UpdateCustomerInput {
    pub customer: CustomerId,
    pub name: Option<String>,
}

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
#[graphql(complex)]
pub struct Customer {
    #[graphql(skip)]
    #[serde(flatten)]
    pub id: EntityId,
    pub name: String,
    pub created: Modification,
    pub modified: Option<Modification>,
}

#[ComplexObject]
impl Customer {
    async fn id(&self) -> CustomerId {
        CustomerId {
            id: self.id.id.clone().unwrap(),
        }
    }
}

impl AsMut<EntityId> for Customer {
    fn as_mut(&mut self) -> &mut EntityId {
        &mut self.id
    }
}

impl<C> Create<Customer, C> for CustomerData
where
    C: UserId,
{
    fn create(self, c: &C) -> EntityResult<Customer> {
        let user_id = c.user_id().ok_or(EntityError::Forbidden)?.to_owned();
        Ok(Customer {
            id: EntityId::default(),
            name: self.0,
            created: Modification::new(user_id),
            modified: None,
        })
    }
}

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct CustomerList {
    pub items: Vec<Customer>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}
