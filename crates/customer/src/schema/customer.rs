use std::sync::Arc;

use async_graphql::{Context, Object, ResultExt};

use qm_entity::ctx::CustomerFilter;
use qm_entity::err;
use qm_entity::error::EntityResult;
use qm_entity::ids::CustomerId;
use qm_entity::list::ListCtx;
use qm_entity::model::ListFilter;
use qm_entity::Create;
use qm_mongodb::bson::doc;
use qm_mongodb::bson::Uuid;
use qm_mongodb::DB;

use crate::cleanup::CleanupTask;
use crate::cleanup::CleanupTaskType;
use crate::context::RelatedAccessLevel;
use crate::context::RelatedStorage;
use crate::context::{RelatedAuth, RelatedPermission, RelatedResource};
use crate::groups::RelatedBuiltInGroup;
use crate::marker::Marker;
use crate::model::CreateCustomerInput;
use crate::model::CreateUserPayload;
use crate::model::Customer;
use crate::model::{CustomerData, CustomerList};
use crate::roles;
use crate::schema::auth::AuthCtx;

pub const DEFAULT_COLLECTION: &str = "customers";

pub trait CustomerDB: AsRef<DB> {
    fn collection(&self) -> &str {
        DEFAULT_COLLECTION
    }
    fn customers(&self) -> qm_entity::Collection<Customer> {
        let collection = self.collection();
        qm_entity::Collection(self.as_ref().get().collection::<Customer>(collection))
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
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<CustomerList> {
        ListCtx::new(self.0.store.customers())
            .list(filter)
            .await
            .extend()
    }

    pub async fn by_id(&self, id: CustomerId) -> Option<Arc<Customer>> {
        self.0.store.cache().customer().customer_by_id(&id.id).await
    }

    pub async fn create(&self, customer: CustomerData) -> EntityResult<Customer> {
        let name = customer.0.clone();
        let lock_key = format!("v1_customer_lock_{name}");
        let lock = self.0.store.redis().lock(&lock_key, 5000, 20, 250).await?;
        let (result, exists) = async {
            EntityResult::Ok(
                if let Some(item) = self.0.store.customers().by_name(&customer.0).await? {
                    (item, true)
                } else {
                    let result = self
                        .0
                        .store
                        .customers()
                        .save(customer.create(&self.0.auth)?)
                        .await?;
                    let id = result.as_id();
                    let access = qm_role::Access::new(AccessLevel::customer())
                        .with_fmt_id(Some(&id))
                        .to_string();
                    let roles =
                        roles::ensure(self.0.store.keycloak(), Some(access).into_iter()).await?;
                    let cache = self.0.store.cache();
                    cache
                        .customer()
                        .new_customer(self.0.store.redis().as_ref(), result.clone())
                        .await?;
                    cache
                        .user()
                        .new_roles(self.0.store, self.0.store.redis().as_ref(), roles)
                        .await?;
                    if let Some(producer) = self.0.store.mutation_event_producer() {
                        producer
                            .create_event(
                                &qm_kafka::producer::EventNs::Customer,
                                CustomerDB::collection(self.0.store),
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
            return err!(name_conflict::<Customer>(name));
        }
        Ok(result)
    }

    pub async fn remove(&self, ids: Arc<[CustomerId]>) -> EntityResult<u64> {
        let db = self.0.store.as_ref();
        let mut session = db.session().await?;
        let docs = ids
            .iter()
            .map(|cid| {
                doc! {"_id": cid.as_ref()}
            })
            .collect::<Vec<_>>();
        if !docs.is_empty() {
            let result = self
                .0
                .store
                .customers()
                .as_ref()
                .delete_many_with_session(doc! {"$or": docs}, None, &mut session)
                .await?;
            self.0
                .store
                .cache()
                .customer()
                .reload_customers(self.0.store, Some(self.0.store.redis().as_ref()))
                .await?;
            if result.deleted_count != 0 {
                let id = Uuid::new();
                self.0
                    .store
                    .cleanup_task_producer()
                    .add_item(&CleanupTask {
                        id,
                        ty: CleanupTaskType::Customers(ids),
                    })
                    .await?;
                log::debug!("emit cleanup task {}", id.to_string());
                return Ok(result.deleted_count);
            }
        }
        Ok(0)
    }
}

pub struct CustomerQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for CustomerQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
    CustomerQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    // async fn customer_by_id(
    //     &self,
    //     ctx: &Context<'_>,
    //     id: CustomerId,
    // ) -> async_graphql::FieldResult<Option<Arc<Customer>>> {
    //     Ok(Ctx(
    //         AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
    //             ctx,
    //             (Resource::customer(), Permission::view()),
    //         )
    //         .await
    //         .extend()?,
    //     )
    //     .by_id(id)
    //     .await)
    // }

    async fn customers(
        &self,
        ctx: &Context<'_>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<CustomerList> {
        Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::customer(), Permission::list()),
            )
            .await?,
        )
        .list(filter)
        .await
        .extend()
    }
}

pub struct CustomerMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for CustomerMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
    CustomerMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn create_customer(
        &self,
        ctx: &Context<'_>,
        input: CreateCustomerInput,
    ) -> async_graphql::FieldResult<Customer> {
        let result = Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::customer(), Permission::create()),
            )
            .await?,
        )
        .create(CustomerData(input.name))
        .await
        .extend()?;
        let id = result.as_id();
        if let Some(user) = input.initial_user {
            crate::schema::user::Ctx(
                AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                    ctx,
                    (Resource::customer(), Permission::create()),
                )
                .await?,
            )
            .create(CreateUserPayload {
                access: qm_role::Access::new(AccessLevel::customer())
                    .with_fmt_id(Some(&id))
                    .to_string(),
                user,
                group: Auth::create_customer_owner_group().name,
                context: qm_entity::ctx::ContextFilterInput::Customer(CustomerFilter {
                    customer: id.id,
                }),
            })
            .await
            .extend()?;
        }
        Ok(result)
    }

    // async fn update_customer(
    //     &self,
    //     ctx: &Context<'_>,
    //     context: CustomerFilter,
    //     input: UpdateCustomerInput,
    // ) -> async_graphql::FieldResult<Customer> {
    //     Ctx(
    //         AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
    //             ctx,
    //             (Resource::customer(), Permission::update()),
    //         )
    //         .await?,
    //     )
    //     .update(context, input)
    //     .await
    //     .extend()
    // }

    async fn remove_customers(
        &self,
        ctx: &Context<'_>,
        ids: Arc<[CustomerId]>,
    ) -> async_graphql::FieldResult<u64> {
        Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::customer(), Permission::delete()),
            )
            .await?,
        )
        .remove(ids)
        .await
        .extend()
    }
}
