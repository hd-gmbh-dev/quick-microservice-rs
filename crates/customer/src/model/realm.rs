use sqlx::FromRow;
use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
pub struct RealmUpdate {
    pub id: Arc<str>,
    pub name: Arc<str>,
}

#[derive(Debug, FromRow)]
pub struct KcRealmQuery {
    pub id: Option<String>,
}
