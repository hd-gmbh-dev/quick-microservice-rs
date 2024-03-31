use async_graphql::SimpleObject;
use qm_entity::ids::{InfraContext, InstitutionId, PartialEqual};
use sqlx::FromRow;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
pub struct KeycloakGroupUpdate {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub realm_id: Option<Arc<str>>,
}

#[derive(Debug, FromRow)]
pub struct KcGroupQuery {
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub context: Option<String>,
    pub built_in: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct Group {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub built_in: bool,
    #[graphql(skip)]
    pub context: Option<InfraContext>,
}
pub type GroupIdMap = HashMap<Arc<str>, Arc<Group>>;
pub type GroupMap = HashMap<Arc<str>, Arc<Group>>;

#[derive(Debug, Clone, SimpleObject)]
pub struct GroupList {
    pub items: Arc<[Arc<Group>]>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

impl PartialEqual<'_, InfraContext> for Group {
    fn partial_equal(&'_ self, r: &'_ InfraContext) -> bool {
        if self.built_in {
            true
        } else if let Some(context) = self.context.as_ref() {
            match r {
                InfraContext::Customer(v) => context.has_customer(v),
                InfraContext::Organization(v) => context.has_organization(v),
                InfraContext::OrganizationUnit(v) => context.has_organization_unit(v),
                InfraContext::Institution(v) => context.has_institution(v),
            }
        } else {
            false
        }
    }
}

impl PartialEqual<'_, InstitutionId> for Group {
    fn partial_equal(&'_ self, r: &'_ InstitutionId) -> bool {
        if let Some(context) = self.context.as_ref() {
            context.has_institution(r)
        } else {
            false
        }
    }
}
