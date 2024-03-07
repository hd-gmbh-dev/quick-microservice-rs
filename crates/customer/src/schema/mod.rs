use async_graphql::MergedObject;

pub mod auth;
pub mod customer;
pub mod institution;
pub mod organization;
pub mod organization_unit;
pub mod user;

use crate::context::RelatedAccessLevel;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;

#[derive(MergedObject)]
pub struct QmCustomerQueryRoot<Auth, Store, AccessLevel, Resource, Permission>(
    customer::CustomerQueryRoot<Auth, Store, AccessLevel, Resource, Permission>,
    organization::OrganizationQueryRoot<Auth, Store, AccessLevel, Resource, Permission>,
    organization_unit::OrganizationUnitQueryRoot<Auth, Store, AccessLevel, Resource, Permission>,
    institution::InstitutionQueryRoot<Auth, Store, AccessLevel, Resource, Permission>,
)
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission;

impl<Auth, Store, AccessLevel, Resource, Permission> Default
    for QmCustomerQueryRoot<Auth, Store, AccessLevel, Resource, Permission>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    fn default() -> Self {
        Self(
            customer::CustomerQueryRoot::<Auth, Store, AccessLevel, Resource, Permission>::default(),
            organization::OrganizationQueryRoot::<Auth, Store, AccessLevel, Resource, Permission>::default(),
            organization_unit::OrganizationUnitQueryRoot::<Auth, Store, AccessLevel, Resource, Permission>::default(),
            institution::InstitutionQueryRoot::<Auth, Store, AccessLevel, Resource, Permission>::default(),
        )
    }
}

#[derive(MergedObject)]
pub struct QmCustomerMutationRoot<Auth, Store, AccessLevel, Resource, Permission>(
    customer::CustomerMutationRoot<Auth, Store, AccessLevel, Resource, Permission>,
    organization::OrganizationMutationRoot<Auth, Store, AccessLevel, Resource, Permission>,
    organization_unit::OrganizationUnitMutationRoot<Auth, Store, AccessLevel, Resource, Permission>,
    institution::InstitutionMutationRoot<Auth, Store, AccessLevel, Resource, Permission>,
)
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission;

impl<Auth, Store, AccessLevel, Resource, Permission> Default
    for QmCustomerMutationRoot<Auth, Store, AccessLevel, Resource, Permission>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    fn default() -> Self {
        Self(
            customer::CustomerMutationRoot::<Auth, Store, AccessLevel, Resource, Permission>::default(),
            organization::OrganizationMutationRoot::<Auth, Store, AccessLevel, Resource, Permission>::default(),
            organization_unit::OrganizationUnitMutationRoot::<Auth, Store, AccessLevel, Resource, Permission>::default(),
            institution::InstitutionMutationRoot::<Auth, Store, AccessLevel, Resource, Permission>::default(),
        )
    }
}
