use qm_mongodb::bson::oid::ObjectId;

use crate::ids::InfraContext;

use super::{CustomerId, InstitutionId, OrganizationId};

pub type ID = ObjectId;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct OwnerId {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oid: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iid: Option<i64>,
}

impl From<CustomerId> for OwnerId {
    fn from(value: CustomerId) -> Self {
        Self {
            cid: Some(value.unzip()),
            ..Default::default()
        }
    }
}

impl From<OrganizationId> for OwnerId {
    fn from(value: OrganizationId) -> Self {
        let (cid, oid) = value.unzip();
        Self {
            cid: Some(cid),
            oid: Some(oid),
            ..Default::default()
        }
    }
}

impl From<InstitutionId> for OwnerId {
    fn from(value: InstitutionId) -> Self {
        let (cid, oid, iid) = value.unzip();
        Self {
            cid: Some(cid),
            oid: Some(oid),
            iid: Some(iid),
            ..Default::default()
        }
    }
}

impl From<InfraContext> for OwnerId {
    fn from(value: InfraContext) -> Self {
        match value {
            InfraContext::Customer(v) => v.into(),
            InfraContext::Organization(v) => v.into(),
            InfraContext::Institution(v) => v.into(),
        }
    }
}

impl<'a> TryFrom<&'a OwnerId> for InfraContext {
    type Error = anyhow::Error;

    fn try_from(value: &'a OwnerId) -> Result<Self, Self::Error> {
        match value {
            OwnerId {
                cid: Some(cid),
                oid: Some(oid),
                iid: Some(iid),
            } => Ok(InfraContext::Institution((*cid, *oid, *iid).into())),
            OwnerId {
                cid: Some(cid),
                oid: Some(oid),
                iid: None,
            } => Ok(InfraContext::Organization((*cid, *oid).into())),
            OwnerId {
                cid: Some(cid),
                oid: None,
                iid: None,
            } => Ok(InfraContext::Customer((*cid).into())),
            _ => anyhow::bail!("invalid owner id"),
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(transparent)]
pub struct Owner {
    #[serde(skip_serializing_if = "Owner::is_none")]
    o: OwnerType,
}

impl Owner {
    pub fn new(o: OwnerType) -> Self {
        Self { o }
    }

    pub fn as_owner_id(&self) -> Option<&OwnerId> {
        self.o.as_owner_id()
    }
}

impl From<InfraContext> for Owner {
    fn from(value: InfraContext) -> Self {
        Self { o: value.into() }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(tag = "ty", content = "id")]
pub enum OwnerType {
    #[default]
    None,
    Customer(OwnerId),
    Organization(OwnerId),
    Institution(OwnerId),
}

impl OwnerType {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn as_owner_id(&self) -> Option<&OwnerId> {
        match self {
            OwnerType::None => None,
            OwnerType::Customer(id) | OwnerType::Organization(id) | OwnerType::Institution(id) => {
                Some(id)
            }
        }
    }
}

impl From<InfraContext> for OwnerType {
    fn from(value: InfraContext) -> Self {
        match value {
            InfraContext::Customer(v) => OwnerType::Customer(v.into()),
            InfraContext::Organization(v) => OwnerType::Organization(v.into()),
            InfraContext::Institution(v) => OwnerType::Institution(v.into()),
        }
    }
}
