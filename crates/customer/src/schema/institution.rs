use std::sync::Arc;

use async_graphql::ResultExt;
use async_graphql::{Context, Object};

use async_graphql::ErrorExtensions;
use qm_entity::ctx::CustOrOrgFilter;
use qm_entity::ctx::MutationContext;
use qm_entity::ctx::OrganizationFilter;
use qm_entity::error::EntityResult;
use qm_entity::ids::InstitutionIds;
use qm_entity::ids::OrganizationId;
use qm_entity::ids::{InstitutionId, InstitutionIdRef};
use qm_entity::list::ListCtx;
use qm_entity::model::ListFilter;
use qm_entity::Create;
use qm_entity::{err, exerr};
use qm_mongodb::bson::{doc, Uuid};
use qm_mongodb::DB;

use crate::cleanup::{CleanupTask, CleanupTaskType};
use crate::context::RelatedAccessLevel;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::groups::RelatedBuiltInGroup;
use crate::marker::Marker;
use crate::model::CreateInstitutionInput;
use crate::model::CreateUserPayload;
use crate::model::Institution;
use crate::model::{InstitutionData, InstitutionList};
use crate::roles;
use crate::schema::auth::AuthCtx;

pub const DEFAULT_COLLECTION: &str = "institutions";

pub trait InstitutionDB: AsRef<DB> {
    fn collection(&self) -> &str {
        DEFAULT_COLLECTION
    }
    fn institutions(&self) -> qm_entity::Collection<Institution> {
        let collection = self.collection();
        qm_entity::Collection(self.as_ref().get().collection::<Institution>(collection))
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
        cust_or_org_filter: Option<CustOrOrgFilter>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<InstitutionList> {
        ListCtx::new(self.0.store.institutions())
            .with_query(
                self.0
                    .build_context_query(cust_or_org_filter.map(Into::into).as_ref())
                    .await
                    .extend()?,
            )
            .list(filter)
            .await
            .extend()
    }

    pub async fn by_id(&self, id: InstitutionId) -> Option<Arc<Institution>> {
        self.0.store.cache().customer().institution_by_id(&id).await
    }

    pub async fn create(&self, institution: InstitutionData) -> EntityResult<Institution> {
        let OrganizationId { cid, id: oid } = institution.0.clone();
        let name = institution.1.clone();
        let lock_key = format!(
            "v1_institution_lock_{}_{}_{name}",
            cid.to_hex(),
            oid.to_hex()
        );
        let lock = self.0.store.redis().lock(&lock_key, 5000, 20, 250).await?;
        let (result, exists) = async {
            EntityResult::Ok(
                if let Some(item) = self
                    .0
                    .store
                    .institutions()
                    .by_field_with_customer_filter(&cid, "name", &name)
                    .await?
                {
                    (item, true)
                } else {
                    let result = self
                        .0
                        .store
                        .institutions()
                        .save(institution.create(&self.0.auth)?)
                        .await?;
                    let id = result.as_id();
                    let access = qm_role::Access::new(AccessLevel::institution())
                        .with_fmt_id(Some(&id))
                        .to_string();
                    let roles =
                        roles::ensure(self.0.store.keycloak(), Some(access).into_iter()).await?;
                    let cache = self.0.store.cache();
                    cache
                        .customer()
                        .new_institution(self.0.store.redis().as_ref(), result.clone())
                        .await?;
                    cache
                        .user()
                        .new_roles(self.0.store, self.0.store.redis().as_ref(), roles)
                        .await?;
                    if let Some(producer) = self.0.store.mutation_event_producer() {
                        producer
                            .create_event(
                                &qm_kafka::producer::EventNs::Institution,
                                InstitutionDB::collection(self.0.store),
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
            return err!(name_conflict::<Institution>(name));
        }
        Ok(result)
    }

    pub async fn remove(&self, ids: InstitutionIds) -> EntityResult<u64> {
        let db = self.0.store.as_ref();
        let mut session = db.session().await?;
        let docs = ids
            .iter()
            .map(|v| {
                let InstitutionIdRef { cid, oid, iid } = v.into();
                doc! {"_id": iid, "owner.entityId.cid": cid, "owner.entityId.oid": oid }
            })
            .collect::<Vec<_>>();
        if !docs.is_empty() {
            let result = self
                .0
                .store
                .institutions()
                .as_ref()
                .delete_many_with_session(doc! {"$or": docs}, None, &mut session)
                .await?;
            self.0
                .store
                .cache()
                .customer()
                .reload_institutions(self.0.store, Some(self.0.store.redis().as_ref()))
                .await?;
            if result.deleted_count != 0 {
                let id = Uuid::new();
                self.0
                    .store
                    .cleanup_task_producer()
                    .add_item(&CleanupTask {
                        id,
                        ty: CleanupTaskType::Institutions(ids),
                    })
                    .await?;
                log::debug!("emit cleanup task {}", id.to_string());
                return Ok(result.deleted_count);
            }
        }
        Ok(0)
    }
}

pub struct InstitutionQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for InstitutionQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
    InstitutionQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
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
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::institution(), Permission::view()),
            )
            .await
            .extend()?,
        )
        .by_id(id)
        .await)
    }

    async fn institutions(
        &self,
        ctx: &Context<'_>,
        context: Option<CustOrOrgFilter>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<InstitutionList> {
        Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::institution(), Permission::list()),
            )
            .await?,
        )
        .list(context, filter)
        .await
        .extend()
    }
}

