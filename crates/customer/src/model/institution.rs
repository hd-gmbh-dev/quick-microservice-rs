use std::sync::Arc;

use async_graphql::{ComplexObject, Context, FieldResult, InputObject, SimpleObject};
use qm_entity::list::NewList;
use serde::{Deserialize, Serialize};

use crate::cache::Cache;
use crate::model::UserInput;
use qm_entity::error::{EntityError, EntityResult};
use qm_entity::ids::{
    CustomerResourceId, EntityId, InstitutionId, OrganizationId, OrganizationResourceId,
    StrictInstitutionId,
};
use qm_entity::model::Modification;
use qm_entity::{Create, UserId};

use super::{Customer, Organization};

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
    #[graphql(skip)]
    #[serde(flatten)]
    pub id: EntityId,
    pub name: String,
    pub created: Modification,
    pub modified: Option<Modification>,
}

impl TryInto<StrictInstitutionId> for Institution {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<StrictInstitutionId, Self::Error> {
        let cid = self.id.cid.ok_or(anyhow::anyhow!("'cid' is required"))?;
        let oid = self.id.oid.ok_or(anyhow::anyhow!("'oid' is required"))?;
        let id = self.id.id.ok_or(anyhow::anyhow!("'id' is required"))?;
        Ok((cid, oid, id).into())
    }
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
    async fn id(&self) -> FieldResult<InstitutionId> {
        Ok(self.id.clone().into())
    }

    async fn customer(&self, ctx: &Context<'_>) -> Option<Arc<Customer>> {
        if let Some((cache, id)) = ctx.data::<Cache>().ok().zip(self.id.cid.as_ref()) {
            cache.customer().customer_by_id(id).await
        } else {
            log::warn!("qm::customer::Cache is not installed in schema context");
            None
        }
    }

    async fn organization(&self, ctx: &Context<'_>) -> Option<Arc<Organization>> {
        if let Some((cache, (cid, oid))) = ctx
            .data::<Cache>()
            .ok()
            .zip(self.id.cid.as_ref().zip(self.id.oid.as_ref()))
        {
            cache
                .customer()
                .organization_by_id(&CustomerResourceId {
                    cid: cid.clone(),
                    id: oid.clone(),
                })
                .await
        } else {
            log::warn!("qm::customer::Cache is not installed in schema context");
            None
        }
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
