use std::sync::Arc;

use async_graphql::ComplexObject;
use async_graphql::ErrorExtensions;
use async_graphql::ResultExt;
use async_graphql::{Context, Object};
use futures::stream;
use futures::StreamExt;
use qm_entity::err;
use qm_entity::error::EntityError;
use qm_entity::error::EntityResult;
use qm_entity::exerr;
use qm_entity::ids::CustomerOrOrganization;
use qm_entity::ids::OrganizationUnitId;
use qm_entity::ids::OrganizationUnitIds;
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
use crate::model::CreateOrganizationUnitInput;
use crate::model::CreateUserPayload;
use crate::model::Institution;
use crate::model::OrganizationUnit;
use crate::model::OrganizationUnitData;
use crate::model::OrganizationUnitList;
use crate::mutation::remove_organization_units;
use crate::roles;
use crate::schema::auth::AuthCtx;

#[ComplexObject]
impl OrganizationUnit {
    async fn id(&self) -> async_graphql::FieldResult<OrganizationUnitId> {
        Ok(self.into())
    }
    async fn institutions(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::FieldResult<Vec<Arc<Institution>>> {
        let cache = ctx.data_unchecked::<CacheDB>();
        let organization_unit = cache.organization_unit_by_id(&self.id).await;
        if let Some(organization_unit) = organization_unit {
            return Ok(stream::iter(organization_unit.members.iter())
                .filter_map(|m| async move { cache.institution_by_id(&m.iid.into()).await })
                .collect()
                .await);
        }
        Ok(vec![])
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
    ) -> async_graphql::FieldResult<OrganizationUnitList> {
        context = self
            .0
            .enforce_customer_or_organization_context(context)
            .await?;
        Ok(self
            .0
            .store
            .cache_db()
            .organization_unit_list(context, filter)
            .await)
    }

    pub async fn by_id(&self, id: OrganizationUnitId) -> Option<Arc<OrganizationUnit>> {
        self.0
            .store
            .cache_db()
            .organization_unit_by_id(&id.into())
            .await
    }

    pub async fn create(
        &self,
        access_level: AccessLevel,
        organization_unit: OrganizationUnitData,
    ) -> EntityResult<Arc<OrganizationUnit>> {
        let user_id = self.0.auth.user_id().unwrap();
        let cid = organization_unit.cid;
        let oid = organization_unit.oid;
        let name: Arc<str> = Arc::from(organization_unit.name.clone());
        let ty = organization_unit.ty;
        let lock_key = format!("v1_organization_unit_lock_{:X}_{name}", cid.as_ref());
        let lock = self.0.store.redis().lock(&lock_key, 5000, 20, 250).await?;
        let (result, exists) = async {
            EntityResult::Ok(
                if let Some(item) = self
                    .0
                    .store
                    .cache_db()
                    .organization_unit_by_name(cid, oid, name.clone())
                    .await
                {
                    (item, true)
                } else {
                    let result = crate::mutation::create_organization_unit(
                        self.0.store.customer_db().pool(),
                        &name,
                        ty.as_deref(),
                        cid,
                        oid,
                        user_id,
                        organization_unit.members,
                    )
                    .await?;
                    let id: OrganizationUnitId = (&result).into();
                    let access = qm_role::Access::new(access_level)
                        .with_fmt_id(Some(&id))
                        .to_string();
                    let roles =
                        roles::ensure(self.0.store.keycloak(), Some(access).into_iter()).await?;
                    self.0.store.cache_db().user().new_roles(roles).await;
                    if let Some(producer) = self.0.store.mutation_event_producer() {
                        producer
                            .create_event(
                                &qm_kafka::producer::EventNs::OrganizationUnit,
                                "organization_unit",
                                &result,
                            )
                            .await?;
                    }
                    let organization_unit = Arc::new(result);
                    self.0
                        .store
                        .cache_db()
                        .infra()
                        .new_organization_unit(organization_unit.clone())
                        .await;
                    (organization_unit, false)
                },
            )
        }
        .await?;
        self.0.store.redis().unlock(&lock_key, &lock.id).await?;
        if exists {
            return err!(name_conflict::<OrganizationUnit>(name.to_string()));
        }
        Ok(result)
    }

    pub async fn remove(&self, ids: OrganizationUnitIds) -> EntityResult<u64> {
        let v: Vec<i64> = ids.iter().map(OrganizationUnitId::id).collect();
        let delete_count = remove_organization_units(self.0.store.customer_db().pool(), &v).await?;
        if delete_count != 0 {
            let id = Uuid::new_v4();
            self.0
                .store
                .cleanup_task_producer()
                .add_item(&CleanupTask {
                    id,
                    ty: CleanupTaskType::OrganizationUnits(ids),
                })
                .await?;
            log::debug!("emit cleanup task {}", id.to_string());
            return Ok(delete_count);
        }
        Ok(0)
    }
}

pub struct OrganizationUnitQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for OrganizationUnitQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    OrganizationUnitQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
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
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
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
        context: Option<CustomerOrOrganization>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<OrganizationUnitList> {
        Ctx(
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
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

pub struct OrganizationUnitMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for OrganizationUnitMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    OrganizationUnitMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn create_organization_unit(
        &self,
        ctx: &Context<'_>,
        context: CustomerOrOrganization,
        input: CreateOrganizationUnitInput,
    ) -> async_graphql::FieldResult<Arc<OrganizationUnit>> {
        let (result, auth_ctx, access_level, group_id) = match context {
            CustomerOrOrganization::Customer(context) => {
                let group_path = Auth::customer_unit_owner_group()
                    .ok_or(EntityError::bad_request(
                        "OrganizationUnit",
                        "create OrganizationUnit::Customer is not activated",
                    ))
                    .extend()?;
                let auth_ctx = AuthCtx::<Auth, Store, Resource, Permission>::mutate_with_role(
                    ctx,
                    qm_entity::ids::InfraContext::Customer(context),
                    (Resource::organization_unit(), Permission::create()),
                )
                .await?;
                let group_id = auth_ctx
                    .store
                    .cache_db()
                    .group_id_by_path(group_path)
                    .await
                    .ok_or(EntityError::internal())
                    .extend()?;
                let access_level = AccessLevel::CustomerUnit;
                let result = Ctx(&auth_ctx)
                    .create(
                        access_level,
                        OrganizationUnitData {
                            cid: context.into(),
                            oid: None,
                            name: input.name,
                            ty: input.ty,
                            members: input.members,
                        },
                    )
                    .await
                    .extend()?;
                (result, auth_ctx, access_level, group_id)
            }
            CustomerOrOrganization::Organization(context) => {
                let group_path = Auth::institution_unit_owner_group()
                    .ok_or(EntityError::bad_request(
                        "OrganizationUnit",
                        "create OrganizationUnit::Organization is not activated",
                    ))
                    .extend()?;
                let (cid, oid) = context.unzip();
                let auth_ctx = AuthCtx::<Auth, Store, Resource, Permission>::mutate_with_role(
                    ctx,
                    qm_entity::ids::InfraContext::Organization(context),
                    (Resource::organization_unit(), Permission::create()),
                )
                .await?;
                let group_id = auth_ctx
                    .store
                    .cache_db()
                    .group_id_by_path(group_path)
                    .await
                    .ok_or(EntityError::internal())
                    .extend()?;
                let access_level = AccessLevel::InstitutionUnit;
                let result = Ctx(&auth_ctx)
                    .create(
                        access_level,
                        OrganizationUnitData {
                            cid: cid.into(),
                            oid: Some(oid.into()),
                            name: input.name,
                            ty: input.ty,
                            members: input.members,
                        },
                    )
                    .await
                    .extend()?;
                (result, auth_ctx, access_level, group_id)
            }
        };
        if let Some(user) = input.initial_user {
            let id: OrganizationUnitId = result.as_ref().into();
            crate::schema::user::Ctx(&auth_ctx)
                .create(CreateUserPayload {
                    access: Some(
                        qm_role::Access::new(access_level)
                            .with_fmt_id(Some(&id))
                            .to_string(),
                    ),
                    user,
                    group_id: Some(group_id),
                    context: Some(qm_entity::ids::InfraContext::OrganizationUnit(id)),
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
    //         AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
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
        let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
            ctx,
            (Resource::organization_unit(), Permission::delete()),
        )
        .await?;
        let cache = auth_ctx.store.cache_db();
        for id in ids.iter() {
            let infra_id = id.into();
            if cache.organization_unit_by_id(&infra_id).await.is_some() {
                let object_owner = id.parent();
                auth_ctx.can_mutate(Some(&object_owner)).await.extend()?;
            } else {
                return exerr!(not_found_by_id::<OrganizationUnit>(id.to_string()));
            }
        }
        Ctx(&auth_ctx).remove(ids).await.extend()
    }
}