pub struct InstitutionMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for InstitutionMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
    InstitutionMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn create_institution(
        &self,
        ctx: &Context<'_>,
        context: OrganizationFilter,
        input: CreateInstitutionInput,
    ) -> async_graphql::FieldResult<Institution> {
        let result = Ctx(
            AuthCtx::<Auth, Store, AccessLevel, Resource, Permission>::mutate_with_role(
                ctx,
                MutationContext::Organization(context.clone()),
                (Resource::institution(), Permission::create()),
            )
            .await?,
        )
        .create(InstitutionData(context.into(), input.name))
        .await
        .extend()?;
        if let Some(user) = input.initial_user {
            let id = result.as_id();
            crate::schema::user::Ctx(
                AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                    ctx,
                    (Resource::user(), Permission::create()),
                )
                .await?,
            )
            .create(CreateUserPayload {
                access: qm_role::Access::new(AccessLevel::institution())
                    .with_fmt_id(Some(&id))
                    .to_string(),
                user,
                group: Auth::create_institution_owner_group().name,
                context: qm_entity::ctx::ContextFilterInput::Institution(id.into()),
            })
            .await
            .extend()?;
        }
        Ok(result)
    }

    // async fn update_institution(
    //     &self,
    //     ctx: &Context<'_>,
    //     context: InstitutionFilter,
    //     input: UpdateInstitutionInput,
    // ) -> async_graphql::FieldResult<Institution> {
    //     Ctx(
    //         AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
    //             ctx,
    //             (Resource::institution(), Permission::update()),
    //         )
    //         .await?,
    //     )
    //     .update(context, input)
    //     .await
    //     .extend()
    // }

    async fn remove_institutions(
        &self,
        ctx: &Context<'_>,
        ids: InstitutionIds,
    ) -> async_graphql::FieldResult<u64> {
        let auth_ctx =
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::institution(), Permission::delete()),
            )
            .await?;
        let cache = auth_ctx.store.cache();
        for id in ids.iter() {
            let id = id.clone();
            let v = cache.customer().institution_by_id(&id).await;
            if let Some(v) = v {
                auth_ctx.can_mutate(&v.owner).await.extend()?;
            } else {
                return exerr!(not_found_by_id::<Institution>(id.to_string()));
            }
        }
        Ctx(auth_ctx).remove(ids).await.extend()
    }
}
