use qm_mongodb::bson::Document;
use serde::de::DeserializeOwned;

use crate::{error::EntityResult, model::ListResult};

/// Trait for creating list results.
pub trait NewList<T>
where
    T: DeserializeOwned + Send + Sync + Unpin + 'static,
{
    /// Creates a new list result.
    fn new(items: Vec<T>, limit: Option<i64>, total: Option<i64>, page: Option<i64>) -> Self;
}

/// List context for executing list queries.
pub struct ListCtx<T>
where
    T: Send + Sync,
{
    collection: crate::Collection<T>,
    query: Option<Document>,
}

impl<T> ListCtx<T>
where
    T: DeserializeOwned + Send + Sync + Unpin + 'static,
{
    /// Creates a new ListCtx.
    pub fn new(collection: crate::Collection<T>) -> Self {
        Self {
            collection,
            query: None,
        }
    }

    /// Adds a query filter to the list context.
    pub fn with_query(mut self, query: Document) -> Self {
        self.query = Some(query);
        self
    }

    /// Executes the list query.
    pub async fn list<R>(&mut self, filter: Option<crate::model::ListFilter>) -> EntityResult<R>
    where
        R: NewList<T>,
    {
        let ListResult {
            items,
            limit,
            total,
            page,
        } = self.collection.list(self.query.take(), filter).await?;
        Ok(R::new(items, limit, total, page))
    }
}
