use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use qm_mongodb::bson::{self, oid::ObjectId, Bson};

use crate::ids::InfraContext;

use super::{CustomerId, InstitutionId, OrganizationId};

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
/// MongoDB ObjectId wrapper for GraphQL ID type.
pub struct ID(ObjectId);

#[Scalar]
impl ScalarType for ID {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => Ok(ID(ObjectId::parse_str(s)?)),
            Value::Object(o) => {
                let json = Value::Object(o).into_json()?;
                let bson: Bson = Bson::try_from(json)?;
                bson.as_object_id()
                    .map(ID)
                    .ok_or_else(|| InputValueError::custom("could not parse the value as an ID"))
            }
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}

impl std::ops::Deref for ID {
    type Target = ObjectId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ID {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::str::FromStr for ID {
    type Err = bson::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ObjectId::from_str(s).map(ID)
    }
}

impl From<ObjectId> for ID {
    fn from(value: ObjectId) -> Self {
        Self(value)
    }
}

impl From<ID> for Bson {
    fn from(val: ID) -> Self {
        val.0.into()
    }
}

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
/// Owner identifier that can represent a customer, organization, or institution.
pub struct OwnerId {
    /// Customer ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<i64>,
    /// Organization ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oid: Option<i64>,
    /// Institution ID.
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
/// Owner wrapper type.
pub struct Owner {
    #[serde(skip_serializing_if = "Owner::is_none")]
    o: OwnerType,
}

impl Owner {
    /// Creates a new Owner from an OwnerType.
    pub fn new(o: OwnerType) -> Self {
        Self { o }
    }

    /// Returns the OwnerId if present.
    pub fn as_owner_id(&self) -> Option<&OwnerId> {
        self.o.as_owner_id()
    }
}

impl From<InfraContext> for Owner {
    fn from(value: InfraContext) -> Self {
        Self { o: value.into() }
    }
}

/// Owner type enumeration.
#[derive(Default, serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(tag = "ty", content = "id")]
pub enum OwnerType {
    /// No owner.
    #[default]
    None,
    /// Customer owner.
    Customer(OwnerId),
    /// Organization owner.
    Organization(OwnerId),
    /// Institution owner.
    Institution(OwnerId),
}

impl OwnerType {
    /// Returns true if this is None.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns the OwnerId if present.
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
