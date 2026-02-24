use async_graphql::{InputObject, SimpleObject};
use serde::{Deserialize, Serialize};

/// Input for realm config errors.
#[derive(Default, Debug, Serialize, Deserialize, SimpleObject, InputObject, Clone)]
pub struct RealmConfigErrorInput {
    /// Unique id.
    pub id: String,
}

impl From<RealmConfigError> for RealmConfigErrorInput {
    fn from(value: RealmConfigError) -> Self {
        Self { id: value.id }
    }
}

/// Realm configuration error.
#[derive(Default, Debug, Serialize, Deserialize, SimpleObject)]
pub struct RealmConfigError {
    /// Unique id
    pub id: String,
    /// Key to be used for the error message
    pub key: String,
}

impl RealmConfigError {
    /// Creates a new RealmConfigError.
    pub fn new(id: String, key: String) -> Self {
        Self { id, key }
    }
}
