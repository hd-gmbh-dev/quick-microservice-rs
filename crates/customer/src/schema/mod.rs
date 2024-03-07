use async_graphql::MergedObject;

pub mod auth;
pub mod customer;
pub mod institution;
pub mod organization;
pub mod organization_unit;
pub mod user;

use crate::context::RelatedAccess;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;

#[derive(MergedObject)]
pub struct QmCustomerQueryRoot<Auth, Store, Access, Resource, Permission>(
    customer::CustomerQueryRoot<Auth, Store, Access, Resource, Permission>,
    organization::OrganizationQueryRoot<Auth, Store, Access, Resource, Permission>,
    organization_unit::OrganizationUnitQueryRoot<Auth, Store, Access, Resource, Permission>,
    institution::InstitutionQueryRoot<Auth, Store, Access, Resource, Permission>,
)
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission;

impl<Auth, Store, Access, Resource, Permission> Default
    for QmCustomerQueryRoot<Auth, Store, Access, Resource, Permission>
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    fn default() -> Self {
        Self(
            customer::CustomerQueryRoot::<Auth, Store, Access, Resource, Permission>::default(),
            organization::OrganizationQueryRoot::<Auth, Store, Access, Resource, Permission>::default(),
            organization_unit::OrganizationUnitQueryRoot::<Auth, Store, Access, Resource, Permission>::default(),
            institution::InstitutionQueryRoot::<Auth, Store, Access, Resource, Permission>::default(),
        )
    }
}

#[derive(MergedObject)]
pub struct QmCustomerMutationRoot<Auth, Store, Access, Resource, Permission>(
    customer::CustomerMutationRoot<Auth, Store, Access, Resource, Permission>,
    organization::OrganizationMutationRoot<Auth, Store, Access, Resource, Permission>,
    organization_unit::OrganizationUnitMutationRoot<Auth, Store, Access, Resource, Permission>,
    institution::InstitutionMutationRoot<Auth, Store, Access, Resource, Permission>,
)
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission;

impl<Auth, Store, Access, Resource, Permission> Default
    for QmCustomerMutationRoot<Auth, Store, Access, Resource, Permission>
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    fn default() -> Self {
        Self(
            customer::CustomerMutationRoot::<Auth, Store, Access, Resource, Permission>::default(),
            organization::OrganizationMutationRoot::<Auth, Store, Access, Resource, Permission>::default(),
            organization_unit::OrganizationUnitMutationRoot::<Auth, Store, Access, Resource, Permission>::default(),
            institution::InstitutionMutationRoot::<Auth, Store, Access, Resource, Permission>::default(),
        )
    }
}
