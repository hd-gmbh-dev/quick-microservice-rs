//! ID Implementations for multiple scenarios of infrastructure and ownership.
//!
//! The smallest unit has a min length of 3 characters. The biggest Resource ID can go up to 76 characters.
//!
//! |Prefix|                            Structure                                  |           Type            | min length | max length | real size  |
//! |------|-----------------------------------------------------------------------|---------------------------|------------|------------|------------|
//! |  V   | CustomerId                                                            | CustomerId                |     3      |     18     |     8      |
//! |  U   | CustomerId + ID (24 Characters)                                       | CustomerResourceId        |     27     |     42     |     20     |
//! |  T   | CustomerId + OrganizationId                                           | OrganizationId            |     5      |     35     |     16     |
//! |  S   | CustomerId + OrganizationId + ID (24 Characters)                      | OrganizationResourceId    |     29     |     59     |     28     |
//! |  R   | CustomerId + OrganizationId + InstitutionId                           | InstitutionId             |     7      |     52     |     24     |
//! |  Q   | CustomerId + OrganizationId + InstitutionId + ID (24 Characters)      | InstitutionResourceId     |     31     |     76     |     36     |
//! |  P   | CustomerId + OrganizationId + OrganizationUnitId                      | InstitutionUnitId         |     7      |     52     |     24     |
//! |  O   | CustomerId + OrganizationId + OrganizationUnitId + ID (24 Characters) | InstitutionUnitResourceId |     31     |     76     |     36     |
//! |  N   | CustomerId + OrganizationUnitId                                       | CustomerUnitId            |     5      |     35     |     16     |
//! |  M   | CustomerId + OrganizationUnitId + ID (24 Characters)                  | CustomerUnitResourceId    |     29     |     59     |     28     |
//!

use async_graphql::OneofObject;

use sqlx::postgres::PgArgumentBuffer;
use sqlx::Encode;
use sqlx::Postgres;
use std::fmt::Write;
use std::str::FromStr;

use super::ID;

pub const CUSTOMER_ID_PREFIX: char = 'V';
pub const CUSTOMER_RESOURCE_ID_PREFIX: char = 'U';
pub const ORGANIZATION_ID_PREFIX: char = 'T';
pub const ORGANIZATION_RESOURCE_ID_PREFIX: char = 'S';
pub const INSTITUTION_ID_PREFIX: char = 'R';
pub const INSTITUTION_RESOURCE_ID_PREFIX: char = 'Q';
pub const INSTITUTION_UNIT_ID_PREFIX: char = 'P';
pub const INSTITUTION_UNIT_RESOURCE_ID_PREFIX: char = 'O';
pub const CUSTOMER_UNIT_ID_PREFIX: char = 'N';
pub const CUSTOMER_UNIT_RESOURCE_ID_PREFIX: char = 'M';
pub const ID_LENGTH: usize = 24;

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
#[repr(C)]
#[serde(transparent)]
pub struct InfraId(i64);
impl AsRef<i64> for InfraId {
    fn as_ref(&self) -> &i64 {
        &self.0
    }
}
impl std::ops::Deref for InfraId {
    type Target = i64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<i64> for InfraId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}
impl From<InfraId> for i64 {
    fn from(val: InfraId) -> Self {
        val.0
    }
}

impl<'q> Encode<'q, Postgres> for InfraId {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> sqlx::encode::IsNull {
        buf.extend(&self.0.to_be_bytes());

        sqlx::encode::IsNull::No
    }
}

trait Prefixed {
    const PREFIX: char;
}

macro_rules! impl_id {
    ($t:ty, $p:expr) => {
        impl $t {
            pub fn parse(value: &str) -> anyhow::Result<Self> {
                Self::from_str(value)
            }
        }

        impl Prefixed for $t {
            const PREFIX: char = $p;
        }
    };
}

macro_rules! impl_display_for_id {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_char(Self::PREFIX)?;
                f.write_str(&self.to_hex())
            }
        }
    };
}

macro_rules! impl_display_for_resource_id {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_char(Self::PREFIX)?;
                f.write_str(&self.parent().to_hex())?;
                f.write_str(&self.id.to_hex())
            }
        }
    };
}

macro_rules! impl_customer_id_from_ty {
    ($n:ty) => {
        impl From<$n> for CustomerId {
            fn from(value: $n) -> Self {
                CustomerId { cid: value as i64 }
            }
        }
    };
}

macro_rules! impl_organization_id_from_ty_tuple {
    ($n:ty) => {
        impl From<($n, $n)> for OrganizationId {
            fn from(value: ($n, $n)) -> Self {
                OrganizationId {
                    cid: value.0 as i64,
                    oid: value.1 as i64,
                }
            }
        }
    };
}

macro_rules! impl_institution_id_from_ty_tuple {
    ($n:ty) => {
        impl From<($n, $n, $n)> for InstitutionId {
            fn from(value: ($n, $n, $n)) -> Self {
                InstitutionId {
                    cid: value.0 as i64,
                    oid: value.1 as i64,
                    iid: value.2 as i64,
                }
            }
        }
        impl From<(($n, $n), $n)> for InstitutionId {
            fn from(value: (($n, $n), $n)) -> Self {
                InstitutionId {
                    cid: value.0 .0 as i64,
                    oid: value.0 .1 as i64,
                    iid: value.1 as i64,
                }
            }
        }
    };
}

macro_rules! impl_institution_unit_id_from_ty_tuple {
    ($n:ty) => {
        impl From<($n, $n, $n)> for InstitutionUnitId {
            fn from(value: ($n, $n, $n)) -> Self {
                InstitutionUnitId {
                    cid: value.0 as i64,
                    oid: value.1 as i64,
                    uid: value.2 as i64,
                }
            }
        }
        impl From<(($n, $n), $n)> for InstitutionUnitId {
            fn from(value: (($n, $n), $n)) -> Self {
                InstitutionUnitId {
                    cid: value.0 .0 as i64,
                    oid: value.0 .1 as i64,
                    uid: value.1 as i64,
                }
            }
        }
    };
}

macro_rules! impl_customer_unit_id_from_ty_tuple {
    ($n:ty) => {
        impl From<($n, $n)> for CustomerUnitId {
            fn from(value: ($n, $n)) -> Self {
                CustomerUnitId {
                    cid: value.0 as i64,
                    uid: value.1 as i64,
                }
            }
        }
    };
}

macro_rules! impl_customer_resource_id_from_ty_tuple {
    ($n:ty) => {
        impl From<($n, ID)> for CustomerResourceId {
            fn from(value: ($n, ID)) -> Self {
                CustomerResourceId {
                    cid: value.0 as i64,
                    id: value.1,
                }
            }
        }
    };
}

macro_rules! impl_organization_resource_id_from_ty_tuple {
    ($n:ty) => {
        impl From<($n, $n, ID)> for OrganizationResourceId {
            fn from(value: ($n, $n, ID)) -> Self {
                OrganizationResourceId {
                    cid: value.0 as i64,
                    oid: value.1 as i64,
                    id: value.2,
                }
            }
        }
    };
}

macro_rules! impl_institution_resource_id_from_ty_tuple {
    ($n:ty) => {
        impl From<($n, $n, $n, ID)> for InstitutionResourceId {
            fn from(value: ($n, $n, $n, ID)) -> Self {
                InstitutionResourceId {
                    cid: value.0 as i64,
                    oid: value.1 as i64,
                    iid: value.2 as i64,
                    id: value.3,
                }
            }
        }
    };
}

/// Customer Id
///
/// - Prefix: V
/// - Min Length: 3
/// - Max Length: 18
/// - Real size: 8
///
/// # Examples
///
/// ```rust
/// use qm_entity::ids::CustomerId;
///
/// let id1 = CustomerId::parse("V01").expect("Customer Id");
/// let id2 = CustomerId::parse("V120").expect("Customer Id");
/// let id3 = CustomerId::parse("V2500").expect("Customer Id");
///
/// assert_eq!(1, id1.unzip());
/// assert_eq!(0x20, id2.unzip());
/// assert_eq!(0x500, id3.unzip());
/// ```
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
    async_graphql::Description,
)]
pub struct CustomerId {
    cid: i64,
}

impl CustomerId {
    fn to_hex(self) -> String {
        StringWriter::from(self.cid).into_inner()
    }

    pub fn unzip(&self) -> i64 {
        self.cid
    }
}

impl FromStr for CustomerId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(Self::PREFIX) {
            anyhow::bail!("Invalid CustomerId")
        }
        StringParser::<1>::new(&s[1..])
            .next()
            .map(From::from)
            .ok_or(anyhow::anyhow!("unable to parse '{s}' into CustomerId"))
    }
}

impl<'a> From<&'a CustomerId> for InfraId {
    fn from(value: &'a CustomerId) -> Self {
        InfraId(value.cid)
    }
}

impl From<CustomerId> for InfraId {
    fn from(value: CustomerId) -> Self {
        InfraId(value.cid)
    }
}

impl_id!(CustomerId, CUSTOMER_ID_PREFIX);
impl_display_for_id!(CustomerId);
impl_customer_id_from_ty!(i64);
impl_customer_id_from_ty!(u64);
impl_customer_id_from_ty!(i32);
impl_customer_id_from_ty!(u32);
impl_customer_id_from_ty!(u16);
impl_customer_id_from_ty!(i16);
impl_customer_id_from_ty!(u8);
impl_customer_id_from_ty!(i8);

/// Customer Resource Id
///
/// - Prefix: U
/// - Min Length: 27
/// - Max Length: 42
/// - Real size: 20
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use qm_entity::ids::{CustomerResourceId, ID};
///
/// let id1 = CustomerResourceId::parse("U016603f7b32b1753f84a719e01").expect("Customer Resource Id");
/// let id2 = CustomerResourceId::parse("U1206603f7b32b1753f84a719e02").expect("Customer Resource Id");
/// let id3 = CustomerResourceId::parse("U25006603f7b32b1753f84a719e03").expect("Customer Resource Id");
///
/// assert_eq!((1, ID::from_str("6603f7b32b1753f84a719e01").expect("Object ID")), id1.unzip());
/// assert_eq!((0x20, ID::from_str("6603f7b32b1753f84a719e02").expect("Object ID")), id2.unzip());
/// assert_eq!((0x500, ID::from_str("6603f7b32b1753f84a719e03").expect("Object ID")), id3.unzip());
/// ```
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
    async_graphql::Description,
)]
pub struct CustomerResourceId {
    cid: i64,
    id: ID,
}

