use serde::de::DeserializeOwned;

use crate::{error::EntityResult, model::ListResult};

pub trait NewList<T>
where
    T: DeserializeOwned + Send + Sync + Unpin + 'static,
{
    fn new(items: Vec<T>, limit: Option<i64>, total: Option<i64>, page: Option<i64>) -> Self;
}

pub struct ListCtx<T> {
    collection: crate::Collection<T>,
}

impl<T> ListCtx<T>
where
    T: DeserializeOwned + Send + Sync + Unpin + 'static,
{
    pub fn new(collection: crate::Collection<T>) -> Self {
        Self { collection }
    }

    pub async fn list<R>(&self, filter: Option<crate::model::ListFilter>) -> EntityResult<R>
    where
        R: NewList<T>,
    {
        let ListResult {
            items,
            limit,
            total,
            page,
        } = self.collection.list(filter).await?;
        Ok(R::new(items, limit, total, page))
    }
}
