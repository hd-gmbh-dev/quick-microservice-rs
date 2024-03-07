use qm_entity::FromGraphQLContext;
use qm_entity::HasAccess;
use qm_entity::HasRole;
use qm_entity::IsAdmin;
use qm_entity::MutatePermissions;
use qm_entity::UserAccessLevel;
use qm_entity::UserId;
pub use qm_kafka::producer::Producer;
use qm_redis::Redis;

use crate::cache::Cache;
use crate::groups::RelatedGroups;
use crate::roles::RoleDB;
use crate::schema::customer::CustomerDB;
use crate::schema::institution::InstitutionDB;
use crate::schema::organization::OrganizationDB;
use crate::schema::organization_unit::OrganizationUnitDB;
use crate::schema::user::KeycloakClient;
use crate::schema::user::UserDB;

pub trait MutationEventProducer {
    fn mutation_event_producer(&self) -> Option<&Producer> {
        None
    }
}

pub trait InMemoryCache {
    fn cache(&self) -> Option<&Cache> {
        None
    }
}

pub trait RedisClient {
    fn redis(&self) -> &Redis;
}

impl<T> RedisClient for T
where
    T: AsRef<Redis>,
{
    fn redis(&self) -> &Redis {
        self.as_ref()
    }
}

pub trait RelatedStorage:
    UserDB
    + CustomerDB
    + OrganizationDB
    + OrganizationUnitDB
    + InstitutionDB
    + RoleDB
    + UserDB
    + RedisClient
    + KeycloakClient
    + InMemoryCache
    + MutationEventProducer
    + Send
    + Sync
    + 'static
{
}

pub trait UserContext<A, R, P>:
    IsAdmin + HasRole<R, P> + HasAccess<A> + UserAccessLevel + UserId + Send + Sync + 'static
{
}

pub trait AdminContext: IsAdmin + UserAccessLevel + UserId + Send + Sync + 'static {}

pub trait RelatedAuth<A, R, P>:
    RelatedGroups<A, R, P>
    + FromGraphQLContext
    + IsAdmin
    + UserContext<A, R, P>
    + UserAccessLevel
    + UserId
    + Send
    + Sync
    + 'static
{
}

pub trait OrganizationAccess {
    fn organization() -> Self;
}
pub trait InstitutionAccess {
    fn institution() -> Self;
}
pub trait OrganizationUnitAccess {
    fn organization_unit() -> Self;
}
pub trait CustomerAccess {
    fn customer() -> Self;
}

pub trait RelatedAccessLevel:
    OrganizationAccess
    + InstitutionAccess
    + OrganizationUnitAccess
    + CustomerAccess
    + AsRef<str>
    + Ord
    + Send
    + Sync
    + 'static
{
}

pub trait OrganizationResource {
    fn organization() -> Self;
}
pub trait InstitutionResource {
    fn institution() -> Self;
}
pub trait OrganizationUnitResource {
    fn organization_unit() -> Self;
}
pub trait CustomerResource {
    fn customer() -> Self;
}
pub trait UserResource {
    fn user() -> Self;
}

pub trait RelatedResource:
    OrganizationResource
    + InstitutionResource
    + OrganizationUnitResource
    + CustomerResource
    + UserResource
    + Send
    + Sync
    + 'static
{
}
pub trait RelatedPermission: MutatePermissions + Send + Sync + 'static {}
