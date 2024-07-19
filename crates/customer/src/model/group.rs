use qm_entity::ids::InfraContext;
use qm_role::AccessLevel;
use sqlx::FromRow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
pub struct KeycloakGroupUpdate {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub realm_id: Option<Arc<str>>,
    pub parent_group: Option<Arc<str>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct GroupRoleMappingUpdate {
    pub role_id: Arc<str>,
    pub group_id: Arc<str>,
}

#[derive(Debug, FromRow)]
pub struct KcGroupQuery {
    pub id: Option<String>,
    pub name: Option<String>,
    pub parent_group: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct KcGroupByIdQuery {
    pub name: Option<String>,
    pub parent_group: Option<String>,
    pub parent_name: Option<String>,
    pub group_id: Option<String>,
    pub context: Option<String>,
    pub allowed_access_levels: Option<String>,
    pub display_name: Option<String>,
    pub built_in: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct KcGroupRoleQuery {
    pub group_id: Option<String>,
    pub role_id: Option<String>,
}

impl KcGroupRoleQuery {
    pub fn has_all_fields(&self) -> bool {
        [self.group_id.as_ref(), self.role_id.as_ref()]
            .iter()
            .all(Option::is_some)
    }
}

#[derive(Debug, FromRow)]
pub struct KcGroupDetailsQuery {
    pub group_id: Option<String>,
    pub context: Option<String>,
    pub allowed_access_levels: Option<String>,
    pub allowed_types: Option<String>,
    pub display_name: Option<String>,
    pub built_in: Option<String>,
}

impl KcGroupDetailsQuery {
    pub fn has_all_fields(&self) -> bool {
        [
            self.group_id.as_ref(),
            self.display_name.as_ref(),
            self.allowed_access_levels.as_ref(),
            self.allowed_types.as_ref(),
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
    pub allowed_types: Option<Arc<[Arc<str>]>>,
    pub context: Option<InfraContext>,
}

pub type GroupIdMap = HashMap<Arc<str>, Arc<Group>>;
pub type GroupMap = HashMap<Arc<str>, HashMap<Arc<str>, Arc<Group>>>;
pub type GroupDetailsMap = HashMap<Arc<str>, Arc<GroupDetail>>;
pub type GroupRoleMap = HashMap<Arc<str>, HashSet<Arc<str>>>;
