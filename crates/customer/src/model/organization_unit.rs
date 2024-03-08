use qm_entity::error::{EntityError, EntityResult};
use qm_entity::ids::{EntityId, MemberId, OrganizationUnitId, StrictOrganizationUnitId, ID};

use qm_entity::list::NewList;
use qm_entity::{Create, UserId};

use async_graphql::{ComplexObject, FieldResult, InputObject, SimpleObject};
use serde::{Deserialize, Serialize};

use crate::model::UserInput;
use qm_entity::model::Modification;

#[derive(Debug, InputObject)]
pub struct CreateOrganizationUnitInput {
    pub name: String,
    pub initial_user: Option<UserInput>,
    pub members: Vec<MemberId>,
}

#[derive(Debug, InputObject)]
pub struct UpdateOrganizationUnitInput {
    pub organization_unit: StrictOrganizationUnitId,
    pub name: Option<String>,
}

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
#[graphql(complex)]
pub struct OrganizationUnit {
    #[graphql(skip)]
    #[serde(flatten)]
    pub id: EntityId,
    pub name: String,
    pub members: Vec<MemberId>,
    pub created: Modification,
    pub modified: Option<Modification>,
}

impl AsMut<EntityId> for OrganizationUnit {
    fn as_mut(&mut self) -> &mut EntityId {
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
        Ok(OrganizationUnit {
            id: EntityId {
                cid: Some(self.cid),
                oid: self.oid,
                ..Default::default()
            },
            members: self.members,
            name: self.name,
            created: Modification::new(user_id),
            modified: None,
        })
    }
}

#[ComplexObject]
impl OrganizationUnit {
    async fn id(&self) -> FieldResult<OrganizationUnitId> {
        Ok(self.id.clone().try_into()?)
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
        self.id.clone().try_into()
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
