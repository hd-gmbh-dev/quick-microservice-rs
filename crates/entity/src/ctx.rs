// use crate::ids::OrganizationUnitId;

// use crate::ids::{CustomerResourceId, OrganizationResourceId};

// CustomerFilter is used when filtering from the perspective of a customer using no filter or from the perspective of
// an admin filtering for a specific customer
// #[derive(Default, Debug, Clone, InputObject, Serialize, Deserialize)]
// pub struct CustomerFilter {
//     pub customer: InfraId,
// }

// OrganizationFilter is used when filtering from the perspective of a organization using no filter, from the
// perspective of an admin or a customer filtering for a specific organization
// #[derive(Default, Debug, Clone, InputObject, Serialize, Deserialize)]
// pub struct OrganizationFilter {
//     pub customer: InfraId,
//     pub organization: InfraId,
// }

// impl From<CustomerResourceId> for OrganizationFilter {
//     fn from(value: CustomerResourceId) -> Self {
//         OrganizationFilter {
//             customer: value.cid,
//             organization: value.id,
//         }
//     }
// }

// impl From<OrganizationFilter> for CustomerResourceId {
//     fn from(value: OrganizationFilter) -> Self {
//         Self {
//             cid: value.customer,
//             id: value.organization,
//         }
//     }
// }

// OrganizationUnitFilter is used when filtering from the perspective of a organization using no filter, from the
// perspective of an admin or a customer filtering for a specific organization
// #[derive(Default, Debug, Clone, InputObject, Serialize, Deserialize)]
// pub struct OrganizationUnitFilter {
//     pub customer: InfraId,
//     pub organization: Option<InfraId>,
//     pub organization_unit: InfraId,
// }

// impl From<OrganizationUnitId> for OrganizationUnitFilter {
//     fn from(value: OrganizationUnitId) -> Self {
//         match value {
//             OrganizationUnitId::Customer(v) => OrganizationUnitFilter {
//                 customer: v.cid,
//                 organization: None,
//                 organization_unit: v.id,
//             },
//             OrganizationUnitId::Organization(v) => OrganizationUnitFilter {
//                 customer: v.cid,
//                 organization: Some(v.oid),
//                 organization_unit: v.id,
//             },
//         }
//     }
// }

// impl From<OrganizationUnitFilter> for OrganizationUnitId {
//     fn from(value: OrganizationUnitFilter) -> Self {
//         if let Some(organization) = value.organization {
//             OrganizationUnitId::Organization(OrganizationResourceId {
//                 cid: value.customer,
//                 oid: organization,
//                 id: value.organization_unit,
//             })
//         } else {
//             OrganizationUnitId::Customer(CustomerResourceId {
//                 cid: value.customer,
//                 id: value.organization_unit,
//             })
//         }
//     }
// }

// InstitutionFilter is used when filtering from the perspective of a institution using no filter, from the perspective
// of an admin, a customer or an organization filtering for a specific institution
// #[derive(Default, Debug, Clone, InputObject, Serialize, Deserialize)]
// pub struct InstitutionFilter {
//     pub customer: InfraId,
//     pub organization: InfraId,
//     pub institution: InfraId,
// }

// impl From<OrganizationResourceId> for InstitutionFilter {
//     fn from(value: OrganizationResourceId) -> Self {
//         InstitutionFilter {
//             customer: value.cid,
//             organization: value.oid,
//             institution: value.id,
//         }
//     }
// }

// impl From<InstitutionFilter> for OrganizationResourceId {
//     fn from(value: InstitutionFilter) -> Self {
//         Self {
//             cid: value.customer,
//             oid: value.organization,
//             id: value.institution,
//         }
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize, OneofObject)]
// pub enum OrgOrInstFilter {
//     Organization(OrganizationFilter),
//     Institution(InstitutionFilter),
// }

// /// Oneof input object in GraphQL
// /// Oneof input objects requires have exactly one field
// #[derive(Debug, Clone, Serialize, Deserialize, OneofObject)]
// pub enum CustOrOrgFilter {
//     #[graphql(name = "customerFilter")]
//     Customer(CustomerFilter),
//     #[graphql(name = "organizationFilter")]
//     Organization(OrganizationFilter),
// }

