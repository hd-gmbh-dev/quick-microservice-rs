use async_graphql::InputObject;
use async_graphql::SimpleObject;
use std::str::FromStr;
use std::sync::Arc;

use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use serde::{Deserialize, Serialize};

use qm_mongodb::bson::oid::ObjectId;
pub type ID = Arc<ObjectId>;

pub const EMPTY_ID: &str = "000000000000000000000000";

fn parse_object_id(id: &str) -> anyhow::Result<Option<ID>> {
    if id == EMPTY_ID {
        Ok(None)
    } else {
        Ok(Some(Arc::new(ObjectId::from_str(id)?)))
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct EntityId {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<ID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oid: Option<ID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iid: Option<ID>,
}

impl EntityId {
    pub fn with_customer(mut self, cid: ID) -> Self {
        self.cid = Some(cid);
        self
    }

    pub fn as_customer_id(&self) -> Option<CustomerId> {
        self.id.clone().map(|id| CustomerId { id })
    }

    pub fn as_organization_id(&self) -> Option<OrganizationId> {
        self.cid
            .clone()
            .zip(self.id.clone())
            .map(|(cid, id)| OrganizationId { cid, id })
    }

    pub fn as_organization_unit_id(&self) -> Option<OrganizationUnitId> {
        if let Some(oid) = self.oid.clone() {
            self.cid.clone().zip(self.id.clone()).map(|(cid, id)| {
                OrganizationUnitId::Organization(OrganizationResourceId { cid, oid, id })
            })
        } else {
            self.cid
                .clone()
                .zip(self.id.clone())
                .map(|(cid, id)| OrganizationUnitId::Customer(CustomerResourceId { cid, id }))
        }
    }

    pub fn as_institution_id(&self) -> Option<InstitutionId> {
        self.cid
            .clone()
            .zip(self.oid.clone().zip(self.id.clone()))
            .map(|(cid, (oid, id))| InstitutionId { cid, oid, id })
    }
}

pub type EntityIds = Arc<[EntityId]>;

impl FromStr for EntityId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let l = s.len();
        match l {
            24 => Ok(Self {
                cid: parse_object_id(&s[0..24])?,
                oid: None,
                iid: None,
                id: None,
            }),
            48 => Ok(Self {
                cid: parse_object_id(&s[0..24])?,
                oid: parse_object_id(&s[24..48])?,
                iid: None,
                id: None,
            }),
            72 => Ok(Self {
                cid: parse_object_id(&s[0..24])?,
                oid: parse_object_id(&s[24..48])?,
                iid: parse_object_id(&s[48..72])?,
                id: None,
            }),
            96 => Ok(Self {
                cid: parse_object_id(&s[0..24])?,
                oid: parse_object_id(&s[24..48])?,
                iid: parse_object_id(&s[48..72])?,
                id: parse_object_id(&s[72..96])?,
            }),
            _ => Err(anyhow::anyhow!(
                "invalid length, EntityId should have 24, 48, 72 or 96 characters .. it has {l} characters"
            )),
        }
    }
}

#[Scalar]
impl ScalarType for EntityId {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            // Parse the integer value
            Ok(
                EntityId::from_str(value)
                    .map_err(|err| InputValueError::custom(err.to_string()))?,
            )
        } else {
            // If the type does not match
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(
            [
                self.cid
                    .as_ref()
                    .map(|v| v.to_hex())
                    .as_deref()
                    .unwrap_or(EMPTY_ID),
                self.oid
                    .as_ref()
                    .map(|v| v.to_hex())
                    .as_deref()
                    .unwrap_or(EMPTY_ID),
                self.iid
                    .as_ref()
                    .map(|v| v.to_hex())
                    .as_deref()
                    .unwrap_or(EMPTY_ID),
                self.id
                    .as_ref()
                    .map(|v| v.to_hex())
                    .as_deref()
                    .unwrap_or(EMPTY_ID),
            ]
            .join(""),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct CustomerId {
    #[serde(rename = "_id")]
    pub id: ID,
}

impl From<CustomerId> for EntityId {
    fn from(value: CustomerId) -> Self {
        Self {
            id: Some(value.id),
            cid: None,
            oid: None,
            iid: None,
        }
    }
}

impl AsRef<ObjectId> for CustomerId {
    fn as_ref(&self) -> &ObjectId {
        &self.id
    }
}

impl std::fmt::Display for CustomerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id.to_hex())
    }
}

