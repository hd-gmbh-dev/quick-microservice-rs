use qm_entity::AsNumber;
use qm_entity::FromGraphQLContext;
use qm_entity::HasAccess;
use qm_entity::HasRole;
use qm_entity::IsAdmin;
use qm_entity::MutatePermissions;
use qm_entity::QueryPermissions;
use qm_entity::SessionAccess;
use qm_entity::UserId;
pub use qm_kafka::producer::Producer;
use qm_redis::Redis;

// use crate::cache::Cache;
// use crate::cache::CacheDB;
use crate::groups::RelatedGroups;
// use crate::roles::RoleDB;
// use crate::schema::customer::CustomerDB;
// use crate::schema::institution::InstitutionDB;
// use crate::schema::organization::OrganizationDB;
// use crate::schema::organization_unit::OrganizationUnitDB;
use crate::schema::user::KeycloakClient;
// use crate::schema::user::UserDB;
use crate::worker::CleanupTaskProducer;

pub trait MutationEventProducer {
    fn mutation_event_producer(&self) -> Option<&Producer> {
        None
    }
}

pub trait InMemoryCache {
    // fn cache(&self) -> &Cache;
    fn cache_db(&self) -> &crate::cache::CacheDB;
}

pub trait RedisClient {
    fn redis(&self) -> &Redis;
}

pub trait KeycloakDB {
    fn keycloak_db(&self) -> &qm_pg::DB;
}
pub trait CustomerDB {
    fn customer_db(&self) -> &qm_pg::DB;
}
pub trait ObjectDB {
    fn object_db(&self) -> &qm_mongodb::DB;
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
    // UserDB
    // + CustomerDB
    // + OrganizationDB
    // + OrganizationUnitDB
    // + InstitutionDB
    // + RoleDB
    // + UserDB
    KeycloakDB
    + AsRef<qm_mongodb::DB>
    + CustomerDB
    + RedisClient
    + KeycloakClient
    + InMemoryCache
    // + CacheDB
    + MutationEventProducer
    + CleanupTaskProducer
    + Clone
    + Send
    + Sync
    + 'static
{
}

pub trait UserContext<A, R, P>:
    IsAdmin + HasRole<R, P> + HasAccess<A> + AsNumber + UserId + Send + Sync + 'static
{
}

pub trait AdminContext: IsAdmin + AsNumber + UserId + Send + Sync + 'static {}

pub trait RelatedAuth<A, R, P>:
    RelatedGroups<A, R, P>
    + FromGraphQLContext
    + IsAdmin
    + UserContext<A, R, P>
    + AsNumber
    + UserId
    + SessionAccess<A>
    + Send
    + Sync
    + 'static
where
    A: AsRef<str>,
    R: std::fmt::Debug,
    P: std::fmt::Debug,
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
pub trait IdRequired {
    fn id_required(&self) -> bool;
}

pub trait RelatedAccessLevel:
    IsAdmin
    + OrganizationAccess
    + InstitutionAccess
    + OrganizationUnitAccess
    + CustomerAccess
    + Default
    + IdRequired
    + async_graphql::InputType
    + Eq
    + AsRef<str>
    + AsNumber
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
    + std::fmt::Debug
    + Send
    + Sync
    + 'static
{
}
pub trait RelatedPermission:
    MutatePermissions + QueryPermissions + std::fmt::Debug + Send + Sync + 'static
{
}
