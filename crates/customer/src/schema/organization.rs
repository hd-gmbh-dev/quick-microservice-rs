use std::sync::Arc;

use async_graphql::ResultExt;
use async_graphql::{Context, Object};

use qm_entity::ctx::CustomerFilter;
use qm_entity::ctx::MutationContext;
use qm_entity::ctx::OrganizationFilter;
use qm_entity::err;
use qm_entity::error::EntityResult;
use qm_entity::ids::{Cid, Oid, OrganizationId, StrictOrganizationIds};
use qm_entity::list::ListCtx;
use qm_entity::model::ListFilter;
use qm_entity::Create;
use qm_mongodb::bson::{doc, Uuid};
use qm_mongodb::DB;

use crate::cleanup::{CleanupTask, CleanupTaskType};
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
        customer_filter: Option<CustomerFilter>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<OrganizationList> {
        let mut ctx = ListCtx::new(self.0.store.organizations());
        if let Some(CustomerFilter { customer }) = customer_filter {
            ctx = ctx.with_additional_query_params(doc! {
                "cid": customer.as_ref()
            })
        }
        ctx.list(filter).await.extend()
    }

    pub async fn by_id(&self, id: OrganizationId) -> Option<Arc<Organization>> {
        self.0
            .store
            .cache()
            .customer()
            .organization_by_id(&id)
            .await
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

    pub async fn remove(&self, ids: StrictOrganizationIds) -> EntityResult<u64> {
        let db = self.0.store.as_ref();
        let mut session = db.session().await?;
        let docs = ids
            .iter()
            .map(|v| {
                let cid: &Cid = v.as_ref();
                let oid: &Oid = v.as_ref();
                doc! {"_id": **oid, "cid": **cid }
            })
            .collect::<Vec<_>>();
        if !docs.is_empty() {
            let result = self
                .0
                .store
                .organizations()
                .as_ref()
                .delete_many_with_session(doc! {"$or": docs}, None, &mut session)
                .await?;
            self.0
                .store
                .cache()
                .customer()
                .reload_organizations(self.0.store, Some(self.0.store.redis().as_ref()))
                .await?;
            if result.deleted_count != 0 {
                let id = Uuid::new();
                self.0
                    .store
                    .cleanup_task_producer()
                    .add_item(&CleanupTask {
                        id,
                        ty: CleanupTaskType::Organizations(ids),
                    })
                    .await?;
                log::debug!("emit cleanup task {}", id.to_string());
                return Ok(result.deleted_count);
            }
        }
        Ok(0)
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
        ctx: &Context<'_>,
        id: OrganizationId,
    ) -> async_graphql::FieldResult<Option<Arc<Organization>>> {
        Ok(Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::organization(), Permission::view()),
            )
            .await
            .extend()?,
        )
        .by_id(id)
        .await)
    }

    async fn organizations(
        &self,
        ctx: &Context<'_>,
        context: Option<CustomerFilter>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<OrganizationList> {
        Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::organization(), Permission::list()),
            )
            .await?,
        )
        .list(context, filter)
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
        ctx: &Context<'_>,
        ids: StrictOrganizationIds,
    ) -> async_graphql::FieldResult<u64> {
        Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::organization(), Permission::delete()),
            )
            .await?,
        )
        .remove(ids)
        .await
        .extend()
    }
}
