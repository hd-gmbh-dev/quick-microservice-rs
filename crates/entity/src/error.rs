use crate::UserId;
use async_graphql::ErrorExtensions;
use qm_keycloak::KeycloakError;
use sqlx::types::Uuid;
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
    /// A unhandled Database error occurred.
    #[error("{0}")]
    SQLDatabase(#[from] sea_orm::DbErr),
    /// Keycloak request failure.
    #[error(transparent)]
    KeycloakRequest(#[from] reqwest::Error),
    /// Keycloak error
    #[error(transparent)]
    KeycloakError(#[from] KeycloakError),
    /// distributed locks error
    #[error(transparent)]
    DistributedLocksError(#[from] qm_nats::DistributedLocksError),
    /// lock manager error
    #[error(transparent)]
    LockManagerError(#[from] qm_nats::LockManagerError),
    /// sequence manager error
    #[error(transparent)]
    SequenceManagerError(#[from] qm_nats::SequenceManagerError),
    /// A unexpected error occured.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    /// Conflicting error, because resource already exists.
    #[error("the resource {0} with id '{1}' already exists")]
    IdConflict(String, String),
    /// Conflicting error, because resource already exists.
    #[error("the resource {0} with name '{1}' already exists")]
    NameConflict(String, String),
    /// Conflicting error, because resource already exists.
    #[error("the resource {0} with name '{1}' has conflicting unique fields")]
    FieldsConflict(String, String, async_graphql::Value),
    /// Forbidden because of missing session.
    #[error("forbidden")]
    Forbidden,
    #[error("internal server error")]
    Internal,
    #[error("not found")]
    NotFound,
    #[error("Required fields are missing")]
    RequiredFields,
    /// Unauthorized user.
    #[error("the user with id '{0}' is unauthorized")]
    Unauthorized(String),
    /// not found by id.
    #[error("the resource {0} with id '{1}' was not found")]
    NotFoundById(String, String),
    /// not found by field.
    #[error("the resource {0} with {1} '{2}' was not found")]
    NotFoundByField(String, String, String),
    /// not allowed
    #[error("the feature '{0}' is not enabled")]
    NotAllowed(String),
    /// bad request.
    #[error("{1}")]
    BadRequest(String, String),
    #[error("No id field in inserted entity")]
    NoId,
    #[error("Query document cannot be empty")]
    NotEmpty,
    #[error("List of ids only allowed with same owner")]
    NotSameOwner,
    #[error("Bson could not be serialized: {0}")]
    Bson(String),
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
        Self::NameConflict(tynm::type_name::<T>(), name.into())
    }

    pub fn fields_conflict<T>(
        name: impl Into<String>,
        fields: impl Into<async_graphql::Value>,
    ) -> Self {
        Self::FieldsConflict(tynm::type_name::<T>(), name.into(), fields.into())
    }

    pub fn not_found_by_id<T>(id: impl Into<String>) -> Self {
        Self::NotFoundById(tynm::type_name::<T>(), id.into())
    }

    pub fn not_found_by_field<T>(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self::NotFoundByField(tynm::type_name::<T>(), field.into(), value.into())
    }

    pub fn bad_request(err_type: impl Into<String>, err_msg: impl Into<String>) -> Self {
        Self::BadRequest(err_type.into(), err_msg.into())
    }

    pub fn not_allowed(err_msg: impl Into<String>) -> Self {
        Self::NotAllowed(err_msg.into())
    }

    pub fn internal() -> Self {
        Self::Internal
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
            EntityError::FieldsConflict(ty, _, fields) => {
                e.set("code", 409);
                e.set("type", ty);
                e.set("details", fields.clone());
            }
            EntityError::Unauthorized(_) => e.set("code", 401),
            EntityError::NotAllowed(_) => e.set("code", 405),
            EntityError::Forbidden => e.set("code", 403),
            EntityError::Internal => e.set("code", 500),
            EntityError::BadRequest(ty, _) => {
                e.set("code", 400);
                e.set("details", ty);
            }
            _ => {}
        })
    }
}
