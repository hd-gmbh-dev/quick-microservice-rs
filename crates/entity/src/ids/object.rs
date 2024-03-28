use qm_mongodb::bson::oid::ObjectId;

use crate::ids::InfraContext;

use super::{
    CustomerId, CustomerUnitId, InstitutionId, InstitutionUnitId, OrganizationId,
    OrganizationUnitId,
};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<i64>,
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

impl From<InstitutionUnitId> for OwnerId {
    fn from(value: InstitutionUnitId) -> Self {
        let (cid, oid, uid) = value.unzip();
        Self {
            cid: Some(cid),
            oid: Some(oid),
            uid: Some(uid),
            ..Default::default()
        }
    }
}

impl From<CustomerUnitId> for OwnerId {
    fn from(value: CustomerUnitId) -> Self {
        let (cid, uid) = value.unzip();
        Self {
            cid: Some(cid),
            uid: Some(uid),
            ..Default::default()
        }
    }
}

impl From<OrganizationUnitId> for OwnerId {
    fn from(value: OrganizationUnitId) -> Self {
        match value {
            OrganizationUnitId::Customer(v) => v.into(),
            OrganizationUnitId::Organization(v) => v.into(),
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(transparent)]
pub struct Owner {
    #[serde(skip_serializing_if = "Owner::is_none")]
    o: OwnerType,
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(tag = "ty", content = "id")]
pub enum OwnerType {
    #[default]
    None,
    Customer(OwnerId),
    Organization(OwnerId),
    Institution(OwnerId),
    InstitutionUnit(OwnerId),
    CustomerUnit(OwnerId),
}

impl OwnerType {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl From<InfraContext> for OwnerType {
    fn from(value: InfraContext) -> Self {
        match value {
            InfraContext::Customer(v) => OwnerType::Customer(v.into()),
            InfraContext::Organization(v) => OwnerType::Organization(v.into()),
            InfraContext::Institution(v) => OwnerType::Institution(v.into()),
            InfraContext::OrganizationUnit(v) => match v {
                OrganizationUnitId::Customer(v) => OwnerType::CustomerUnit(v.into()),
                OrganizationUnitId::Organization(v) => OwnerType::InstitutionUnit(v.into()),
            },
        }
    }
}
