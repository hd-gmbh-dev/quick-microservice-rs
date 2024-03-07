use std::{collections::BTreeSet, str::FromStr, sync::Arc};

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

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct Access<T> {
    ty: T,
    id: Option<Arc<str>>,
}

impl<T> Access<T> {
    pub fn new(ty: T) -> Self {
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
}

impl<T> std::fmt::Display for Access<T>
where
    T: AsRef<str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(id) = &self.id {
            write!(f, "{}:access_{id}", self.ty.as_ref())
        } else {
            write!(f, "{}:access", self.ty.as_ref())
        }
    }
}

impl<T> FromStr for Access<T>
where
    T: FromStr<Err = anyhow::Error>,
{
    type Err = anyhow::Error;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let mut s = v.split("_");
        if let Some((access, id)) = s.next().zip(s.next()) {
            if let Some((access, method)) = access.split_once(":") {
                if method == "access" {
                    let ty = T::from_str(access)?;
                    return Ok(Access {
                        ty,
                        id: Some(Arc::from(id.to_string())),
                    });
                }
            }
        } else {
            if let Some((access, method)) = v.split_once(":") {
                if method == "access" {
                    let ty = T::from_str(access)?;
                    return Ok(Access { ty, id: None });
                }
            }
        }
        anyhow::bail!("invalid access role {v}");
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct Role<R, P> {
    ty: R,
    permission: Option<P>,
}

impl<R, P> Role<R, P> {
    pub fn new(ty: R, permission: Option<P>) -> Self {
        Self { ty, permission }
    }
}

impl<R, P> From<(R, P)> for Role<R, P> {
    fn from(value: (R, P)) -> Self {
        Self {
            ty: value.0,
            permission: Some(value.1),
        }
    }
}

impl<R, P> FromStr for Role<R, P>
where
    R: FromStr<Err = anyhow::Error>,
    P: FromStr<Err = anyhow::Error>,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains(":") {
            let mut s = s.split(":");
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
    R: AsRef<str>,
    P: AsRef<str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(permission) = &self.permission {
            write!(f, "{}:{}", self.ty.as_ref(), permission.as_ref())
        } else {
            write!(f, "{}", self.ty.as_ref())
        }
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum AccessOrRole<A, R, P> {
    Access(Access<A>),
    Role(Role<R, P>),
}

impl<A, R, P> FromStr for AccessOrRole<A, R, P>
where
    A: FromStr<Err = strum::ParseError>,
    R: FromStr<Err = strum::ParseError>,
    P: FromStr<Err = strum::ParseError>,
{
    type Err = anyhow::Error;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let mut s = v.split("_");
        if let Some((access, id)) = s.next().zip(s.next()) {
            if let Some((access, method)) = access.split_once(":") {
                if method == "access" {
                    let ty = A::from_str(access)?;
                    return Ok(AccessOrRole::Access(Access {
                        ty,
                        id: Some(Arc::from(id.to_string())),
                    }));
                }
            }
        } else {
            if let Some((role, permission)) = v.split_once(":") {
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
        }
        anyhow::bail!("invalid access or role {v}");
    }
}

pub struct ParseResult<A, R, P> {
    pub access: BTreeSet<Access<A>>,
    pub roles: BTreeSet<Role<R, P>>,
}

impl<A, R, P> Default for ParseResult<A, R, P> {
    fn default() -> Self {
        Self {
            access: BTreeSet::default(),
            roles: BTreeSet::default(),
        }
    }
}

pub fn parse<A, R, P>(roles: &[Arc<str>]) -> ParseResult<A, R, P>
where
    A: Ord + FromStr<Err = strum::ParseError>,
    R: Ord + FromStr<Err = strum::ParseError>,
    P: Ord + FromStr<Err = strum::ParseError>,
{
    roles
        .iter()
        .fold(ParseResult::<A, R, P>::default(), |mut state, s| {
            if let Ok(v) = AccessOrRole::<A, R, P>::from_str(s) {
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

pub struct Group<A, R, P> {
    pub name: String,
    resource_roles: Vec<Role<R, P>>,
    access_level: A,
}

impl<A, R, P> Group<A, R, P> {
    pub fn new(name: String, access_level: A, resource_roles: Vec<Role<R, P>>) -> Self {
        Self {
            name,
            resource_roles,
            access_level,
        }
    }
}

impl<A, R, P> Group<A, R, P>
where
    A: Copy,
{
    pub fn access_role(&self) -> Access<A> {
        Access::new(*&self.access_level)
    }
}

impl<A, R, P> Group<A, R, P>
where
    A: AsRef<str>,
    R: AsRef<str>,
    P: AsRef<str>,
{
    pub fn resources(&self) -> Vec<String> {
        self.resource_roles.iter().map(|r| r.to_string()).collect()
    }
}

struct Inner<T> {
    encoded: Option<Arc<str>>,
    decoded: RwLock<Option<T>>,
}

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
