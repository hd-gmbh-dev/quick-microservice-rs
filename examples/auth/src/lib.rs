use std::{collections::BTreeSet, sync::Arc};

use async_graphql::ResultExt;
use qm::{
    customer::{
        context::{
            AdminContext, CustomerResource, InstitutionResource, OrganizationResource,
            OrganizationUnitResource, RelatedAuth, RelatedPermission, RelatedResource, UserContext,
            UserResource,
        },
        groups::{
            CustomerOwnerGroup, CustomerUnitOwnerGroup, InstitutionOwnerGroup,
            InstitutionUnitOwnerGroup, OrganizationOwnerGroup, RelatedBuiltInGroup, RelatedGroups,
        },
    },
    entity::{
        err, AsNumber, FromGraphQLContext, HasAccess, HasRole, IsAdmin, MutatePermissions,
        QueryPermissions, SessionAccess, UserId,
    },
    keycloak::token::jwt::Claims,
    role::{Access, AccessLevel},
};
use qm_example_ctx::Storage;
use roles::BuiltInGroup;
use sqlx::types::Uuid;

pub mod roles;
use crate::roles::{Permission, Resource, BUILT_IN_GROUPS};

pub type AuthContainer = qm::role::AuthContainer<Authorization>;
pub type Role = qm::role::Role<Resource, Permission>;
pub type Group = qm::role::Group<Resource, Permission>;

#[derive(Default)]
struct Inner {
    _claims: Option<Claims>,
    access: Option<Access>,
    roles: BTreeSet<Role>,
    is_admin: bool,
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

            let access = if is_admin {
                Access::new(AccessLevel::Admin)
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

impl AdminContext for Authorization {}
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
            .map(|v| v.ty().as_number())
            .unwrap_or(0)
    }
}

impl CustomerResource for Resource {
    fn customer() -> Self {
        Self::Customer
    }
}
impl OrganizationUnitResource for Resource {
    fn organization_unit() -> Self {
        Self::Customer
    }
}
impl OrganizationResource for Resource {
    fn organization() -> Self {
        Self::Customer
    }
}
impl InstitutionResource for Resource {
    fn institution() -> Self {
        Self::Institution
    }
}
impl UserResource for Resource {
    fn user() -> Self {
        Self::User
    }
}
impl RelatedResource for Resource {}

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
impl RelatedPermission for Permission {}
impl HasAccess for Authorization {
    fn has_access(&self, a: &qm::role::Access) -> bool {
        self.inner.access.as_ref().map(|v| a == v).unwrap_or(false)
    }
}
impl CustomerOwnerGroup<Resource, Permission> for Authorization {
    fn customer_owner_group() -> Option<&'static str> {
        Some(roles::CUSTOMER_OWNER_PATH)
    }
}

impl OrganizationOwnerGroup<Resource, Permission> for Authorization {
    fn organization_owner_group() -> Option<&'static str> {
        None
    }
}

impl InstitutionOwnerGroup<Resource, Permission> for Authorization {
    fn institution_owner_group() -> Option<&'static str> {
        Some(roles::INSTITUTION_OWNER_PATH)
    }
}

impl CustomerUnitOwnerGroup<Resource, Permission> for Authorization {
    fn customer_unit_owner_group() -> Option<&'static str> {
        None
    }
}

impl InstitutionUnitOwnerGroup<Resource, Permission> for Authorization {
    fn institution_unit_owner_group() -> Option<&'static str> {
        None
    }
}

impl UserContext<Resource, Permission> for Authorization {}
impl RelatedGroups<Resource, Permission> for Authorization {
    fn built_in_groups() -> &'static [&'static str] {
        &BUILT_IN_GROUPS
    }
}
impl SessionAccess for Authorization {
    fn session_access(&self) -> Option<&qm::role::Access> {
        self.inner.access.as_ref()
    }
}
impl RelatedAuth<Resource, Permission> for Authorization {}
impl RelatedBuiltInGroup for BuiltInGroup {}