impl CustomerResourceId {
    pub fn root(&self) -> CustomerId {
        CustomerId::from(self.cid)
    }

    pub fn parent(&self) -> CustomerId {
        CustomerId::from(self.cid)
    }

    pub fn unzip(&self) -> (i64, ID) {
        (self.cid, self.id)
    }
}

impl FromStr for CustomerResourceId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(Self::PREFIX) {
            anyhow::bail!("Invalid CustomerResourceId")
        }
        let mut parser = StringParser::<1>::new(&s[1..]).with_object_id();
        let CustomerId { cid }: CustomerId = parser.next().map(From::from).ok_or(
            anyhow::anyhow!("unable to parse '{s}' into CustomerResourceId"),
        )?;
        let start = parser.end();
        let end = start + ID_LENGTH;
        if end > s.len() {
            anyhow::bail!("Invalid length for CustomerResourceId");
        }
        let id = ID::from_str(&s[start..end])?;
        Ok(Self { cid, id })
    }
}

impl_id!(CustomerResourceId, CUSTOMER_RESOURCE_ID_PREFIX);
impl_display_for_resource_id!(CustomerResourceId);
impl_customer_resource_id_from_ty_tuple!(i64);
impl_customer_resource_id_from_ty_tuple!(u64);
impl_customer_resource_id_from_ty_tuple!(i32);
impl_customer_resource_id_from_ty_tuple!(u32);
impl_customer_resource_id_from_ty_tuple!(u16);
impl_customer_resource_id_from_ty_tuple!(i16);
impl_customer_resource_id_from_ty_tuple!(u8);
impl_customer_resource_id_from_ty_tuple!(i8);

/// Organization Id
///
/// - Prefix: T
/// - Min Length: 5
/// - Max Length: 35
/// - Real size: 16
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use qm_entity::ids::OrganizationId;
///
/// let id1 = OrganizationId::parse("T0102").expect("Organization Id");
/// let id2 = OrganizationId::parse("T120121").expect("Organization Id");
/// let id3 = OrganizationId::parse("T25002501").expect("Organization Id");
///
/// assert_eq!((1, 2), id1.unzip());
/// assert_eq!((0x20, 0x21), id2.unzip());
/// assert_eq!((0x500, 0x501), id3.unzip());
/// ```
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
    async_graphql::Description,
)]
pub struct OrganizationId {
    cid: i64,
    oid: i64,
}

impl OrganizationId {
    pub fn id(&self) -> i64 {
        self.oid
    }

    pub fn root(&self) -> CustomerId {
        CustomerId::from(self.cid)
    }

    pub fn parent(&self) -> CustomerId {
        CustomerId::from(self.cid)
    }

    fn to_hex(self) -> String {
        StringWriter::from((self.cid, self.oid)).into_inner()
    }

    pub fn unzip(&self) -> (i64, i64) {
        (self.cid, self.oid)
    }

    pub fn resource(&self, id: ID) -> OrganizationResourceId {
        OrganizationResourceId::from((self.cid, self.oid, id))
    }
}

impl FromStr for OrganizationId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(Self::PREFIX) {
            anyhow::bail!("Invalid OrganizationId")
        }
        let mut parser = StringParser::<2>::new(&s[1..]);
        parser
            .next()
            .zip(parser.next())
            .map(From::from)
            .ok_or(anyhow::anyhow!("unable to get OrganizationId from '{s}'"))
    }
}

impl<'a> From<&'a OrganizationId> for InfraId {
    fn from(value: &'a OrganizationId) -> Self {
        InfraId(value.oid)
    }
}

impl From<OrganizationId> for InfraId {
    fn from(value: OrganizationId) -> Self {
        InfraId(value.oid)
    }
}

impl_id!(OrganizationId, ORGANIZATION_ID_PREFIX);
impl_display_for_id!(OrganizationId);
impl_organization_id_from_ty_tuple!(i64);
impl_organization_id_from_ty_tuple!(u64);
impl_organization_id_from_ty_tuple!(i32);
impl_organization_id_from_ty_tuple!(u32);
impl_organization_id_from_ty_tuple!(u16);
impl_organization_id_from_ty_tuple!(i16);
impl_organization_id_from_ty_tuple!(u8);
impl_organization_id_from_ty_tuple!(i8);

/// Organization Resource Id
///
/// - Prefix: S
/// - Min Length: 29
/// - Max Length: 59
/// - Real size: 28
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use qm_entity::ids::{OrganizationResourceId, ID};
///
/// let id1 = OrganizationResourceId::parse("S01026603f7b32b1753f84a719e01").expect("Organization Resource Id");
/// let id2 = OrganizationResourceId::parse("S1201216603f7b32b1753f84a719e02").expect("Organization Resource Id");
/// let id3 = OrganizationResourceId::parse("S250025016603f7b32b1753f84a719e03").expect("Organization Resource Id");
///
/// assert_eq!((1, 2, ID::from_str("6603f7b32b1753f84a719e01").expect("Object ID")), id1.unzip());
/// assert_eq!((0x20, 0x21, ID::from_str("6603f7b32b1753f84a719e02").expect("Object ID")), id2.unzip());
/// assert_eq!((0x500, 0x501, ID::from_str("6603f7b32b1753f84a719e03").expect("Object ID")), id3.unzip());
/// ```
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
    async_graphql::Description,
)]
pub struct OrganizationResourceId {
    cid: i64,
    oid: i64,
    id: ID,
}

impl OrganizationResourceId {
    pub fn root(&self) -> CustomerId {
        CustomerId::from(self.cid)
    }

    pub fn parent(&self) -> OrganizationId {
        OrganizationId::from((self.cid, self.oid))
    }

    pub fn id(&self) -> &ID {
        &self.id
    }

    pub fn unzip(&self) -> (i64, i64, ID) {
        (self.cid, self.oid, self.id)
    }
}

impl FromStr for OrganizationResourceId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(Self::PREFIX) {
            anyhow::bail!("Invalid OrganizationResourceId")
        }
        let mut parser = StringParser::<2>::new(&s[1..]).with_object_id();
        let OrganizationId { cid, oid }: OrganizationId = parser
            .next()
            .zip(parser.next())
            .map(From::from)
            .ok_or(anyhow::anyhow!(
                "unable to parse '{s}' into OrganizationResourceId"
            ))?;
        let start = parser.end();
        let end = start + ID_LENGTH;
        if end > s.len() {
            anyhow::bail!("Invalid length for OrganizationResourceId");
        }
        let id = ID::from_str(&s[start..end])?;
        Ok(Self { cid, oid, id })
    }
}

impl_id!(OrganizationResourceId, ORGANIZATION_RESOURCE_ID_PREFIX);
impl_display_for_resource_id!(OrganizationResourceId);
impl_organization_resource_id_from_ty_tuple!(i64);
impl_organization_resource_id_from_ty_tuple!(u64);
impl_organization_resource_id_from_ty_tuple!(i32);
impl_organization_resource_id_from_ty_tuple!(u32);
impl_organization_resource_id_from_ty_tuple!(u16);
impl_organization_resource_id_from_ty_tuple!(i16);
impl_organization_resource_id_from_ty_tuple!(u8);
impl_organization_resource_id_from_ty_tuple!(i8);

/// Institution Id
///
/// - Prefix: R
/// - Min Length: 7
/// - Max Length: 52
/// - Real size: 24
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use qm_entity::ids::InstitutionId;
///
/// let id1 = InstitutionId::parse("R010203").expect("Institution Id");
/// let id2 = InstitutionId::parse("R120121122").expect("Institution Id");
/// let id3 = InstitutionId::parse("R250025012502").expect("Institution Id");
///
/// assert_eq!((1, 2, 3), id1.unzip());
/// assert_eq!((0x20, 0x21, 0x22), id2.unzip());
/// assert_eq!((0x500, 0x501, 0x502), id3.unzip());
/// ```
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
    async_graphql::Description,
)]
pub struct InstitutionId {
    pub cid: i64,
    pub oid: i64,
    pub iid: i64,
}

impl InstitutionId {
    pub fn id(&self) -> i64 {
        self.iid
    }

    pub fn root(&self) -> CustomerId {
        CustomerId::from(self.cid)
    }

    pub fn parent(&self) -> OrganizationId {
        OrganizationId::from((self.cid, self.oid))
    }

    fn to_hex(self) -> String {
        StringWriter::from((self.cid, self.oid, self.iid)).into_inner()
    }

    pub fn unzip(&self) -> (i64, i64, i64) {
        (self.cid, self.oid, self.iid)
    }
    pub fn untuple(&self) -> (i64, (i64, i64)) {
        (self.cid, (self.oid, self.iid))
    }

    pub fn resource(&self, id: ID) -> InstitutionResourceId {
        InstitutionResourceId::from((self.cid, self.oid, self.iid, id))
    }
}

impl FromStr for InstitutionId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(Self::PREFIX) {
            anyhow::bail!("Invalid InstitutionId")
        }
        let mut parser = StringParser::<3>::new(&s[1..]);
        parser
            .next()
            .zip(parser.next())
            .zip(parser.next())
            .map(From::from)
            .ok_or(anyhow::anyhow!("unable to get InstitutionId from '{s}'"))
    }
}

impl<'a> From<&'a InstitutionId> for InfraId {
    fn from(value: &'a InstitutionId) -> Self {
        InfraId(value.iid)
    }
}

impl From<InstitutionId> for InfraId {
    fn from(value: InstitutionId) -> Self {
        InfraId(value.iid)
    }
}

impl_id!(InstitutionId, INSTITUTION_ID_PREFIX);
impl_display_for_id!(InstitutionId);
impl_institution_id_from_ty_tuple!(i64);
impl_institution_id_from_ty_tuple!(u64);
impl_institution_id_from_ty_tuple!(i32);
impl_institution_id_from_ty_tuple!(u32);
impl_institution_id_from_ty_tuple!(u16);
impl_institution_id_from_ty_tuple!(i16);
impl_institution_id_from_ty_tuple!(u8);
impl_institution_id_from_ty_tuple!(i8);

