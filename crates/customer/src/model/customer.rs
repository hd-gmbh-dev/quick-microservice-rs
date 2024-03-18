use async_graphql::{ComplexObject, InputObject, SimpleObject};
use qm_entity::list::NewList;
use serde::{Deserialize, Serialize};

use qm_entity::error::{EntityError, EntityResult};
use qm_entity::ids::{CustomerId, ID};
use qm_entity::model::Modification;
use qm_entity::{Create, UserId};

use super::CreateUserInput;

pub struct CustomerData(pub String);

#[derive(Debug, InputObject)]
pub struct CreateCustomerInput {
    pub name: String,
    pub initial_user: Option<CreateUserInput>,
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
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    pub name: String,
    pub created: Modification,
    pub modified: Option<Modification>,
}

impl Customer {
    pub fn as_id(&self) -> CustomerId {
        CustomerId {
            id: self
                .id
                .clone()
                .unwrap_or_else(|| panic!("customer '{}' is invalid, id missing", &self.name)),
        }
    }
}

#[ComplexObject]
impl Customer {
    async fn id(&self) -> CustomerId {
        self.as_id()
    }
}

impl AsMut<Option<ID>> for Customer {
    fn as_mut(&mut self) -> &mut Option<ID> {
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
            id: None,
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

impl NewList<Customer> for CustomerList {
    fn new(
        items: Vec<Customer>,
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
