use async_graphql::{Context, Object, ResultExt};

use qm_entity::ctx::MutationContext;
use qm_entity::ctx::OrganizationFilter;
use qm_entity::err;
use qm_entity::ids::InstitutionId;
use qm_entity::model::ListFilter;
use qm_entity::Create;
use qm_mongodb::DB;

use crate::context::RelatedAccessLevel;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::marker::Marker;
use crate::model::CreateInstitutionInput;
use crate::model::Institution;
use crate::model::{InstitutionData, InstitutionList, UpdateInstitutionInput};
use crate::schema::auth::AuthCtx;
// use crate::schema::user::Owner;

pub const DEFAULT_COLLECTION: &str = "institutions";

pub trait InstitutionDB {
    fn collection(&self) -> &str {
        DEFAULT_COLLECTION
    }
    fn institution_db(&self) -> &DB;
    fn institutions(&self) -> qm_entity::Collection<Institution> {
        let collection = self.collection();
        qm_entity::Collection(
            self.institution_db()
                .get()
                .collection::<Institution>(collection),
        )
    }
}

impl<T> InstitutionDB for T
where
    T: AsRef<DB>,
{
    fn institution_db(&self) -> &DB {
        self.as_ref()
    }
}

pub struct InstitutionQueryRoot<Auth, Store, AccessLevel, Resource, Permission> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission>,
}

impl<Auth, Store, AccessLevel, Resource, Permission> Default
    for InstitutionQueryRoot<Auth, Store, AccessLevel, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission>
    InstitutionQueryRoot<Auth, Store, AccessLevel, Resource, Permission>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    async fn institution_by_id(
        &self,
        _ctx: &Context<'_>,
        _id: InstitutionId,
    ) -> async_graphql::FieldResult<Option<Institution>> {
        // Ok(InstitutionCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .by_id(&id)
        //     .await?)
        unimplemented!()
    }

    async fn institutions(
        &self,
        _ctx: &Context<'_>,
        _filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<InstitutionList> {
        // Ok(InstitutionCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .list(filter)
        //     .await?)
        unimplemented!()
    }
}

pub trait InstitutionResource {
    fn institution() -> Self;
}

pub struct InstitutionMutationRoot<Auth, Store, AccessLevel, Resource, Permission> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission>,
}

impl<Auth, Store, AccessLevel, Resource, Permission> Default
    for InstitutionMutationRoot<Auth, Store, AccessLevel, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission>
    InstitutionMutationRoot<Auth, Store, AccessLevel, Resource, Permission>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    async fn create_institution(
        &self,
        ctx: &Context<'_>,
        context: OrganizationFilter,
        input: CreateInstitutionInput,
    ) -> async_graphql::FieldResult<Institution> {
        let auth_ctx = AuthCtx::<Auth, Store, AccessLevel, Resource, Permission>::mutate_with_role(
            ctx,
            MutationContext::Organization(context.clone()),
            (Resource::institution(), Permission::create()),
        )
        .await?;
        let institution = InstitutionData(context.into(), input.name);
        let item = auth_ctx
            .store
            .institutions()
            .by_name(&institution.1)
            .await?;
        if item.is_some() {
            return err!(name_conflict::<Institution>(institution.1)).extend();
        }
        let result = auth_ctx
            .store
            .institutions()
            .save(institution.create(&auth_ctx.auth).extend()?)
            .await?;

        if let Some(_initial_user) = input.initial_user {
            // UserCtx::<Auth, Store, AccessLevel, Resource, Permission>::from_graphql(ctx)
            //     .await?
            //     .create(
            //         Auth::create_institution_owner_group(),
            //         Owner::Institution(result.id.clone().into()),
            //         initial_user,
            //     )
            //     .await?;
        }
        Ok(result)
    }

    async fn update_institution(
        &self,
        _ctx: &Context<'_>,
        _input: UpdateInstitutionInput,
    ) -> async_graphql::FieldResult<Institution> {
        // Ok(InstitutionCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .update(&input)
        //     .await?)
        unimplemented!()
    }

    async fn remove_institutions(
        &self,
        _ctx: &Context<'_>,
        _ids: Vec<InstitutionId>,
    ) -> async_graphql::FieldResult<usize> {
        // Ok(InstitutionCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .remove(&ids)
        //     .await?)
        unimplemented!()
    }
}
