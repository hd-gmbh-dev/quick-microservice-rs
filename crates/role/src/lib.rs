#![deny(missing_docs)]

//! Role abstraction layer for quick-microservice.
//!
//! This crate provides role-based access control (RBAC) abstractions,
//! including access types, roles, and authentication containers.

use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use std::{
    collections::{BTreeSet, HashSet},
    str::FromStr,
    sync::Arc,
};
use tokio::sync::RwLock;

/// Includes generated role code at compile time.
///
/// This macro includes the generated role code from the build process.
/// The filename is relative to the OUT_DIR environment variable.
#[macro_export]
macro_rules! include_roles {
    ($filename:tt) => {
        include!(concat!(env!("OUT_DIR"), "/", $filename, ".rs"));
    };
}

/// Creates a new role with the given resource type.
///
/// This macro provides a convenient way to create roles without explicitly
/// calling the Role constructor.
///
/// # Arguments
/// * `$resource` - The resource type
/// * `$permission` - Optional permission (if not provided, uses None)
///
/// # Example
/// ```
/// let role = role!("customer");
/// let role_with_perm = role!("customer", "read");
/// ```
#[macro_export]
macro_rules! role {
    ($resource:expr) => {
        $crate::Role::new($resource, None)
    };
    ($resource:expr, $permission:expr) => {
        $crate::Role::new($resource, Some($permission))
    };
}

/// An access.
///
/// Represents an access in the system.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[cfg_attr(
    feature = "serde-str",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub struct Access {
    ty: Arc<str>,
    id: Option<Arc<str>>,
}

impl Access {
    /// Creates a new access with the given type.
    ///
    /// # Arguments
    /// * `ty` - The type of access
    pub fn new(ty: Arc<str>) -> Self {
        Self { ty, id: None }
    }

    /// Sets the ID for this access.
    ///
    /// # Arguments
    /// * `id` - The access ID
    ///
    /// # Returns
    /// A new Access with the specified ID.
    pub fn with_id(mut self, id: Arc<str>) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the ID for this access using a displayable type.
    ///
    /// # Arguments
    /// * `id` - The access ID as a displayable type
    ///
    /// # Returns
    /// A new Access with the specified ID.
    pub fn with_fmt_id(mut self, id: Option<&impl std::fmt::Display>) -> Self {
        if let Some(id) = id {
            self.id = Some(Arc::from(id.to_string()));
        }
        self
    }

    /// Returns the type of this access.
    ///
    /// # Returns
    /// The access type as a string slice.
    pub fn ty(&self) -> &str {
        &self.ty
    }

    /// Returns the ID of this access, if any.
    ///
    /// # Returns
    /// The access ID as a string slice, or None if not set.
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
                    return Ok(Access {
                        ty: Arc::from(access.to_string()),
                        id: Some(Arc::from(id.to_string())),
                    });
                }
            }
        } else if let Some((access, method)) = v.split_once(':') {
            if method == "access" {
                return Ok(Access {
                    ty: Arc::from(access.to_string()),
                    id: None,
                });
            }
        }
        anyhow::bail!("invalid access role {v}");
    }
}

/// A role.
///
/// Represents a role in the system.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Copy)]
#[cfg_attr(
    feature = "serde-str",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
/// A role with a type and optional permission.
///
/// Represents a role in the system that can be used for access control.
/// The role consists of a type (resource) and an optional permission.
pub struct Role<R, P>
where
    R: std::fmt::Debug + std::marker::Copy + Clone,
    P: std::fmt::Debug + std::marker::Copy + Clone,
{
    /// The type (resource) of this role.
    pub ty: R,
    /// The optional permission associated with this role.
    pub permission: Option<P>,
}

impl<R, P> Role<R, P>
where
    R: std::fmt::Debug + std::marker::Copy + Clone,
    P: std::fmt::Debug + std::marker::Copy + Clone,
{
    /// Creates a new role with the given type and optional permission.
    ///
    /// # Arguments
    /// * `ty` - The role type (resource)
    /// * `permission` - Optional permission
    ///
    /// # Returns
    /// A new Role instance.
    pub fn new(ty: R, permission: Option<P>) -> Self {
        Self { ty, permission }
    }
}

