use std::sync::Arc;

use async_graphql::ResultExt;
use async_graphql::{Context, Object};

use qm_entity::ctx::MutationContext;
use qm_entity::ctx::OrganizationFilter;
use qm_entity::ctx::{CustOrOrgFilter, CustomerFilter, InstitutionFilter};
use qm_entity::err;
use qm_entity::error::EntityResult;
use qm_entity::ids::OrganizationId;
use qm_entity::ids::{Cid, Iid, InstitutionId, Oid, StrictInstitutionIds};
use qm_entity::list::ListCtx;
use qm_entity::model::ListFilter;
use qm_entity::Create;
use qm_mongodb::bson::{doc, Uuid};
use qm_mongodb::DB;

use crate::cleanup::{CleanupTask, CleanupTaskType};
use crate::context::RelatedAccessLevel;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::marker::Marker;
use crate::model::CreateInstitutionInput;
use crate::model::CreateUserInput;
use crate::model::Institution;
use crate::model::{InstitutionData, InstitutionList, UpdateInstitutionInput};
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
        let mut ctx = ListCtx::new(self.0.store.institutions());
        if let Some(cust_or_org_filter) = cust_or_org_filter {
            let query = match cust_or_org_filter {
                CustOrOrgFilter::Customer(CustomerFilter { customer }) => {
                    doc! { "cid" : customer.as_ref() }
                }
                CustOrOrgFilter::Organization(OrganizationFilter {
                    customer,
                    organization,
                }) => doc! { "cid" : customer.as_ref(), "oid": organization.as_ref() },
            };
            ctx = ctx.with_additional_query_params(query);
        }
        ctx.list(filter).await.extend()
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
                    let access = qm_role::Access::new(AccessLevel::institution())
                        .with_fmt_id(result.id.as_institution_id().as_ref())
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

    pub async fn remove(&self, ids: StrictInstitutionIds) -> EntityResult<u64> {
        let db = self.0.store.as_ref();
        let mut session = db.session().await?;
        let docs = ids
            .iter()
            .map(|v| {
                let cid: &Cid = v.as_ref();
                let oid: &Oid = v.as_ref();
                let iid: &Iid = v.as_ref();
                doc! {"_id": **iid, "cid": **cid, "oid": **oid }
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
            crate::schema::user::Ctx(
                AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                    ctx,
                    (Resource::user(), Permission::create()),
                )
                .await?,
            )
            .create(CreateUserInput {
                access: qm_role::Access::new(AccessLevel::institution())
                    .with_fmt_id(result.id.as_institution_id().as_ref())
                    .to_string(),
                user,
                group: Auth::create_institution_owner_group().name,
                context: qm_entity::ctx::ContextFilterInput::Institution(InstitutionFilter {
                    customer: result.id.cid.clone().unwrap(),
                    organization: result.id.oid.clone().unwrap(),
                    institution: result.id.id.clone().unwrap(),
                }),
            })
            .await
            .extend()?;
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
        ctx: &Context<'_>,
        ids: StrictInstitutionIds,
    ) -> async_graphql::FieldResult<u64> {
        Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::institution(), Permission::delete()),
            )
            .await?,
        )
        .remove(ids)
        .await
        .extend()
    }
}
