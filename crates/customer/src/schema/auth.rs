use async_graphql::Context;
use async_graphql::FieldResult;
use async_graphql::ResultExt;
use qm_entity::ctx::ContextFilterInput;
use qm_entity::ctx::OrganizationFilter;
use qm_entity::error::EntityResult;

use qm_entity::ids::CustomerResourceId;
use qm_entity::ids::InstitutionId;
use qm_entity::ids::OrganizationId;
use qm_entity::ids::OrganizationResourceId;
use qm_mongodb::bson::doc;
use qm_mongodb::bson::oid::ObjectId;
use qm_mongodb::bson::Document;
use std::str::FromStr;
use std::sync::Arc;

use qm_entity::ctx::CustomerFilter;
use qm_entity::ctx::MutationContext;
use qm_entity::err;
use qm_entity::error::EntityError;
use qm_entity::ids::OrganizationUnitId;

use crate::context::RelatedAccessLevel;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::marker::ArpMarker;
use crate::model::Customer;
use crate::model::Organization;
use crate::model::OrganizationUnit;
use crate::model::Owner;

enum Lvl {
    Customer,
    OrganizationUnit,
    Organization,
    Institution,
    None,
}

#[derive(Clone)]
pub struct AuthCtx<'ctx, Auth, Store, AccessLevel, Resource, Permission> {
    pub auth: Auth,
    pub store: &'ctx Store,
    pub is_admin: bool,
    _marker: ArpMarker<AccessLevel, Resource, Permission>,
    // access: Access,
    // resource: Resource,
    // permission: Permission,
}

