use async_graphql::FieldError;
use async_graphql::{Context, Object, ResultExt};

use qm_entity::ctx::ContextFilterInput;
use qm_entity::err;
use qm_entity::ids::StrictOrganizationUnitIds;
use qm_entity::ids::ID;
use qm_entity::model::ListFilter;
use qm_entity::Create;
use qm_mongodb::DB;

use crate::context::RelatedAccess;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::marker::Marker;
use crate::model::CreateOrganizationUnitInput;
use crate::model::OrganizationUnit;
use crate::model::{OrganizationUnitData, OrganizationUnitList, UpdateOrganizationUnitInput};
use crate::schema::auth::AuthCtx;
use crate::schema::user::KeycloakClient;
use crate::schema::user::Owner;
use crate::schema::user::UserCtx;

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

pub trait OrganizationUnitStorage:
    OrganizationUnitDB + KeycloakClient + Send + Sync + 'static
{
}

pub struct OrganizationUnitQueryRoot<Auth, Store, Access, Resource, Permission> {
    _marker: Marker<Auth, Store, Access, Resource, Permission>,
}

impl<Auth, Store, Access, Resource, Permission> Default
    for OrganizationUnitQueryRoot<Auth, Store, Access, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Access, Resource, Permission>
    OrganizationUnitQueryRoot<Auth, Store, Access, Resource, Permission>
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    async fn organization_unit_by_id(
        &self,
        _ctx: &Context<'_>,
        _id: ID,
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

pub trait OrganizationUnitResource {
    fn organization_unit() -> Self;
}

pub struct OrganizationUnitMutationRoot<Auth, Store, Access, Resource, Permission> {
    _marker: Marker<Auth, Store, Access, Resource, Permission>,
}

impl<Auth, Store, Access, Resource, Permission> Default
    for OrganizationUnitMutationRoot<Auth, Store, Access, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Access, Resource, Permission>
    OrganizationUnitMutationRoot<Auth, Store, Access, Resource, Permission>
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    async fn create_organization_unit(
        &self,
        ctx: &Context<'_>,
        context: ContextFilterInput,
        input: CreateOrganizationUnitInput,
    ) -> async_graphql::FieldResult<OrganizationUnit> {
        let auth_ctx = AuthCtx::<Auth, Store, Access, Resource, Permission>::new_with_role(
            ctx,
            (Resource::organization_unit(), Permission::create()),
        )
        .await?;
        if let ContextFilterInput::Institution(_) = &context {
            return Err(FieldError::new("invalid ContextFilterInput, organization units requires CustomerFilter or OrganizationFilter"));
        }
        let organization_unit = match context {
            ContextFilterInput::Customer(v) => OrganizationUnitData {
                cid: v.customer,
                members: input.members,
                name: input.name,
                oid: None,
            },
            ContextFilterInput::Organization(v) => OrganizationUnitData {
                cid: v.customer,
                members: input.members,
                name: input.name,
                oid: Some(v.organization),
            },
            _ => unreachable!(),
        };
        let item = auth_ctx
            .store
            .organization_units()
            .by_name(&organization_unit.name)
            .await?;
        if item.is_some() {
            return err!(name_conflict::<OrganizationUnit>(organization_unit.name)).extend();
        }
        let result = auth_ctx
            .store
            .organization_units()
            .save(organization_unit.create(&auth_ctx.auth).extend()?)
            .await?;

        if let Some(initial_user) = input.initial_user {
            UserCtx::<Auth, Store, Access, Resource, Permission>::from_graphql(ctx)
                .await?
                .create(
                    Auth::create_organization_unit_owner_group(),
                    Owner::OrganizationUnit(result.id.clone().try_into()?),
                    initial_user,
                )
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
        _ids: StrictOrganizationUnitIds,
    ) -> async_graphql::FieldResult<usize> {
        // Ok(OrganizationUnitCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .remove(&ids)
        //     .await?)
        unimplemented!()
    }
}
