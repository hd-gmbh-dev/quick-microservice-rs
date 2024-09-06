use std::sync::Arc;

use async_graphql::{Context, Object, ResultExt};

use qm_entity::err;

use qm_entity::error::EntityError;
use qm_entity::error::EntityResult;
use qm_entity::ids::CustomerId;
use qm_entity::ids::CustomerIds;

use qm_entity::ids::InfraId;
use qm_entity::model::ListFilter;
use qm_mongodb::bson::doc;
use qm_role::AccessLevel;
use sqlx::types::Uuid;

use crate::cleanup::CleanupTask;
use crate::cleanup::CleanupTaskType;
use crate::context::RelatedStorage;
use crate::context::{RelatedAuth, RelatedPermission, RelatedResource};
use crate::groups::RelatedBuiltInGroup;
use crate::marker::Marker;
use crate::model::CreateCustomerInput;
use crate::model::CustomerData;
use crate::model::QmCustomer;
use crate::model::QmCustomerList;
use crate::model::UpdateCustomerInput;
use crate::mutation::remove_customers;
use crate::mutation::update_customer;
use crate::roles;
use crate::schema::auth::AuthCtx;
use async_graphql::ComplexObject;

#[ComplexObject]
impl QmCustomer {
    async fn id(&self) -> async_graphql::FieldResult<CustomerId> {
        Ok(self.into())
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
        filter: Option<ListFilter>,
        ty: Option<String>,
    ) -> async_graphql::FieldResult<QmCustomerList> {
        Ok(self.0.store.cache_db().customer_list(filter, ty).await)
    }

    pub async fn by_id(&self, id: CustomerId) -> Option<Arc<QmCustomer>> {
        self.0.store.cache_db().customer_by_id(&id.into()).await
    }

    pub async fn create(&self, customer: CustomerData) -> EntityResult<Arc<QmCustomer>> {
        let user_id = self.0.auth.user_id().unwrap();
        let name = customer.0.clone();
        let ty = customer.1;
        let lock_key = format!("v1_customer_lock_{name}");
        let lock = self.0.store.redis().lock(&lock_key, 5000, 20, 250).await?;
        let (result, exists) = async {
            EntityResult::Ok(
                if let Some(item) = self.0.store.cache_db().customer_by_name(&customer.0).await {
                    (item, true)
                } else {
                    let result = crate::mutation::create_customer(
                        self.0.store.customer_db().pool(),
                        customer.2,
                        &name,
                        ty.as_deref(),
                        user_id,
                    )
                    .await?;
                    let id: CustomerId = (&result).into();
                    let access = qm_role::Access::new(AccessLevel::Customer)
                        .with_fmt_id(Some(&id))
                        .to_string();
                    let roles =
                        roles::ensure(self.0.store.keycloak(), Some(access).into_iter()).await?;
                    self.0.store.cache_db().user().new_roles(roles).await;
                    if let Some(producer) = self.0.store.mutation_event_producer() {
                        producer
                            .create_event(
                                &qm_kafka::producer::EventNs::Customer,
                                "customer",
                                &result,
                            )
                            .await?;
                    }
                    let customer = Arc::new(result);
                    self.0
                        .store
                        .cache_db()
                        .infra()
                        .new_customer(customer.clone())
                        .await;
                    (customer, false)
                },
            )
        }
        .await?;
        self.0.store.redis().unlock(&lock_key, &lock.id).await?;
        if exists {
            return err!(name_conflict::<QmCustomer>(name));
        }
        Ok(result)
    }

    pub async fn update(&self, id: CustomerId, name: String) -> EntityResult<Arc<QmCustomer>> {
        let user_id = self.0.auth.user_id().unwrap();
        let id: InfraId = id.into();
        let old = self
            .0
            .store
            .cache_db()
            .customer_by_id(&id)
            .await
            .ok_or(EntityError::not_found_by_field::<QmCustomer>("name", &name))?;
        let result = update_customer(self.0.store.customer_db().pool(), id, &name, user_id).await?;
        let new = Arc::new(result);
        self.0
            .store
            .cache_db()
            .infra()
            .update_customer(new.clone(), old.as_ref().into())
            .await;
        Ok(new)
    }

    pub async fn remove(&self, ids: CustomerIds) -> EntityResult<u64> {
        let v: Vec<i64> = ids.iter().map(CustomerId::unzip).collect();
        let delete_count = remove_customers(self.0.store.customer_db().pool(), &v).await?;
        if delete_count != 0 {
            let id = Uuid::new_v4();
            self.0
                .store
                .cleanup_task_producer()
                .add_item(&CleanupTask {
                    id,
                    ty: CleanupTaskType::Customers(ids),
                })
                .await?;
            tracing::debug!("emit cleanup task {}", id.to_string());
            return Ok(delete_count);
        }
        Ok(0)
    }
}

pub struct CustomerQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for CustomerQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    CustomerQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn customer_by_id(
        &self,
        ctx: &Context<'_>,
        id: CustomerId,
    ) -> async_graphql::FieldResult<Option<Arc<QmCustomer>>> {
        Ok(Ctx(
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
                ctx,
                &qm_role::role!(Resource::customer(), Permission::view()),
            )
            .await
            .extend()?,
        )
        .by_id(id)
        .await)
    }

    async fn qm_customers(
        &self,
        ctx: &Context<'_>,
        filter: Option<ListFilter>,
        ty: Option<String>,
    ) -> async_graphql::FieldResult<QmCustomerList> {
        Ctx(
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
                ctx,
                &qm_role::role!(Resource::customer(), Permission::list()),
            )
            .await?,
        )
        .list(filter, ty)
        .await
        .extend()
    }
}

pub struct CustomerMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for CustomerMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    CustomerMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn create_customer(
        &self,
        ctx: &Context<'_>,
        input: CreateCustomerInput,
    ) -> async_graphql::FieldResult<Arc<QmCustomer>> {
        let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
            ctx,
            &qm_role::role!(Resource::customer(), Permission::create()),
        )
        .await?;
        Ctx(&auth_ctx)
            .create(CustomerData(input.name, input.ty, input.id))
            .await
            .extend()
    }

    async fn update_customer(
        &self,
        ctx: &Context<'_>,
        context: CustomerId,
        input: UpdateCustomerInput,
    ) -> async_graphql::FieldResult<Arc<QmCustomer>> {
        Ctx(
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
                ctx,
                &qm_role::role!(Resource::customer(), Permission::update()),
            )
            .await?,
        )
        .update(context, input.name)
        .await
        .extend()
    }

    async fn remove_customers(
        &self,
        ctx: &Context<'_>,
        ids: CustomerIds,
    ) -> async_graphql::FieldResult<u64> {
        Ctx(
            &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
                ctx,
                &qm_role::role!(Resource::customer(), Permission::delete()),
            )
            .await?,
        )
        .remove(ids)
        .await
        .extend()
    }
}
