use async_graphql::MergedObject;

pub mod auth;
pub mod customer;
pub mod institution;
pub mod organization;
pub mod organization_unit;
pub mod user;
pub mod groups;

use crate::context::RelatedAccessLevel;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::groups::RelatedBuiltInGroup;

#[derive(MergedObject)]
pub struct QmCustomerQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>(
    customer::CustomerQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
    organization::OrganizationQueryRoot<
        Auth,
        Store,
        AccessLevel,
        Resource,
        Permission,
        BuiltInGroup,
    >,
    organization_unit::OrganizationUnitQueryRoot<
        Auth,
        Store,
        AccessLevel,
        Resource,
        Permission,
        BuiltInGroup,
    >,
    institution::InstitutionQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
    user::UserQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
    groups::GroupQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
)
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup;

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for QmCustomerQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    fn default() -> Self {
        Self(
            customer::CustomerQueryRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
            organization::OrganizationQueryRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
            organization_unit::OrganizationUnitQueryRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
            institution::InstitutionQueryRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
            user::UserQueryRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
            groups::GroupQueryRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
        )
    }
}

#[derive(MergedObject)]
pub struct QmCustomerMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>(
    customer::CustomerMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
    organization::OrganizationMutationRoot<
        Auth,
        Store,
        AccessLevel,
        Resource,
        Permission,
        BuiltInGroup,
    >,
    organization_unit::OrganizationUnitMutationRoot<
        Auth,
        Store,
        AccessLevel,
        Resource,
        Permission,
        BuiltInGroup,
    >,
    institution::InstitutionMutationRoot<
        Auth,
        Store,
        AccessLevel,
        Resource,
        Permission,
        BuiltInGroup,
    >,
    user::UserMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
    groups::GroupMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
)
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup;

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for QmCustomerMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    fn default() -> Self {
        Self(
            customer::CustomerMutationRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
            organization::OrganizationMutationRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
            organization_unit::OrganizationUnitMutationRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
            institution::InstitutionMutationRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
            user::UserMutationRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
            groups::GroupMutationRoot::<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>::default(),
        )
    }
}
