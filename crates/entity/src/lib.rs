use crate::ids::ID;
use crate::model::ListFilter;
use crate::model::ListResult;
use async_graphql::{Context, ErrorExtensions, FieldResult};
use error::EntityResult;
use futures::stream::TryStreamExt;

use qm_mongodb::bson::Document;
use qm_mongodb::bson::Uuid;
use qm_mongodb::bson::{doc, oid::ObjectId};
use qm_mongodb::options::FindOptions;
use qm_mongodb::results::DeleteResult;
use serde::{de::DeserializeOwned, Serialize};

pub mod ctx;
pub mod error;
pub mod ids;
pub mod list;
pub mod model;

pub trait MutatePermissions {
    fn create() -> Self;
    fn update() -> Self;
    fn delete() -> Self;
}

pub trait QueryPermissions {
    fn list() -> Self;
    fn view() -> Self;
}

pub fn conflict<E>(err: E) -> async_graphql::Error
where
    E: ErrorExtensions,
{
    err.extend_with(|_err, e| e.set("code", 409))
}

pub fn conflicting_name<T>(ty: &str, name: &str) -> Result<T, async_graphql::Error> {
    Err(conflict(async_graphql::Error::new(format!(
        "{ty} with the name '{name}' already exists."
    ))))
}

pub fn unauthorized<E>(err: E) -> async_graphql::Error
where
    E: ErrorExtensions,
{
    err.extend_with(|_err, e| e.set("code", 401))
}

pub fn unauthorized_name<T>(ty: &str, name: &str) -> Result<T, async_graphql::Error> {
    Err(unauthorized(async_graphql::Error::new(format!(
        "{ty} '{name}' nicht authorisiert."
    ))))
}

#[async_trait::async_trait]
pub trait FromGraphQLContext: Sized {
    async fn from_graphql_context(ctx: &Context<'_>) -> FieldResult<Self>;
}

pub trait IsAdmin {
    fn is_admin(&self) -> bool {
        false
    }
}

pub trait HasAccess<A> {
    fn has_access(&self, a: &qm_role::Access<A>) -> bool;
}

pub trait HasRole<R, P> {
    fn has_role(&self, r: &R, p: &P) -> bool;
}

pub trait UserId {
    fn user_id(&self) -> Option<&sqlx::types::Uuid>;
}
pub trait SessionAccess<A> {
    fn session_access(&self) -> Option<&qm_role::Access<A>>;
}

pub trait AsNumber {
    fn as_number(&self) -> u32;
}

pub struct Collection<T>(pub qm_mongodb::Collection<T>);
impl<T> AsRef<qm_mongodb::Collection<T>> for Collection<T> {
    fn as_ref(&self) -> &qm_mongodb::Collection<T> {
        &self.0
    }
}

impl<T> Collection<T>
where
    T: DeserializeOwned + Send + Sync + Unpin,
{
    pub async fn by_id(&self, id: &ObjectId) -> qm_mongodb::error::Result<Option<T>> {
        self.as_ref().find_one(doc! { "_id": id }, None).await
    }

    pub async fn by_name(&self, name: &str) -> qm_mongodb::error::Result<Option<T>> {
        self.as_ref().find_one(doc! { "name": name }, None).await
    }

    pub async fn by_field(&self, field: &str, value: &str) -> qm_mongodb::error::Result<Option<T>> {
        self.as_ref().find_one(doc! { field: value }, None).await
    }

    pub async fn remove_all_by_strings(
        &self,
        field: &str,
        values: &[String],
    ) -> qm_mongodb::error::Result<DeleteResult> {
        self.as_ref()
            .delete_many(doc! { field: { "$in": values } }, None)
            .await
    }

    pub async fn remove_all_by_uuids(
        &self,
        field: &str,
        values: &[&Uuid],
    ) -> qm_mongodb::error::Result<DeleteResult> {
        self.as_ref()
            .delete_many(doc! { field: { "$in": values } }, None)
            .await
    }

    pub async fn by_field_with_customer_filter(
        &self,
        cid: &ObjectId,
        field: &str,
        value: &str,
    ) -> qm_mongodb::error::Result<Option<T>> {
        self.as_ref()
            .find_one(
                doc! {
                    "owner.entityId.cid": &cid,
                    field: value
                },
                None,
            )
            .await
    }

    pub async fn list(
        &self,
        query: Option<Document>,
        filter: Option<ListFilter>,
    ) -> qm_mongodb::error::Result<ListResult<T>> {
        let query = query.unwrap_or_default();
        let limit = filter
            .as_ref()
            .and_then(|filter| filter.limit.as_ref().copied())
            .unwrap_or(1000) as i64;
        let page = filter
            .as_ref()
            .and_then(|filter| filter.page.as_ref().copied())
            .unwrap_or(0);
        let offset = page as u64 * limit as u64;
        let total = self.as_ref().count_documents(query.clone(), None).await?;
        let options = FindOptions::builder().limit(limit).skip(offset).build();
        let items = self
            .as_ref()
            .find(query, options)
            .await?
            .try_collect::<Vec<T>>()
            .await?;
        Ok(ListResult {
            items,
            limit: Some(limit),
            total: Some(total as i64),
            page: Some(page as i64),
        })
    }
}

impl<T> Collection<T>
where
    T: Serialize + Send + Sync + Unpin + AsMut<Option<ID>>,
{
    pub async fn save(&self, mut value: T) -> qm_mongodb::error::Result<T> {
        let id: qm_mongodb::bson::Bson = self.as_ref().insert_one(&value, None).await?.inserted_id;
        if let qm_mongodb::bson::Bson::ObjectId(cid) = id {
            *value.as_mut() = Some(cid);
        }
        Ok(value)
    }
}

pub trait Create<T, C: UserId> {
    fn create(self, ctx: &C) -> EntityResult<T>;
}

#[doc(hidden)]
pub mod __private {
    pub use crate::error::EntityError;
    #[doc(hidden)]
    pub use core::result::Result::Err;
}

#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => {
        $crate::__private::Err($crate::__private::EntityError::$($arg)*)
    };
}
#[macro_export]
macro_rules! exerr {
    ($($arg:tt)*) => {
        $crate::__private::Err($crate::__private::EntityError::$($arg)*.extend())
    };
}
