use qm_entity::ids::InfraContext;
use qm_role::AccessLevel;
use sqlx::FromRow;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
pub struct KeycloakGroupUpdate {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub realm_id: Option<Arc<str>>,
    pub parent_group: Option<Arc<str>>,
}

#[derive(Debug, FromRow)]
pub struct KcGroupQuery {
    pub id: Option<String>,
    pub name: Option<String>,
    pub parent_group: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct KcGroupDetailsQuery {
    pub group_id: Option<String>,
    pub context: Option<String>,
    pub allowed_access_levels: Option<String>,
    pub display_name: Option<String>,
    pub built_in: Option<String>,
}

impl KcGroupDetailsQuery {
    pub fn has_all_fields(&self) -> bool {
        [
            self.group_id.as_ref(),
            self.display_name.as_ref(),
            self.allowed_access_levels.as_ref(),
        ]
        .iter()
        .all(Option::is_some)
    }
}

#[derive(Debug, Clone)]
pub struct Group {
    pub id: Arc<str>,
    pub parent_group: Option<Arc<str>>,
    pub name: Arc<str>,
}

#[derive(Debug, Clone)]
pub struct GroupDetail {
    pub built_in: bool,
    pub display_name: Option<Arc<str>>,
    pub allowed_access_levels: Option<Arc<[AccessLevel]>>,
    pub context: Option<InfraContext>,
}

pub type GroupIdMap = HashMap<Arc<str>, Arc<Group>>;
pub type GroupMap = HashMap<Arc<str>, HashMap<Arc<str>, Arc<Group>>>;
pub type GroupDetailsMap = HashMap<Arc<str>, Arc<GroupDetail>>;