impl<'ctx, Auth, Store, AccessLevel, Resource, Permission>
    AuthCtx<'ctx, Auth, Store, AccessLevel, Resource, Permission>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
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
        log::debug!("new with role {resource:?} -> {permission:?}");
        let result = Self::new(graphql_context).await?;

        if result.is_admin {
            log::debug!("new with role {resource:?} -> {permission:?} resolved to admin");
            return Ok(result);
        }

        if !result.auth.has_role(&resource, &permission) {
            return err!(unauthorized(&result.auth)).extend();
        }
        log::debug!("new with role {resource:?} -> {permission:?} resolved as non admin");
        Ok(result)
    }

    async fn with_customer(self, customer_filter: CustomerFilter) -> FieldResult<Self> {
        let cache = self.store.cache();
        let _ = cache
            .customer()
            .customer_by_id(&customer_filter.customer)
            .await
            .ok_or(EntityError::not_found_by_id::<Customer>(
                customer_filter.customer.to_hex(),
            ))
            .extend()?;

        if self.is_admin {
            return Ok(self);
        }

        if !self.auth.has_access(
            &qm_role::Access::new(AccessLevel::customer())
                .with_id(Arc::from(customer_filter.customer.to_hex())),
        ) {
            return err!(unauthorized(&self.auth)).extend();
        }
        Ok(self)
    }

    async fn with_organization(self, organization_filter: OrganizationFilter) -> FieldResult<Self> {
        let organization_id = organization_filter.into();
        let cache = self.store.cache();
        let _ = cache
            .customer()
            .organization_by_id(&organization_id)
            .await
            .ok_or(EntityError::not_found_by_id::<Organization>(
                organization_id.to_string(),
            ))
            .extend()?;

        if self.is_admin {
            return Ok(self);
        }

        let customer_access = self.auth.has_access(
            &qm_role::Access::new(AccessLevel::customer())
                .with_id(Arc::from(organization_id.cid.to_string())),
        );

        let organization_access = self.auth.has_access(
            &qm_role::Access::new(AccessLevel::organization())
                .with_id(Arc::from(organization_id.to_string())),
        );

        if !(customer_access || organization_access) {
            return err!(unauthorized(&self.auth)).extend();
        }
        Ok(self)
    }

    pub async fn mutate_with_role(
        graphql_context: &'ctx Context<'_>,
        mutation_context: MutationContext,
        role: (Resource, Permission),
    ) -> FieldResult<Self> {
        let result = Self::new_with_role(graphql_context, role).await?;
        match mutation_context {
            MutationContext::Customer(filter) => result.with_customer(filter).await,
            MutationContext::Organization(filter) => result.with_organization(filter).await,
            _ => {
                unimplemented!()
            }
        }
    }

    fn lvl(&self) -> Lvl {
        let session = self.auth.session_access();
        if let Some(session) = session {
            if session.ty() == &AccessLevel::customer() {
                return Lvl::Customer;
            } else if session.ty() == &AccessLevel::organization() {
                return Lvl::Organization;
            } else if session.ty() == &AccessLevel::organization_unit() {
                return Lvl::OrganizationUnit;
            } else if session.ty() == &AccessLevel::institution() {
                return Lvl::Institution;
            }
        }
        Lvl::None
    }

    pub async fn build_context_query(
        &self,
        context: Option<&ContextFilterInput>,
    ) -> EntityResult<Document> {
        if self.is_admin {
            return if let Some(context) = context {
                match context {
                    ContextFilterInput::Customer(v) => Ok(doc! {
                        "owner.entityId.cid": v.customer.as_ref(),
                    }),
                    ContextFilterInput::Organization(v) => Ok(doc! {
                        "owner.entityId.cid": v.customer.as_ref(),
                        "owner.entityId.oid": v.organization.as_ref(),
                    }),
                    ContextFilterInput::OrganizationUnit(v) => {
                        let organization_unit = self
                            .store
                            .cache()
                            .customer()
                            .organization_unit_by_id(&v.clone().into())
                            .await
                            .ok_or(EntityError::internal())?;
                        let mut docs: Vec<Document> = organization_unit
                            .members
                            .iter()
                            .map(|v| {
                                doc! {
                                    "entityId.cid": v.cid.as_ref(),
                                    "entityId.oid": v.oid.as_ref(),
                                    "entityId.iid": v.iid.as_ref(),
                                }
                            })
                            .collect();
                        let mut unit = doc! {
                            "entityId.cid": v.customer.as_ref(),
                            "entityId.iid": v.organization_unit.as_ref(),
                        };
                        if let Some(oid) = v.organization.as_ref() {
                            unit.insert("entityId.oid", oid.as_ref());
                        }
                        docs.push(unit);
                        Ok(doc! {
                            "owner": {
                                "$in": &docs,
                            }
                        })
                    }
                    ContextFilterInput::Institution(v) => Ok(doc! {
                        "owner.entityId.cid": v.customer.as_ref(),
                        "owner.entityId.oid": v.organization.as_ref(),
                        "owner.entityId.iid": v.institution.as_ref(),
                    }),
                }
            } else {
                Ok(doc! {})
            };
        }
        let id = self
            .auth
            .session_access()
            .ok_or(EntityError::internal())?
            .id()
            .ok_or(EntityError::internal())?;
        match self.lvl() {
            Lvl::Customer => {
                if id.len() != 24 {
                    return err!(unauthorized(&self.auth));
                }
                match context {
                    Some(ContextFilterInput::Organization(organization_filter)) => {
                        let cid = ObjectId::parse_str(id).map_err(|_| EntityError::internal())?;
                        if organization_filter.customer.as_ref() == &cid {
                            let oid = organization_filter.organization.as_ref();
                            Ok(doc! {
                                "owner.entityId.cid": &cid,
                                "owner.entityId.oid": oid,
                            })
                        } else {
                            err!(unauthorized(&self.auth))
                        }
                    }
                    Some(ContextFilterInput::OrganizationUnit(v)) => {
                        let cid = ObjectId::parse_str(id).map_err(|_| EntityError::internal())?;
                        if v.customer.as_ref() == &cid {
                            let organization_unit = self
                                .store
                                .cache()
                                .customer()
                                .organization_unit_by_id(&v.clone().into())
                                .await
                                .ok_or(EntityError::internal())?;
                            let mut docs: Vec<Document> = organization_unit
                                .members
                                .iter()
                                .map(|v| {
                                    doc! {
                                        "ty": "Institution",
                                        "entityId": {
                                            "cid": v.cid.as_ref(),
                                            "oid": v.oid.as_ref(),
                                            "iid": v.iid.as_ref(),
                                        }
                                    }
                                })
                                .collect();
                            let entity_id = if let Some(oid) = v.organization.as_ref() {
                                doc! {
                                    "cid": v.customer.as_ref(),
                                    "oid": oid.as_ref(),
                                    "iid": v.organization_unit.as_ref(),
                                }
                            } else {
                                doc! {
                                    "cid": v.customer.as_ref(),
                                    "iid": v.organization_unit.as_ref(),
                                }
                            };
                            docs.push(doc! {
                                "ty": "OrganizationUnit",
                                "entityId": entity_id
                            });
                            Ok(doc! {
                                "owner": {
                                    "$in": &docs,
                                }
                            })
                        } else {
                            err!(unauthorized(&self.auth))
                        }
                    }
                    Some(ContextFilterInput::Institution(institution_filter)) => {
                        let cid =
                            ObjectId::parse_str(&id[0..24]).map_err(|_| EntityError::internal())?;
                        if institution_filter.customer.as_ref() == &cid {
                            let oid = institution_filter.organization.as_ref();
                            let iid = institution_filter.institution.as_ref();
                            Ok(doc! {
                                "owner.entityId.cid": &cid,
                                "owner.entityId.oid": oid,
                                "owner.entityId.iid": iid,
                            })
                        } else {
                            err!(unauthorized(&self.auth))
                        }
                    }
                    _ => {
                        let cid =
                            ObjectId::parse_str(&id[0..24]).map_err(|_| EntityError::internal())?;
                        Ok(doc! {
                            "owner.entityId.cid": &cid,
                        })
                    }
                }
            }
            Lvl::Organization => {
                if id.len() != 48 {
                    return err!(unauthorized(&self.auth));
                }
                match context {
                    Some(ContextFilterInput::OrganizationUnit(v)) => {
                        let cid =
                            ObjectId::parse_str(&id[0..24]).map_err(|_| EntityError::internal())?;
                        let oid = ObjectId::parse_str(&id[24..48])
                            .map_err(|_| EntityError::internal())?;
                        if v.customer.as_ref() == &cid && v.organization.as_deref() == Some(&oid) {
                            let organization_unit = self
                                .store
                                .cache()
                                .customer()
                                .organization_unit_by_id(&v.clone().into())
                                .await
                                .ok_or(EntityError::internal())?;
                            let mut docs: Vec<Document> = organization_unit
                                .members
                                .iter()
                                .map(|v| {
                                    doc! {
                                        "ty": "Institution",
                                        "entityId": {
                                            "cid": v.cid.as_ref(),
                                            "oid": v.oid.as_ref(),
                                            "iid": v.iid.as_ref(),
                                        }
                                    }
                                })
                                .collect();
                            docs.push(doc! {
                                "ty": "OrganizationUnit",
                                "entityId": {
                                    "cid": v.customer.as_ref(),
                                    "oid": &oid,
                                    "iid": v.organization_unit.as_ref(),
                                }
                            });
                            Ok(doc! {
                                "owner": {
                                    "$in": &docs,
                                }
                            })
                        } else {
                            err!(unauthorized(&self.auth))
                        }
                    }
                    Some(ContextFilterInput::Institution(institution_filter)) => {
                        let cid =
                            ObjectId::parse_str(&id[0..24]).map_err(|_| EntityError::internal())?;
                        let oid = ObjectId::parse_str(&id[24..48])
                            .map_err(|_| EntityError::internal())?;
                        if institution_filter.customer.as_ref() == &cid
                            && institution_filter.organization.as_ref() == &oid
                        {
                            let iid = institution_filter.institution.as_ref();
                            let cid = ObjectId::parse_str(&id[0..24])
                                .map_err(|_| EntityError::internal())?;
                            let oid = ObjectId::parse_str(&id[24..48])
                                .map_err(|_| EntityError::internal())?;
                            Ok(doc! {
                                "owner.entityId.cid": &cid,
                                "owner.entityId.oid": &oid,
                                "owner.entityId.iid": iid,
                            })
                        } else {
                            err!(unauthorized(&self.auth))
                        }
                    }
                    _ => {
                        let id = self.auth.session_access().unwrap().id().unwrap();
                        let cid =
                            ObjectId::parse_str(&id[0..24]).map_err(|_| EntityError::internal())?;
                        let oid = ObjectId::parse_str(&id[24..48])
                            .map_err(|_| EntityError::internal())?;
                        Ok(doc! {
                            "owner.entityId.cid": &cid,
                            "owner.entityId.oid": &oid,
                        })
                    }
                }
            }
            Lvl::OrganizationUnit => {
                let l = id.len();
                if l == 48 {
                    let rid = CustomerResourceId::from_str(id)?;
                    let organization_unit_id = OrganizationUnitId::Customer(rid.clone());
                    let organization_unit = self
                        .store
                        .cache()
                        .customer()
                        .organization_unit_by_id(&organization_unit_id)
                        .await
                        .ok_or(EntityError::internal())?;
                    let mut docs: Vec<Document> = organization_unit
                        .members
                        .iter()
                        .map(|v| {
                            doc! {
                                "ty": "Institution",
                                "entityId": {
                                    "cid": v.cid.as_ref(),
                                    "oid": v.oid.as_ref(),
                                    "iid": v.iid.as_ref(),
                                }
                            }
                        })
                        .collect();
                    docs.push(doc! {
                        "ty": "OrganizationUnit",
                        "entityId": {
                            "cid": rid.cid.as_ref(),
                            "iid": rid.id.as_ref(),
                        }
                    });
                    Ok(doc! {
                        "owner": {
                            "$in": &docs,
                        }
                    })
                } else if l == 72 {
                    let rid = OrganizationResourceId::from_str(id)?;
                    let organization_unit_id = OrganizationUnitId::Organization(rid.clone());
                    let organization_unit = self
                        .store
                        .cache()
                        .customer()
                        .organization_unit_by_id(&organization_unit_id)
                        .await
                        .ok_or(EntityError::internal())?;

                    let mut docs: Vec<Document> = organization_unit
                        .members
                        .iter()
                        .map(|v| {
                            doc! {
                                "ty": "Institution",
                                "entityId": {
                                    "cid": v.cid.as_ref(),
                                    "oid": v.oid.as_ref(),
                                    "iid": v.iid.as_ref(),
                                }
                            }
                        })
                        .collect();
                    docs.push(doc! {
                        "ty": "OrganizationUnit",
                        "entityId": {
                            "cid": rid.cid.as_ref(),
                            "oid": rid.oid.as_ref(),
                            "iid": rid.id.as_ref(),
                        }
                    });
                    Ok(doc! {
                        "owner": {
                            "$in": &docs,
                        }
                    })
                } else {
                    err!(unauthorized(&self.auth))
                }
            }
            Lvl::Institution => {
                // Ignore context for institutions
                let id = self.auth.session_access().unwrap().id().unwrap();
                if id.len() == 72 {
                    let cid =
                        ObjectId::parse_str(&id[0..24]).map_err(|_| EntityError::internal())?;
                    let oid =
                        ObjectId::parse_str(&id[24..48]).map_err(|_| EntityError::internal())?;
                    let iid =
                        ObjectId::parse_str(&id[48..72]).map_err(|_| EntityError::internal())?;
                    Ok(doc! {
                        "owner.entityId.cid": &cid,
                        "owner.entityId.oid": &oid,
                        "owner.entityId.iid": &iid,
                    })
                } else {
                    err!(unauthorized(&self.auth))
                }
            }
            Lvl::None => err!(unauthorized(&self.auth)),
        }
    }

    pub async fn can_mutate(&self, owner: &Owner) -> EntityResult<()> {
        if self.is_admin {
            return Ok(());
        }
        match self.lvl() {
            Lvl::Customer => match owner {
                Owner::Customer(v)
                | Owner::Organization(v)
                | Owner::Institution(v)
                | Owner::OrganizationUnit(v) => {
                    let cid = v.cid.as_ref().ok_or(EntityError::bad_request(
                        "Owner",
                        "owner does not have 'cid'",
                    ))?;
                    let customer_access = self.auth.has_access(
                        &qm_role::Access::new(AccessLevel::customer())
                            .with_id(Arc::from(cid.to_hex().to_string())),
                    );
                    if !customer_access {
                        return err!(unauthorized(&self.auth));
                    }
                    Ok(())
                }
            },
            Lvl::Organization => match owner {
                Owner::Customer(_) => {
                    err!(unauthorized(&self.auth))
                }
                Owner::Organization(v) | Owner::Institution(v) | Owner::OrganizationUnit(v) => {
                    let cid = v.cid.as_ref().ok_or(EntityError::bad_request(
                        "Owner",
                        "owner does not have 'cid'",
                    ))?;
                    let oid = v.oid.as_ref().ok_or(EntityError::bad_request(
                        "Owner",
                        "owner does not have 'oid'",
                    ))?;
                    let organization_access = self.auth.has_access(
                        &qm_role::Access::new(AccessLevel::organization()).with_fmt_id(Some(
                            &OrganizationId {
                                cid: cid.clone(),
                                id: oid.clone(),
                            },
                        )),
                    );
                    if !organization_access {
                        return err!(unauthorized(&self.auth));
                    }
                    Ok(())
                }
            },
            Lvl::OrganizationUnit => match owner {
                Owner::Customer(_) | Owner::Organization(_) => {
                    err!(unauthorized(&self.auth))
                }
                Owner::OrganizationUnit(v) => {
                    let cid = v.cid.as_ref().ok_or(EntityError::bad_request(
                        "Owner",
                        "owner does not have 'cid'",
                    ))?;
                    let iid = v.iid.as_ref().ok_or(EntityError::bad_request(
                        "Owner",
                        "owner does not have 'iid'",
                    ))?;
                    let organization_unit_access = if let Some(oid) = v.oid.as_ref() {
                        self.auth.has_access(
                            &qm_role::Access::new(AccessLevel::organization_unit()).with_fmt_id(
                                Some(&OrganizationUnitId::Organization(OrganizationResourceId {
                                    cid: cid.clone(),
                                    oid: oid.clone(),
                                    id: iid.clone(),
                                })),
                            ),
                        )
                    } else {
                        self.auth.has_access(
                            &qm_role::Access::new(AccessLevel::organization_unit()).with_fmt_id(
                                Some(&OrganizationUnitId::Customer(CustomerResourceId {
                                    cid: cid.clone(),
                                    id: iid.clone(),
                                })),
                            ),
                        )
                    };
                    if !organization_unit_access {
                        return err!(unauthorized(&self.auth));
                    }
                    Ok(())
                }
                Owner::Institution(v) => {
                    let cid = v.cid.as_ref().ok_or(EntityError::bad_request(
                        "Owner",
                        "owner does not have 'cid'",
                    ))?;
                    let oid = v.oid.as_ref().ok_or(EntityError::bad_request(
                        "Owner",
                        "owner does not have 'oid'",
                    ))?;
                    let iid = v.iid.as_ref().ok_or(EntityError::bad_request(
                        "Owner",
                        "owner does not have 'iid'",
                    ))?;

                    let id = self
                        .auth
                        .session_access()
                        .ok_or(EntityError::internal())?
                        .id()
                        .ok_or(EntityError::internal())?;

                    let l = id.len();
                    let organization_unit_id = if l == 48 {
                        let rid = CustomerResourceId::from_str(id)?;
                        Some(OrganizationUnitId::Customer(rid.clone()))
                    } else if l == 72 {
                        let rid = OrganizationResourceId::from_str(id)?;
                        Some(OrganizationUnitId::Organization(rid.clone()))
                    } else {
                        None
                    }
                    .ok_or(EntityError::internal())?;
                    let organization_unit = self
                        .store
                        .cache()
                        .customer()
                        .organization_unit_by_id(&organization_unit_id)
                        .await
                        .ok_or(EntityError::not_found_by_id::<OrganizationUnit>(id))?;

                    let has_member = organization_unit
                        .members
                        .iter()
                        .any(|v| &v.cid == cid && &v.oid == oid && &v.iid == iid);
                    if !has_member {
                        return err!(unauthorized(&self.auth));
                    }
                    Ok(())
                }
            },
            Lvl::Institution => match owner {
                Owner::Customer(_) | Owner::Organization(_) | Owner::OrganizationUnit(_) => {
                    err!(unauthorized(&self.auth))
                }
                Owner::Institution(v) => {
                    let cid = v.cid.as_ref().ok_or(EntityError::bad_request(
                        "Owner",
                        "owner does not have 'cid'",
                    ))?;
                    let oid = v.oid.as_ref().ok_or(EntityError::bad_request(
                        "Owner",
                        "owner does not have 'oid'",
                    ))?;
                    let iid = v.iid.as_ref().ok_or(EntityError::bad_request(
                        "Owner",
                        "owner does not have 'iid'",
                    ))?;
                    let institution_access = self.auth.has_access(
                        &qm_role::Access::new(AccessLevel::institution()).with_fmt_id(Some(
                            &InstitutionId {
                                cid: cid.clone(),
                                oid: oid.clone(),
                                id: iid.clone(),
                            },
                        )),
                    );
                    if !institution_access {
                        return err!(unauthorized(&self.auth));
                    }
                    Ok(())
                }
            },
            Lvl::None => err!(unauthorized(&self.auth)),
        }
    }
}