/// Institution Resource Id
///
/// - Prefix: Q
/// - Min Length: 31
/// - Max Length: 76
/// - Real size: 36
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use qm_entity::ids::{InstitutionResourceId, ID};
///
/// let id1 = InstitutionResourceId::parse("Q0102036603f7b32b1753f84a719e01").expect("Institution Resource Id");
/// let id2 = InstitutionResourceId::parse("Q1201211226603f7b32b1753f84a719e02").expect("Institution Resource Id");
/// let id3 = InstitutionResourceId::parse("Q2500250125026603f7b32b1753f84a719e03").expect("Institution Resource Id");
///
/// assert_eq!((1, 2, 3, ID::from_str("6603f7b32b1753f84a719e01").expect("Object ID")), id1.unzip());
/// assert_eq!((0x20, 0x21, 0x22, ID::from_str("6603f7b32b1753f84a719e02").expect("Object ID")), id2.unzip());
/// assert_eq!((0x500, 0x501, 0x502, ID::from_str("6603f7b32b1753f84a719e03").expect("Object ID")), id3.unzip());
/// ```
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
    async_graphql::Description,
)]
pub struct InstitutionResourceId {
    cid: i64,
    oid: i64,
    iid: i64,
    id: ID,
}

impl InstitutionResourceId {
    pub fn root(&self) -> CustomerId {
        CustomerId::from(self.cid)
    }

    pub fn parent(&self) -> InstitutionId {
        InstitutionId::from((self.cid, self.oid, self.iid))
    }

    pub fn unzip(&self) -> (i64, i64, i64, ID) {
        (self.cid, self.oid, self.iid, self.id)
    }
}

impl FromStr for InstitutionResourceId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(Self::PREFIX) {
            anyhow::bail!("Invalid InstitutionResourceId")
        }
        let mut parser = StringParser::<3>::new(&s[1..]).with_object_id();
        let InstitutionId { cid, oid, iid }: InstitutionId = parser
            .next()
            .zip(parser.next())
            .zip(parser.next())
            .map(From::from)
            .ok_or(anyhow::anyhow!(
                "unable to parse '{s}' into InstitutionResourceId"
            ))?;
        let start = parser.end();
        let end = start + ID_LENGTH;
        if end > s.len() {
            anyhow::bail!("Invalid length for InstitutionResourceId");
        }
        let id = ID::from_str(&s[start..end])?;
        Ok(Self { cid, oid, iid, id })
    }
}

impl_id!(InstitutionResourceId, INSTITUTION_RESOURCE_ID_PREFIX);
impl_display_for_resource_id!(InstitutionResourceId);
impl_institution_resource_id_from_ty_tuple!(i64);
impl_institution_resource_id_from_ty_tuple!(u64);
impl_institution_resource_id_from_ty_tuple!(i32);
impl_institution_resource_id_from_ty_tuple!(u32);
impl_institution_resource_id_from_ty_tuple!(u16);
impl_institution_resource_id_from_ty_tuple!(i16);
impl_institution_resource_id_from_ty_tuple!(u8);
impl_institution_resource_id_from_ty_tuple!(i8);

/// Institution Unit Id
///
/// - Prefix: P
/// - Min Length: 7
/// - Max Length: 52
/// - Real size: 24
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use qm_entity::ids::InstitutionUnitId;
///
/// let id1 = InstitutionUnitId::parse("P010203").expect("Institution Unit Id");
/// let id2 = InstitutionUnitId::parse("P120121122").expect("Institution Unit Id");
/// let id3 = InstitutionUnitId::parse("P250025012502").expect("Institution Unit Id");
///
/// assert_eq!((1, 2, 3), id1.unzip());
/// assert_eq!((0x20, 0x21, 0x22), id2.unzip());
/// assert_eq!((0x500, 0x501, 0x502), id3.unzip());
/// ```
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
    async_graphql::Description,
)]
pub struct InstitutionUnitId {
    cid: i64,
    oid: i64,
    uid: i64,
}

impl InstitutionUnitId {
    pub fn root(&self) -> CustomerId {
        CustomerId::from(self.cid)
    }

    pub fn parent(&self) -> OrganizationId {
        OrganizationId::from((self.cid, self.oid))
    }

    fn to_hex(self) -> String {
        StringWriter::from((self.cid, self.oid, self.uid)).into_inner()
    }

    pub fn unzip(&self) -> (i64, i64, i64) {
        (self.cid, self.oid, self.uid)
    }
}

impl FromStr for InstitutionUnitId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(Self::PREFIX) {
            anyhow::bail!("Invalid InstitutionUnitId")
        }
        let mut parser = StringParser::<3>::new(&s[1..]);
        parser
            .next()
            .zip(parser.next())
            .zip(parser.next())
            .map(From::from)
            .ok_or(anyhow::anyhow!(
                "unable to get InstitutionUnitId from '{s}'"
            ))
    }
}

impl<'a> From<&'a InstitutionUnitId> for InfraId {
    fn from(value: &'a InstitutionUnitId) -> Self {
        InfraId(value.uid)
    }
}

impl From<InstitutionUnitId> for InfraId {
    fn from(value: InstitutionUnitId) -> Self {
        InfraId(value.uid)
    }
}

impl_id!(InstitutionUnitId, INSTITUTION_UNIT_ID_PREFIX);
impl_display_for_id!(InstitutionUnitId);
impl_institution_unit_id_from_ty_tuple!(i64);
impl_institution_unit_id_from_ty_tuple!(u64);
impl_institution_unit_id_from_ty_tuple!(i32);
impl_institution_unit_id_from_ty_tuple!(u32);
impl_institution_unit_id_from_ty_tuple!(u16);
impl_institution_unit_id_from_ty_tuple!(i16);
impl_institution_unit_id_from_ty_tuple!(u8);
impl_institution_unit_id_from_ty_tuple!(i8);

/// Institution Unit Resource Id
///
/// - Prefix: O
/// - Min Length: 31
/// - Max Length: 76
/// - Real size: 36
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use qm_entity::ids::{InstitutionUnitResourceId, ID};
///
/// let id1 = InstitutionUnitResourceId::parse("O0102036603f7b32b1753f84a719e01").expect("Institution Unit Resource Id");
/// let id2 = InstitutionUnitResourceId::parse("O1201211226603f7b32b1753f84a719e02").expect("Institution Unit Resource Id");
/// let id3 = InstitutionUnitResourceId::parse("O2500250125026603f7b32b1753f84a719e03").expect("Institution Unit Resource Id");
///
/// assert_eq!((1, 2, 3, ID::from_str("6603f7b32b1753f84a719e01").expect("Object ID")), id1.unzip());
/// assert_eq!((0x20, 0x21, 0x22, ID::from_str("6603f7b32b1753f84a719e02").expect("Object ID")), id2.unzip());
/// assert_eq!((0x500, 0x501, 0x502, ID::from_str("6603f7b32b1753f84a719e03").expect("Object ID")), id3.unzip());
/// ```
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
    async_graphql::Description,
)]
pub struct InstitutionUnitResourceId {
    cid: i64,
    oid: i64,
    uid: i64,
    id: ID,
}

impl InstitutionUnitResourceId {
    pub fn root(&self) -> CustomerId {
        CustomerId::from(self.cid)
    }

    pub fn parent(&self) -> InstitutionUnitId {
        InstitutionUnitId::from((self.cid, self.oid, self.uid))
    }

    pub fn unzip(&self) -> (i64, i64, i64, ID) {
        (self.cid, self.oid, self.uid, self.id)
    }
}

impl FromStr for InstitutionUnitResourceId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(Self::PREFIX) {
            anyhow::bail!("Invalid InstitutionUnitResourceId")
        }
        let mut parser = StringParser::<3>::new(&s[1..]).with_object_id();
        let InstitutionUnitId { cid, oid, uid }: InstitutionUnitId = parser
            .next()
            .zip(parser.next())
            .zip(parser.next())
            .map(From::from)
            .ok_or(anyhow::anyhow!(
                "unable to parse '{s}' into InstitutionUnitResourceId"
            ))?;
        let start = parser.end();
        let end = start + ID_LENGTH;
        if end > s.len() {
            anyhow::bail!("Invalid length for InstitutionUnitResourceId");
        }
        let id = ID::from_str(&s[start..end])?;
        Ok(Self { cid, oid, uid, id })
    }
}

impl_id!(
    InstitutionUnitResourceId,
    INSTITUTION_UNIT_RESOURCE_ID_PREFIX
);
impl_display_for_resource_id!(InstitutionUnitResourceId);

/// Customer Unit Id
///
/// - Prefix: N
/// - Min Length: 5
/// - Max Length: 35
/// - Real size: 16
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use qm_entity::ids::CustomerUnitId;
///
/// let id1 = CustomerUnitId::parse("N0102").expect("Customer Unit Id");
/// let id2 = CustomerUnitId::parse("N120121").expect("Customer Unit Id");
/// let id3 = CustomerUnitId::parse("N25002501").expect("Customer Unit Id");
///
/// assert_eq!((1, 2), id1.unzip());
/// assert_eq!((0x20, 0x21), id2.unzip());
/// assert_eq!((0x500, 0x501), id3.unzip());
/// ```
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
    async_graphql::Description,
)]
pub struct CustomerUnitId {
    cid: i64,
    uid: i64,
}

impl CustomerUnitId {
    pub fn root(&self) -> CustomerId {
        CustomerId::from(self.cid)
    }

    pub fn parent(&self) -> CustomerId {
        self.root()
    }

    fn to_hex(self) -> String {
        StringWriterResult::<2>::from_iter([self.cid, self.uid])
            .into_inner()
            .unwrap()
            .into_inner()
    }

    pub fn unzip(&self) -> (i64, i64) {
        (self.cid, self.uid)
    }
}

impl FromStr for CustomerUnitId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(Self::PREFIX) {
            anyhow::bail!("Invalid CustomerUnitId")
        }
        let mut parser = StringParser::<2>::new(&s[1..]);
        parser
            .next()
            .zip(parser.next())
            .map(From::from)
            .ok_or(anyhow::anyhow!("unable to get CustomerUnitId from '{s}'"))
    }
}

impl<'a> From<&'a CustomerUnitId> for InfraId {
    fn from(value: &'a CustomerUnitId) -> Self {
        InfraId(value.uid)
    }
}

impl From<CustomerUnitId> for InfraId {
    fn from(value: CustomerUnitId) -> Self {
        InfraId(value.uid)
    }
}

impl_id!(CustomerUnitId, CUSTOMER_UNIT_ID_PREFIX);
impl_display_for_id!(CustomerUnitId);
impl_customer_unit_id_from_ty_tuple!(i64);
impl_customer_unit_id_from_ty_tuple!(u64);
impl_customer_unit_id_from_ty_tuple!(i32);
impl_customer_unit_id_from_ty_tuple!(u32);
impl_customer_unit_id_from_ty_tuple!(u16);
impl_customer_unit_id_from_ty_tuple!(i16);
impl_customer_unit_id_from_ty_tuple!(u8);
impl_customer_unit_id_from_ty_tuple!(i8);

