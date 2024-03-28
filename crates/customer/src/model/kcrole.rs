use async_graphql::SimpleObject;
use sqlx::FromRow;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
pub struct KeycloakRoleUpdate {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub realm_id: Option<Arc<str>>,
}

#[derive(Debug, FromRow)]
pub struct KcRoleQuery {
    pub role_id: Option<String>,
    pub role_name: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
pub struct Role {
    pub id: Arc<str>,
    pub name: Arc<str>,
}
pub type RoleIdMap = HashMap<Arc<str>, Arc<Role>>;
pub type RoleMap = HashMap<Arc<str>, Arc<Role>>;
