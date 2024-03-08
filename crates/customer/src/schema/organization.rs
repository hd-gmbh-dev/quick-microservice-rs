use async_graphql::ResultExt;
use async_graphql::{Context, Object};

use qm_entity::ctx::CustomerFilter;
use qm_entity::ctx::MutationContext;
use qm_entity::ctx::OrganizationFilter;
use qm_entity::err;
use qm_entity::error::EntityResult;
use qm_entity::ids::OrganizationId;
use qm_entity::list::ListCtx;
use qm_entity::model::ListFilter;
use qm_entity::Create;
use qm_mongodb::DB;

use crate::context::RelatedAccessLevel;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::marker::Marker;
use crate::model::CreateOrganizationInput;
use crate::model::CreateUserInput;
use crate::model::Organization;
use crate::model::{OrganizationData, OrganizationList, UpdateOrganizationInput};
use crate::roles;
use crate::schema::auth::AuthCtx;

pub const DEFAULT_COLLECTION: &str = "organizations";

pub trait OrganizationDB: AsRef<DB> {
    fn collection(&self) -> &str {
        DEFAULT_COLLECTION
    }
    fn organizations(&self) -> qm_entity::Collection<Organization> {
        let collection = self.collection();
        qm_entity::Collection(self.as_ref().get().collection::<Organization>(collection))
    }
}

pub struct Ctx<'a, Auth, Store, AccessLevel, Resource, Permission>(
    pub AuthCtx<'a, Auth, Store, AccessLevel, Resource, Permission>,
)
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission;
impl<'a, Auth, Store, AccessLevel, Resource, Permission>
    Ctx<'a, Auth, Store, AccessLevel, Resource, Permission>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    pub async fn list(
        &self,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<OrganizationList> {
        ListCtx::new(self.0.store.organizations())
            .list(filter)
            .await
            .extend()
    }

    pub async fn create(&self, organization: OrganizationData) -> EntityResult<Organization> {
        let cid = organization.0.clone();
        let name = organization.1.clone();
        let lock_key = format!("v1_organization_lock_{}_{name}", organization.0.to_hex());
        let lock = self.0.store.redis().lock(&lock_key, 5000, 20, 250).await?;
        let (result, exists) = async {
            EntityResult::Ok(
                if let Some(item) = self
                    .0
                    .store
                    .organizations()
                    .by_field_with_customer_filter(&cid, "name", &name)
                    .await?
                {
                    (item, true)
                } else {
                    let result = self
                        .0
                        .store
                        .organizations()
                        .save(organization.create(&self.0.auth)?)
                        .await?;
                    let access = qm_role::Access::new(AccessLevel::organization())
                        .with_fmt_id(result.id.as_organization_id().as_ref())
                        .to_string();
                    let roles =
                        roles::ensure(self.0.store.keycloak(), Some(access).into_iter()).await?;

                    let cache = self.0.store.cache();
                    cache
                        .customer()
                        .new_organization(self.0.store.redis().as_ref(), result.clone())
                        .await?;
                    cache
                        .user()
                        .new_roles(self.0.store, self.0.store.redis().as_ref(), roles)
                        .await?;
                    if let Some(producer) = self.0.store.mutation_event_producer() {
                        producer
                            .create_event(
                                &qm_kafka::producer::EventNs::Organization,
                                OrganizationDB::collection(self.0.store),
                                &result,
                            )
                            .await?;
                    }
                    (result, false)
                },
            )
        }
        .await?;
        self.0.store.redis().unlock(&lock_key, &lock.id).await?;
        if exists {
            return err!(name_conflict::<Organization>(name));
        }
        Ok(result)
    }
}

pub struct OrganizationQueryRoot<Auth, Store, AccessLevel, Resource, Permission> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission>,
}

impl<Auth, Store, AccessLevel, Resource, Permission> Default
    for OrganizationQueryRoot<Auth, Store, AccessLevel, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission>
    OrganizationQueryRoot<Auth, Store, AccessLevel, Resource, Permission>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
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
        ctx: &Context<'_>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<OrganizationList> {
        Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::organization(), Permission::list()),
            )
            .await?,
        )
        .list(filter)
        .await
        .extend()
    }
}

pub struct OrganizationMutationRoot<Auth, Store, AccessLevel, Resource, Permission> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission>,
}

impl<Auth, Store, AccessLevel, Resource, Permission> Default
    for OrganizationMutationRoot<Auth, Store, AccessLevel, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission>
    OrganizationMutationRoot<Auth, Store, AccessLevel, Resource, Permission>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    async fn create_organization(
        &self,
        ctx: &Context<'_>,
        context: CustomerFilter,
        input: CreateOrganizationInput,
    ) -> async_graphql::FieldResult<Organization> {
        let result = Ctx(
            AuthCtx::<Auth, Store, AccessLevel, Resource, Permission>::mutate_with_role(
                ctx,
                MutationContext::Customer(context.clone()),
                (Resource::organization(), Permission::create()),
            )
            .await?,
        )
        .create(OrganizationData(context.customer, input.name))
        .await
        .extend()?;
        if let Some(user) = input.initial_user {
            crate::schema::user::Ctx(
                AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                    ctx,
                    (Resource::user(), Permission::create()),
                )
                .await?,
            )
            .create(CreateUserInput {
                access: qm_role::Access::new(AccessLevel::organization())
                    .with_fmt_id(result.id.as_organization_id().as_ref())
                    .to_string(),
                user,
                group: Auth::create_organization_owner_group().name,
                context: qm_entity::ctx::ContextFilterInput::Organization(OrganizationFilter {
                    customer: result.id.cid.clone().unwrap(),
                    organization: result.id.id.clone().unwrap(),
                }),
            })
            .await
            .extend()?;
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