/// Customer Unit Resource Id
///
/// - Prefix: M
/// - Min Length: 29
/// - Max Length: 59
/// - Real size: 28
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use qm_entity::ids::{CustomerUnitResourceId, ID};
///
/// let id1 = CustomerUnitResourceId::parse("M01026603f7b32b1753f84a719e01").expect("Customer Unit Resource Id");
/// let id2 = CustomerUnitResourceId::parse("M1201216603f7b32b1753f84a719e02").expect("Customer Unit Resource Id");
/// let id3 = CustomerUnitResourceId::parse("M250025016603f7b32b1753f84a719e03").expect("Customer Unit Resource Id");
///
/// assert_eq!((1, 2, ID::from_str("6603f7b32b1753f84a719e01").expect("Object ID")), id1.unzip());
/// assert_eq!((0x20, 0x21, ID::from_str("6603f7b32b1753f84a719e02").expect("Object ID")), id2.unzip());
/// assert_eq!((0x500, 0x501, ID::from_str("6603f7b32b1753f84a719e03").expect("Object ID")), id3.unzip());
/// ```
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
    async_graphql::Description,
)]
pub struct CustomerUnitResourceId {
    cid: i64,
    uid: i64,
    id: ID,
}

impl CustomerUnitResourceId {
    pub fn root(&self) -> CustomerId {
        CustomerId::from(self.cid)
    }

    pub fn parent(&self) -> CustomerUnitId {
        CustomerUnitId::from((self.cid, self.uid))
    }

    pub fn unzip(&self) -> (i64, i64, ID) {
        (self.cid, self.uid, self.id)
    }
}

impl FromStr for CustomerUnitResourceId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(Self::PREFIX) {
            anyhow::bail!("Invalid CustomerUnitResourceId")
        }
        let mut parser = StringParser::<2>::new(&s[1..]).with_object_id();
        let CustomerUnitId { cid, uid }: CustomerUnitId = parser
            .next()
            .zip(parser.next())
            .map(From::from)
            .ok_or(anyhow::anyhow!(
                "unable to parse '{s}' into CustomerUnitResourceId"
            ))?;
        let start = parser.end();
        let end = start + ID_LENGTH;
        if end > s.len() {
            anyhow::bail!("Invalid length for CustomerUnitResourceId");
        }
        let id = ID::from_str(&s[start..end])?;
        Ok(Self { cid, uid, id })
    }
}

impl_id!(CustomerUnitResourceId, CUSTOMER_UNIT_RESOURCE_ID_PREFIX);
impl_display_for_resource_id!(CustomerUnitResourceId);

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    async_graphql::Description,
)]
pub enum OrganizationUnitId {
    Customer(CustomerUnitId),
    Organization(InstitutionUnitId),
}

impl From<(i64, i64)> for OrganizationUnitId {
    fn from(value: (i64, i64)) -> Self {
        Self::Customer(value.into())
    }
}

impl From<(i64, i64, i64)> for OrganizationUnitId {
    fn from(value: (i64, i64, i64)) -> Self {
        Self::Organization(value.into())
    }
}

impl OrganizationUnitId {
    pub fn id(&self) -> i64 {
        match self {
            Self::Customer(v) => v.uid,
            Self::Organization(v) => v.uid,
        }
    }

    pub fn parent(&self) -> InfraContext {
        match self {
            Self::Customer(v) => InfraContext::Customer(v.parent()),
            Self::Organization(v) => InfraContext::Organization(v.parent()),
        }
    }

    pub fn untuple(&self) -> (i64, i64) {
        match self {
            Self::Customer(v) => (v.cid, v.uid),
            Self::Organization(v) => (v.cid, v.uid),
        }
    }

    pub fn parse(value: &str) -> anyhow::Result<Self> {
        Self::from_str(value)
    }
}

impl std::fmt::Display for OrganizationUnitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Customer(v) => v.fmt(f),
            Self::Organization(v) => v.fmt(f),
        }
    }
}

impl std::str::FromStr for OrganizationUnitId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with(CustomerUnitId::PREFIX) {
            return Ok(Self::Customer(CustomerUnitId::from_str(s)?));
        }
        if s.starts_with(InstitutionUnitId::PREFIX) {
            return Ok(Self::Organization(InstitutionUnitId::from_str(s)?));
        }
        anyhow::bail!("Invalid OrganizationUnitId")
    }
}

impl<'a> From<&'a OrganizationUnitId> for InfraId {
    fn from(value: &'a OrganizationUnitId) -> Self {
        match value {
            OrganizationUnitId::Customer(v) => v.into(),
            OrganizationUnitId::Organization(v) => v.into(),
        }
    }
}

impl From<OrganizationUnitId> for InfraId {
    fn from(value: OrganizationUnitId) -> Self {
        match value {
            OrganizationUnitId::Customer(v) => v.into(),
            OrganizationUnitId::Organization(v) => v.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, OneofObject, PartialEq, Eq, PartialOrd, Ord)]
pub enum InfraContext {
    Customer(CustomerId),
    Organization(OrganizationId),
    Institution(InstitutionId),
    OrganizationUnit(OrganizationUnitId),
}

impl InfraContext {
    pub fn customer_id(&self) -> InfraId {
        match self {
            InfraContext::Customer(b) => b.cid.into(),
            InfraContext::Organization(b) => b.cid.into(),
            InfraContext::Institution(b) => b.cid.into(),
            InfraContext::OrganizationUnit(v) => match v {
                OrganizationUnitId::Customer(b) => b.cid.into(),
                OrganizationUnitId::Organization(b) => b.cid.into(),
            },
        }
    }

    pub fn organization_id(&self) -> Option<InfraId> {
        match self {
            InfraContext::Customer(_) => None,
            InfraContext::Organization(b) => Some(b.oid.into()),
            InfraContext::Institution(b) => Some(b.oid.into()),
            InfraContext::OrganizationUnit(v) => match v {
                OrganizationUnitId::Customer(_) => None,
                OrganizationUnitId::Organization(b) => Some(b.oid.into()),
            },
        }
    }

    pub fn organization_unit_id(&self) -> Option<InfraId> {
        match self {
            InfraContext::Customer(_) => None,
            InfraContext::Organization(_) => None,
            InfraContext::Institution(_) => None,
            InfraContext::OrganizationUnit(v) => match v {
                OrganizationUnitId::Customer(b) => Some(b.uid.into()),
                OrganizationUnitId::Organization(b) => Some(b.uid.into()),
            },
        }
    }

    pub fn institution_id(&self) -> Option<InfraId> {
        match self {
            InfraContext::Customer(_) => None,
            InfraContext::Organization(_) => None,
            InfraContext::Institution(b) => Some(b.iid.into()),
            InfraContext::OrganizationUnit(v) => match v {
                OrganizationUnitId::Customer(_) => None,
                OrganizationUnitId::Organization(_) => None,
            },
        }
    }

    pub fn has_customer(&self, a: &CustomerId) -> bool {
        match self {
            InfraContext::Customer(b) => a.cid == b.cid,
            InfraContext::Organization(b) => a.cid == b.cid,
            InfraContext::Institution(b) => a.cid == b.cid,
            InfraContext::OrganizationUnit(v) => match v {
                OrganizationUnitId::Customer(b) => a.cid == b.cid,
                OrganizationUnitId::Organization(b) => a.cid == b.cid,
            },
        }
    }
    pub fn has_organization(&self, a: &OrganizationId) -> bool {
        match self {
            InfraContext::Customer(_) => false,
            InfraContext::Organization(b) => a == b,
            InfraContext::Institution(b) => a.cid == b.cid && a.oid == b.oid,
            InfraContext::OrganizationUnit(v) => match v {
                OrganizationUnitId::Customer(_) => false,
                OrganizationUnitId::Organization(b) => a.cid == b.cid && a.oid == b.oid,
            },
        }
    }
    pub fn has_organization_unit(&self, a: &OrganizationUnitId) -> bool {
        match self {
            InfraContext::Customer(_) => false,
            InfraContext::Organization(_) => false,
            InfraContext::Institution(_) => false,
            InfraContext::OrganizationUnit(b) => a == b,
        }
    }
    pub fn has_institution(&self, a: &InstitutionId) -> bool {
        match self {
            InfraContext::Customer(_) => false,
            InfraContext::Organization(_) => false,
            InfraContext::Institution(b) => a == b,
            InfraContext::OrganizationUnit(_) => false,
        }
    }

    // Call from user context
    pub fn combine(self, query_context: Self) -> Self {
        match &self {
            InfraContext::Customer(v) => {
                if query_context.has_customer(v) {
                    query_context
                } else {
                    self
                }
            }
            InfraContext::Organization(v) => {
                if query_context.has_organization(v) {
                    query_context
                } else {
                    self
                }
            }
            InfraContext::Institution(v) => {
                if query_context.has_institution(v) {
                    query_context
                } else {
                    self
                }
            }
            InfraContext::OrganizationUnit(v) => {
                if query_context.has_organization_unit(v) {
                    query_context
                } else {
                    self
                }
            }
        }
    }
}

impl std::fmt::Display for InfraContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Customer(v) => v.fmt(f),
            Self::Organization(v) => v.fmt(f),
            Self::Institution(v) => v.fmt(f),
            Self::OrganizationUnit(v) => v.fmt(f),
        }
    }
}

impl InfraContext {
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        Self::from_str(s)
    }
}

impl std::str::FromStr for InfraContext {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(first_char) = s.chars().next() {
            return match first_char {
                CustomerId::PREFIX => CustomerId::parse(s).map(InfraContext::Customer),
                OrganizationId::PREFIX => OrganizationId::parse(s).map(InfraContext::Organization),
                InstitutionId::PREFIX => InstitutionId::parse(s).map(InfraContext::Institution),
                CustomerUnitId::PREFIX => CustomerUnitId::parse(s)
                    .map(|v| InfraContext::OrganizationUnit(OrganizationUnitId::Customer(v))),
                InstitutionUnitId::PREFIX => InstitutionUnitId::parse(s)
                    .map(|v| InfraContext::OrganizationUnit(OrganizationUnitId::Organization(v))),
                _ => anyhow::bail!("invalid prefix '{first_char}'"),
            };
        }
        anyhow::bail!("unable to parse InfraContext from '{s}'");
    }
}

