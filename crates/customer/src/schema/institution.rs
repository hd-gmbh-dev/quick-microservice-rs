use std::sync::Arc;

use async_graphql::ResultExt;
use async_graphql::{Context, Object};

use async_graphql::ComplexObject;
use async_graphql::ErrorExtensions;
use qm_entity::error::{EntityError, EntityResult};
use qm_entity::ids::{CustomerOrOrganization, InstitutionIds};
use qm_entity::ids::{InfraContext, InstitutionId};
use qm_entity::ids::{InfraId, OrganizationId};
use qm_entity::model::ListFilter;
use qm_entity::{err, exerr};
use qm_mongodb::bson::doc;
use qm_role::AccessLevel;
use sqlx::types::Uuid;

use crate::cache::CacheDB;

use crate::cleanup::{CleanupTask, CleanupTaskType};
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::groups::RelatedBuiltInGroup;
use crate::marker::Marker;
use crate::model::Customer;
use crate::model::Institution;
use crate::model::Organization;
use crate::model::{CreateInstitutionInput, UpdateInstitutionInput};
use crate::model::{InstitutionData, InstitutionList};
use crate::mutation::{remove_institutions, update_institution};
use crate::roles;
use crate::schema::auth::AuthCtx;

#[ComplexObject]
impl Institution {
    async fn id(&self) -> async_graphql::FieldResult<InstitutionId> {
        Ok(self.into())
    }

    async fn customer(&self, ctx: &Context<'_>) -> Option<Arc<Customer>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            tracing::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        cache.customer_by_id(&self.customer_id).await
    }

    async fn organization(&self, ctx: &Context<'_>) -> Option<Arc<Organization>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            tracing::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        cache.organization_by_id(&self.organization_id).await
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
        mut context: Option<CustomerOrOrganization>,
        filter: Option<ListFilter>,
        ty: Option<String>,
    ) -> async_graphql::FieldResult<InstitutionList> {
        context = self
            .0
            .enforce_customer_or_organization_context(context)
            .await
            .extend()?;
        Ok(self
            .0
            .store
            .cache_db()
            .institution_list(context, filter, ty)
            .await)
    }

    pub async fn by_id(&self, id: InstitutionId) -> Option<Arc<Institution>> {
        self.0.store.cache_db().institution_by_id(&id.into()).await
    }

    pub async fn exists(&self, cid: InfraId, oid: InfraId, name: Arc<str>) -> bool {
        self.0
            .store
            .cache_db()
            .institution_by_name(cid, oid, name)
            .await
            .is_some()
    }

    pub async fn create(&self, institution: InstitutionData) -> EntityResult<Arc<Institution>> {
        let user_id = self.0.auth.user_id().unwrap();
        let (cid, oid) = institution.0.unzip();
        let name: Arc<str> = Arc::from(institution.1.clone());
        let ty = institution.2;
        let lock_key = format!("v1_institution_lock_{cid:X}_{oid:X}_{name}",);
        let lock = self.0.store.redis().lock(&lock_key, 5000, 20, 250).await?;
        let (result, exists) = async {
            EntityResult::Ok(
                if let Some(item) = self
                    .0
                    .store
                    .cache_db()
                    .institution_by_name(cid.into(), oid.into(), name.clone())
                    .await
                {
                    (item, true)
                } else {
                    let result = crate::mutation::create_institution(
                        self.0.store.customer_db().pool(),
                        institution.3,
                        &name,
                        ty.as_deref(),
                        cid.into(),
                        oid.into(),
                        user_id,
                    )
                    .await?;
                    let id: InstitutionId = (&result).into();
                    let access = qm_role::Access::new(AccessLevel::Institution)
                        .with_fmt_id(Some(&id))
                        .to_string();
                    let roles =
                        roles::ensure(self.0.store.keycloak(), Some(access).into_iter()).await?;
                    self.0.store.cache_db().user().new_roles(roles).await;
                    if let Some(producer) = self.0.store.mutation_event_producer() {
                        producer
                            .create_event(
                                &qm_kafka::producer::EventNs::Institution,
                                "institution",
                                &result,
                            )
                            .await?;
                    }
                    let institution = Arc::new(result);
                    self.0
                        .store
                        .cache_db()
                        .infra()
                        .new_institution(institution.clone())
                        .await;
                    (institution, false)
                },
            )
        }
        .await?;
        self.0.store.redis().unlock(&lock_key, &lock.id).await?;
        if exists {
            return err!(name_conflict::<Institution>(name.to_string()));
        }
        Ok(result)
    }

