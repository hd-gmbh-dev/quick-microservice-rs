use async_graphql::{Context, Object};

use qm_entity::ctx::CustOrOrgFilter;
use qm_entity::ctx::MutationContext;
use qm_entity::ctx::OrganizationUnitFilter;
use qm_entity::err;
use qm_entity::error::EntityResult;
use qm_entity::ids::OrganizationUnitId;
use qm_entity::model::ListFilter;
use qm_entity::Create;
use qm_mongodb::DB;

use crate::context::RelatedAccessLevel;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::marker::Marker;
use crate::model::CreateOrganizationUnitInput;
use crate::model::CreateUserInput;
use crate::model::OrganizationUnit;
use crate::model::{OrganizationUnitData, OrganizationUnitList, UpdateOrganizationUnitInput};
use crate::roles;
use crate::schema::auth::AuthCtx;

pub const DEFAULT_COLLECTION: &str = "organization_units";

pub trait OrganizationUnitDB {
    fn collection(&self) -> &str {
        DEFAULT_COLLECTION
    }
    fn organization_unit_db(&self) -> &DB;
    fn organization_units(&self) -> qm_entity::Collection<OrganizationUnit> {
        let collection = self.collection();
        qm_entity::Collection(
            self.organization_unit_db()
                .get()
                .collection::<OrganizationUnit>(collection),
        )
    }
}

impl<T> OrganizationUnitDB for T
where
    T: AsRef<DB>,
{
    fn organization_unit_db(&self) -> &DB {
        self.as_ref()
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
                    let access = qm_role::Access::new(AccessLevel::organization_unit())
                        .with_fmt_id(result.id.as_organization_unit_id().as_ref())
                        .to_string();
                    let roles =
                        roles::ensure(self.0.store.keycloak(), Some(access).into_iter()).await?;
                    if let Some(cache) = self.0.store.cache() {
                        cache
                            .customer()
                            .new_organization_unit(self.0.store.redis().as_ref(), result.clone())
                            .await?;
                        cache
                            .user()
                            .new_roles(
                                self.0.store.organization_unit_db(),
                                self.0.store.redis().as_ref(),
                                roles,
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
}

pub struct OrganizationUnitQueryRoot<Auth, Store, AccessLevel, Resource, Permission> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission>,
}

impl<Auth, Store, AccessLevel, Resource, Permission> Default
    for OrganizationUnitQueryRoot<Auth, Store, AccessLevel, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission>
    OrganizationUnitQueryRoot<Auth, Store, AccessLevel, Resource, Permission>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: Send + Sync + 'static,
    Permission: Send + Sync + 'static,
{
    async fn organization_unit_by_id(
        &self,
        _ctx: &Context<'_>,
        _id: OrganizationUnitId,
    ) -> async_graphql::FieldResult<Option<OrganizationUnit>> {
        // Ok(OrganizationUnitCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .by_id(&id)
        //     .await?)
        unimplemented!()
    }

    async fn organization_units(
        &self,
        _ctx: &Context<'_>,
        _filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<OrganizationUnitList> {
        // Ok(OrganizationUnitCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .list(filter)
        //     .await?)
        unimplemented!()
    }
}

pub struct OrganizationUnitMutationRoot<Auth, Store, AccessLevel, Resource, Permission> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission>,
}

impl<Auth, Store, AccessLevel, Resource, Permission> Default
    for OrganizationUnitMutationRoot<Auth, Store, AccessLevel, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission>
    OrganizationUnitMutationRoot<Auth, Store, AccessLevel, Resource, Permission>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
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
                .await?
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
                .await?
            }
        };
        if let Some(user) = input.initial_user {
            crate::schema::user::Ctx(
                AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                    ctx,
                    (Resource::user(), Permission::create()),
                )
                .await?,
            )
            .create(CreateUserInput {
                access: qm_role::Access::new(AccessLevel::organization_unit())
                    .with_fmt_id(result.id.as_organization_unit_id().as_ref())
                    .to_string(),
                user,
                group: Auth::create_organization_unit_owner_group().name,
                context: qm_entity::ctx::ContextFilterInput::OrganizationUnit(
                    OrganizationUnitFilter {
                        customer: result.id.cid.clone().unwrap(),
                        organization: result.id.oid.clone(),
                        organization_unit: result.id.id.clone().unwrap(),
                    },
                ),
            })
            .await?;
        }
        Ok(result)
    }

    async fn update_organization_unit(
        &self,
        _ctx: &Context<'_>,
        _input: UpdateOrganizationUnitInput,
    ) -> async_graphql::FieldResult<OrganizationUnit> {
        // Ok(OrganizationUnitCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .update(&input)
        //     .await?)
        unimplemented!()
    }

    async fn remove_organization_units(
        &self,
        _ctx: &Context<'_>,
        _ids: Vec<OrganizationUnitId>,
    ) -> async_graphql::FieldResult<usize> {
        // Ok(OrganizationUnitCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .remove(&ids)
        //     .await?)
        unimplemented!()
    }
}
