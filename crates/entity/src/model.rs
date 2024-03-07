use async_graphql::{InputObject, SimpleObject};
use chrono::{DateTime, Utc};
use qm_mongodb::bson::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Modification {
    #[graphql(skip)]
    pub user_id: Option<Uuid>,
    pub at: DateTime<Utc>,
}

impl Modification {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id: Some(user_id),
            at: Utc::now(),
        }
    }
}

#[derive(
    Default, Debug, Clone, InputObject, Serialize, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord,
)]
pub struct ListFilter {
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

pub struct ListResult<T> {
    pub items: Vec<T>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}
