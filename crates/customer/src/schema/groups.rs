use async_graphql::{Context, FieldResult, Object};
use futures::StreamExt;

use qm_entity::ids::InfraContext;

use std::collections::HashSet;

use std::sync::Arc;

use crate::cache::CacheDB;
use crate::schema::auth::AuthGuard;
use sqlx::types::Uuid;

use crate::groups::RelatedBuiltInGroup;
use crate::marker::Marker;
use crate::model::UserGroup;
use qm_role::AccessLevel;

use crate::model::{Customer, Institution, Organization, OrganizationUnit};
// use crate::model::User;
// use crate::model::{CreateUserInput, CreateUserPayload, UserList};
// use crate::model::{RequiredUserAction, UserData, UserDetails};
use crate::schema::auth::AuthCtx;
use crate::schema::RelatedAuth;
use crate::schema::RelatedPermission;
use crate::schema::RelatedResource;
use crate::schema::RelatedStorage;

#[Object]
impl UserGroup {
    async fn id(&self) -> Arc<str> {
        self.group_id.clone()
    }

    async fn name(&self) -> Option<Arc<str>> {
        self.group_detail.display_name.clone()
    }

    async fn customer(&self, ctx: &Context<'_>) -> Option<Arc<Customer>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        if let Some(id) = self
            .group_detail
            .context
            .as_ref()
            .map(InfraContext::customer_id)
        {
            return cache.customer_by_id(&id).await;
        }
        None
    }

    async fn organization(&self, ctx: &Context<'_>) -> Option<Arc<Organization>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        if let Some(id) = self
            .group_detail
            .context
            .as_ref()
            .and_then(InfraContext::organization_id)
        {
            return cache.organization_by_id(&id).await;
        }
        None
    }

    async fn organization_unit(&self, ctx: &Context<'_>) -> Option<Arc<OrganizationUnit>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        if let Some(id) = self
            .group_detail
            .context
            .as_ref()
            .and_then(InfraContext::organization_unit_id)
        {
            return cache.organization_unit_by_id(&id).await;
        }
        None
    }

    async fn institution(&self, ctx: &Context<'_>) -> Option<Arc<Institution>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        if let Some(id) = self
            .group_detail
            .context
            .as_ref()
            .and_then(InfraContext::institution_id)
        {
            return cache.institution_by_id(&id).await;
        }
        None
    }

    async fn allowed_access_levels(&self) -> Option<Arc<[AccessLevel]>> {
        self.group_detail.allowed_access_levels.clone()
    }
}

pub struct Groups;
impl Default for Groups {
    fn default() -> Self {
        Self
    }
}

#[Object]
impl Groups {
    async fn app(&self, ctx: &Context<'_>) -> FieldResult<Vec<UserGroup>> {
        let cache = ctx.data_unchecked::<CacheDB>();
        // Ok(Arc::from(cache.groups_by_parent("app").await.into_iter().map(|group| UserGroup { group, _marker: Default::default()}).collect::<Vec<UserGroup>>()))
        let groups = cache.groups_by_parent("app").await;
        Ok(futures::stream::iter(groups)
            .filter_map(|g| async move {
                cache.group_detail_by_id(&g.id).await.map(|v| UserGroup {
                    group_id: g.id.clone(),
                    group_detail: v,
                })
            })
            .collect()
            .await)
    }

