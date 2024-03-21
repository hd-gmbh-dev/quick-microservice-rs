use std::sync::Arc;

use async_graphql::ErrorExtensions;
use async_graphql::ResultExt;
use async_graphql::{Context, Object};

use qm_entity::ctx::CustOrOrgFilter;
use qm_entity::ctx::MutationContext;
use qm_entity::error::EntityResult;
use qm_entity::ids::OrganizationUnitId;
use qm_entity::ids::OrganizationUnitIds;
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
use crate::model::CreateOrganizationUnitInput;
use crate::model::CreateUserPayload;
use crate::model::OrganizationUnit;
use crate::model::{OrganizationUnitData, OrganizationUnitList};
use crate::roles;
use crate::schema::auth::AuthCtx;

pub const DEFAULT_COLLECTION: &str = "organization_units";

pub trait OrganizationUnitDB: AsRef<DB> {
    fn collection(&self) -> &str {
        DEFAULT_COLLECTION
    }
    fn organization_units(&self) -> qm_entity::Collection<OrganizationUnit> {
        let collection = self.collection();
        qm_entity::Collection(
            self.as_ref()
                .get()
                .collection::<OrganizationUnit>(collection),
        )
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
        context: Option<CustOrOrgFilter>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<OrganizationUnitList> {
        ListCtx::new(self.0.store.organization_units())
            .with_query(
                self.0
                    .build_context_query(context.map(Into::into).as_ref())
                    .await
                    .extend()?,
            )
            .list(filter)
            .await
            .extend()
    }

    pub async fn by_id(&self, id: OrganizationUnitId) -> Option<Arc<OrganizationUnit>> {
        self.0
            .store
            .cache()
            .customer()
            .organization_unit_by_id(&id)
            .await
    }

    pub async fn create(
        &self,
        organization_unit: OrganizationUnitData,
    ) -> EntityResult<OrganizationUnit> {
        let cid = organization_unit.cid.clone();
        let name = organization_unit.name.clone();
        let lock_key = format!("v1_organization_unit_lock_{}_{name}", cid.to_hex());
        let lock = self.0.store.redis().lock(&lock_key, 5000, 20, 250).await?;
        let (result, exists) = async {
            EntityResult::Ok(
                if let Some(item) = self
                    .0
                    .store
                    .organization_units()
                    .by_field_with_customer_filter(&cid, "name", &name)
                    .await?
                {
                    (item, true)
                } else {
                    let result = self
                        .0
                        .store
                        .organization_units()
                        .save(organization_unit.create(&self.0.auth)?)
                        .await?;
                    let id = result.as_id();
                    let access = qm_role::Access::new(AccessLevel::organization_unit())
                        .with_fmt_id(Some(&id))
                        .to_string();
                    let roles =
                        roles::ensure(self.0.store.keycloak(), Some(access).into_iter()).await?;
                    let cache = self.0.store.cache();
                    cache
                        .customer()
                        .new_organization_unit(self.0.store.redis().as_ref(), result.clone())
                        .await?;
                    cache
                        .user()
                        .new_roles(self.0.store, self.0.store.redis().as_ref(), roles)
                        .await?;
                    if let Some(producer) = self.0.store.mutation_event_producer() {
                        producer
                            .create_event(
                                &qm_kafka::producer::EventNs::OrganizationUnit,
                                OrganizationUnitDB::collection(self.0.store),
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
            return err!(name_conflict::<OrganizationUnit>(name));
        }
        Ok(result)
    }

    pub async fn remove(&self, ids: OrganizationUnitIds) -> EntityResult<u64> {
        let db = self.0.store.as_ref();
        let mut session = db.session().await?;
        let docs = ids
            .iter()
            .map(|v| match v {
                OrganizationUnitId::Customer(v) => {
                    doc! {
                        "_id": v.id.as_ref(),
                        "owner.entityId.cid": v.cid.as_ref(),
                    }
                }
                OrganizationUnitId::Organization(v) => {
                    doc! {
                        "_id": v.id.as_ref(),
                        "owner.entityId.cid": v.cid.as_ref(),
                        "owner.entityId.oid": v.oid.as_ref(),
                    }
                }
            })
            .collect::<Vec<_>>();
        if !docs.is_empty() {
            let result = self
                .0
                .store
                .organization_units()
                .as_ref()
                .delete_many_with_session(doc! {"$or": docs}, None, &mut session)
                .await?;
            self.0
                .store
                .cache()
                .customer()
                .reload_organization_units(self.0.store, Some(self.0.store.redis().as_ref()))
                .await?;
            if result.deleted_count != 0 {
                let id = Uuid::new();
                self.0
                    .store
                    .cleanup_task_producer()
                    .add_item(&CleanupTask {
                        id,
                        ty: CleanupTaskType::OrganizationUnits(ids),
                    })
                    .await?;
                log::debug!("emit cleanup task {}", id.to_string());
                return Ok(result.deleted_count);
            }
        }
        Ok(0)
    }
}

pub struct OrganizationUnitQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for OrganizationUnitQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
    OrganizationUnitQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn organization_unit_by_id(
        &self,
        ctx: &Context<'_>,
        id: OrganizationUnitId,
    ) -> async_graphql::FieldResult<Option<Arc<OrganizationUnit>>> {
        Ok(Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::organization_unit(), Permission::view()),
            )
            .await
            .extend()?,
        )
        .by_id(id)
        .await)
    }

    async fn organization_units(
        &self,
        ctx: &Context<'_>,
        context: Option<CustOrOrgFilter>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<OrganizationUnitList> {
        Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::organization_unit(), Permission::list()),
            )
            .await?,
        )
        .list(context, filter)
        .await
        .extend()
    }
}

