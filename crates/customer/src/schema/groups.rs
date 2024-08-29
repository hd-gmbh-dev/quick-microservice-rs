use async_graphql::{Context, FieldResult, Object};
use async_graphql::{ErrorExtensions, ResultExt};
use futures::StreamExt;
use qm_entity::error::EntityError;
use qm_entity::exerr;
use qm_entity::ids::InfraContext;
use qm_keycloak::realm::ensure_groups_with_roles;

use std::collections::HashSet;

use std::sync::Arc;

use crate::cache::CacheDB;
use crate::query::fetch_group_by_id;
use crate::schema::auth::AuthGuard;
use sqlx::types::Uuid;

use crate::groups::RelatedBuiltInGroup;
use crate::marker::Marker;
use crate::model::{Group, GroupDetail, Role, UserGroup};
use qm_role::AccessLevel;

use crate::model::{Customer, Institution, Organization /* OrganizationUnit */};
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

    async fn roles(&self, ctx: &Context<'_>) -> Option<Arc<[Arc<Role>]>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            tracing::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        cache.roles_by_group_id(&self.group_id).await
    }

    async fn customer(&self, ctx: &Context<'_>) -> Option<Arc<Customer>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            tracing::warn!("qm::customer::cache::CacheDB is not installed in schema context");
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
            tracing::warn!("qm::customer::cache::CacheDB is not installed in schema context");
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

    // async fn organization_unit(&self, ctx: &Context<'_>) -> Option<Arc<OrganizationUnit>> {
    //     let cache = ctx.data::<CacheDB>().ok();
    //     if cache.is_none() {
    //         tracing::warn!("qm::customer::cache::CacheDB is not installed in schema context");
    //         return None;
    //     }
    //     let cache = cache.unwrap();
    //     if let Some(id) = self
    //         .group_detail
    //         .context
    //         .as_ref()
    //         .and_then(InfraContext::organization_unit_id)
    //     {
    //         return cache.organization_unit_by_id(&id).await;
    //     }
    //     None
    // }

    async fn institution(&self, ctx: &Context<'_>) -> Option<Arc<Institution>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            tracing::warn!("qm::customer::cache::CacheDB is not installed in schema context");
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

    async fn allowed_types(&self) -> Option<Arc<[Arc<str>]>> {
        self.group_detail.allowed_types.clone()
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
        ctx: &Context<'_>,
        context: InfraContext,
    ) -> FieldResult<Vec<UserGroup>> {
        let cache = ctx.data_unchecked::<CacheDB>();
        let parent = format!("custom@{context}");
        let groups = cache.groups_by_parent(&parent).await;
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
        name: String,
        context: InfraContext,
        allowed_access_levels: HashSet<AccessLevel>,
        allowed_types: HashSet<String>,
        roles: HashSet<qm_role::Role<Resource, Permission>>,
    ) -> async_graphql::FieldResult<Arc<UserGroup>> {
        let path = format!(
            "/custom@{context}/{}",
            inflector::cases::snakecase::to_snake_case(name.replace('/', "").trim())
        );
        if self
            .0
            .store
            .cache_db()
            .group_id_by_path(&path)
            .await
            .is_some()
        {
            return exerr!(name_conflict::<Group>(name));
        }
        let groups = ensure_groups_with_roles(
            self.0.store.keycloak().config().realm(),
            self.0.store.keycloak(),
            vec![qm_role::Group::<Resource, Permission>::new(
                name,
                path.clone(),
                allowed_access_levels.clone().into_iter().collect(),
                allowed_types.clone().into_iter().collect(),
                roles.into_iter().collect(),
            )],
            false,
        )
        .await?;
        let kc_group = groups.get(&path).ok_or(EntityError::internal())?;
        let group_query = fetch_group_by_id(
            self.0.store.keycloak_db(),
            kc_group.id.as_ref().ok_or(EntityError::internal())?,
        )
        .await?;
        let parent_name = Arc::from(group_query.parent_name.unwrap());
        let group = Arc::new(Group {
            id: Arc::from(group_query.group_id.unwrap()),
            parent_group: group_query.parent_group.map(Arc::from),
            name: Arc::from(group_query.name.unwrap()),
        });
        let group_detail = Arc::new(GroupDetail {
            allowed_access_levels: Some(allowed_access_levels.into_iter().collect()),
            allowed_types: Some(allowed_types.into_iter().map(|s| s.into()).collect()),
            built_in: false,
            context: Some(context),
            display_name: group_query.display_name.map(Arc::from),
        });
        let group_id = group.id.clone();
        self.0
            .store
            .cache_db()
            .user()
            .new_group(group, parent_name, group_detail.clone())
            .await;
        Ok(Arc::new(UserGroup {
            group_detail,
            group_id,
        }))
    }

    pub async fn remove(&self, ids: &[Arc<str>]) -> async_graphql::FieldResult<u64> {
        let mut i = 0;
        for id in ids {
            self.0
                .store
                .keycloak()
                .remove_group(self.0.store.keycloak().config().realm(), id)
                .await?;
            i += 1;
        }
        Ok(i)
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
        guard = "AuthGuard::<Auth, Store, Resource, Permission>::new(qm_role::role!(Resource::user(), Permission::create()))"
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
        allowed_types: HashSet<String>,
        roles: HashSet<qm_role::Role<Resource, Permission>>,
    ) -> async_graphql::FieldResult<Arc<UserGroup>> {
        let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
            ctx,
            &qm_role::role!(Resource::user(), Permission::create()),
        )
        .await?;
        auth_ctx.can_mutate(Some(&context)).await?;
        if allowed_access_levels
            .iter()
            .any(|lvl| matches!(lvl, &AccessLevel::Admin | AccessLevel::None))
        {
            return exerr!(bad_request(
                "UserGroup",
                "unable to create custom group with allowed access level ADMIN or NONE"
            ));
        }
        if roles.iter().any(|r| r.ty.is_admin()) {
            return exerr!(bad_request(
                "UserGroup",
                "unable to create custom group with role 'administration'"
            ));
        }
        if !auth_ctx.is_admin {
            for role in roles.iter() {
                if !auth_ctx.auth.has_role_object(role) {
                    return exerr!(unauthorized(&auth_ctx.auth));
                }
            }
        }
        let roles = roles.into_iter().collect();
        Ctx(&auth_ctx)
            .create(name, context, allowed_access_levels, allowed_types, roles)
            .await
    }

    async fn remove_groups(
        &self,
        ctx: &Context<'_>,
        ids: Arc<[Uuid]>,
    ) -> async_graphql::FieldResult<u64> {
        let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
            ctx,
            &qm_role::role!(Resource::user(), Permission::create()),
        )
        .await?;
        let mut group_ids = vec![];
        for id in ids.iter() {
            let id: Arc<str> = Arc::from(id.to_string());
            auth_ctx
                .store
                .cache_db()
                .group_by_id(&id)
                .await
                .ok_or(EntityError::not_found_by_id::<Group>(id.as_ref()))
                .extend()?;
            let group_detail = auth_ctx
                .store
                .cache_db()
                .group_detail_by_id(&id)
                .await
                .ok_or(EntityError::not_found_by_id::<Group>(id.as_ref()))
                .extend()?;
            if group_detail.built_in {
                return exerr!(bad_request("Group", "unable to remove built in groups"));
            }
            auth_ctx.can_mutate(group_detail.context.as_ref()).await?;
            group_ids.push(id);
        }
        Ctx(&auth_ctx).remove(&group_ids).await
    }
}