    async fn custom(
        &self,
        _ctx: &Context<'_>,
        _context: InfraContext,
    ) -> FieldResult<Arc<[UserGroup]>> {
        unimplemented!()
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
    pub async fn create(
        &self,
        _name: String,
        _context: InfraContext,
        _allowed_access_levels: HashSet<AccessLevel>,
        _roles: HashSet<qm_role::Role<Resource, Permission>>,
    ) -> async_graphql::FieldResult<Arc<UserGroup>> {
        //     let ctx = context.to_string();
        //     let path = format!("/custom@{ctx}/{}", inflector::cases::snakecase::to_snake_case(&name.replace("/", "").trim()));
        //     let groups = ensure_groups_with_roles(
        //         self.0.store.keycloak().config().realm(),
        //         self.0.store.keycloak(),
        //         vec![
        //             qm_role::Group::<Resource, Permission>::new(name, path.clone(), allowed_access_levels.into_iter().collect(), roles.into_iter().collect())
        //         ], false).await?;
        //     let group = groups.get(&path).ok_or(EntityError::internal())?;
        //     let group = fetch_group_by_id(self.0.store.keycloak_db(), self.0.store.keycloak().config().realm(), group.id.as_ref().ok_or(EntityError::internal())?).await?;
        //     let group = Arc::new(Group {
        //         id: Arc::from(group.group_id.unwrap()),
        //         parent_group: group.parent_group.map(|v| Arc::from(v)),
        //         built_in: group.built_in.map(|v| v == "1").unwrap_or(false),
        //         allowed_access_levels: None,
        //         context: group.context.and_then(|s| s.parse().ok()),
        //         display_name: None,
        //         name: Arc::from(group.group_name.unwrap()),
        //     });
        //     self.0.store.cache_db().user().new_group(group.clone()).await;
        //     Ok(group)
        unimplemented!()
    }

    pub async fn remove(&self, _ids: Arc<[Uuid]>) -> async_graphql::FieldResult<u64> {
        //     let ctx = context.to_string();
        //     let path = format!("/custom@{ctx}/{}", inflector::cases::snakecase::to_snake_case(&name.replace("/", "").trim()));
        //     let groups = ensure_groups_with_roles(
        //         self.0.store.keycloak().config().realm(),
        //         self.0.store.keycloak(),
        // vec![
        //             qm_role::Group::<Resource, Permission>::new(name, path.clone(), allowed_access_levels.into_iter().collect(), roles.into_iter().collect())
        //         ], false).await?;
        //     let group = groups.get(&path).ok_or(EntityError::internal())?;
        //     let group = fetch_group_by_id(self.0.store.keycloak_db(), self.0.store.keycloak().config().realm(), group.id.as_ref().ok_or(EntityError::internal())?).await?;
        //     let group = Arc::new(Group {
        //         id: Arc::from(group.group_id.unwrap()),
        //         parent_group: group.parent_group.map(|v| Arc::from(v)),
        //         built_in: group.built_in.map(|v| v == "1").unwrap_or(false),
        //         allowed_access_levels: None,
        //         context: group.context.and_then(|s| s.parse().ok()),
        //         display_name: None,
        //         name: Arc::from(group.group_name.unwrap()),
        //     });
        //     self.0.store.cache_db().user().new_group(group.clone()).await;
        //     Ok(group)
        unimplemented!()
    }
}

pub struct GroupQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for GroupQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    GroupQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    #[graphql(
        guard = "AuthGuard::<Auth, Store, Resource, Permission>::new(Resource::user(), Permission::create())"
    )]
    async fn groups(&self) -> Groups {
        Groups
    }
}

pub struct GroupMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for GroupMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    GroupMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn create_group(
        &self,
        ctx: &Context<'_>,
        context: InfraContext,
        name: String,
        allowed_access_levels: HashSet<AccessLevel>,
        roles: HashSet<qm_role::Role<Resource, Permission>>,
    ) -> async_graphql::FieldResult<Arc<UserGroup>> {
        let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
            ctx,
            (Resource::user(), Permission::create()),
        )
        .await?;
        auth_ctx.can_mutate(Some(&context)).await?;
        let roles = roles.into_iter().collect();
        Ctx(&auth_ctx)
            .create(name, context, allowed_access_levels, roles)
            .await
    }

    async fn remove_groups(
        &self,
        ctx: &Context<'_>,
        ids: Arc<[Uuid]>,
    ) -> async_graphql::FieldResult<u64> {
        let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
            ctx,
            (Resource::user(), Permission::create()),
        )
        .await?;

        Ctx(&auth_ctx).remove(ids).await
    }
}