const HEX_CHARS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];

struct StringWriter<const N: usize>(String);

struct StringWriterResult<const N: usize>(anyhow::Result<StringWriter<N>>);

impl<const N: usize> StringWriter<N> {
    fn into_inner(self) -> String {
        self.0
    }
    fn string_length() -> usize {
        N * 17
    }
}

impl<const N: usize> StringWriterResult<N> {
    fn into_inner(self) -> anyhow::Result<StringWriter<N>> {
        self.0
    }
}

impl<const N: usize> FromIterator<i64> for StringWriterResult<N> {
    fn from_iter<T: IntoIterator<Item = i64>>(iter: T) -> Self {
        let mut iter = iter.into_iter();
        let mut s = String::with_capacity(StringWriter::<N>::string_length());
        let mut idx: usize;
        for i in 0..N {
            let item = iter.next();
            if item.is_none() {
                return Self(Err(anyhow::anyhow!(
                    "expected {} elements got {}",
                    N,
                    i + 1
                )));
            }
            let n = item.unwrap();
            idx = s.len();
            {
                s.write_fmt(format_args!("0{n:X}")).unwrap();
            }
            let l = s.len();
            let s_bytes: &mut [u8] = unsafe { s.as_bytes_mut() };
            s_bytes[idx] = HEX_CHARS[l - (idx + 2)] as u8;
        }

        Self(Ok(StringWriter::<N>(s)))
    }
}

impl From<(i64, i64, i64)> for StringWriter<3> {
    fn from(value: (i64, i64, i64)) -> Self {
        StringWriterResult::<3>::from_iter([value.0, value.1, value.2])
            .into_inner()
            .unwrap()
    }
}

impl From<(i64, i64)> for StringWriter<2> {
    fn from(value: (i64, i64)) -> Self {
        StringWriterResult::<2>::from_iter([value.0, value.1])
            .into_inner()
            .unwrap()
    }
}

impl From<i64> for StringWriter<1> {
    fn from(n: i64) -> Self {
        StringWriterResult::<1>::from_iter([n])
            .into_inner()
            .unwrap()
    }
}

fn is_valid_range(s: &str, start: usize, end: usize) -> bool {
    !s.is_empty() && start < end && end <= s.len()
}

struct StringParser<'a, const N: usize> {
    count: usize,
    start: usize,
    end: usize,
    has_object_id_at_end: bool,
    s: &'a str,
}

