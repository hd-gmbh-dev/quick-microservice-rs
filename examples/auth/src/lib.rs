use std::{collections::HashSet, sync::Arc};

use async_graphql::ResultExt;
use qm::{
    entity::{
        err, AsNumber, FromGraphQLContext, HasAccess, HasRole, IsAdmin, IsSupport,
        MutatePermissions, QueryPermissions, SessionAccess, UserId,
    },
    keycloak::token::jwt::Claims,
    role::Access,
};
use qm_example_ctx::Storage;
use sqlx::types::Uuid;

pub mod roles;
use crate::roles::{Permission, Resource};

pub type AuthContainer = qm::role::AuthContainer<Authorization>;
pub type Role = qm::role::Role<Resource, Permission>;
pub type Group = qm::role::Group<Resource, Permission>;

#[derive(Default)]
struct Inner {
    _claims: Option<Claims>,
    access: Option<Access>,
    roles: HashSet<Role>,
    is_admin: bool,
    is_support: bool,
    user_id: Option<Uuid>,
}

#[derive(Default, Clone)]
pub struct Authorization {
    inner: Arc<Inner>,
}

#[async_trait::async_trait]
impl FromGraphQLContext for Authorization {
    async fn from_graphql_context(
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::FieldResult<Self> {
        let auth_container = ctx.data_unchecked::<AuthContainer>();
        if let Some(v) = auth_container.read().await.clone() {
            return Ok(v);
        }
        if let Some(encoded) = auth_container.encoded() {
            let mut v = auth_container.write().await;
            let storage = ctx.data_unchecked::<Storage>();
            let claims: Claims = storage.jwt_store().decode(encoded).await?;
            let user_id = Uuid::parse_str(&claims.sub)?;
            let mut parsed = qm::role::parse(&claims.realm_access.roles);
            let is_admin = parsed
                .roles
                .contains(&qm::role::role!(Resource::Administration));
            let is_support = parsed.roles.contains(&qm::role::role!(Resource::Support));

            let access = if is_admin {
                Access::new("admin".into())
            } else {
                match parsed.access.pop_first() {
                    Some(access) => access,
                    None => err!(unauthorized_user(Some(&user_id))).extend()?,
                }
            };
            let result = Self {
                inner: Arc::new(Inner {
                    _claims: Some(claims),
                    access: Some(access),
                    roles: parsed.roles,
                    is_admin,
                    is_support,
                    user_id: Some(user_id),
                }),
            };
            v.replace(result.clone());
            return Ok(result);
        }
        Ok(Self::default())
    }
}

impl IsAdmin for Authorization {
    fn is_admin(&self) -> bool {
        self.inner.is_admin
    }
}

impl IsSupport for Authorization {
    fn is_support(&self) -> bool {
        self.inner.is_support
    }
}

impl IsSupport for Resource {
    fn is_support(&self) -> bool {
        matches!(self, Resource::Support)
    }
}

impl IsAdmin for Resource {
    fn is_admin(&self) -> bool {
        matches!(self, Resource::Administration)
    }
}

impl UserId for Authorization {
    fn user_id(&self) -> Option<&Uuid> {
        self.inner.user_id.as_ref()
    }
}

impl HasRole<Resource, Permission> for Authorization {
    fn has_role(&self, r: &Resource, p: &Permission) -> bool {
        self.inner.roles.contains(&Role::from((*r, *p)))
    }
    fn has_role_object(&self, role: &qm::role::Role<Resource, Permission>) -> bool {
        self.inner.roles.contains(role)
    }
}

impl AsNumber for Authorization {
    fn as_number(&self) -> u32 {
        self.inner
            .access
            .as_ref()
            .map(|v| match v.ty() {
                "admin" => u32::MAX,
                "support" => u32::MAX - 1,
                "customer" => u32::MAX - 2,
                "organization" => u32::MAX - 3,
                "institution" => u32::MAX - 4,
                _ => 0,
            })
            .unwrap_or(0)
    }
}

impl MutatePermissions for Permission {
    fn create() -> Self {
        Permission::Create
    }

    fn update() -> Self {
        Permission::Update
    }

    fn delete() -> Self {
        Permission::Delete
    }
}

impl QueryPermissions for Permission {
    fn list() -> Self {
        Permission::List
    }

    fn view() -> Self {
        Permission::View
    }
}
impl HasAccess for Authorization {
    fn has_access(&self, a: &qm::role::Access) -> bool {
        self.inner.access.as_ref().map(|v| a == v).unwrap_or(false)
    }
}
impl SessionAccess for Authorization {
    fn session_access(&self) -> Option<&qm::role::Access> {
        self.inner.access.as_ref()
    }
}
