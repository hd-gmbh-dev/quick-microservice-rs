use crate::UserId;
use async_graphql::ErrorExtensions;
use qm_mongodb::bson::Uuid;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EntityError {
    /// A unhandled Database error occurred.
    #[error("{0}")]
    Lock(#[from] qm_redis::lock::Error),
    /// A unhandled Database error occurred.
    #[error("{0}")]
    Database(#[from] qm_mongodb::error::Error),
    /// A unexpected error occured.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    /// Conflicting error, because resource already exists.
    #[error("the resource {0} with name '{1}' already exists")]
    NameConflict(String, String),
    /// Forbidden because of missing session.
    #[error("forbidden")]
    Forbidden,
    /// Unauthorized user.
    #[error("the user with id '{0}' is unauthorized")]
    Unauthorized(String),
    /// not found.
    #[error("the resource {0} with id '{1}' was not found")]
    NotFound(String, String),
}

pub type EntityResult<T> = Result<T, EntityError>;

impl EntityError {
    pub fn unauthorized_user(user_id: Option<&Uuid>) -> Self {
        if let Some(user_id) = user_id {
            EntityError::Unauthorized(user_id.to_string())
        } else {
            EntityError::Forbidden
        }
    }

    pub fn unauthorized<T>(ctx: &T) -> Self
    where
        T: UserId,
    {
        if let Some(user_id) = ctx.user_id() {
            EntityError::Unauthorized(user_id.to_string())
        } else {
            EntityError::Forbidden
        }
    }

    pub fn name_conflict<T>(name: impl Into<String>) -> Self {
        Self::NameConflict(tynm::type_name::<T>().into(), name.into())
    }

    pub fn not_found_by_id<T>(id: impl Into<String>) -> Self {
        Self::NotFound(tynm::type_name::<T>().into(), id.into())
    }
}

impl ErrorExtensions for EntityError {
    fn extend(&self) -> async_graphql::Error {
        async_graphql::Error::new(format!("{}", self)).extend_with(|_err, e| match self {
            EntityError::NameConflict(ty, _) => {
                e.set("code", 409);
                e.set("type", ty);
                e.set("field", "name");
            }
            EntityError::Unauthorized(_) => e.set("code", 401),
            EntityError::Forbidden => e.set("code", 403),
            _ => {}
        })
    }
}
