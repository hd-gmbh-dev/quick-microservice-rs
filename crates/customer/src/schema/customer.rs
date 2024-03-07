use async_graphql::FieldResult;
use async_graphql::{Context, Object, ResultExt};

use crate::context::RelatedAccess;
use crate::context::RelatedStorage;
use crate::context::{CustomerResource, RelatedAuth, RelatedPermission, RelatedResource};
use crate::marker::{Marker, RpMarker};
use crate::model::CreateCustomerInput;
use crate::model::Customer;
use crate::model::{CustomerData, CustomerList, UpdateCustomerInput};
use crate::schema::auth::AuthCtx;
use crate::schema::user::Owner;
use crate::schema::user::UserCtx;
use qm_entity::error::EntityResult;
use qm_entity::ids::CustomerId;
use qm_entity::model::ListFilter;
use qm_entity::IsAdmin;
use qm_entity::UserId;
use qm_entity::{err, HasRole};
use qm_entity::{Create, FromGraphQLContext};
use qm_mongodb::DB;

pub const DEFAULT_COLLECTION: &str = "customers";

pub trait CustomerDB {
    fn collection(&self) -> &str {
        DEFAULT_COLLECTION
    }
    fn customer_db(&self) -> &DB;
    fn customers(&self) -> qm_entity::Collection<Customer> {
        let collection = self.collection();
        qm_entity::Collection(self.customer_db().get().collection::<Customer>(collection))
    }
}

impl<T> CustomerDB for T
where
    T: AsRef<DB>,
{
    fn customer_db(&self) -> &DB {
        self.as_ref()
    }
}

// pub struct CustomerCtx<'ctx, Auth, Store, Resource, Permission> {
//     auth: Auth,
//     store: &'ctx Store,
//     _marker: RpMarker<Resource, Permission>,
// }

// impl<'ctx, Auth, Store, Resource, Permission> CustomerCtx<'ctx, Auth, Store, Resource, Permission> {
//     pub fn new(auth: Auth, store: &'ctx Store) -> Self {
//         Self { auth, store, _marker: std::marker::PhantomData }
//     }
// }

// impl<'ctx, Auth, Store, Resource, Permission> CustomerCtx<'ctx, Auth, Store, Resource, Permission>
// where
//     Auth: FromGraphQLContext + UserId + IsAdmin,
//     Store: Send + Sync + 'static,
//     Resource: Send + Sync + 'static,
//     Permission: Send + Sync + 'static,
// {
//     pub async fn from_graphql(ctx: &'ctx Context<'_>) -> FieldResult<Self> {
//         Ok(Self::new(
//             Auth::from_graphql_context(ctx).await?,
//             ctx.data_unchecked::<Store>(),
//         ))
//     }
// }

// impl<'ctx, Auth, Store, Resource, Permission> CustomerCtx<'ctx, Auth, Store, Resource, Permission>
// where
//     Auth: FromGraphQLContext + UserId + IsAdmin + HasRole<Resource, Permission>,
//     Store: RelatedStorage,
//     Resource: CustomerResource,
// {
//     pub async fn list(&self, _filter: Option<ListFilter>) -> FieldResult<CustomerList> {
//         // if !self.auth.is_admin() {
//         //     return Err(unauthorized(async_graphql::Error::new("invalid permission to list customers")));
//         // }
//         // let result = self.store.customers()
//         //     .list(filter).await?;
//         // Ok(CustomerList {
//         //     items: result.items,
//         //     limit: result.limit,
//         //     total: result.total,
//         //     page: result.page,
//         // })
//         unimplemented!()
//     }

//     pub async fn by_id(&self, _id: &CustomerId) -> FieldResult<Option<Customer>> {
//         // if !self.auth.is_admin() {
//         //     return Err(unauthorized(async_graphql::Error::new("invalid permission to get customer by id")));
//         // }
//         // Ok(self.store.customers().by_id(&id.id).await?)
//         unimplemented!()
//     }
//     pub async fn update(&self, _input: &UpdateCustomerInput) -> anyhow::Result<Customer> {
//         unimplemented!()
//     }

//     pub async fn remove(&self, _ids: &[CustomerId]) -> anyhow::Result<usize> {
//         unimplemented!()
//     }
// }

struct Ctx<'a, Auth, Store, Access, Resource, Permission>(
    pub AuthCtx<'a, Auth, Store, Access, Resource, Permission>,
)
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission;
impl<'a, Auth, Store, Access, Resource, Permission>
    Ctx<'a, Auth, Store, Access, Resource, Permission>
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
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
                    if let Some(cache) = self.0.store.cache() {
                        cache
                            .customer()
                            .new_customer(self.0.store.redis().as_ref(), result.clone())
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
}

pub struct CustomerQueryRoot<Auth, Store, Access, Resource, Permission> {
    _marker: Marker<Auth, Store, Access, Resource, Permission>,
}

impl<Auth, Store, Access, Resource, Permission> Default
    for CustomerQueryRoot<Auth, Store, Access, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Access, Resource, Permission>
    CustomerQueryRoot<Auth, Store, Access, Resource, Permission>
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    async fn customer_by_id(
        &self,
        _ctx: &Context<'_>,
        _id: CustomerId,
    ) -> async_graphql::FieldResult<Option<Customer>> {
        // CustomerCtx::<Auth, Store, Resource, Permission>::from_graphql(ctx)
        //     .await?
        //     .by_id(&id)
        //     .await
        unimplemented!()
    }

    async fn customers(
        &self,
        _ctx: &Context<'_>,
        _filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<CustomerList> {
        // if !self.auth.is_admin() {
        //     return Err(unauthorized(async_graphql::Error::new("invalid permission to get customer by id")));
        // }
        // Ok(self.store.customers().by_id(&id.id).await?)
        unimplemented!()
    }
}

pub struct CustomerMutationRoot<Auth, Store, Access, Resource, Permission> {
    _marker: Marker<Auth, Store, Access, Resource, Permission>,
}

impl<Auth, Store, Access, Resource, Permission> Default
    for CustomerMutationRoot<Auth, Store, Access, Resource, Permission>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Access, Resource, Permission>
    CustomerMutationRoot<Auth, Store, Access, Resource, Permission>
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    async fn create_customer(
        &self,
        ctx: &Context<'_>,
        input: CreateCustomerInput,
    ) -> async_graphql::FieldResult<Customer> {
        Ctx(
            AuthCtx::<'_, Auth, Store, Access, Resource, Permission>::new_with_role(
                ctx,
                (Resource::customer(), Permission::create()),
            )
            .await?,
        )
        .create(CustomerData(input.name))
        .await
        .extend()
    }

    async fn update_customer(
        &self,
        _ctx: &Context<'_>,
        _input: UpdateCustomerInput,
    ) -> async_graphql::FieldResult<Customer> {
        // Ok(CustomerCtx::<Auth, Store, Resource, Permission>::from_graphql(ctx)
        //     .await?
        //     .update(&input)
        //     .await?)
        unimplemented!()
    }

    async fn remove_customers(
        &self,
        _ctx: &Context<'_>,
        _ids: Vec<CustomerId>,
    ) -> async_graphql::FieldResult<usize> {
        // Ok(CustomerCtx::<Auth, Store, Resource, Permission>::from_graphql(ctx)
        //     .await?
        //     .remove(&ids)
        //     .await?)
        unimplemented!()
    }
}
