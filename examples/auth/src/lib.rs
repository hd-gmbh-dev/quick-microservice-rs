use std::{collections::BTreeSet, sync::Arc};

use async_graphql::ResultExt;
use qm::{
    customer::{
        context::{
            AdminContext, CustomerAccess, CustomerResource, IdRequired, InstitutionAccess,
            InstitutionResource, OrganizationAccess, OrganizationResource, OrganizationUnitAccess,
            OrganizationUnitResource, RelatedAuth, RelatedPermission, RelatedResource, UserContext,
            UserResource,
        },
        groups::{
            CreateCustomerOwnerGroup, CreateInstitutionOwnerGroup, CreateOrganizationOwnerGroup,
            CreateOrganizationUnitOwnerGroup, RelatedBuiltInGroup, RelatedGroups,
        },
    },
    entity::{
        err, AsNumber, FromGraphQLContext, HasAccess, HasRole, IsAdmin, MutatePermissions,
        QueryPermissions, SessionAccess, UserId,
    },
    keycloak::token::jwt::Claims,
};
use qm_example_ctx::Storage;
use roles::BuiltInGroup;
use sqlx::types::Uuid;

pub mod roles;
use crate::roles::{AccessLevel, Permission, Resource, BUILT_IN_GROUPS};

pub type AuthContainer = qm::role::AuthContainer<Authorization>;
pub type Access = qm::role::Access<AccessLevel>;
pub type Role = qm::role::Role<Resource, Permission>;
pub type Group = qm::role::Group<AccessLevel, Resource, Permission>;

impl AsNumber for AccessLevel {
    fn as_number(&self) -> u32 {
        match self {
            Self::Admin => u32::MAX,
            Self::Customer => u32::MAX - 1,
            // Self::Organization => u32::MAX -2,
            // Self::OrganizationUnit => u32::MAX -3,
            Self::Institution => u32::MAX - 3,
            Self::None => 0,
        }
    }
}

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

impl IsAdmin for AccessLevel {
    fn is_admin(&self) -> bool {
        matches!(self, AccessLevel::Admin)
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
}

impl CreateCustomerOwnerGroup<AccessLevel, Resource, Permission> for Authorization {
    fn create_customer_owner_group() -> Group {
        roles::customer_owner_group()
    }
}

impl CreateOrganizationOwnerGroup<AccessLevel, Resource, Permission> for Authorization {
    fn create_organization_owner_group() -> Group {
        roles::customer_owner_group()
    }
}

impl CreateOrganizationUnitOwnerGroup<AccessLevel, Resource, Permission> for Authorization {
    fn create_organization_unit_owner_group() -> qm::role::Group<AccessLevel, Resource, Permission>
    {
        roles::customer_owner_group()
    }
}

impl CreateInstitutionOwnerGroup<AccessLevel, Resource, Permission> for Authorization {
    fn create_institution_owner_group() -> qm::role::Group<AccessLevel, Resource, Permission> {
        roles::institution_owner_group()
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

impl CustomerAccess for AccessLevel {
    fn customer() -> Self {
        Self::Customer
    }
}
impl OrganizationUnitAccess for AccessLevel {
    fn organization_unit() -> Self {
        Self::Customer
    }
}
impl OrganizationAccess for AccessLevel {
    fn organization() -> Self {
        Self::Customer
    }
}
impl InstitutionAccess for AccessLevel {
    fn institution() -> Self {
        Self::Institution
    }
}
impl IdRequired for AccessLevel {
    fn id_required(&self) -> bool {
        matches!(self, AccessLevel::Customer | AccessLevel::Institution)
    }
}
impl qm::customer::context::RelatedAccessLevel for AccessLevel {}

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
impl HasAccess<AccessLevel> for Authorization {
    fn has_access(&self, a: &qm::role::Access<AccessLevel>) -> bool {
        self.inner.access.as_ref().map(|v| a == v).unwrap_or(false)
    }
}

impl UserContext<AccessLevel, Resource, Permission> for Authorization {}
impl RelatedGroups<AccessLevel, Resource, Permission> for Authorization {
    fn built_in_groups() -> &'static [&'static str] {
        &BUILT_IN_GROUPS
    }
}
impl SessionAccess<AccessLevel> for Authorization {
    fn session_access(&self) -> Option<&qm::role::Access<AccessLevel>> {
        self.inner.access.as_ref()
    }
}
impl RelatedAuth<AccessLevel, Resource, Permission> for Authorization {}

impl RelatedBuiltInGroup for BuiltInGroup {}
