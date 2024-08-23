use async_graphql::Context;
use async_graphql::ErrorExtensions;
use async_graphql::FieldResult;
use async_graphql::Guard;
use async_graphql::ResultExt;

use qm_entity::error::EntityResult;
use qm_entity::ids::CustomerId;
use qm_entity::ids::CustomerOrOrganization;
use qm_entity::ids::InfraContext;
use qm_entity::ids::InstitutionId;
use qm_entity::ids::OrganizationId;
use qm_entity::ids::OrganizationOrInstitution;
use qm_mongodb::bson::Document;
use qm_role::AccessLevel;

use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;

use qm_entity::err;
use qm_entity::error::EntityError;
use qm_entity::ids::OrganizationUnitId;

use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::marker::RpMarker;
use crate::marker::StoreMarker;
use crate::model::Customer;
use crate::model::Organization;

// use crate::model::Customer;
// use crate::model::Organization;
// use crate::model::OrganizationUnit;

enum Lvl {
    Customer,
    OrganizationUnit,
    Organization,
    Institution,
    None,
}

#[derive(Clone)]
pub struct AuthCtx<'ctx, Auth, Store, Resource, Permission> {
    pub auth: Auth,
    pub store: &'ctx Store,
    pub requires_context: bool,
    pub context: Arc<RwLock<Option<InfraContext>>>,
    pub is_admin: bool,
    pub is_support: bool,
    _marker: RpMarker<Resource, Permission>,
}

impl<'ctx, Auth, Store, Resource, Permission> TryFrom<(&'ctx Store, Auth)>
    for AuthCtx<'ctx, Auth, Store, Resource, Permission>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    type Error = async_graphql::FieldError;
    fn try_from((store, auth): (&'ctx Store, Auth)) -> FieldResult<Self> {
        let is_admin = auth.is_admin();
        let is_support = auth.is_support();
        if auth.user_id().is_none() {
            return Err(EntityError::Forbidden).extend();
        }
        let has_id = auth
            .session_access()
            .map(|v| v.id().is_some())
            .ok_or(EntityError::unauthorized(&auth).extend())?;
        let requires_context = !(is_admin || is_support) && has_id;
        Ok(Self {
            is_admin,
            is_support,
            auth,
            store,
            requires_context,
            context: Default::default(),
            _marker: Default::default(),
        })
    }
}

