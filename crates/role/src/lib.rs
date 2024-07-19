use std::{collections::BTreeSet, str::FromStr, sync::Arc};

use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use strum::{AsRefStr, EnumString};
use tokio::sync::RwLock;

#[macro_export]
macro_rules! include_roles {
    ($filename:tt) => {
        include!(concat!(env!("OUT_DIR"), "/", $filename, ".rs"));
    };
}

#[macro_export]
macro_rules! role {
    ($resource:expr) => {
        $crate::Role::new($resource, None)
    };
    ($resource:expr, $permission:expr) => {
        $crate::Role::new($resource, Some($permission))
    };
}

#[derive(
    Default,
    Clone,
    Debug,
    Copy,
    EnumString,
    async_graphql::Enum,
    AsRefStr,
    Hash,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
)]
pub enum AccessLevel {
    #[default]
    #[strum(serialize = "none")]
    None,
    #[strum(serialize = "admin")]
    Admin,
    #[strum(serialize = "support")]
    Support,
    #[strum(serialize = "customer")]
    Customer,
    #[strum(serialize = "organization")]
    Organization,
    #[strum(serialize = "customer_unit")]
    CustomerUnit,
    #[strum(serialize = "institution_unit")]
    InstitutionUnit,
    #[strum(serialize = "institution")]
    Institution,
}

impl AccessLevel {
    pub fn is_admin(&self) -> bool {
        matches!(self, AccessLevel::Admin)
    }

    pub fn id_required(&self) -> bool {
        matches!(
            self,
            AccessLevel::Customer
                | AccessLevel::Organization
                | AccessLevel::CustomerUnit
                | AccessLevel::InstitutionUnit
                | AccessLevel::Institution
        )
    }

    pub fn as_number(&self) -> u32 {
        match self {
            Self::Admin => u32::MAX,
            Self::Support => u32::MAX - 1,
            Self::Customer => u32::MAX - 2,
            Self::Organization | Self::CustomerUnit => u32::MAX - 3,
            Self::Institution | Self::InstitutionUnit => u32::MAX - 4,
            Self::None => 0,
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Access {
    ty: AccessLevel,
    id: Option<Arc<str>>,
}

impl Access {
    pub fn new(ty: AccessLevel) -> Self {
        Self { ty, id: None }
    }

    pub fn with_id(mut self, id: Arc<str>) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_fmt_id(mut self, id: Option<&impl std::fmt::Display>) -> Self {
        if let Some(id) = id {
            self.id = Some(Arc::from(id.to_string()));
        }
        self
    }

    pub fn ty(&self) -> &AccessLevel {
        &self.ty
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

impl std::fmt::Display for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(id) = &self.id {
            write!(f, "{}:access@{id}", self.ty.as_ref())
        } else {
            write!(f, "{}:access", self.ty.as_ref())
        }
    }
}

impl FromStr for Access {
    type Err = anyhow::Error;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let mut s = v.split('@');
        if let Some((access, id)) = s.next().zip(s.next()) {
            if let Some((access, method)) = access.split_once(':') {
                if method == "access" {
                    let ty = AccessLevel::from_str(access)?;
                    return Ok(Access {
                        ty,
                        id: Some(Arc::from(id.to_string())),
                    });
                }
            }
        } else if let Some((access, method)) = v.split_once(':') {
            if method == "access" {
                let ty = AccessLevel::from_str(access)?;
                return Ok(Access { ty, id: None });
            }
        }
        anyhow::bail!("invalid access role {v}");
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Role<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    pub ty: R,
    pub permission: Option<P>,
}

impl<R, P> Role<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    pub fn new(ty: R, permission: Option<P>) -> Self {
        Self { ty, permission }
    }
}

impl<R, P> From<(R, P)> for Role<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn from(value: (R, P)) -> Self {
        Self {
            ty: value.0,
            permission: Some(value.1),
        }
    }
}

impl<R, P> FromStr for Role<R, P>
where
    R: FromStr<Err = strum::ParseError> + std::fmt::Debug,
    P: FromStr<Err = strum::ParseError> + std::fmt::Debug,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains(':') {
            let mut s = s.split(':');
            if let Some((role, permission)) = s.next().zip(s.next()) {
                return Ok(Self {
                    ty: R::from_str(role)?,
                    permission: Some(P::from_str(permission)?),
                });
            }
        } else {
            return Ok(Self {
                ty: R::from_str(s)?,
                permission: None,
            });
        }

        anyhow::bail!("invalid role {s}");
    }
}

impl<R, P> std::fmt::Display for Role<R, P>
where
    R: AsRef<str> + std::fmt::Debug,
    P: AsRef<str> + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(permission) = &self.permission {
            write!(f, "{}:{}", self.ty.as_ref(), permission.as_ref())
        } else {
            write!(f, "{}", self.ty.as_ref())
        }
    }
}

