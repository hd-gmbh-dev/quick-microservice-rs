use async_graphql::{ComplexObject, InputObject, SimpleObject};
use serde::{Deserialize, Serialize};

use crate::model::UserInput;
use qm_entity::error::{EntityError, EntityResult};
use qm_entity::ids::{EntityId, InstitutionId, OrganizationId, OrganizationResourceId, ID};
use qm_entity::model::Modification;
use qm_entity::{Create, UserId};

#[derive(Debug, InputObject)]
pub struct CreateInstitutionInput {
    pub name: String,
    pub initial_user: Option<UserInput>,
}

#[derive(Debug, InputObject)]
pub struct UpdateInstitutionInput {
    pub institution: InstitutionId,
    pub name: Option<String>,
}

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
#[graphql(complex)]
pub struct Institution {
    #[serde(flatten)]
    pub id: EntityId,
    pub name: String,
    pub created: Modification,
    pub modified: Option<Modification>,
}

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct InstitutionList {
    pub items: Vec<Institution>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

#[ComplexObject]
impl Institution {
    pub async fn cid(&self) -> Option<ID> {
        self.id.cid.clone()
    }

    pub async fn oid(&self) -> Option<ID> {
        self.id.oid.clone()
    }
}

impl AsMut<EntityId> for Institution {
    fn as_mut(&mut self) -> &mut EntityId {
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
            id: EntityId {
                cid: Some(self.0.cid),
                oid: Some(self.0.id),
                ..Default::default()
            },
            name: self.1,
            created: Modification::new(user_id),
            modified: None,
        })
    }
}

impl<'a> TryInto<OrganizationResourceId> for &'a Institution {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<OrganizationResourceId, Self::Error> {
        Ok(OrganizationResourceId {
            cid: self
                .id
                .cid
                .clone()
                .ok_or(anyhow::anyhow!("cid is missing"))?,
            oid: self
                .id
                .oid
                .clone()
                .ok_or(anyhow::anyhow!("oid is missing"))?,
            id: self.id.id.clone().ok_or(anyhow::anyhow!("id is missing"))?,
        })
    }
}
