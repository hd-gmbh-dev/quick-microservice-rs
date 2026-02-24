#![deny(missing_docs)]

//! Entity abstraction layer for quick-microservice.
//!
//! This crate provides common entity abstractions, utilities, and traits
//! for building microservices with MongoDB and PostgreSQL.
//!
//! ## Features
//!
//! - **Permission Traits**: Define create/update/delete and list/view permissions
//! - **Collection Helpers**: MongoDB collection wrappers with common operations
//! - **ID Types**: Standardized ID types and conversions
//! - **Error Handling**: Entity-specific error types and helpers
//! - **GraphQL Integration**: Context extraction and error helpers
//! - **Macros**: Error creation macros for entity operations
//!
//! ## Usage
//!
//! Define permissions on your entity types:
//!
//! \```ignore
//! use qm_entity::{MutatePermissions, QueryPermissions};
//!
//! #[derive(Clone)]
//! struct MyPermissions;
//!
//! impl MutatePermissions for MyPermissions {
//!     fn create() -> Self { Self }
//!     fn update() -> Self { Self }
//!     fn delete() -> Self { Self }
//! }
//!
//! impl QueryPermissions for MyPermissions {
//!     fn list() -> Self { Self }
//!     fn view() -> Self { Self }
//! }
//! \```
//!
//! Use Collection for MongoDB operations:
//!
//! \```ignore
//! use qm_entity::Collection;
//! use mongodb::bson::oid::ObjectId;
//!
//! let collection = Collection(my_mongodb_collection);
//! let item = collection.by_id(&ObjectId::new()).await?;
//! \```

use async_graphql::{Context, ErrorExtensions, FieldResult};
use error::EntityResult;
use futures::stream::TryStreamExt;
use serde::{de::DeserializeOwned, Serialize};

use qm_mongodb::{
    bson::{doc, oid::ObjectId, Document, Uuid},
    options::FindOptions,
    results::DeleteResult,
};

use crate::{
    ids::ID,
    model::{ListFilter, ListResult},
};

/// Error types and helpers.
pub mod error;
/// ID type definitions and conversions.
pub mod ids;
/// List filter and pagination utilities.
pub mod list;
/// Model types for entities.
pub mod model;
/// Owned resource types.
pub mod owned;

/// Trait for defining mutation permissions.
///
/// Implement this trait to define which roles can create, update, or delete entities.
pub trait MutatePermissions {
    /// Permission for creating entities.
    fn create() -> Self;
    /// Permission for updating entities.
    fn update() -> Self;
    /// Permission for deleting entities.
    fn delete() -> Self;
}

/// Trait for defining query permissions.
///
/// Implement this trait to define which roles can list or view entities.
pub trait QueryPermissions {
    /// Permission for listing entities.
    fn list() -> Self;
    /// Permission for viewing entities.
    fn view() -> Self;
}

/// Create a conflict error (HTTP 409).
///
/// Use this for errors indicating resource conflicts (e.g., duplicate names).
pub fn conflict<E>(err: E) -> async_graphql::Error
where
    E: ErrorExtensions,
{
    err.extend_with(|_err, e| e.set("code", 409))
}

/// Creates a conflict error for duplicate names.
pub fn conflicting_name<T>(ty: &str, name: &str) -> Result<T, async_graphql::Error> {
    Err(conflict(async_graphql::Error::new(format!(
        "{ty} with the name '{name}' already exists."
    ))))
}

/// Create an unauthorized error (HTTP 401).
///
/// Use this for authentication/authorization errors.
pub fn unauthorized<E>(err: E) -> async_graphql::Error
where
    E: ErrorExtensions,
{
    err.extend_with(|_err, e| e.set("code", 401))
}

/// Creates an unauthorized error for a named entity.
pub fn unauthorized_name<T>(ty: &str, name: &str) -> Result<T, async_graphql::Error> {
    Err(unauthorized(async_graphql::Error::new(format!(
        "{ty} '{name}' nicht authorisiert."
    ))))
}

