use async_graphql::{Context, Object, ResultExt};

use qm_entity::ctx::CustomerFilter;
use qm_entity::ctx::MutationContext;
use qm_entity::err;
use qm_entity::ids::OrganizationId;
use qm_entity::model::ListFilter;
use qm_entity::Create;
use qm_mongodb::DB;

use crate::context::RelatedAccess;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::marker::Marker;
use crate::model::CreateOrganizationInput;
use crate::model::Organization;
use crate::model::{OrganizationData, OrganizationList, UpdateOrganizationInput};
use crate::schema::auth::AuthCtx;
use crate::schema::user::KeycloakClient;
use crate::schema::user::Owner;
use crate::schema::user::UserCtx;

pub const DEFAULT_COLLECTION: &str = "organizations";

pub trait OrganizationDB {
    fn collection(&self) -> &str {
        DEFAULT_COLLECTION
    }
    fn organization_db(&self) -> &DB;
    fn organizations(&self) -> qm_entity::Collection<Organization> {
        let collection = self.collection();
        qm_entity::Collection(
            self.organization_db()
                .get()
                .collection::<Organization>(collection),
        )
    }
}

impl<T> OrganizationDB for T
where
    T: AsRef<DB>,
{
    fn organization_db(&self) -> &DB {
        self.as_ref()
    }
}

pub trait OrganizationStorage: OrganizationDB + KeycloakClient + Send + Sync + 'static {}

pub trait CreateOrganizationOwnerGroup<A, R, P> {
    fn create_organization_owner_group() -> qm_role::Group<A, R, P>;
}

pub struct OrganizationQueryRoot<Auth, Store, Access, Resource, Permission> {
    _marker: Marker<Auth, Store, Access, Resource, Permission>,
}

impl<Auth, Store, Access, Resource, Permission> Default
    for OrganizationQueryRoot<Auth, Store, Access, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Access, Resource, Permission>
    OrganizationQueryRoot<Auth, Store, Access, Resource, Permission>
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: Send + Sync + 'static,
    Permission: Send + Sync + 'static,
{
    async fn organization_by_id(
        &self,
        _ctx: &Context<'_>,
        _id: OrganizationId,
    ) -> async_graphql::FieldResult<Option<Organization>> {
        // Ok(OrganizationCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .by_id(&id)
        //     .await?)
        unimplemented!()
    }

    async fn organizations(
        &self,
        _ctx: &Context<'_>,
        _filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<OrganizationList> {
        // Ok(OrganizationCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .list(filter)
        //     .await?)
        unimplemented!()
    }
}

pub trait OrganizationResource {
    fn organization() -> Self;
}

pub struct OrganizationMutationRoot<Auth, Store, Access, Resource, Permission> {
    _marker: Marker<Auth, Store, Access, Resource, Permission>,
}

impl<Auth, Store, Access, Resource, Permission> Default
    for OrganizationMutationRoot<Auth, Store, Access, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Access, Resource, Permission>
    OrganizationMutationRoot<Auth, Store, Access, Resource, Permission>
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    async fn create_organization(
        &self,
        ctx: &Context<'_>,
        context: CustomerFilter,
        input: CreateOrganizationInput,
    ) -> async_graphql::FieldResult<Organization> {
        let auth_ctx = AuthCtx::<Auth, Store, Access, Resource, Permission>::mutate_with_role(
            ctx,
            MutationContext::Customer(context.clone()),
            (Resource::organization(), Permission::create()),
        )
        .await?;
        let organization = OrganizationData(context.customer, input.name);
        let item = auth_ctx
            .store
            .organizations()
            .by_name(&organization.1)
            .await?;
        if item.is_some() {
            return err!(name_conflict::<Organization>(organization.1)).extend();
        }
        let result = auth_ctx
            .store
            .organizations()
            .save(organization.create(&auth_ctx.auth).extend()?)
            .await?;

        if let Some(initial_user) = input.initial_user {
            UserCtx::<Auth, Store, Access, Resource, Permission>::from_graphql(ctx)
                .await?
                .create(
                    Auth::create_organization_owner_group(),
                    Owner::Organization(result.id.clone().into()),
                    initial_user,
                )
                .await?;
        }
        Ok(result)
    }

    async fn update_organization(
        &self,
        _ctx: &Context<'_>,
        _input: UpdateOrganizationInput,
    ) -> async_graphql::FieldResult<Organization> {
        // Ok(OrganizationCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .update(&input)
        //     .await?)
        unimplemented!()
    }

    async fn remove_organizations(
        &self,
        _ctx: &Context<'_>,
        _ids: Vec<OrganizationId>,
    ) -> async_graphql::FieldResult<usize> {
        // Ok(OrganizationCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .remove(&ids)
        //     .await?)
        unimplemented!()
    }
}
