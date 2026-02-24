use async_graphql::{InputObject, SimpleObject};
use chrono::{DateTime, Utc};
use qm_mongodb::bson::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Modification tracking for entities.
pub struct Modification {
    /// User ID who made the modification.
    #[graphql(skip)]
    pub user_id: Option<Uuid>,
    /// Timestamp of modification.
    pub at: DateTime<Utc>,
}

impl Modification {
    /// Creates a new Modification with the given user ID.
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id: Some(user_id),
            at: Utc::now(),
        }
    }
}

/// Filter for list queries.
#[derive(
    Default, Debug, Clone, InputObject, Serialize, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord,
)]
pub struct ListFilter {
    /// Page number.
    pub page: Option<usize>,
    /// Items per page limit.
    pub limit: Option<usize>,
}

/// Result of a list query.
pub struct ListResult<T> {
    /// Items returned.
    pub items: Vec<T>,
    /// Limit used.
    pub limit: Option<i64>,
    /// Total count.
    pub total: Option<i64>,
    /// Page number.
    pub page: Option<i64>,
}
