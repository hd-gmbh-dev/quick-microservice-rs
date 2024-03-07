use async_graphql::Context;
use async_graphql::FieldResult;
use async_graphql::ResultExt;
use std::sync::Arc;

use qm_entity::ctx::CustomerFilter;
use qm_entity::ctx::MutationContext;
use qm_entity::err;
use qm_entity::error::EntityError;

use crate::context::RelatedAccess;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::marker::ArpMarker;
use crate::model::Customer;

pub struct AuthCtx<'ctx, Auth, Store, Access, Resource, Permission> {
    pub auth: Auth,
    pub store: &'ctx Store,
    pub is_admin: bool,
    _marker: ArpMarker<Access, Resource, Permission>,
    // access: Access,
    // resource: Resource,
    // permission: Permission,
}

impl<'ctx, Auth, Store, Access, Resource, Permission>
    AuthCtx<'ctx, Auth, Store, Access, Resource, Permission>
where
    Auth: RelatedAuth<Access, Resource, Permission>,
    Store: RelatedStorage,
    Access: RelatedAccess,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    pub async fn new(graphql_context: &'ctx Context<'_>) -> FieldResult<Self> {
        let auth = Auth::from_graphql_context(graphql_context).await.extend()?;
        let store = graphql_context.data_unchecked::<Store>();
        let is_admin = auth.is_admin();
        Ok(Self {
            is_admin,
            auth,
            store,
            _marker: Default::default(),
        })
    }

    pub async fn new_with_role(
        graphql_context: &'ctx Context<'_>,
        (resource, permission): (Resource, Permission),
    ) -> FieldResult<Self> {
        let result = Self::new(graphql_context).await?;

        if !result.is_admin && !result.auth.has_role(&resource, &permission) {
            return err!(unauthorized(&result.auth)).extend();
        }

        Ok(result)
    }

    async fn with_customer(self, customer_filter: CustomerFilter) -> FieldResult<Self> {
        if let Some(cache) = self.store.cache() {
            let _ = cache
                .customer()
                .customer_by_id(&customer_filter.customer)
                .await
                .ok_or(EntityError::not_found_by_id::<Customer>(
                    customer_filter.customer.to_hex(),
                ))
                .extend()?;

            if !self.auth.has_access(
                &qm_role::Access::new(Access::customer())
                    .with_id(Arc::from(customer_filter.customer.to_hex())),
            ) {
                return err!(unauthorized(&self.auth)).extend();
            }
            Ok(self)
        } else {
            // TODO: check if customer exists against db
            unimplemented!()
        }
    }

    pub async fn mutate_with_role(
        graphql_context: &'ctx Context<'_>,
        mutation_context: MutationContext,
        role: (Resource, Permission),
    ) -> FieldResult<Self> {
        let result = Self::new_with_role(graphql_context, role).await?;
        match mutation_context {
            MutationContext::Customer(customer_filter) => {
                result.with_customer(customer_filter).await
            }
            _ => {
                unimplemented!()
            }
        }
    }
}