#[allow(async_fn_in_trait)]
/// Trait for extracting types from GraphQL context.
///
/// Implement this on your user/session types to extract them from the GraphQL context.
pub trait FromGraphQLContext: Sized {
    /// Extracts the type from the GraphQL context.
    async fn from_graphql_context(ctx: &Context<'_>) -> FieldResult<Self>;
}

/// Trait for admin role detection.
pub trait IsAdmin {
    /// Returns whether the user is an admin.
    fn is_admin(&self) -> bool {
        false
    }
}

/// Trait for support role detection.
pub trait IsSupport {
    /// Returns whether the user is support.
    fn is_support(&self) -> bool {
        false
    }
}

/// Trait for access control.
pub trait HasAccess {
    /// Checks if the user has the given access.
    fn has_access(&self, a: &qm_role::Access) -> bool;
}

/// Trait for role-based access control.
///
/// Implement this trait to check if a user has a specific role with a permission scope.
pub trait HasRole<R, P>
where
    R: std::fmt::Debug + std::marker::Copy + Clone,
    P: std::fmt::Debug + std::marker::Copy + Clone,
{
    /// Checks if the user has the given role with the given permission.
    fn has_role(&self, r: &R, p: &P) -> bool;
    /// Checks if the user has the given role object.
    fn has_role_object(&self, role: &qm_role::Role<R, P>) -> bool;
}

/// Trait for extracting user ID from session.
pub trait UserId {
    /// Returns the user ID if available.
    fn user_id(&self) -> Option<&sqlx::types::Uuid>;
}

/// Trait for extracting session access permissions.
pub trait SessionAccess {
    /// Returns the session access permissions if available.
    fn session_access(&self) -> Option<&qm_role::Access>;
}

/// Trait for converting types to numeric codes.
pub trait AsNumber {
    /// Returns the numeric code.
    fn as_number(&self) -> u32;
}

/// MongoDB collection wrapper with common CRUD operations.
///
/// Provides typed access to MongoDB collections with methods for
/// finding, listing, saving, and removing documents.
pub struct Collection<T>(pub qm_mongodb::Collection<T>)
where
    T: Send + Sync;

impl<T> AsRef<qm_mongodb::Collection<T>> for Collection<T>
where
    T: Send + Sync,
{
    fn as_ref(&self) -> &qm_mongodb::Collection<T> {
        &self.0
    }
}

impl<T> Collection<T>
where
    T: DeserializeOwned + Send + Sync + Unpin,
{
    /// Finds a document by its ID.
    pub async fn by_id(&self, id: &ObjectId) -> qm_mongodb::error::Result<Option<T>> {
        self.as_ref().find_one(doc! { "_id": id }).await
    }

    /// Finds a document by its name field.
    pub async fn by_name(&self, name: &str) -> qm_mongodb::error::Result<Option<T>> {
        self.as_ref().find_one(doc! { "name": name }).await
    }

    /// Finds a document by an arbitrary field and value.
    pub async fn by_field(&self, field: &str, value: &str) -> qm_mongodb::error::Result<Option<T>> {
        self.as_ref().find_one(doc! { field: value }).await
    }

    /// Removes all documents matching string values in a field.
    pub async fn remove_all_by_strings(
        &self,
        field: &str,
        values: &[String],
    ) -> qm_mongodb::error::Result<DeleteResult> {
        self.as_ref()
            .delete_many(doc! { field: { "$in": values } })
            .await
    }

    /// Removes all documents matching UUID values in a field.
    pub async fn remove_all_by_uuids(
        &self,
        field: &str,
        values: &[&Uuid],
    ) -> qm_mongodb::error::Result<DeleteResult> {
        self.as_ref()
            .delete_many(doc! { field: { "$in": values } })
            .await
    }

    /// Finds a document with a customer ID filter.
    pub async fn by_field_with_customer_filter(
        &self,
        cid: &ObjectId,
        field: &str,
        value: &str,
    ) -> qm_mongodb::error::Result<Option<T>> {
        self.as_ref()
            .find_one(doc! {
                "owner.entityId.cid": &cid,
                field: value
            })
            .await
    }

    /// Lists documents with optional query and filter.
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
        let total = self.as_ref().count_documents(query.clone()).await?;
        let options = FindOptions::builder().limit(limit).skip(offset).build();

        let items = self
            .as_ref()
            .find(query)
            .with_options(options)
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
    /// Saves a document and returns it with the generated ID.
    pub async fn save(&self, mut value: T) -> qm_mongodb::error::Result<T> {
        let id: qm_mongodb::bson::Bson = self.as_ref().insert_one(&value).await?.inserted_id;
        if let qm_mongodb::bson::Bson::ObjectId(cid) = id {
            *value.as_mut() = Some(cid.into());
        }
        Ok(value)
    }
}

/// Trait for entity creation logic.
///
/// Implement this trait on your entity types to define creation logic
/// that validates and creates entities based on user context.
pub trait Create<T, C: UserId> {
    /// Creates an entity with the given context.
    fn create(self, ctx: &C) -> EntityResult<T>;
}

#[doc(hidden)]
pub mod __private {
    pub use crate::error::EntityError;
    #[doc(hidden)]
    pub use core::result::Result::Err;
}

/// Macro for creating entity errors.
#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => {
        $crate::__private::Err($crate::__private::EntityError::$($arg)*)
    };
}

/// Macro for creating extended entity errors.
#[macro_export]
macro_rules! exerr {
    ($($arg:tt)*) => {
        $crate::__private::Err($crate::__private::EntityError::$($arg)*.extend())
    };
}