pub struct OrganizationUnitMutationRoot<
    Auth,
    Store,
    AccessLevel,
    Resource,
    Permission,
    BuiltInGroup,
> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for OrganizationUnitMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
    OrganizationUnitMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn create_organization_unit(
        &self,
        ctx: &Context<'_>,
        context: CustOrOrgFilter,
        input: CreateOrganizationUnitInput,
    ) -> async_graphql::FieldResult<OrganizationUnit> {
        let result = match context {
            CustOrOrgFilter::Customer(context) => {
                Ctx(
                    AuthCtx::<Auth, Store, AccessLevel, Resource, Permission>::mutate_with_role(
                        ctx,
                        MutationContext::Customer(context.clone()),
                        (Resource::organization_unit(), Permission::create()),
                    )
                    .await?,
                )
                .create(OrganizationUnitData {
                    cid: context.customer.clone(),
                    oid: None,
                    name: input.name,
                    members: input.members,
                })
                .await
                .extend()?
            }
            CustOrOrgFilter::Organization(context) => {
                Ctx(
                    AuthCtx::<Auth, Store, AccessLevel, Resource, Permission>::mutate_with_role(
                        ctx,
                        MutationContext::Organization(context.clone()),
                        (Resource::organization_unit(), Permission::create()),
                    )
                    .await?,
                )
                .create(OrganizationUnitData {
                    cid: context.customer.clone(),
                    oid: Some(context.organization.clone()),
                    name: input.name,
                    members: input.members,
                })
                .await
                .extend()?
            }
        };
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
                access: Some(
                    qm_role::Access::new(AccessLevel::organization_unit())
                        .with_fmt_id(Some(&id))
                        .to_string(),
                ),
                user,
                group: Some(Auth::create_organization_unit_owner_group().name),
                context: Some(qm_entity::ctx::ContextFilterInput::OrganizationUnit(
                    id.into(),
                )),
            })
            .await
            .extend()?;
        }
        Ok(result)
    }

    // async fn update_organization_unit(
    //     &self,
    //     ctx: &Context<'_>,
    //     context: OrganizationUnitFilter,
    //     input: UpdateOrganizationUnitInput,
    // ) -> async_graphql::FieldResult<OrganizationUnit> {
    //     Ctx(
    //         AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
    //             ctx,
    //             (Resource::organization_unit(), Permission::update()),
    //         )
    //         .await?,
    //     )
    //     .update(context, input)
    //     .await
    //     .extend()
    // }

    async fn remove_organization_units(
        &self,
        ctx: &Context<'_>,
        ids: OrganizationUnitIds,
    ) -> async_graphql::FieldResult<u64> {
        let auth_ctx =
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::organization_unit(), Permission::delete()),
            )
            .await?;
        let cache = auth_ctx.store.cache();
        for id in ids.iter() {
            let id = id.clone();
            let v = cache.customer().organization_unit_by_id(&id).await;
            if let Some(v) = v {
                auth_ctx.can_mutate(&v.owner).await.extend()?;
            } else {
                return exerr!(not_found_by_id::<OrganizationUnit>(id.to_string()));
            }
        }
        Ctx(auth_ctx).remove(ids).await.extend()
    }
}
