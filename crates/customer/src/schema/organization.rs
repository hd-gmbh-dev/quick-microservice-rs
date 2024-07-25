use std::sync::Arc;

use async_graphql::ComplexObject;
use async_graphql::ErrorExtensions;
use async_graphql::ResultExt;
use async_graphql::{Context, Object};

use qm_entity::error::EntityError;
use qm_entity::error::EntityResult;
use qm_entity::exerr;
use qm_entity::ids::CustomerId;
use qm_entity::ids::InfraContext;
use qm_entity::ids::InfraId;
use qm_entity::ids::OrganizationId;
use qm_entity::ids::OrganizationIds;

use qm_entity::err;
use qm_entity::model::ListFilter;
use qm_mongodb::bson::doc;
use qm_role::AccessLevel;
use sqlx::types::Uuid;

use crate::cache::CacheDB;

use crate::cleanup::CleanupTask;
use crate::cleanup::CleanupTaskType;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::groups::RelatedBuiltInGroup;
use crate::marker::Marker;
use crate::model::CreateOrganizationInput;
use crate::model::Customer;
use crate::model::Organization;
use crate::model::OrganizationData;
use crate::model::OrganizationList;
use crate::model::UpdateOrganizationInput;
use crate::mutation::remove_organizations;
use crate::mutation::update_organization;
use crate::roles;
use crate::schema::auth::AuthCtx;

#[ComplexObject]
impl Organization {
    async fn id(&self) -> async_graphql::FieldResult<OrganizationId> {
        Ok(self.into())
    }

    async fn customer(&self, ctx: &Context<'_>) -> Option<Arc<Customer>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        cache.customer_by_id(&self.customer_id).await
    }
}

pub struct Ctx<'a, Auth, Store, Resource, Permission>(
    pub &'a AuthCtx<'a, Auth, Store, Resource, Permission>,
)
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission;
impl<'a, Auth, Store, Resource, Permission> Ctx<'a, Auth, Store, Resource, Permission>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    pub async fn list(
        &self,
        mut context: Option<CustomerId>,
        filter: Option<ListFilter>,
        ty: Option<String>,
    ) -> async_graphql::FieldResult<OrganizationList> {
        context = self.0.enforce_customer_context(context).await.extend()?;
        Ok(self
            .0
            .store
            .cache_db()
            .organization_list(context, filter, ty)
            .await)
    }

    pub async fn by_id(&self, id: OrganizationId) -> Option<Arc<Organization>> {
        self.0.store.cache_db().organization_by_id(&id.into()).await
    }

    pub async fn exists(&self, cid: InfraId, name: Arc<str>) -> bool {
        self.0
            .store
            .cache_db()
            .organization_by_name(cid, name)
            .await
            .is_some()
    }

    pub async fn create(&self, organization: OrganizationData) -> EntityResult<Arc<Organization>> {
        let user_id = self.0.auth.user_id().unwrap();
        let cid = organization.0;
        let name: Arc<str> = Arc::from(organization.1.clone());
        let ty = organization.2;
        let lock_key = format!("v1_organization_lock_{:X}_{name}", cid.as_ref());
        let lock = self.0.store.redis().lock(&lock_key, 5000, 20, 250).await?;
        let (result, exists) = async {
            EntityResult::Ok(
                if let Some(item) = self
                    .0
                    .store
                    .cache_db()
                    .organization_by_name(cid, name.clone())
                    .await
                {
                    (item, true)
                } else {
                    let result = crate::mutation::create_organization(
                        self.0.store.customer_db().pool(),
                        &name,
                        ty.as_deref(),
                        cid,
                        user_id,
                    )
                    .await?;
                    let id: OrganizationId = (&result).into();
                    let access = qm_role::Access::new(AccessLevel::Organization)
                        .with_fmt_id(Some(&id))
                        .to_string();
                    let roles =
                        roles::ensure(self.0.store.keycloak(), Some(access).into_iter()).await?;
                    self.0.store.cache_db().user().new_roles(roles).await;
                    if let Some(producer) = self.0.store.mutation_event_producer() {
                        producer
                            .create_event(
                                &qm_kafka::producer::EventNs::Organization,
                                "organization",
                                &result,
                            )
                            .await?;
                    }
                    let organization = Arc::new(result);
                    self.0
                        .store
                        .cache_db()
                        .infra()
                        .new_organization(organization.clone())
                        .await;
                    (organization, false)
                },
            )
        }
        .await?;
        self.0.store.redis().unlock(&lock_key, &lock.id).await?;
        if exists {
            return err!(name_conflict::<Organization>(name.to_string()));
        }
        Ok(result)
    }

    pub async fn update(
        &self,
        id: OrganizationId,
        name: String,
    ) -> EntityResult<Arc<Organization>> {
        let user_id = self.0.auth.user_id().unwrap();
        let id: InfraId = id.into();
        let old = self
            .0
            .store
            .cache_db()
            .organization_by_id(&id)
            .await
            .ok_or(EntityError::not_found_by_field::<Organization>(
                "name", &name,
            ))?;
        let result =
            update_organization(self.0.store.customer_db().pool(), id, &name, user_id).await?;
        let new = Arc::new(result);
        self.0
            .store
            .cache_db()
            .infra()
            .update_organization(new.clone(), old.as_ref().into())
            .await;
        Ok(new)
    }

    pub async fn remove(&self, ids: OrganizationIds) -> EntityResult<u64> {
        let v: Vec<i64> = ids.iter().map(OrganizationId::id).collect();
        let delete_count = remove_organizations(self.0.store.customer_db().pool(), &v).await?;
        if delete_count != 0 {
            let id = Uuid::new_v4();
            self.0
                .store
                .cleanup_task_producer()
                .add_item(&CleanupTask {
                    id,
                    ty: CleanupTaskType::Organizations(ids),
                })
                .await?;
            log::debug!("emit cleanup task {}", id.to_string());
            return Ok(delete_count);
        }
        Ok(0)
    }
}