impl FromStr for CustomerId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let l = s.len();
        if l != 24 {
            anyhow::bail!(
                "invalid length, CustomerId should have 24 characters .. it has {l} characters"
            );
        }
        Ok(Self {
            id: parse_object_id(&s[0..24])?
                .ok_or(anyhow::anyhow!("'id' is required on CustomerId"))?,
        })
    }
}

#[Scalar]
impl ScalarType for CustomerId {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            // Parse the integer value
            Ok(CustomerId::from_str(value)
                .map_err(|err| InputValueError::custom(err.to_string()))?)
        } else {
            // If the type does not match
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.id.to_hex())
    }
}

#[derive(
    Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct CustomerResourceId {
    #[serde(rename = "_id")]
    pub id: ID,
    pub cid: ID,
}

pub struct OrganizationIdRef<'a> {
    pub cid: &'a ObjectId,
    pub oid: &'a ObjectId,
}

impl<'a> Into<OrganizationIdRef<'a>> for &'a CustomerResourceId {
    fn into(self) -> OrganizationIdRef<'a> {
        OrganizationIdRef {
            cid: self.cid.as_ref(),
            oid: self.id.as_ref(),
        }
    }
}

impl From<CustomerResourceId> for EntityId {
    fn from(value: CustomerResourceId) -> Self {
        Self {
            id: Some(value.id),
            cid: Some(value.cid),
            oid: None,
            iid: None,
        }
    }
}

impl FromStr for CustomerResourceId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 48 {
            anyhow::bail!("invalid length, CustomerResourceId should have 48 characters");
        }
        Ok(Self {
            cid: parse_object_id(&s[0..24])?
                .ok_or(anyhow::anyhow!("'cid' is required on CustomerResourceId"))?,
            id: parse_object_id(&s[24..48])?
                .ok_or(anyhow::anyhow!("'oid' is required on CustomerResourceId"))?,
        })
    }
}

#[Scalar]
impl ScalarType for OrganizationId {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            // Parse the integer value
            Ok(CustomerResourceId::from_str(value)
                .map_err(|err| InputValueError::custom(err.to_string()))?)
        } else {
            // If the type does not match
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String([self.cid.to_hex().as_str(), self.id.to_hex().as_str()].join(""))
    }
}

#[derive(
    Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct OrganizationResourceId {
    #[serde(rename = "_id")]
    pub id: ID,
    pub cid: ID,
    pub oid: ID,
}

pub struct InstitutionIdRef<'a> {
    pub cid: &'a ObjectId,
    pub oid: &'a ObjectId,
    pub iid: &'a ObjectId,
}

impl<'a> Into<InstitutionIdRef<'a>> for &'a OrganizationResourceId {
    fn into(self) -> InstitutionIdRef<'a> {
        InstitutionIdRef {
            cid: self.cid.as_ref(),
            oid: self.oid.as_ref(),
            iid: self.id.as_ref(),
        }
    }
}

impl From<OrganizationResourceId> for EntityId {
    fn from(value: OrganizationResourceId) -> Self {
        Self {
            id: Some(value.id),
            cid: Some(value.cid),
            oid: Some(value.oid),
            iid: None,
        }
    }
}

