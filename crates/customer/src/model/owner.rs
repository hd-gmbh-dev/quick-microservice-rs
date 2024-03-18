use qm_entity::{
    ctx::ContextFilterInput,
    ids::{
        CustomerId, CustomerResourceId, EntityId, InstitutionId, OrganizationId,
        OrganizationResourceId, OrganizationUnitId,
    },
};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(tag = "ty", content = "entityId")]
pub enum Owner {
    Customer(EntityId),
    Organization(EntityId),
    Institution(EntityId),
    OrganizationUnit(EntityId),
}

impl From<ContextFilterInput> for Owner {
    fn from(value: ContextFilterInput) -> Self {
        match value {
            ContextFilterInput::Customer(v) => Owner::Customer(v.into()),
            ContextFilterInput::Organization(v) => Owner::Organization(v.into()),
            ContextFilterInput::Institution(v) => Owner::Institution(v.into()),
            ContextFilterInput::OrganizationUnit(v) => Owner::OrganizationUnit(v.into()),
        }
    }
}

impl Owner {
    pub fn customer(&self) -> Option<CustomerId> {
        match &self {
            Owner::Customer(EntityId { cid: Some(cid), .. }) => {
                Some(CustomerId { id: cid.clone() })
            }
            Owner::Organization(EntityId { cid: Some(cid), .. }) => {
                Some(CustomerId { id: cid.clone() })
            }
            Owner::OrganizationUnit(EntityId { cid: Some(cid), .. }) => {
                Some(CustomerId { id: cid.clone() })
            }
            Owner::Institution(EntityId { cid: Some(cid), .. }) => {
                Some(CustomerId { id: cid.clone() })
            }
            _ => None,
        }
    }
    pub fn organization(&self) -> Option<OrganizationId> {
        match &self {
            Owner::Organization(EntityId {
                cid: Some(cid),
                oid: Some(oid),
                ..
            }) => Some(OrganizationId {
                cid: cid.clone(),
                id: oid.clone(),
            }),
            Owner::OrganizationUnit(EntityId {
                cid: Some(cid),
                oid: Some(oid),
                ..
            }) => Some(OrganizationId {
                cid: cid.clone(),
                id: oid.clone(),
            }),
            Owner::Institution(EntityId {
                cid: Some(cid),
                oid: Some(oid),
                ..
            }) => Some(OrganizationId {
                cid: cid.clone(),
                id: oid.clone(),
            }),
            _ => None,
        }
    }
    pub fn organization_unit(&self) -> Option<OrganizationUnitId> {
        match &self {
            Owner::OrganizationUnit(EntityId {
                cid: Some(cid),
                oid: Some(oid),
                iid: Some(iid),
                ..
            }) => Some(OrganizationUnitId::Organization(OrganizationResourceId {
                id: iid.clone(),
                oid: oid.clone(),
                cid: cid.clone(),
            })),
            Owner::OrganizationUnit(EntityId {
                cid: Some(cid),
                oid: None,
                iid: Some(iid),
                ..
            }) => Some(OrganizationUnitId::Customer(CustomerResourceId {
                id: iid.clone(),
                cid: cid.clone(),
            })),
            _ => None,
        }
    }
    pub fn institution(&self) -> Option<InstitutionId> {
        match &self {
            Owner::Institution(EntityId {
                cid: Some(cid),
                oid: Some(oid),
                iid: Some(iid),
                ..
            }) => Some(InstitutionId {
                cid: cid.clone(),
                oid: oid.clone(),
                id: iid.clone(),
            }),
            _ => None,
        }
    }
}