pub struct OrganizationQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for OrganizationQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    OrganizationQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn organization_by_id(
        &self,
        ctx: &Context<'_>,
        id: OrganizationId,
    ) -> async_graphql::FieldResult<Option<Arc<Organization>>> {
        Ok(Ctx(
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
                ctx,
                (Resource::organization(), Permission::view()),
            )
            .await
            .extend()?,
        )
        .by_id(id)
        .await)
    }

    async fn organization_exists(
        &self,
        ctx: &Context<'_>,
        id: CustomerId,
        name: Arc<str>,
    ) -> async_graphql::FieldResult<bool> {
        Ok(Ctx(
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
                ctx,
                (Resource::organization(), Permission::view()),
            )
            .await
            .extend()?,
        )
        .exists(id.into(), name)
        .await)
    }

    async fn organizations(
        &self,
        ctx: &Context<'_>,
        context: Option<CustomerId>,
        filter: Option<ListFilter>,
        ty: Option<String>,
    ) -> async_graphql::FieldResult<OrganizationList> {
        Ctx(
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
                ctx,
                (Resource::organization(), Permission::list()),
            )
            .await?,
        )
        .list(context, filter, ty)
        .await
        .extend()
    }
}

pub struct OrganizationMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for OrganizationMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    OrganizationMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn create_organization(
        &self,
        ctx: &Context<'_>,
        context: CustomerId,
        input: CreateOrganizationInput,
    ) -> async_graphql::FieldResult<Arc<Organization>> {
        let auth_ctx = AuthCtx::<Auth, Store, Resource, Permission>::mutate_with_role(
            ctx,
            qm_entity::ids::InfraContext::Customer(context),
            (Resource::organization(), Permission::create()),
        )
        .await?;
        Ctx(&auth_ctx)
            .create(OrganizationData(context.into(), input.name, input.ty))
            .await
            .extend()
    }

    async fn update_organization(
        &self,
        ctx: &Context<'_>,
        context: OrganizationId,
        input: UpdateOrganizationInput,
    ) -> async_graphql::FieldResult<Arc<Organization>> {
        let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
            ctx,
            (Resource::organization(), Permission::update()),
        )
        .await?;
        auth_ctx
            .can_mutate(Some(&InfraContext::Organization(context)))
            .await?;
        Ctx(&auth_ctx).update(context, input.name).await.extend()
    }

    async fn remove_organizations(
        &self,
        ctx: &Context<'_>,
        ids: OrganizationIds,
    ) -> async_graphql::FieldResult<u64> {
        let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
            ctx,
            (Resource::organization(), Permission::delete()),
        )
        .await?;
        let cache = auth_ctx.store.cache_db();
        for id in ids.iter() {
            let infra_id = id.into();
            if cache.organization_by_id(&infra_id).await.is_some() {
                let object_owner = InfraContext::Customer(id.parent());
                auth_ctx.can_mutate(Some(&object_owner)).await.extend()?;
            } else {
                return exerr!(not_found_by_id::<Organization>(id.to_string()));
            }
        }
        Ctx(&auth_ctx).remove(ids).await.extend()
    }
}