impl FromStr for OrganizationResourceId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 72 {
            anyhow::bail!("invalid length, OrganizationResourceId should have 72 characters");
        }
        Ok(Self {
            cid: parse_object_id(&s[0..24])?.ok_or(anyhow::anyhow!(
                "'cid' is required on OrganizationResourceId"
            ))?,
            oid: parse_object_id(&s[24..48])?.ok_or(anyhow::anyhow!(
                "'oid' is required on OrganizationResourceId"
            ))?,
            id: parse_object_id(&s[48..72])?.ok_or(anyhow::anyhow!(
                "'id' is required on OrganizationResourceId"
            ))?,
        })
    }
}

#[Scalar]
impl ScalarType for InstitutionId {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            // Parse the integer value
            Ok(OrganizationResourceId::from_str(value)
                .map_err(|err| InputValueError::custom(err.to_string()))?)
        } else {
            // If the type does not match
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(
            [
                self.cid.to_hex().as_str(),
                self.oid.to_hex().as_str(),
                self.id.to_hex().as_str(),
            ]
            .join(""),
        )
    }
}

#[derive(
    Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct InstitutionResourceId {
    #[serde(rename = "_id")]
    pub id: ID,
    pub cid: ID,
    pub oid: ID,
    pub iid: ID,
}

impl From<InstitutionResourceId> for EntityId {
    fn from(value: InstitutionResourceId) -> Self {
        Self {
            id: Some(value.id),
            cid: Some(value.cid),
            oid: Some(value.oid),
            iid: Some(value.iid),
        }
    }
}

impl FromStr for InstitutionResourceId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 96 {
            anyhow::bail!("invalid length, InstitutionResourceId should have 96 characters");
        }
        Ok(Self {
            cid: parse_object_id(&s[0..24])?.ok_or(anyhow::anyhow!(
                "'cid' is required on InstitutionResourceId"
            ))?,
            oid: parse_object_id(&s[24..48])?.ok_or(anyhow::anyhow!(
                "'oid' is required on InstitutionResourceId"
            ))?,
            iid: parse_object_id(&s[48..72])?.ok_or(anyhow::anyhow!(
                "'iid' is required on InstitutionResourceId"
            ))?,
            id: parse_object_id(&s[72..96])?
                .ok_or(anyhow::anyhow!("'id' is required on InstitutionResourceId"))?,
        })
    }
}

#[Scalar]
impl ScalarType for InstitutionResourceId {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            // Parse the integer value
            Ok(InstitutionResourceId::from_str(value)
                .map_err(|err| InputValueError::custom(err.to_string()))?)
        } else {
            // If the type does not match
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(
            [
                self.cid.to_hex().as_str(),
                self.oid.to_hex().as_str(),
                self.iid.to_hex().as_str(),
                self.id.to_hex().as_str(),
            ]
            .join(""),
        )
    }
}

#[derive(
    Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum OrganizationUnitId {
    Customer(CustomerResourceId),
    Organization(OrganizationResourceId),
}

impl TryFrom<EntityId> for OrganizationUnitId {
    type Error = anyhow::Error;

    fn try_from(value: EntityId) -> Result<Self, Self::Error> {
        let cid = value.cid.ok_or(anyhow::anyhow!("cid is missing"))?;
        let uid = value.id.ok_or(anyhow::anyhow!("id is missing"))?;
        if let Some(oid) = value.oid {
            Ok(OrganizationUnitId::Organization(OrganizationResourceId {
                cid,
                oid,
                id: uid,
            }))
        } else {
            Ok(OrganizationUnitId::Customer(CustomerResourceId {
                cid,
                id: uid,
            }))
        }
    }
}

