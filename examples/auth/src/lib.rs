use std::{collections::BTreeSet, sync::Arc};

use async_graphql::ResultExt;
use qm::{
    customer::{
        context::{
            AdminContext, CustomerAccess, CustomerResource, InstitutionAccess, InstitutionResource,
            OrganizationAccess, OrganizationResource, OrganizationUnitAccess,
            OrganizationUnitResource, RelatedAuth, RelatedPermission, RelatedResource, UserContext,
        },
        groups::{
            CreateCustomerOwnerGroup, CreateInstitutionOwnerGroup, CreateOrganizationOwnerGroup,
            CreateOrganizationUnitOwnerGroup, RelatedGroups,
        },
    },
    entity::{
        err, FromGraphQLContext, HasAccess, HasRole, IsAdmin, MutatePermissions, UserAccessLevel,
        UserId,
    },
    keycloak::token::jwt::Claims,
    mongodb::bson::Uuid,
};
use qm_example_ctx::Storage;

pub mod roles;
use crate::roles::{AccessLevel, Permission, Resource};

pub type AuthContainer = qm::role::AuthContainer<Authorization>;
pub type Access = qm::role::Access<AccessLevel>;
pub type Role = qm::role::Role<Resource, Permission>;
pub type Group = qm::role::Group<AccessLevel, Resource, Permission>;

impl AccessLevel {
    fn as_u32(&self) -> u32 {
        match self {
            Self::Admin => u32::MAX,
            Self::Customer => u32::MAX - 1,
            // Self::Organization => u32::MAX -2,
            // Self::OrganizationUnit => u32::MAX -3,
            Self::Institution => u32::MAX - 3,
        }
    }
}
impl PartialOrd for AccessLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_u32().partial_cmp(&other.as_u32())
    }
}
impl Ord for AccessLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
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
            let user_id = Uuid::parse_str(&claims.sub).ok();
            let mut parsed = qm::role::parse(&claims.realm_access.roles);
            let is_admin = parsed
                .roles
                .contains(&qm::role::role!(Resource::Administration));

            let access = if is_admin {
                Access::new(AccessLevel::Admin)
            } else {
                match parsed.access.pop_first() {
                    Some(access) => access,
                    None => err!(unauthorized_user(user_id.as_ref())).extend()?,
                }
            };
            let result = Self {
                inner: Arc::new(Inner {
                    _claims: Some(claims),
                    access: Some(access),
                    roles: parsed.roles,
                    is_admin,
                    user_id,
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

impl UserAccessLevel for Authorization {
    fn user_access_level(&self) -> Option<&impl Ord> {
        self.inner.access.as_ref()
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

impl qm::customer::context::RelatedAccess for AccessLevel {}

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
impl RelatedPermission for Permission {}
impl HasAccess<AccessLevel> for Authorization {
    fn has_access(&self, a: &qm::role::Access<AccessLevel>) -> bool {
        self.inner.access.as_ref().map(|v| a == v).unwrap_or(false)
    }
}
impl UserContext<AccessLevel, Resource, Permission> for Authorization {}
impl RelatedGroups<AccessLevel, Resource, Permission> for Authorization {}
impl RelatedAuth<AccessLevel, Resource, Permission> for Authorization {}