impl<R, P> From<(R, P)> for Role<R, P>
where
    R: std::fmt::Debug + std::marker::Copy + Clone,
    P: std::fmt::Debug + std::marker::Copy + Clone,
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
    R: FromStr<Err = strum::ParseError> + std::fmt::Debug + std::marker::Copy + Clone,
    P: FromStr<Err = strum::ParseError> + std::fmt::Debug + std::marker::Copy + Clone,
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
    R: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
    P: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
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
    R: FromStr<Err = strum::ParseError>
        + AsRef<str>
        + std::fmt::Debug
        + std::marker::Copy
        + Clone
        + Send
        + Sync
        + 'static,
    P: FromStr<Err = strum::ParseError>
        + AsRef<str>
        + std::fmt::Debug
        + std::marker::Copy
        + Clone
        + Send
        + Sync
        + 'static,
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

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "serde-str", derive(serde_with::DeserializeFromStr))]
/// Represents either an access or a role.
///
/// This enum is used when parsing or serializing data that can be either
/// an access or a role, providing flexibility in handling both types.
pub enum AccessOrRole<R, P>
where
    R: std::fmt::Debug + Clone + std::marker::Copy,
    P: std::fmt::Debug + Clone + std::marker::Copy,
{
    /// An access entity.
    Access(Access),
    /// A role entity.
    Role(Role<R, P>),
}

#[cfg(feature = "serde-str")]
impl<R, P> serde::Serialize for AccessOrRole<R, P>
where
    R: AsRef<str> + std::fmt::Debug + Clone + std::marker::Copy,
    P: AsRef<str> + std::fmt::Debug + Clone + std::marker::Copy,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value = match self {
            Self::Access(access) => access.to_string(),
            Self::Role(role) => role.to_string(),
        };
        serializer.serialize_str(&value)
    }
}

impl<R, P> std::fmt::Display for AccessOrRole<R, P>
where
    R: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
    P: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(access) => access.fmt(f),
            Self::Role(role) => role.fmt(f),
        }
    }
}