impl FromStr for OrganizationUnitId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let l = s.len();
        if l == 72 {
            return Ok(Self::Organization(OrganizationResourceId {
                cid: parse_object_id(&s[0..24])?.ok_or(anyhow::anyhow!(
                    "'cid' is required on OrganizationUnitId::Organization"
                ))?,
                oid: parse_object_id(&s[24..48])?.ok_or(anyhow::anyhow!(
                    "'oid' is required on OrganizationUnitId::Organization"
                ))?,
                id: parse_object_id(&s[48..72])?.ok_or(anyhow::anyhow!(
                    "'id' is required on OrganizationUnitId::Organization"
                ))?,
            }));
        }
        if l == 48 {
            return Ok(Self::Customer(CustomerResourceId {
                cid: parse_object_id(&s[0..24])?.ok_or(anyhow::anyhow!(
                    "'cid' is required on OrganizationUnitId::Customer"
                ))?,
                id: parse_object_id(&s[24..48])?.ok_or(anyhow::anyhow!(
                    "'id' is required on OrganizationUnitId::Customer"
                ))?,
            }));
        }

        anyhow::bail!("invalid length, OrganizationUnitId should have 48 or 72 characters .. it has {l} characters")
    }
}

#[Scalar]
impl ScalarType for OrganizationUnitId {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            // Parse the integer value
            Ok(OrganizationUnitId::from_str(value)
                .map_err(|err| InputValueError::custom(err.to_string()))?)
        } else {
            // If the type does not match
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        match self {
            OrganizationUnitId::Customer(v) => v.to_value(),
            OrganizationUnitId::Organization(v) => v.to_value(),
        }
    }
}

#[derive(
    Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct OrganizationUnitResourceId {
    #[serde(rename = "_id")]
    pub id: ID,
    pub cid: ID,
    pub oid: Option<ID>,
    pub uid: ID,
}

impl From<OrganizationUnitResourceId> for EntityId {
    fn from(value: OrganizationUnitResourceId) -> Self {
        Self {
            id: Some(value.id),
            cid: Some(value.cid),
            oid: value.oid,
            iid: Some(value.uid),
        }
    }
}

impl FromStr for OrganizationUnitResourceId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let l = s.len();
        if l == 96 {
            return Ok(Self {
                cid: parse_object_id(&s[0..24])?.ok_or(anyhow::anyhow!(
                    "'cid' is required on OrganizationUnitResourceId"
                ))?,
                oid: Some(parse_object_id(&s[24..48])?.ok_or(anyhow::anyhow!(
                    "'oid' is required on OrganizationUnitResourceId"
                ))?),
                uid: parse_object_id(&s[48..72])?.ok_or(anyhow::anyhow!(
                    "'iid' is required on OrganizationUnitResourceId"
                ))?,
                id: parse_object_id(&s[72..96])?.ok_or(anyhow::anyhow!(
                    "'id' is required on OrganizationUnitResourceId"
                ))?,
            });
        }
        if l == 72 {
            return Ok(Self {
                cid: parse_object_id(&s[0..24])?.ok_or(anyhow::anyhow!(
                    "'cid' is required on OrganizationUnitResourceId"
                ))?,
                oid: None,
                uid: parse_object_id(&s[24..48])?.ok_or(anyhow::anyhow!(
                    "'iid' is required on OrganizationUnitResourceId"
                ))?,
                id: parse_object_id(&s[48..72])?.ok_or(anyhow::anyhow!(
                    "'id' is required on OrganizationUnitResourceId"
                ))?,
            });
        }
        anyhow::bail!("invalid length, OrganizationUnitResourceId should have 72 or 96 characters .. it has {l}")
    }
}

#[Scalar]
impl ScalarType for OrganizationUnitResourceId {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            // Parse the integer value
            Ok(OrganizationUnitResourceId::from_str(value)
                .map_err(|err| InputValueError::custom(err.to_string()))?)
        } else {
            // If the type does not match
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        if let Some(oid) = self.oid.as_ref() {
            Value::String(
                [
                    self.cid.to_hex().as_str(),
                    oid.to_hex().as_str(),
                    self.uid.to_hex().as_str(),
                    self.id.to_hex().as_str(),
                ]
                .join(""),
            )
        } else {
            Value::String(
                [
                    self.cid.to_hex().as_str(),
                    self.uid.to_hex().as_str(),
                    self.id.to_hex().as_str(),
                ]
                .join(""),
            )
        }
    }
}