impl<'ctx, Auth, Store, Resource, Permission> AuthCtx<'ctx, Auth, Store, Resource, Permission>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    pub async fn new(graphql_context: &'ctx Context<'_>) -> FieldResult<Self> {
        let auth = Auth::from_graphql_context(graphql_context).await.extend()?;
        let store = graphql_context.data_unchecked::<Store>();
        Self::try_from((store, auth))
    }

    pub async fn new_with_role(
        graphql_context: &'ctx Context<'_>,
        role: &qm_role::Role<Resource, Permission>,
    ) -> FieldResult<Self> {
        tracing::debug!("new with role {role:?}");
        let result = Self::new(graphql_context).await?;

        if result.is_admin {
            tracing::debug!("new with role {role:?} resolved to admin");
            return Ok(result);
        }
        if !result.auth.has_role_object(role) {
            return err!(unauthorized(&result.auth)).extend();
        }
        tracing::debug!("new with role {role:?} resolved as non admin");
        Ok(result)
    }

    pub async fn new_with_roles<I>(
        graphql_context: &'ctx Context<'_>,
        roles: I,
    ) -> FieldResult<Self>
    where
        I: IntoIterator<Item = qm_role::Role<Resource, Permission>>,
    {
        let result = Self::new(graphql_context).await?;
        if result.is_admin {
            return Ok(result);
        }
        for role in roles {
            if !result.auth.has_role_object(&role) {
                return err!(unauthorized(&result.auth)).extend();
            }
        }
        Ok(result)
    }

    async fn with_customer(self, customer_id: CustomerId) -> FieldResult<Self> {
        let cache = self.store.cache_db();
        let _ = cache
            .customer_by_id(&customer_id.into())
            .await
            .ok_or(EntityError::not_found_by_id::<Customer>(
                customer_id.to_string(),
            ))
            .extend()?;

        if self.is_admin || self.is_support {
            return Ok(self);
        }

        if !self.auth.has_access(
            &qm_role::Access::new(AccessLevel::Customer)
                .with_id(Arc::from(customer_id.to_string())),
        ) {
            return err!(unauthorized(&self.auth)).extend();
        }
        Ok(self)
    }

    async fn with_organization(self, organization_id: OrganizationId) -> FieldResult<Self> {
        let cache = self.store.cache_db();
        let _ = cache
            .organization_by_id(&organization_id.into())
            .await
            .ok_or(EntityError::not_found_by_id::<Organization>(
                organization_id.to_string(),
            ))
            .extend()?;

        if self.is_admin || self.is_support {
            return Ok(self);
        }

        let customer_access = self.auth.has_access(
            &qm_role::Access::new(AccessLevel::Customer)
                .with_id(Arc::from(organization_id.root().to_string())),
        );

        let organization_access = self.auth.has_access(
            &qm_role::Access::new(AccessLevel::Organization)
                .with_id(Arc::from(organization_id.to_string())),
        );

        if !(customer_access || organization_access) {
            return err!(unauthorized(&self.auth)).extend();
        }
        Ok(self)
    }

    pub async fn mutate_with_role(
        graphql_context: &'ctx Context<'_>,
        mutation_context: InfraContext,
        role: &qm_role::Role<Resource, Permission>,
    ) -> FieldResult<Self> {
        let result = Self::new_with_role(graphql_context, role).await?;
        match mutation_context {
            InfraContext::Customer(filter) => result.with_customer(filter).await,
            InfraContext::Organization(filter) => result.with_organization(filter).await,
            _ => {
                unimplemented!()
            }
        }
    }

    fn lvl(&self) -> Lvl {
        let session = self.auth.session_access();
        if let Some(session) = session {
            if session.ty() == &AccessLevel::Customer {
                return Lvl::Customer;
            } else if session.ty() == &AccessLevel::Organization {
                return Lvl::Organization;
            } else if session.ty() == &AccessLevel::CustomerUnit
                || session.ty() == &AccessLevel::InstitutionUnit
            {
                return Lvl::OrganizationUnit;
            } else if session.ty() == &AccessLevel::Institution {
                return Lvl::Institution;
            }
        }
        Lvl::None
    }

    pub async fn enforce_customer_context(
        &self,
        context: Option<CustomerId>,
    ) -> EntityResult<Option<CustomerId>> {
        if self.is_admin || self.is_support {
            return Ok(context);
        }
        let lvl = self.lvl();
        if matches!(lvl, Lvl::Customer) {
            let access = self
                .auth
                .session_access()
                .ok_or(EntityError::unauthorized(&self.auth))?;
            let id = access.id().ok_or(EntityError::unauthorized(&self.auth))?;
            return Ok(Some(
                CustomerId::parse(id)
                    .ok()
                    .ok_or(EntityError::unauthorized(&self.auth))?,
            ));
        }
        Err(EntityError::unauthorized(&self.auth))
    }

    pub async fn enforce_organization_context(
        &self,
        context: Option<OrganizationId>,
    ) -> EntityResult<Option<OrganizationId>> {
        if self.is_admin || self.is_support {
            return Ok(context);
        }
        let lvl = self.lvl();
        if matches!(lvl, Lvl::Organization) {
            let access = self
                .auth
                .session_access()
                .ok_or(EntityError::unauthorized(&self.auth))?;
            let id = access.id().ok_or(EntityError::unauthorized(&self.auth))?;
            return Ok(Some(
                OrganizationId::parse(id)
                    .ok()
                    .ok_or(EntityError::unauthorized(&self.auth))?,
            ));
        }
        Err(EntityError::unauthorized(&self.auth))
    }

    pub async fn enforce_institution_context(
        &self,
        context: Option<InstitutionId>,
    ) -> EntityResult<Option<InstitutionId>> {
        if self.is_admin || self.is_support {
            return Ok(context);
        }
        let lvl = self.lvl();
        if matches!(lvl, Lvl::Institution) {
            let access = self
                .auth
                .session_access()
                .ok_or(EntityError::unauthorized(&self.auth))?;
            let id = access.id().ok_or(EntityError::unauthorized(&self.auth))?;
            return Ok(Some(
                InstitutionId::parse(id)
                    .ok()
                    .ok_or(EntityError::unauthorized(&self.auth))?,
            ));
        }
        Err(EntityError::unauthorized(&self.auth))
    }

    pub async fn enforce_customer_or_organization_context(
        &self,
        context: Option<CustomerOrOrganization>,
    ) -> EntityResult<Option<CustomerOrOrganization>> {
        if self.is_admin || self.is_support {
            return Ok(context);
        }
        let lvl = self.lvl();
        if matches!(lvl, Lvl::Customer) {
            let access = self
                .auth
                .session_access()
                .ok_or(EntityError::unauthorized(&self.auth))?;
            let id = access.id().ok_or(EntityError::unauthorized(&self.auth))?;
            return Ok(Some(
                CustomerId::parse(id)
                    .map(CustomerOrOrganization::Customer)
                    .ok()
                    .ok_or(EntityError::unauthorized(&self.auth))?,
            ));
        }
        if matches!(lvl, Lvl::Organization) {
            let access = self
                .auth
                .session_access()
                .ok_or(EntityError::unauthorized(&self.auth))?;
            let id = access.id().ok_or(EntityError::unauthorized(&self.auth))?;
            return Ok(Some(
                OrganizationId::parse(id)
                    .map(CustomerOrOrganization::Organization)
                    .ok()
                    .ok_or(EntityError::unauthorized(&self.auth))?,
            ));
        }
        Err(EntityError::unauthorized(&self.auth))
    }

    pub async fn enforce_organization_or_institution_context(
        &self,
        context: Option<OrganizationOrInstitution>,
    ) -> EntityResult<Option<OrganizationOrInstitution>> {
        if self.is_admin || self.is_support {
            return Ok(context);
        }
        let lvl = self.lvl();
        if matches!(lvl, Lvl::Organization) {
            let access = self
                .auth
                .session_access()
                .ok_or(EntityError::unauthorized(&self.auth))?;
            let id = access.id().ok_or(EntityError::unauthorized(&self.auth))?;
            return Ok(Some(
                OrganizationId::parse(id)
                    .map(OrganizationOrInstitution::Organization)
                    .ok()
                    .ok_or(EntityError::unauthorized(&self.auth))?,
            ));
        }
        if matches!(lvl, Lvl::Institution) {
            let access = self
                .auth
                .session_access()
                .ok_or(EntityError::unauthorized(&self.auth))?;
            let id = access.id().ok_or(EntityError::unauthorized(&self.auth))?;
            return Ok(Some(
                InstitutionId::parse(id)
                    .map(OrganizationOrInstitution::Institution)
                    .ok()
                    .ok_or(EntityError::unauthorized(&self.auth))?,
            ));
        }
        Err(EntityError::unauthorized(&self.auth))
    }

    pub async fn enforce_current_context(
        &self,
        context: Option<InfraContext>,
    ) -> EntityResult<Option<InfraContext>> {
        if self.is_admin || self.is_support {
            return Ok(context);
        }
        let lvl = self.lvl();
        if let Some(id) = self
            .auth
            .session_access()
            .ok_or(EntityError::internal())?
            .id()
        {
            if let Some(context) = context {
                return match lvl {
                    Lvl::Customer => Ok(Some(
                        CustomerId::parse(id)
                            .map(InfraContext::Customer)
                            .ok()
                            .ok_or(EntityError::unauthorized(&self.auth))?,
                    )),
                    Lvl::OrganizationUnit => Ok(Some(
                        OrganizationUnitId::parse(id)
                            .map(InfraContext::OrganizationUnit)
                            .ok()
                            .ok_or(EntityError::unauthorized(&self.auth))?,
                    )),
                    Lvl::Organization => Ok(Some(
                        OrganizationId::parse(id)
                            .map(InfraContext::Organization)
                            .ok()
                            .ok_or(EntityError::unauthorized(&self.auth))?,
                    )),
                    Lvl::Institution => Ok(Some(
                        InstitutionId::parse(id)
                            .map(InfraContext::Institution)
                            .ok()
                            .ok_or(EntityError::unauthorized(&self.auth))?,
                    )),
                    Lvl::None => Err(EntityError::unauthorized(&self.auth))?,
                }
                .map(|c| c.map(|v| v.combine(context)));
            } else {
                return match lvl {
                    Lvl::Customer => Ok(Some(
                        CustomerId::parse(id)
                            .map(InfraContext::Customer)
                            .ok()
                            .ok_or(EntityError::unauthorized(&self.auth))?,
                    )),
                    Lvl::OrganizationUnit => Ok(Some(
                        OrganizationUnitId::parse(id)
                            .map(InfraContext::OrganizationUnit)
                            .ok()
                            .ok_or(EntityError::unauthorized(&self.auth))?,
                    )),
                    Lvl::Organization => Ok(Some(
                        OrganizationId::parse(id)
                            .map(InfraContext::Organization)
                            .ok()
                            .ok_or(EntityError::unauthorized(&self.auth))?,
                    )),
                    Lvl::Institution => Ok(Some(
                        InstitutionId::parse(id)
                            .map(InfraContext::Institution)
                            .ok()
                            .ok_or(EntityError::unauthorized(&self.auth))?,
                    )),
                    Lvl::None => Err(EntityError::unauthorized(&self.auth))?,
                };
            }
        }
        Err(EntityError::unauthorized(&self.auth))
    }

    pub async fn build_context_query(
        &self,
        _context: Option<&InfraContext>,
    ) -> EntityResult<Document> {
        unimplemented!()
    }

    async fn ensure_user_context(&self) -> EntityResult<()> {
        let has_context = self.context.read().await.is_some();
        if has_context {
            return Ok(());
        }
        let id = self
            .auth
            .session_access()
            .ok_or(EntityError::internal())?
            .id();

        if let Some(id) = id {
            let ctx = InfraContext::parse(id)?;
            self.context.write().await.replace(ctx);
        }
        Ok(())
    }

    pub async fn can_mutate(&self, object_context: Option<&InfraContext>) -> EntityResult<()> {
        if !self.requires_context {
            return Ok(());
        }
        self.ensure_user_context().await?;
        let c = self.context.read().await;
        let user_context = c.as_ref().ok_or(EntityError::unauthorized(&self.auth))?;
        let object_context = object_context.ok_or(EntityError::unauthorized(&self.auth))?;
        match user_context {
            InfraContext::Customer(v) => {
                if object_context.has_customer(v) {
                    return Ok(());
                }
                err!(unauthorized(&self.auth))
            }
            InfraContext::Organization(v) => {
                if object_context.has_organization(v) {
                    return Ok(());
                }
                err!(unauthorized(&self.auth))
            }
            InfraContext::Institution(v) => {
                if object_context.has_institution(v) {
                    return Ok(());
                }
                err!(unauthorized(&self.auth))
            }
            InfraContext::OrganizationUnit(v) => {
                let organization_unit = self
                    .store
                    .cache_db()
                    .organization_unit_by_id(&v.into())
                    .await
                    .ok_or(EntityError::internal())?;
                if object_context.has_organization_unit(v)
                    || organization_unit
                        .members
                        .iter()
                        .any(|i| object_context.has_institution(i))
                {
                    return Ok(());
                }
                err!(unauthorized(&self.auth))
            }
        }
    }
}

pub struct AuthGuard<Auth, Store, Resource, Permission>
where
    Resource: Debug,
    Permission: Debug,
{
    role: qm_role::Role<Resource, Permission>,
    _marker: StoreMarker<Auth, Store>,
}

impl<Auth, Store, Resource, Permission> AuthGuard<Auth, Store, Resource, Permission>
where
    Resource: Debug,
    Permission: Debug,
{
    pub fn new(role: qm_role::Role<Resource, Permission>) -> Self {
        Self {
            role,
            _marker: Default::default(),
        }
    }
}

impl<Auth, Store, Resource, Permission> Guard for AuthGuard<Auth, Store, Resource, Permission>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    async fn check(&self, ctx: &Context<'_>) -> FieldResult<()> {
        AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(ctx, &self.role).await?;
        Ok(())
    }
}