impl<R, P> FromStr for AccessOrRole<R, P>
where
    R: FromStr<Err = strum::ParseError> + std::fmt::Debug + std::marker::Copy + Clone,
    P: FromStr<Err = strum::ParseError> + std::fmt::Debug + std::marker::Copy + Clone,
{
    type Err = anyhow::Error;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let mut s = v.split('@');
        if let Some((access, id)) = s.next().zip(s.next()) {
            if let Some((access, method)) = access.split_once(':') {
                if method == "access" {
                    return Ok(AccessOrRole::Access(Access {
                        ty: Arc::from(access.to_string()),
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

/// The result of parsing roles and accesses.
///
/// Contains the parsed access set and role set from a collection of role strings.
pub struct ParseResult<R, P>
where
    R: std::fmt::Debug + std::marker::Copy + Clone,
    P: std::fmt::Debug + std::marker::Copy + Clone,
{
    /// The set of parsed accesses.
    pub access: BTreeSet<Access>,
    /// The set of parsed roles.
    pub roles: HashSet<Role<R, P>>,
}

impl<R, P> Default for ParseResult<R, P>
where
    R: std::fmt::Debug + std::marker::Copy + Clone,
    P: std::fmt::Debug + std::marker::Copy + Clone,
{
    fn default() -> Self {
        Self {
            access: BTreeSet::default(),
            roles: HashSet::default(),
        }
    }
}

/// Parses a collection of role strings into separate access and role sets.
///
/// This function takes a slice of role strings and parses each one,
/// categorizing them into either accesses or roles.
///
/// # Arguments
/// * `roles` - A slice of role strings to parse
///
/// # Type Parameters
/// * `R` - The role type that implements FromStr
/// * `P` - The permission type that implements FromStr
///
/// # Returns
/// A ParseResult containing the parsed accesses and roles.
pub fn parse<R, P>(roles: &[Arc<str>]) -> ParseResult<R, P>
where
    R: Ord
        + FromStr<Err = strum::ParseError>
        + std::fmt::Debug
        + std::marker::Copy
        + Clone
        + std::hash::Hash,
    P: Ord
        + FromStr<Err = strum::ParseError>
        + std::fmt::Debug
        + std::marker::Copy
        + Clone
        + std::hash::Hash,
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

/// A group of roles with a name and path.
///
/// Represents a logical grouping of roles that can be used for
/// organizing and managing access control.
pub struct Group<R, P>
where
    R: std::fmt::Debug + std::marker::Copy + Clone,
    P: std::fmt::Debug + std::marker::Copy + Clone,
{
    /// The name of this group.
    pub name: String,
    /// The path to this group in the hierarchy.
    pub path: String,
    /// The resource roles associated with this group.
    resource_roles: Vec<Role<R, P>>,
    /// The allowed types for this group.
    allowed_types: Vec<String>,
}

impl<R, P> Group<R, P>
where
    R: std::fmt::Debug + std::marker::Copy + Clone,
    P: std::fmt::Debug + std::marker::Copy + Clone,
{
    /// Creates a new group with the given parameters.
    ///
    /// # Arguments
    /// * `name` - The name of the group
    /// * `path` - The path to this group in the hierarchy
    /// * `allowed_types` - The allowed types for this group
    /// * `resource_roles` - The resource roles associated with this group
    ///
    /// # Returns
    /// A new Group instance.
    pub fn new(
        name: String,
        path: String,
        allowed_types: Vec<String>,
        resource_roles: Vec<Role<R, P>>,
    ) -> Self {
        Self {
            name,
            path,
            resource_roles,
            allowed_types,
        }
    }

    /// Returns the allowed types for this group.
    ///
    /// # Returns
    /// A slice of strings representing the allowed types.
    pub fn allowed_types(&self) -> &[String] {
        &self.allowed_types
    }
}

impl<R, P> Group<R, P>
where
    R: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
    P: AsRef<str> + std::fmt::Debug + std::marker::Copy + Clone,
{
    /// Returns the resources associated with this group as strings.
    ///
    /// # Returns
    /// A vector of string representations of the resource roles.
    pub fn resources(&self) -> Vec<String> {
        self.resource_roles.iter().map(|r| r.to_string()).collect()
    }
}

struct Inner<T> {
    encoded: Option<Arc<str>>,
    decoded: RwLock<Option<T>>,
}

/// A container for authentication data with lazy decoding.
///
/// This struct holds encoded authentication data and provides
/// lazy decoding capabilities.
#[derive(Clone)]
pub struct AuthContainer<T> {
    inner: Arc<Inner<T>>,
}

impl<T> AuthContainer<T> {
    /// Creates a new AuthContainer with the given encoded data.
    ///
    /// # Arguments
    /// * `encoded` - The encoded authentication data
    ///
    /// # Returns
    /// A new AuthContainer instance.
    pub fn new(encoded: &str) -> Self {
        Self {
            inner: Arc::new(Inner {
                encoded: Some(Arc::from(encoded)),
                decoded: RwLock::new(None),
            }),
        }
    }

    /// Returns whether this container has encoded data.
    ///
    /// # Returns
    /// True if encoded data is present, false otherwise.
    pub fn has_encoded(&self) -> bool {
        self.inner.encoded.is_some()
    }

    /// Returns the encoded data if present.
    ///
    /// # Returns
    /// The encoded data as a string slice, or None if not set.
    pub fn encoded(&self) -> Option<&str> {
        self.inner.encoded.as_deref()
    }

    /// Returns a write guard for the decoded data.
    ///
    /// # Returns
    /// A write lock guard for the optional decoded data.
    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, Option<T>> {
        self.inner.decoded.write().await
    }

    /// Returns a read guard for the decoded data.
    ///
    /// # Returns
    /// A read lock guard for the optional decoded data.
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

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "serde-str")]
    fn test_serde_str() {
        use serde::Serialize;
        use strum::{AsRefStr, EnumString};

        let mut access: super::Access =
            serde_json::from_str("\"qqq:access\"").expect("Failed to parse JSON");
        assert_eq!(access.ty(), "qqq");
        assert_eq!(access.id(), None);

        access.id = Some("123".into());

        assert_eq!(
            serde_json::to_string(&access).expect("Failed to serialize JSON"),
            "\"qqq:access@123\""
        );

        #[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr, Serialize)]
        #[strum(serialize_all = "snake_case")]
        #[serde(rename_all = "snake_case")]
        enum RoleTy {
            Qqq,
            Bbb,
        }
        #[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr, Serialize)]
        #[strum(serialize_all = "snake_case")]
        #[serde(rename_all = "snake_case")]
        enum RolePerm {
            Grant,
            Deny,
        }
        let mut role: super::Role<RoleTy, RolePerm> =
            serde_json::from_str("\"qqq:grant\"").expect("Failed to parse JSON");
        assert_eq!(role.ty, RoleTy::Qqq);
        assert_eq!(role.permission, Some(RolePerm::Grant));

        role.permission = Some(RolePerm::Deny);

        assert_eq!(
            serde_json::to_string(&role).expect("Failed to serialize JSON"),
            "\"qqq:deny\""
        );

        let access_or_role_as_access: super::AccessOrRole<RoleTy, RolePerm> =
            serde_json::from_str("\"qqq:access@123\"").expect("Failed to parse JSON");
        assert!(
            matches!(&access_or_role_as_access, super::AccessOrRole::Access(a) if a == &access)
        );
        assert_eq!(
            serde_json::to_string(&access_or_role_as_access).expect("Failed to serialize JSON"),
            "\"qqq:access@123\""
        );

        let access_or_role_as_role: super::AccessOrRole<RoleTy, RolePerm> =
            serde_json::from_str("\"qqq:deny\"").expect("Failed to parse JSON");
        assert!(matches!(access_or_role_as_role, super::AccessOrRole::Role(r) if r == role));
        assert_eq!(
            serde_json::to_string(&access_or_role_as_role).expect("Failed to serialize JSON"),
            "\"qqq:deny\""
        );
    }
}