    pub async fn update(&self, id: InstitutionId, name: String) -> EntityResult<Arc<Institution>> {
        let user_id = self.0.auth.user_id().unwrap();
        let id: InfraId = id.into();
        let old = self.0.store.cache_db().institution_by_id(&id).await.ok_or(
            EntityError::not_found_by_field::<Institution>("name", &name),
        )?;
        let result =
            update_institution(self.0.store.customer_db().pool(), id, &name, user_id).await?;
        let new = Arc::new(result);
        self.0
            .store
            .cache_db()
            .infra()
            .update_institution(new.clone(), old.as_ref().into())
            .await;
        Ok(new)
    }

    pub async fn remove(&self, ids: InstitutionIds) -> EntityResult<u64> {
        let v: Vec<i64> = ids.iter().map(InstitutionId::id).collect();
        let delete_count = remove_institutions(self.0.store.customer_db().pool(), &v).await?;
        if delete_count != 0 {
            let id = Uuid::new_v4();
            self.0
                .store
                .cleanup_task_producer()
                .add_item(&CleanupTask {
                    id,
                    ty: CleanupTaskType::Institutions(ids),
                })
                .await?;
            tracing::debug!("emit cleanup task {}", id.to_string());
            return Ok(delete_count);
        }
        Ok(0)
    }
}

pub struct InstitutionQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for InstitutionQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    InstitutionQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn institution_by_id(
        &self,
        ctx: &Context<'_>,
        id: InstitutionId,
    ) -> async_graphql::FieldResult<Option<Arc<Institution>>> {
        Ok(Ctx(
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
                ctx,
                &qm_role::role!(Resource::institution(), Permission::view()),
            )
            .await
            .extend()?,
        )
        .by_id(id)
        .await)
    }

    async fn institution_exists(
        &self,
        ctx: &Context<'_>,
        id: OrganizationId,
        name: Arc<str>,
    ) -> async_graphql::FieldResult<bool> {
        let (cid, oid) = id.unzip();
        Ok(Ctx(
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
                ctx,
                &qm_role::role!(Resource::institution(), Permission::view()),
            )
            .await
            .extend()?,
        )
        .exists(cid.into(), oid.into(), name)
        .await)
    }

    async fn institutions(
        &self,
        ctx: &Context<'_>,
        context: Option<CustomerOrOrganization>,
        filter: Option<ListFilter>,
        ty: Option<String>,
    ) -> async_graphql::FieldResult<InstitutionList> {
        Ctx(
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
                ctx,
                &qm_role::role!(Resource::institution(), Permission::list()),
            )
            .await?,
        )
        .list(context, filter, ty)
        .await
        .extend()
    }
}

pub struct InstitutionMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for InstitutionMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    InstitutionMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn create_institution(
        &self,
        ctx: &Context<'_>,
        context: OrganizationId,
        input: CreateInstitutionInput,
    ) -> async_graphql::FieldResult<Arc<Institution>> {
        let auth_ctx = AuthCtx::<Auth, Store, Resource, Permission>::mutate_with_role(
            ctx,
            qm_entity::ids::InfraContext::Organization(context),
            &qm_role::role!(Resource::institution(), Permission::create()),
        )
        .await?;
        Ctx(&auth_ctx)
            .create(InstitutionData(context, input.name, input.ty, input.id))
            .await
            .extend()
    }

    async fn update_institution(
        &self,
        ctx: &Context<'_>,
        context: InstitutionId,
        input: UpdateInstitutionInput,
    ) -> async_graphql::FieldResult<Arc<Institution>> {
        let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
            ctx,
            &qm_role::role!(Resource::institution(), Permission::update()),
        )
        .await?;
        auth_ctx
            .can_mutate(Some(&InfraContext::Institution(context)))
            .await?;
        Ctx(&auth_ctx).update(context, input.name).await.extend()
    }

    async fn remove_institutions(
        &self,
        ctx: &Context<'_>,
        ids: InstitutionIds,
    ) -> async_graphql::FieldResult<u64> {
        let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
            ctx,
            &qm_role::role!(Resource::institution(), Permission::delete()),
        )
        .await?;
        let cache = auth_ctx.store.cache_db();
        for id in ids.iter() {
            let infra_id = id.into();
            if cache.institution_by_id(&infra_id).await.is_some() {
                let object_owner = InfraContext::Organization(id.parent());
                auth_ctx.can_mutate(Some(&object_owner)).await.extend()?;
            } else {
                return exerr!(not_found_by_id::<Institution>(id.to_string()));
            }
        }
        Ctx(&auth_ctx).remove(ids).await.extend()
    }
}
