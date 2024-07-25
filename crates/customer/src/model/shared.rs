use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
pub struct GroupAttributeUpdate {
    pub group_id: Arc<str>,
    pub name: Option<String>,
    pub value: Option<String>,
}