// /// Oneof input object in GraphQL
// /// Oneof input objects requires have exactly one field
// #[derive(Debug, Clone, Serialize, Deserialize, OneofObject)]
// pub enum ContextFilterInput {
//     #[graphql(name = "customerFilter")]
//     Customer(CustomerFilter),
//     #[graphql(name = "organizationFilter")]
//     Organization(OrganizationFilter),
//     #[graphql(name = "organizationUnitFilter")]
//     OrganizationUnit(OrganizationUnitFilter),
//     #[graphql(name = "institutionFilter")]
//     Institution(InstitutionFilter),
// }

// impl From<CustOrOrgFilter> for ContextFilterInput {
//     fn from(value: CustOrOrgFilter) -> Self {
//         match value {
//             CustOrOrgFilter::Customer(v) => ContextFilterInput::Customer(v),
//             CustOrOrgFilter::Organization(v) => ContextFilterInput::Organization(v),
//         }
//     }
// }

// impl std::fmt::Display for ContextFilterInput {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Customer(CustomerFilter { customer }) => write!(f, "{}", customer.to_hex()),
//             Self::Organization(OrganizationFilter {
//                 customer,
//                 organization,
//             }) => write!(f, "{}{}", customer.to_hex(), organization.to_hex()),
//             Self::OrganizationUnit(OrganizationUnitFilter {
//                 customer,
//                 organization: Some(organization),
//                 organization_unit,
//             }) => write!(
//                 f,
//                 "{}{}{}",
//                 customer.to_hex(),
//                 organization.to_hex(),
//                 organization_unit.to_hex()
//             ),
//             Self::OrganizationUnit(OrganizationUnitFilter {
//                 customer,
//                 organization: None,
//                 organization_unit,
//             }) => write!(f, "{}{}", customer.to_hex(), organization_unit.to_hex()),
//             Self::Institution(InstitutionFilter {
//                 customer,
//                 organization,
//                 institution,
//             }) => write!(
//                 f,
//                 "{}{}{}",
//                 customer.to_hex(),
//                 organization.to_hex(),
//                 institution.to_hex()
//             ),
//         }
//     }
// }

// impl ContextFilterInput {
//     pub fn cid(&self) -> &i64 {
//         match self {
//             ContextFilterInput::Customer(v) => v.customer.as_ref(),
//             ContextFilterInput::Organization(v) => v.customer.as_ref(),
//             ContextFilterInput::OrganizationUnit(v) => v.customer.as_ref(),
//             ContextFilterInput::Institution(v) => v.customer.as_ref(),
//         }
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum MutationContext {
//     Customer(CustomerFilter),
//     Organization(OrganizationFilter),
//     OrganizationUnit(OrganizationUnitFilter),
//     Institution(InstitutionFilter),
// }

// impl Into<OwnerId> for CustomerFilter {
//     fn into(self) -> OwnerId {
//         OwnerId {
//             cid: Some(self.customer.into()),
//             ..Default::default()
//         }
//     }
// }

// impl Into<OwnerId> for OrganizationFilter {
//     fn into(self) -> OwnerId {
//         OwnerId {
//             cid: Some(self.customer.into()),
//             oid: Some(self.organization.into()),
//             ..Default::default()
//         }
//     }
// }

// impl Into<OwnerId> for OrganizationUnitFilter {
//     fn into(self) -> OwnerId {
//         OwnerId {
//             cid: Some(self.customer.into()),
//             oid: self.organization.map(Into::into),
//             uid: Some(self.organization_unit.into()),
//             ..Default::default()
//         }
//     }
// }

// impl Into<OwnerId> for InstitutionFilter {
//     fn into(self) -> OwnerId {
//         OwnerId {
//             cid: Some(self.customer.into()),
//             oid: Some(self.organization.into()),
//             iid: Some(self.institution.into()),
//             ..Default::default()
//         }
//     }
// }
