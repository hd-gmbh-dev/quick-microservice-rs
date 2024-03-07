use async_graphql::Context;
use async_graphql::FieldResult;

use qm_entity::ids::{CustomerId, InstitutionId, OrganizationId, OrganizationUnitId};
use qm_entity::FromGraphQLContext;
use qm_entity::UserId;
use qm_keycloak::Keycloak;
use qm_mongodb::DB;

use crate::context::RelatedAccess;
use crate::model::User;
use crate::model::UserInput;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub enum Owner {
    Customer(CustomerId),
    Organization(OrganizationId),
    Institution(InstitutionId),
    OrganizationUnit(OrganizationUnitId),
}

pub trait KeycloakClient {
    fn keycloak(&self) -> &Keycloak;
}

impl<T> KeycloakClient for T
where
    T: AsRef<Keycloak>,
{
    fn keycloak(&self) -> &Keycloak {
        self.as_ref()
    }
}

pub const DEFAULT_COLLECTION: &str = "users";

pub trait UserDB {
    fn collection(&self) -> &str {
        DEFAULT_COLLECTION
    }
    fn user_db(&self) -> &qm_mongodb::DB;
    fn users(&self) -> qm_entity::Collection<User> {
        let collection = self.collection();
        qm_entity::Collection(self.user_db().get().collection::<User>(collection))
    }
}

impl<T> UserDB for T
where
    T: AsRef<DB>,
{
    fn user_db(&self) -> &DB {
        self.as_ref()
    }
}

pub trait UserStorage: UserDB + KeycloakClient + Send + Sync + 'static {}

pub struct UserCtx<'ctx, Auth, Store, Access, Resource, Permission> {
    _auth: Auth,
    _store: &'ctx Store,
    _marker: std::marker::PhantomData<Option<(Access, Resource, Permission)>>,
}

impl<'ctx, Auth, Store, Access, Resource, Permission>
    UserCtx<'ctx, Auth, Store, Access, Resource, Permission>
{
    pub fn new(auth: Auth, store: &'ctx Store) -> Self {
        Self {
            _auth: auth,
            _store: store,
            _marker: Default::default(),
        }
    }
}

impl<'ctx, Auth, Store, Access, Resource, Permission>
    UserCtx<'ctx, Auth, Store, Access, Resource, Permission>
where
    Auth: FromGraphQLContext,
    Store: Send + Sync + 'static,
    Access: RelatedAccess,
    Resource: Send + Sync + 'static,
    Permission: Send + Sync + 'static,
{
    pub async fn from_graphql(ctx: &'ctx Context<'_>) -> FieldResult<Self> {
        Ok(Self::new(
            Auth::from_graphql_context(ctx).await?,
            ctx.data_unchecked::<Store>(),
        ))
    }
}

impl<'ctx, Auth, Store, Access, Resource, Permission>
    UserCtx<'ctx, Auth, Store, Access, Resource, Permission>
where
    Auth: UserId + Send + Sync + 'static,
    Store: Send + Sync + 'static,
    Access: RelatedAccess,
    Resource: Send + Sync + 'static,
    Permission: Send + Sync + 'static,
{
    pub async fn create(
        &self,
        _group: qm_role::Group<Access, Resource, Permission>,
        _owner: Owner,
        _input: UserInput,
    ) -> anyhow::Result<User> {
        // TODO: check if owner access role exist
        // TODO: create user with group "CustomerOwner"
        unimplemented!()
    }
}
