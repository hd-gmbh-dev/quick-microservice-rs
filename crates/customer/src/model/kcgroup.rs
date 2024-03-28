use async_graphql::SimpleObject;
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
}

#[derive(Debug, Clone, SimpleObject)]
pub struct Group {
    pub id: Arc<str>,
    pub name: Arc<str>,
}
pub type GroupIdMap = HashMap<Arc<str>, Arc<Group>>;
pub type GroupMap = HashMap<Arc<str>, Arc<Group>>;