#[Scalar]
impl<R, P> ScalarType for Role<R, P>
where
    R: FromStr<Err = strum::ParseError> + AsRef<str> + std::fmt::Debug + Send + Sync + 'static,
    P: FromStr<Err = strum::ParseError> + AsRef<str> + std::fmt::Debug + Send + Sync + 'static,
{
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            // Parse the integer value
            Ok(Role::<R, P>::from_str(value)
                .map_err(|err| InputValueError::custom(err.to_string()))?)
        } else {
            // If the type does not match
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum AccessOrRole<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    Access(Access),
    Role(Role<R, P>),
}

impl<R, P> FromStr for AccessOrRole<R, P>
where
    R: FromStr<Err = strum::ParseError> + std::fmt::Debug,
    P: FromStr<Err = strum::ParseError> + std::fmt::Debug,
{
    type Err = anyhow::Error;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let mut s = v.split('@');
        if let Some((access, id)) = s.next().zip(s.next()) {
            if let Some((access, method)) = access.split_once(':') {
                if method == "access" {
                    let ty = AccessLevel::from_str(access)?;
                    return Ok(AccessOrRole::Access(Access {
                        ty,
                        id: Some(Arc::from(id.to_string())),
                    }));
                }
            }
        } else if let Some((role, permission)) = v.split_once(':') {
            return Ok(AccessOrRole::Role(Role {
                ty: R::from_str(role)?,
                permission: Some(P::from_str(permission)?),
            }));
        } else {
            return Ok(AccessOrRole::Role(Role {
                ty: R::from_str(v)?,
                permission: None,
            }));
        }
        anyhow::bail!("invalid access or role {v}");
    }
}

pub struct ParseResult<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    pub access: BTreeSet<Access>,
    pub roles: BTreeSet<Role<R, P>>,
}

impl<R, P> Default for ParseResult<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    fn default() -> Self {
        Self {
            access: BTreeSet::default(),
            roles: BTreeSet::default(),
        }
    }
}

pub fn parse<R, P>(roles: &[Arc<str>]) -> ParseResult<R, P>
where
    R: Ord + FromStr<Err = strum::ParseError> + std::fmt::Debug,
    P: Ord + FromStr<Err = strum::ParseError> + std::fmt::Debug,
{
    roles
        .iter()
        .fold(ParseResult::<R, P>::default(), |mut state, s| {
            if let Ok(v) = AccessOrRole::<R, P>::from_str(s) {
                match v {
                    AccessOrRole::Access(v) => {
                        state.access.insert(v);
                    }
                    AccessOrRole::Role(v) => {
                        state.roles.insert(v);
                    }
                }
            }
            state
        })
}

pub struct Group<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    pub name: String,
    pub path: String,
    resource_roles: Vec<Role<R, P>>,
    allowed_access_levels: Vec<AccessLevel>,
    allowed_types: Vec<String>,
}

impl<R, P> Group<R, P>
where
    R: std::fmt::Debug,
    P: std::fmt::Debug,
{
    pub fn new(
        name: String,
        path: String,
        allowed_access_levels: Vec<AccessLevel>,
        allowed_types: Vec<String>,
        resource_roles: Vec<Role<R, P>>,
    ) -> Self {
        Self {
            name,
            path,
            resource_roles,
            allowed_access_levels,
            allowed_types,
        }
    }

    pub fn allowed_access_levels(&self) -> &[AccessLevel] {
        &self.allowed_access_levels
    }

    pub fn allowed_types(&self) -> &[String] {
        &self.allowed_types
    }
}

impl<R, P> Group<R, P>
where
    R: AsRef<str> + std::fmt::Debug,
    P: AsRef<str> + std::fmt::Debug,
{
    pub fn resources(&self) -> Vec<String> {
        self.resource_roles.iter().map(|r| r.to_string()).collect()
    }
}

struct Inner<T> {
    encoded: Option<Arc<str>>,
    decoded: RwLock<Option<T>>,
}

#[derive(Clone)]
pub struct AuthContainer<T> {
    inner: Arc<Inner<T>>,
}

impl<T> AuthContainer<T> {
    pub fn new(encoded: &str) -> Self {
        Self {
            inner: Arc::new(Inner {
                encoded: Some(Arc::from(encoded)),
                decoded: RwLock::new(None),
            }),
        }
    }

    pub fn has_encoded(&self) -> bool {
        self.inner.encoded.is_some()
    }

    pub fn encoded(&self) -> Option<&str> {
        self.inner.encoded.as_deref()
    }

    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, Option<T>> {
        self.inner.decoded.write().await
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, Option<T>> {
        self.inner.decoded.read().await
    }
}

impl<T> From<&axum::http::HeaderValue> for AuthContainer<T> {
    fn from(value: &axum::http::HeaderValue) -> Self {
        if let Ok(token) = value.to_str() {
            if let Some(stripped) = token.strip_prefix("Bearer ") {
                return Self::new(stripped);
            }
        }
        Self::default()
    }
}

impl<T> Default for AuthContainer<T> {
    fn default() -> Self {
        Self {
            inner: Arc::new(Inner {
                encoded: None,
                decoded: RwLock::new(None),
            }),
        }
    }
}