impl From<EntityId> for CustomerId {
    fn from(value: EntityId) -> Self {
        Self {
            id: value.id.unwrap_or_default(),
        }
    }
}

impl From<EntityId> for OrganizationId {
    fn from(value: EntityId) -> Self {
        Self {
            cid: value.cid.unwrap_or_default(),
            id: value.id.unwrap_or_default(),
        }
    }
}

impl std::fmt::Display for OrganizationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.cid.to_hex(), self.id.to_hex())
    }
}

impl From<EntityId> for InstitutionId {
    fn from(value: EntityId) -> Self {
        Self {
            cid: value.cid.unwrap_or_default(),
            oid: value.oid.unwrap_or_default(),
            id: value.id.unwrap_or_default(),
        }
    }
}

impl std::fmt::Display for InstitutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.cid.to_hex(),
            self.oid.to_hex(),
            self.id.to_hex()
        )
    }
}

impl std::fmt::Display for OrganizationUnitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrganizationUnitId::Customer(v) => v.fmt(f),
            OrganizationUnitId::Organization(v) => v.fmt(f),
        }
    }
}

#[derive(Default, Debug, Clone, SimpleObject, InputObject, Serialize, Deserialize)]
#[graphql(input_name = "MemberIdInput")]
pub struct MemberId {
    pub cid: ID,
    pub oid: ID,
    pub iid: ID,
}

impl<'a> From<&'a OrganizationResourceId> for CustomerResourceId {
    fn from(val: &'a OrganizationResourceId) -> Self {
        CustomerResourceId {
            cid: val.cid.clone(),
            id: val.oid.clone(),
        }
    }
}

impl PartialEq<CustomerResourceId> for OrganizationResourceId {
    fn eq(&self, other: &CustomerResourceId) -> bool {
        self.cid.eq(&other.cid) && self.oid.eq(&other.id)
    }
}

pub type CustomerIds = Arc<[ID]>;
pub type OrganizationId = CustomerResourceId;
pub type OrganizationIds = Arc<[OrganizationId]>;
pub type InstitutionId = OrganizationResourceId;
pub type InstitutionIds = Arc<[InstitutionId]>;
pub type OrganizationUnitIds = Arc<[OrganizationUnitId]>;

pub struct Cid;
pub struct Oid;
pub struct Iid;
pub struct Uid;

pub trait SelectId<T> {
    fn id(&self) -> &ObjectId;
}

impl SelectId<Cid> for OrganizationId {
    fn id(&self) -> &ObjectId {
        self.cid.as_ref()
    }
}

impl SelectId<Oid> for OrganizationId {
    fn id(&self) -> &ObjectId {
        self.id.as_ref()
    }
}

impl SelectId<Cid> for InstitutionId {
    fn id(&self) -> &ObjectId {
        self.cid.as_ref()
    }
}
impl SelectId<Oid> for InstitutionId {
    fn id(&self) -> &ObjectId {
        self.oid.as_ref()
    }
}
impl SelectId<Iid> for InstitutionId {
    fn id(&self) -> &ObjectId {
        self.id.as_ref()
    }
}

impl SelectId<Cid> for OrganizationUnitId {
    fn id(&self) -> &ObjectId {
        match self {
            OrganizationUnitId::Customer(v) => v.cid.as_ref(),
            OrganizationUnitId::Organization(v) => v.cid.as_ref(),
        }
    }
}

impl SelectId<Uid> for OrganizationUnitId {
    fn id(&self) -> &ObjectId {
        match self {
            OrganizationUnitId::Customer(v) => v.id.as_ref(),
            OrganizationUnitId::Organization(v) => v.id.as_ref(),
        }
    }
}

pub fn select_ids<'a, T, U>(ids: &'a [T]) -> Vec<&'a ObjectId>
where
    T: SelectId<U>,
{
    ids.iter().map(|v| v.id()).collect()
}