impl<'a, const N: usize> StringParser<'a, N> {
    fn new(s: &'a str) -> StringParser<'a, N> {
        StringParser {
            count: 0,
            start: 0,
            end: 1,
            has_object_id_at_end: false,
            s,
        }
    }

    fn with_object_id(mut self) -> Self {
        self.has_object_id_at_end = true;
        self
    }

    fn end(&self) -> usize {
        self.end
    }
}

impl<'a, const N: usize> Iterator for StringParser<'a, N> {
    type Item = i64;
    fn next(&mut self) -> Option<i64> {
        if self.count >= N {
            return None;
        }
        if !is_valid_range(self.s, self.start, self.end) {
            return None;
        }
        let l = usize::from_str_radix(&self.s[self.start..self.end], 16);
        if l.is_err() {
            return None;
        }
        self.start = self.end;
        self.end = self.start + l.unwrap() + 1;
        if !is_valid_range(self.s, self.start, self.end) {
            return None;
        }
        let s = &self.s[self.start..self.end];
        let result = i64::from_str_radix(s, 16).ok();
        self.start = self.end;
        self.end = self.start + 1;
        self.count += 1;
        let l = self.s.len();
        if self.has_object_id_at_end {
            if self.count == N && self.end + 23 != l {
                return None;
            }
        } else if self.count == N && self.end != l + 1 {
            return None;
        }
        result
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_parser() {
        let mut parser = StringParser::<3>::new("010101");
        assert_eq!(Some(1), parser.next());
        assert_eq!(Some(1), parser.next());
        assert_eq!(Some(1), parser.next());
        assert_eq!(None, parser.next());
        assert_eq!(None, parser.next());
    }

    #[test]
    fn test_null_parser() {
        let mut parser = StringParser::<0>::new("01010101");
        assert_eq!(None, parser.next());
    }

    #[test]
    fn test_prefix() {
        assert_eq!('V', CustomerId::PREFIX);
        assert_eq!('U', CustomerResourceId::PREFIX);
        assert_eq!('T', OrganizationId::PREFIX);
        assert_eq!('S', OrganizationResourceId::PREFIX);
        assert_eq!('R', InstitutionId::PREFIX);
        assert_eq!('Q', InstitutionResourceId::PREFIX);
        assert_eq!('P', InstitutionUnitId::PREFIX);
        assert_eq!('O', InstitutionUnitResourceId::PREFIX);
        assert_eq!('N', CustomerUnitId::PREFIX);
        assert_eq!('M', CustomerUnitResourceId::PREFIX);
    }

    #[test]
    fn test_invalid_prefix() {
        assert_eq!(None, CustomerId::parse("U01").ok());
        assert_eq!(None, CustomerResourceId::parse("V01").ok());
        assert_eq!(None, OrganizationId::parse("S01").ok());
        assert_eq!(None, OrganizationResourceId::parse("T01").ok());
        assert_eq!(None, InstitutionId::parse("Q01").ok());
        assert_eq!(None, InstitutionResourceId::parse("R01").ok());
        assert_eq!(None, InstitutionUnitId::parse("O01").ok());
        assert_eq!(None, InstitutionUnitResourceId::parse("P01").ok());
        assert_eq!(None, CustomerUnitId::parse("M01").ok());
        assert_eq!(None, CustomerUnitResourceId::parse("N01").ok());
    }

    #[test]
    fn test_customer_id() {
        let id1 = CustomerId::parse("V01").unwrap();
        let id2 = CustomerId::parse("V120").unwrap();
        let id3 = CustomerId::parse("V2500").unwrap();
        let id4 = CustomerId::parse("V36000").unwrap();
        let id5 = CustomerId::parse("V48000F").unwrap();
        let id6 = CustomerId::parse("V5AF000F").unwrap();
        let id7 = CustomerId::parse("V6B5F000F").unwrap();
        let id8 = CustomerId::parse("VF7FFFFFFFFFFFFFFF").unwrap();
        assert_eq!(CustomerId { cid: 1 }, id1);
        assert_eq!(CustomerId { cid: 0x20 }, id2);
        assert_eq!(CustomerId { cid: 0x500 }, id3);
        assert_eq!(CustomerId { cid: 0x6000 }, id4);
        assert_eq!(CustomerId { cid: 0x8000F }, id5);
        assert_eq!(CustomerId { cid: 0xAF000F }, id6);
        assert_eq!(CustomerId { cid: 0xB5F000F, }, id7);
        assert_eq!(CustomerId { cid: i64::MAX }, id8);
        assert_eq!(id1.to_string(), "V01");
        assert_eq!(id2.to_string(), "V120");
        assert_eq!(id3.to_string(), "V2500");
        assert_eq!(id4.to_string(), "V36000");
        assert_eq!(id5.to_string(), "V48000F");
        assert_eq!(id6.to_string(), "V5AF000F");
        assert_eq!(id7.to_string(), "V6B5F000F");
        assert_eq!(id8.to_string(), "VF7FFFFFFFFFFFFFFF");
        assert_eq!(None, CustomerId::parse("VF8FFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerId::parse("VF9FFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerId::parse("VFAFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerId::parse("VFBFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerId::parse("VFCFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerId::parse("VFDFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerId::parse("VFEFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerId::parse("VFFFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerId::parse("VVV").ok());
        assert_eq!(None, CustomerId::parse("V0ABC").ok());
        assert_eq!(id1.unzip(), 1);
    }

    #[test]
    fn test_customer_resource_id() {
        let oid1 = ID::from_str("6603f7b32b1753f84a719e01").unwrap();
        let oid2 = ID::from_str("6603f7b32b1753f84a719e02").unwrap();
        let oid3 = ID::from_str("6603f7b32b1753f84a719e03").unwrap();
        let oid4 = ID::from_str("6603f7b32b1753f84a719e04").unwrap();
        let id1 = CustomerResourceId::parse("U016603f7b32b1753f84a719e01").unwrap();
        let id2 = CustomerResourceId::parse("U1206603f7b32b1753f84a719e02").unwrap();
        let id3 = CustomerResourceId::parse("U25006603f7b32b1753f84a719e03").unwrap();
        let id4 = CustomerResourceId::parse("U360006603f7b32b1753f84a719e04").unwrap();
        let id5 = CustomerResourceId::parse("U48000F6603f7b32b1753f84a719e01").unwrap();
        let id6 = CustomerResourceId::parse("U5AF000F6603f7b32b1753f84a719e02").unwrap();
        let id7 = CustomerResourceId::parse("U6B5F000F6603f7b32b1753f84a719e03").unwrap();
        let id8 = CustomerResourceId::parse("UF7FFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").unwrap();
        assert_eq!(CustomerResourceId { cid: 1, id: oid1, }, id1);
        assert_eq!(CustomerResourceId { cid: 0x20, id: oid2, }, id2);
        assert_eq!(CustomerResourceId { cid: 0x500, id: oid3, }, id3);
        assert_eq!(CustomerResourceId { cid: 0x6000, id: oid4, }, id4);
        assert_eq!(CustomerResourceId { cid: 0x8000F, id: oid1, }, id5);
        assert_eq!(CustomerResourceId { cid: 0xAF000F, id: oid2, }, id6);
        assert_eq!(CustomerResourceId { cid: 0xB5F000F, id: oid3, }, id7);
        assert_eq!(CustomerResourceId { cid: i64::MAX, id: oid4, }, id8);
        assert_eq!(id1.to_string(), "U016603f7b32b1753f84a719e01");
        assert_eq!(id2.to_string(), "U1206603f7b32b1753f84a719e02");
        assert_eq!(id3.to_string(), "U25006603f7b32b1753f84a719e03");
        assert_eq!(id4.to_string(), "U360006603f7b32b1753f84a719e04");
        assert_eq!(id5.to_string(), "U48000F6603f7b32b1753f84a719e01");
        assert_eq!(id6.to_string(), "U5AF000F6603f7b32b1753f84a719e02");
        assert_eq!(id7.to_string(), "U6B5F000F6603f7b32b1753f84a719e03");
        assert_eq!(id8.to_string(), "UF7FFFFFFFFFFFFFFF6603f7b32b1753f84a719e04");
        assert_eq!(None, CustomerResourceId::parse("UF8FFFFFFFFFFFFFFF6603f7b32b1753f84a719e01").ok());
        assert_eq!(None, CustomerResourceId::parse("UF9FFFFFFFFFFFFFFF6603f7b32b1753f84a719e02").ok());
        assert_eq!(None, CustomerResourceId::parse("UFAFFFFFFFFFFFFFFF6603f7b32b1753f84a719e03").ok());
        assert_eq!(None, CustomerResourceId::parse("UFBFFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, CustomerResourceId::parse("UFCFFFFFFFFFFFFFFF6603f7b32b1753f84a719e01").ok());
        assert_eq!(None, CustomerResourceId::parse("UFDFFFFFFFFFFFFFFF6603f7b32b1753f84a719e02").ok());
        assert_eq!(None, CustomerResourceId::parse("UFEFFFFFFFFFFFFFFF6603f7b32b1753f84a719e03").ok());
        assert_eq!(None, CustomerResourceId::parse("UFFFFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, CustomerResourceId::parse("UVV6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, CustomerResourceId::parse("U0ABC6603f7b32b1753f84a719e04").ok());
        assert_eq!(id1.root(), CustomerId { cid: 1 });
        assert_eq!(id1.parent(), CustomerId { cid: 1 });
        assert_eq!(id1.unzip(), (1, oid1));
    }

    #[test]
    fn test_organization_id() {
        let id1 = OrganizationId::parse("T0101").unwrap();
        let id2 = OrganizationId::parse("T120120").unwrap();
        let id3 = OrganizationId::parse("T25002500").unwrap();
        let id4 = OrganizationId::parse("T3600036000").unwrap();
        let id5 = OrganizationId::parse("T48000F48000F").unwrap();
        let id6 = OrganizationId::parse("T5AF000F5AF000F").unwrap();
        let id7 = OrganizationId::parse("T6B5F000F6B5F000F").unwrap();
        let id8 = OrganizationId::parse("TF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF").unwrap();
        assert_eq!(OrganizationId { cid: 1, oid: 1 }, id1);
        assert_eq!(OrganizationId { cid: 0x20, oid: 0x20 }, id2);
        assert_eq!(OrganizationId { cid: 0x500, oid: 0x500 }, id3);
        assert_eq!(OrganizationId { cid: 0x6000, oid: 0x6000 }, id4);
        assert_eq!(OrganizationId { cid: 0x8000F, oid: 0x8000F }, id5);
        assert_eq!(OrganizationId { cid: 0xAF000F, oid: 0xAF000F }, id6);
        assert_eq!(OrganizationId { cid: 0xB5F000F, oid: 0xB5F000F }, id7);
        assert_eq!(OrganizationId { cid: i64::MAX, oid: i64::MAX }, id8);
        assert_eq!(id1.to_string(), "T0101");
        assert_eq!(id2.to_string(), "T120120");
        assert_eq!(id3.to_string(), "T25002500");
        assert_eq!(id4.to_string(), "T3600036000");
        assert_eq!(id5.to_string(), "T48000F48000F");
        assert_eq!(id6.to_string(), "T5AF000F5AF000F");
        assert_eq!(id7.to_string(), "T6B5F000F6B5F000F");
        assert_eq!(id8.to_string(), "TF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF");
        assert_eq!(None, OrganizationId::parse("TF8FFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF").ok());
        assert_eq!(None, OrganizationId::parse("TF9FFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF").ok());
        assert_eq!(None, OrganizationId::parse("TFAFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, OrganizationId::parse("TFBFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, OrganizationId::parse("TFCFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, OrganizationId::parse("TFDFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, OrganizationId::parse("TFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, OrganizationId::parse("TFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, OrganizationId::parse("TVVVU").ok());
        assert_eq!(None, OrganizationId::parse("TFABC1C").ok());
        assert_eq!(id1.root(), CustomerId { cid: 1 });
        assert_eq!(id1.parent(), CustomerId { cid: 1 });
        assert_eq!(id1.unzip(), (1, 1));
    }

    #[test]
    fn test_organization_resource_id() {
        let oid1 = ID::from_str("6603f7b32b1753f84a719e01").unwrap();
        let oid2 = ID::from_str("6603f7b32b1753f84a719e02").unwrap();
        let oid3 = ID::from_str("6603f7b32b1753f84a719e03").unwrap();
        let oid4 = ID::from_str("6603f7b32b1753f84a719e04").unwrap();
        let id1 = OrganizationResourceId::parse("S01016603f7b32b1753f84a719e01").unwrap();
        let id2 = OrganizationResourceId::parse("S1201206603f7b32b1753f84a719e02").unwrap();
        let id3 = OrganizationResourceId::parse("S250025006603f7b32b1753f84a719e03").unwrap();
        let id4 = OrganizationResourceId::parse("S36000360006603f7b32b1753f84a719e04").unwrap();
        let id5 = OrganizationResourceId::parse("S48000F48000F6603f7b32b1753f84a719e01").unwrap();
        let id6 = OrganizationResourceId::parse("S5AF000F5AF000F6603f7b32b1753f84a719e02").unwrap();
        let id7 = OrganizationResourceId::parse("S6B5F000F6B5F000F6603f7b32b1753f84a719e03").unwrap();
        let id8 = OrganizationResourceId::parse("SF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").unwrap();
        assert_eq!(OrganizationResourceId { cid: 1, oid: 1, id: oid1, }, id1);
        assert_eq!(OrganizationResourceId { cid: 0x20, oid: 0x20, id: oid2, }, id2);
        assert_eq!(OrganizationResourceId { cid: 0x500, oid: 0x500, id: oid3, }, id3);
        assert_eq!(OrganizationResourceId { cid: 0x6000, oid: 0x6000, id: oid4, }, id4);
        assert_eq!(OrganizationResourceId { cid: 0x8000F, oid: 0x8000F, id: oid1, }, id5);
        assert_eq!(OrganizationResourceId { cid: 0xAF000F, oid: 0xAF000F, id: oid2, }, id6);
        assert_eq!(OrganizationResourceId { cid: 0xB5F000F, oid: 0xB5F000F, id: oid3, }, id7);
        assert_eq!(OrganizationResourceId { cid: i64::MAX, oid: i64::MAX, id: oid4, }, id8);
        assert_eq!(id1.to_string(), "S01016603f7b32b1753f84a719e01");
        assert_eq!(id2.to_string(), "S1201206603f7b32b1753f84a719e02");
        assert_eq!(id3.to_string(), "S250025006603f7b32b1753f84a719e03");
        assert_eq!(id4.to_string(), "S36000360006603f7b32b1753f84a719e04");
        assert_eq!(id5.to_string(), "S48000F48000F6603f7b32b1753f84a719e01");
        assert_eq!(id6.to_string(), "S5AF000F5AF000F6603f7b32b1753f84a719e02");
        assert_eq!(id7.to_string(), "S6B5F000F6B5F000F6603f7b32b1753f84a719e03");
        assert_eq!(id8.to_string(), "SF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF6603f7b32b1753f84a719e04");
        assert_eq!(None, OrganizationResourceId::parse("SF8FFFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF6603f7b32b1753f84a719e01").ok());
        assert_eq!(None, OrganizationResourceId::parse("SF9FFFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF6603f7b32b1753f84a719e02").ok());
        assert_eq!(None, OrganizationResourceId::parse("SFAFFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFF6603f7b32b1753f84a719e03").ok());
        assert_eq!(None, OrganizationResourceId::parse("SFBFFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, OrganizationResourceId::parse("SFCFFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFF6603f7b32b1753f84a719e01").ok());
        assert_eq!(None, OrganizationResourceId::parse("SFDFFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFF6603f7b32b1753f84a719e02").ok());
        assert_eq!(None, OrganizationResourceId::parse("SFEFFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFF6603f7b32b1753f84a719e03").ok());
        assert_eq!(None, OrganizationResourceId::parse("SFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, OrganizationResourceId::parse("SVV6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, OrganizationResourceId::parse("S0A0A0A0A0A0ABC6603f7b32b1753f84a719e04").ok());
        assert_eq!(id1.root(), CustomerId { cid: 1 });
        assert_eq!(id1.parent(), OrganizationId { cid: 1, oid: 1 });
        assert_eq!(id1.unzip(), (1, 1, oid1));
    }

    #[test]
    fn test_institution_id() {
        let id1 = InstitutionId::parse("R010101").unwrap();
        let id2 = InstitutionId::parse("R120120120").unwrap();
        let id3 = InstitutionId::parse("R250025002500").unwrap();
        let id4 = InstitutionId::parse("R360003600036000").unwrap();
        let id5 = InstitutionId::parse("R48000F48000F48000F").unwrap();
        let id6 = InstitutionId::parse("R5AF000F5AF000F5AF000F").unwrap();
        let id7 = InstitutionId::parse("R6B5F000F6B5F000F6B5F000F").unwrap();
        let id8 = InstitutionId::parse("RF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF").unwrap();
        assert_eq!(InstitutionId { cid: 1, oid: 1, iid: 1, }, id1);
        assert_eq!(InstitutionId { cid: 0x20, oid: 0x20, iid: 0x20, }, id2);
        assert_eq!(InstitutionId { cid: 0x500, oid: 0x500, iid: 0x500, }, id3);
        assert_eq!(InstitutionId { cid: 0x6000, oid: 0x6000, iid: 0x6000, }, id4);
        assert_eq!(InstitutionId { cid: 0x8000F, oid: 0x8000F, iid: 0x8000F, }, id5);
        assert_eq!(InstitutionId { cid: 0xAF000F, oid: 0xAF000F, iid: 0xAF000F, }, id6);
        assert_eq!(InstitutionId { cid: 0xB5F000F, oid: 0xB5F000F, iid: 0xB5F000F, }, id7);
        assert_eq!(InstitutionId { cid: i64::MAX, oid: i64::MAX, iid: i64::MAX }, id8);
        assert_eq!(id1.to_string(), "R010101");
        assert_eq!(id2.to_string(), "R120120120");
        assert_eq!(id3.to_string(), "R250025002500");
        assert_eq!(id4.to_string(), "R360003600036000");
        assert_eq!(id5.to_string(), "R48000F48000F48000F");
        assert_eq!(id6.to_string(), "R5AF000F5AF000F5AF000F");
        assert_eq!(id7.to_string(), "R6B5F000F6B5F000F6B5F000F");
        assert_eq!(id8.to_string(), "RF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF");
        assert_eq!(None, InstitutionId::parse("RF8FFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionId::parse("RF9FFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionId::parse("RFAFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionId::parse("RFBFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionId::parse("RFCFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionId::parse("RFDFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionId::parse("RFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionId::parse("RFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionId::parse("R0FF").ok());
        assert_eq!(None, InstitutionId::parse("RF0").ok());
        assert_eq!(id1.root(), CustomerId { cid: 1 });
        assert_eq!(id1.parent(), OrganizationId { cid: 1, oid: 1 });
        assert_eq!(id1.unzip(), (1, 1, 1));
    }


    #[test]
    fn test_institution_resource_id() {
        let oid1 = ID::from_str("6603f7b32b1753f84a719e01").unwrap();
        let oid2 = ID::from_str("6603f7b32b1753f84a719e02").unwrap();
        let oid3 = ID::from_str("6603f7b32b1753f84a719e03").unwrap();
        let oid4 = ID::from_str("6603f7b32b1753f84a719e04").unwrap();
        let id1 = InstitutionResourceId::parse("Q0101016603f7b32b1753f84a719e01").unwrap();
        let id2 = InstitutionResourceId::parse("Q1201201206603f7b32b1753f84a719e02").unwrap();
        let id3 = InstitutionResourceId::parse("Q2500250025006603f7b32b1753f84a719e03").unwrap();
        let id4 = InstitutionResourceId::parse("Q3600036000360006603f7b32b1753f84a719e04").unwrap();
        let id5 = InstitutionResourceId::parse("Q48000F48000F48000F6603f7b32b1753f84a719e01").unwrap();
        let id6 = InstitutionResourceId::parse("Q5AF000F5AF000F5AF000F6603f7b32b1753f84a719e02").unwrap();
        let id7 = InstitutionResourceId::parse("Q6B5F000F6B5F000F6B5F000F6603f7b32b1753f84a719e03").unwrap();
        let id8 = InstitutionResourceId::parse("QF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").unwrap();
        assert_eq!(InstitutionResourceId { cid: 1, oid: 1, iid: 1, id: oid1, }, id1);
        assert_eq!(InstitutionResourceId { cid: 0x20, oid: 0x20, iid: 0x20, id: oid2, }, id2);
        assert_eq!(InstitutionResourceId { cid: 0x500, oid: 0x500, iid: 0x500, id: oid3, }, id3);
        assert_eq!(InstitutionResourceId { cid: 0x6000, oid: 0x6000, iid: 0x6000, id: oid4, }, id4);
        assert_eq!(InstitutionResourceId { cid: 0x8000F, oid: 0x8000F, iid: 0x8000F, id: oid1, }, id5);
        assert_eq!(InstitutionResourceId { cid: 0xAF000F, oid: 0xAF000F, iid: 0xAF000F, id: oid2, }, id6);
        assert_eq!(InstitutionResourceId { cid: 0xB5F000F, oid: 0xB5F000F, iid: 0xB5F000F, id: oid3, }, id7);
        assert_eq!(InstitutionResourceId { cid: i64::MAX, oid: i64::MAX, iid: i64::MAX, id: oid4, }, id8);
        assert_eq!(id1.to_string(), "Q0101016603f7b32b1753f84a719e01");
        assert_eq!(id2.to_string(), "Q1201201206603f7b32b1753f84a719e02");
        assert_eq!(id3.to_string(), "Q2500250025006603f7b32b1753f84a719e03");
        assert_eq!(id4.to_string(), "Q3600036000360006603f7b32b1753f84a719e04");
        assert_eq!(id5.to_string(), "Q48000F48000F48000F6603f7b32b1753f84a719e01");
        assert_eq!(id6.to_string(), "Q5AF000F5AF000F5AF000F6603f7b32b1753f84a719e02");
        assert_eq!(id7.to_string(), "Q6B5F000F6B5F000F6B5F000F6603f7b32b1753f84a719e03");
        assert_eq!(id8.to_string(), "QF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF6603f7b32b1753f84a719e04");
        assert_eq!(None, InstitutionResourceId::parse("QF8FFFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF6603f7b32b1753f84a719e01").ok());
        assert_eq!(None, InstitutionResourceId::parse("QF9FFFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF6603f7b32b1753f84a719e02").ok());
        assert_eq!(None, InstitutionResourceId::parse("QFAFFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFF6603f7b32b1753f84a719e03").ok());
        assert_eq!(None, InstitutionResourceId::parse("QFBFFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, InstitutionResourceId::parse("QFCFFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFF6603f7b32b1753f84a719e01").ok());
        assert_eq!(None, InstitutionResourceId::parse("QFDFFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFF6603f7b32b1753f84a719e02").ok());
        assert_eq!(None, InstitutionResourceId::parse("QFEFFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFF6603f7b32b1753f84a719e03").ok());
        assert_eq!(None, InstitutionResourceId::parse("QFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, InstitutionResourceId::parse("QVV6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, InstitutionResourceId::parse("Q0A0A0A0A0A0ABC6603f7b32b1753f84a719e04").ok());
        assert_eq!(id1.root(), CustomerId { cid: 1 });
        assert_eq!(id1.parent(), InstitutionId { cid: 1, oid: 1, iid: 1 });
        assert_eq!(id1.unzip(), (1, 1, 1, oid1));
    }

    #[test]
    fn test_institution_unit_id() {
        let id1 = InstitutionUnitId::parse("P010101").unwrap();
        let id2 = InstitutionUnitId::parse("P120120120").unwrap();
        let id3 = InstitutionUnitId::parse("P250025002500").unwrap();
        let id4 = InstitutionUnitId::parse("P360003600036000").unwrap();
        let id5 = InstitutionUnitId::parse("P48000F48000F48000F").unwrap();
        let id6 = InstitutionUnitId::parse("P5AF000F5AF000F5AF000F").unwrap();
        let id7 = InstitutionUnitId::parse("P6B5F000F6B5F000F6B5F000F").unwrap();
        let id8 = InstitutionUnitId::parse("PF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF").unwrap();
        assert_eq!(InstitutionUnitId { cid: 1, oid: 1, uid: 1, }, id1);
        assert_eq!(InstitutionUnitId { cid: 0x20, oid: 0x20, uid: 0x20, }, id2);
        assert_eq!(InstitutionUnitId { cid: 0x500, oid: 0x500, uid: 0x500, }, id3);
        assert_eq!(InstitutionUnitId { cid: 0x6000, oid: 0x6000, uid: 0x6000, }, id4);
        assert_eq!(InstitutionUnitId { cid: 0x8000F, oid: 0x8000F, uid: 0x8000F, }, id5);
        assert_eq!(InstitutionUnitId { cid: 0xAF000F, oid: 0xAF000F, uid: 0xAF000F, }, id6);
        assert_eq!(InstitutionUnitId { cid: 0xB5F000F, oid: 0xB5F000F, uid: 0xB5F000F, }, id7);
        assert_eq!(InstitutionUnitId { cid: i64::MAX, oid: i64::MAX, uid: i64::MAX }, id8);
        assert_eq!(id1.to_string(), "P010101");
        assert_eq!(id2.to_string(), "P120120120");
        assert_eq!(id3.to_string(), "P250025002500");
        assert_eq!(id4.to_string(), "P360003600036000");
        assert_eq!(id5.to_string(), "P48000F48000F48000F");
        assert_eq!(id6.to_string(), "P5AF000F5AF000F5AF000F");
        assert_eq!(id7.to_string(), "P6B5F000F6B5F000F6B5F000F");
        assert_eq!(id8.to_string(), "PF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF");
        assert_eq!(None, InstitutionUnitId::parse("PF8FFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionUnitId::parse("PF9FFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionUnitId::parse("PFAFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionUnitId::parse("PFBFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionUnitId::parse("PFCFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionUnitId::parse("PFDFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionUnitId::parse("PFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionUnitId::parse("PFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, InstitutionUnitId::parse("P0FF").ok());
        assert_eq!(None, InstitutionUnitId::parse("PF0").ok());
        assert_eq!(id1.root(), CustomerId { cid: 1 });
        assert_eq!(id1.parent(), OrganizationId { cid: 1, oid: 1 });
        assert_eq!(id1.unzip(), (1, 1, 1));
    }

    #[test]
    fn test_institution_unit_resource_id() {
        let oid1 = ID::from_str("6603f7b32b1753f84a719e01").unwrap();
        let oid2 = ID::from_str("6603f7b32b1753f84a719e02").unwrap();
        let oid3 = ID::from_str("6603f7b32b1753f84a719e03").unwrap();
        let oid4 = ID::from_str("6603f7b32b1753f84a719e04").unwrap();
        let id1 = InstitutionUnitResourceId::parse("O0101016603f7b32b1753f84a719e01").unwrap();
        let id2 = InstitutionUnitResourceId::parse("O1201201206603f7b32b1753f84a719e02").unwrap();
        let id3 = InstitutionUnitResourceId::parse("O2500250025006603f7b32b1753f84a719e03").unwrap();
        let id4 = InstitutionUnitResourceId::parse("O3600036000360006603f7b32b1753f84a719e04").unwrap();
        let id5 = InstitutionUnitResourceId::parse("O48000F48000F48000F6603f7b32b1753f84a719e01").unwrap();
        let id6 = InstitutionUnitResourceId::parse("O5AF000F5AF000F5AF000F6603f7b32b1753f84a719e02").unwrap();
        let id7 = InstitutionUnitResourceId::parse("O6B5F000F6B5F000F6B5F000F6603f7b32b1753f84a719e03").unwrap();
        let id8 = InstitutionUnitResourceId::parse("OF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").unwrap();
        assert_eq!(InstitutionUnitResourceId { cid: 1, oid: 1, uid: 1, id: oid1, }, id1);
        assert_eq!(InstitutionUnitResourceId { cid: 0x20, oid: 0x20, uid: 0x20, id: oid2, }, id2);
        assert_eq!(InstitutionUnitResourceId { cid: 0x500, oid: 0x500, uid: 0x500, id: oid3, }, id3);
        assert_eq!(InstitutionUnitResourceId { cid: 0x6000, oid: 0x6000, uid: 0x6000, id: oid4, }, id4);
        assert_eq!(InstitutionUnitResourceId { cid: 0x8000F, oid: 0x8000F, uid: 0x8000F, id: oid1, }, id5);
        assert_eq!(InstitutionUnitResourceId { cid: 0xAF000F, oid: 0xAF000F, uid: 0xAF000F, id: oid2, }, id6);
        assert_eq!(InstitutionUnitResourceId { cid: 0xB5F000F, oid: 0xB5F000F, uid: 0xB5F000F, id: oid3, }, id7);
        assert_eq!(InstitutionUnitResourceId { cid: i64::MAX, oid: i64::MAX, uid: i64::MAX, id: oid4, }, id8);
        assert_eq!(id1.to_string(), "O0101016603f7b32b1753f84a719e01");
        assert_eq!(id2.to_string(), "O1201201206603f7b32b1753f84a719e02");
        assert_eq!(id3.to_string(), "O2500250025006603f7b32b1753f84a719e03");
        assert_eq!(id4.to_string(), "O3600036000360006603f7b32b1753f84a719e04");
        assert_eq!(id5.to_string(), "O48000F48000F48000F6603f7b32b1753f84a719e01");
        assert_eq!(id6.to_string(), "O5AF000F5AF000F5AF000F6603f7b32b1753f84a719e02");
        assert_eq!(id7.to_string(), "O6B5F000F6B5F000F6B5F000F6603f7b32b1753f84a719e03");
        assert_eq!(id8.to_string(), "OF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF6603f7b32b1753f84a719e04");
        assert_eq!(None, InstitutionUnitResourceId::parse("OF8FFFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF6603f7b32b1753f84a719e01").ok());
        assert_eq!(None, InstitutionUnitResourceId::parse("OF9FFFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF6603f7b32b1753f84a719e02").ok());
        assert_eq!(None, InstitutionUnitResourceId::parse("OFAFFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFF6603f7b32b1753f84a719e03").ok());
        assert_eq!(None, InstitutionUnitResourceId::parse("OFBFFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, InstitutionUnitResourceId::parse("OFCFFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFF6603f7b32b1753f84a719e01").ok());
        assert_eq!(None, InstitutionUnitResourceId::parse("OFDFFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFF6603f7b32b1753f84a719e02").ok());
        assert_eq!(None, InstitutionUnitResourceId::parse("OFEFFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFF6603f7b32b1753f84a719e03").ok());
        assert_eq!(None, InstitutionUnitResourceId::parse("OFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, InstitutionUnitResourceId::parse("OVV6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, InstitutionUnitResourceId::parse("O0A0A0A0A0A0ABC6603f7b32b1753f84a719e04").ok());
        assert_eq!(id1.root(), CustomerId { cid: 1 });
        assert_eq!(id1.parent(), InstitutionUnitId { cid: 1, oid: 1, uid: 1 });
        assert_eq!(id1.unzip(), (1, 1, 1, oid1));
    }

    #[test]
    fn test_customer_unit_id() {
        let id1 = CustomerUnitId::parse("N0101").unwrap();
        let id2 = CustomerUnitId::parse("N120120").unwrap();
        let id3 = CustomerUnitId::parse("N25002500").unwrap();
        let id4 = CustomerUnitId::parse("N3600036000").unwrap();
        let id5 = CustomerUnitId::parse("N48000F48000F").unwrap();
        let id6 = CustomerUnitId::parse("N5AF000F5AF000F").unwrap();
        let id7 = CustomerUnitId::parse("N6B5F000F6B5F000F").unwrap();
        let id8 = CustomerUnitId::parse("NF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF").unwrap();
        assert_eq!(CustomerUnitId { cid: 1, uid: 1 }, id1);
        assert_eq!(CustomerUnitId { cid: 0x20, uid: 0x20 }, id2);
        assert_eq!(CustomerUnitId { cid: 0x500, uid: 0x500 }, id3);
        assert_eq!(CustomerUnitId { cid: 0x6000, uid: 0x6000 }, id4);
        assert_eq!(CustomerUnitId { cid: 0x8000F, uid: 0x8000F }, id5);
        assert_eq!(CustomerUnitId { cid: 0xAF000F, uid: 0xAF000F }, id6);
        assert_eq!(CustomerUnitId { cid: 0xB5F000F, uid: 0xB5F000F }, id7);
        assert_eq!(CustomerUnitId { cid: i64::MAX, uid: i64::MAX }, id8);
        assert_eq!(id1.to_string(), "N0101");
        assert_eq!(id2.to_string(), "N120120");
        assert_eq!(id3.to_string(), "N25002500");
        assert_eq!(id4.to_string(), "N3600036000");
        assert_eq!(id5.to_string(), "N48000F48000F");
        assert_eq!(id6.to_string(), "N5AF000F5AF000F");
        assert_eq!(id7.to_string(), "N6B5F000F6B5F000F");
        assert_eq!(id8.to_string(), "NF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF");
        assert_eq!(None, CustomerUnitId::parse("NF8FFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerUnitId::parse("NF9FFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerUnitId::parse("NFAFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerUnitId::parse("NFBFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerUnitId::parse("NFCFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerUnitId::parse("NFDFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerUnitId::parse("NFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerUnitId::parse("NFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").ok());
        assert_eq!(None, CustomerUnitId::parse("NVVVU").ok());
        assert_eq!(None, CustomerUnitId::parse("NFABC1C").ok());
        assert_eq!(id1.root(), CustomerId { cid: 1 });
        assert_eq!(id1.parent(), CustomerId { cid: 1 });
        assert_eq!(id1.unzip(), (1, 1));
    }

    #[test]
    fn test_customer_unit_resource_id() {
        let oid1 = ID::from_str("6603f7b32b1753f84a719e01").unwrap();
        let oid2 = ID::from_str("6603f7b32b1753f84a719e02").unwrap();
        let oid3 = ID::from_str("6603f7b32b1753f84a719e03").unwrap();
        let oid4 = ID::from_str("6603f7b32b1753f84a719e04").unwrap();
        let id1 = CustomerUnitResourceId::parse("M01016603f7b32b1753f84a719e01").unwrap();
        let id2 = CustomerUnitResourceId::parse("M1201206603f7b32b1753f84a719e02").unwrap();
        let id3 = CustomerUnitResourceId::parse("M250025006603f7b32b1753f84a719e03").unwrap();
        let id4 = CustomerUnitResourceId::parse("M36000360006603f7b32b1753f84a719e04").unwrap();
        let id5 = CustomerUnitResourceId::parse("M48000F48000F6603f7b32b1753f84a719e01").unwrap();
        let id6 = CustomerUnitResourceId::parse("M5AF000F5AF000F6603f7b32b1753f84a719e02").unwrap();
        let id7 = CustomerUnitResourceId::parse("M6B5F000F6B5F000F6603f7b32b1753f84a719e03").unwrap();
        let id8 = CustomerUnitResourceId::parse("MF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").unwrap();
        assert_eq!(CustomerUnitResourceId { cid: 1, uid: 1, id: oid1, }, id1);
        assert_eq!(CustomerUnitResourceId { cid: 0x20, uid: 0x20, id: oid2, }, id2);
        assert_eq!(CustomerUnitResourceId { cid: 0x500, uid: 0x500, id: oid3, }, id3);
        assert_eq!(CustomerUnitResourceId { cid: 0x6000, uid: 0x6000, id: oid4, }, id4);
        assert_eq!(CustomerUnitResourceId { cid: 0x8000F, uid: 0x8000F, id: oid1, }, id5);
        assert_eq!(CustomerUnitResourceId { cid: 0xAF000F, uid: 0xAF000F, id: oid2, }, id6);
        assert_eq!(CustomerUnitResourceId { cid: 0xB5F000F, uid: 0xB5F000F, id: oid3, }, id7);
        assert_eq!(CustomerUnitResourceId { cid: i64::MAX, uid: i64::MAX, id: oid4, }, id8);
        assert_eq!(id1.to_string(), "M01016603f7b32b1753f84a719e01");
        assert_eq!(id2.to_string(), "M1201206603f7b32b1753f84a719e02");
        assert_eq!(id3.to_string(), "M250025006603f7b32b1753f84a719e03");
        assert_eq!(id4.to_string(), "M36000360006603f7b32b1753f84a719e04");
        assert_eq!(id5.to_string(), "M48000F48000F6603f7b32b1753f84a719e01");
        assert_eq!(id6.to_string(), "M5AF000F5AF000F6603f7b32b1753f84a719e02");
        assert_eq!(id7.to_string(), "M6B5F000F6B5F000F6603f7b32b1753f84a719e03");
        assert_eq!(id8.to_string(), "MF7FFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF6603f7b32b1753f84a719e04");
        assert_eq!(None, CustomerUnitResourceId::parse("MF8FFFFFFFFFFFFFFFF8FFFFFFFFFFFFFFF6603f7b32b1753f84a719e01").ok());
        assert_eq!(None, CustomerUnitResourceId::parse("MF9FFFFFFFFFFFFFFFF9FFFFFFFFFFFFFFF6603f7b32b1753f84a719e02").ok());
        assert_eq!(None, CustomerUnitResourceId::parse("MFAFFFFFFFFFFFFFFFFAFFFFFFFFFFFFFFF6603f7b32b1753f84a719e03").ok());
        assert_eq!(None, CustomerUnitResourceId::parse("MFBFFFFFFFFFFFFFFFFBFFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, CustomerUnitResourceId::parse("MFCFFFFFFFFFFFFFFFFCFFFFFFFFFFFFFFF6603f7b32b1753f84a719e01").ok());
        assert_eq!(None, CustomerUnitResourceId::parse("MFDFFFFFFFFFFFFFFFFDFFFFFFFFFFFFFFF6603f7b32b1753f84a719e02").ok());
        assert_eq!(None, CustomerUnitResourceId::parse("MFEFFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFF6603f7b32b1753f84a719e03").ok());
        assert_eq!(None, CustomerUnitResourceId::parse("MFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, CustomerUnitResourceId::parse("MVV6603f7b32b1753f84a719e04").ok());
        assert_eq!(None, CustomerUnitResourceId::parse("M0A0A0A0A0A0ABC6603f7b32b1753f84a719e04").ok());
        assert_eq!(id1.root(), CustomerId { cid: 1 });
        assert_eq!(id1.parent(), CustomerUnitId { cid: 1, uid: 1 });
        assert_eq!(id1.unzip(), (1, 1, oid1));
    }
}
