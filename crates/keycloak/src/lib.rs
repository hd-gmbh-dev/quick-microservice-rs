#![deny(missing_docs)]

//! Keycloak integration for authentication and authorization.
//!
//! This crate provides utilities for managing Keycloak realms, clients, users,
//! roles, and tokens. Keycloak is used for OAuth2/OIDC authentication and
//! role-based access control.
//!
//! ## Features
//!
//! - **Realm Management**: Create and manage Keycloak realms
//! - **Client Management**: Configure OAuth clients with protocols and mappers
//! - **User Management**: Create, update, delete, and authenticate users
//! - **Role Management**: Client and realm roles
//! - **Token Management**: JWT token validation and refresh
//! - **Session Management**: Browser session handling
//! - **Configuration**: Environment-based configuration
//!
//! ## Usage
//!
//! \```ignore
//! use qm_keycloak::{Keycloak, KeycloakConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = KeycloakConfig::new()?;
//!     let keycloak = Keycloak::new(config).await?;
//!     let users = keycloak.get_users("realm").await?;
//!     Ok(())
//! }
//! \```
//!
//! ## Environment Variables
//!
//! | Variable | Description | Default |
//! |----------|-------------|---------|
//! | `KEYCLOAK_URL` | Keycloak server URL | `http://127.0.0.1:8080` |
//! | `KEYCLOAK_REALM` | Default realm | `master` |
//! | `KEYCLOAK_CLIENT_ID` | Client ID for admin | `admin-cli` |
//! | `KEYCLOAK_USERNAME` | Admin username | (none) |
//! | `KEYCLOAK_PASSWORD` | Admin password | (none) |
//!
//! ## Development
//!
//! In the development environment, Keycloak can be found in a Kubernetes
//! at <https://keycloak.qm.local>.
//! If Keycloak is missing, you need to run
//! `k8s/helm-charts/skaffold/skaffold.infra-1-base.yaml`.
//!
//! Default username/password: `admin`/`Admin123`
mod client;

/// Session management.
pub mod session;
pub use client::*;
/// Configuration for keycloak.
pub mod config;
/// Realm management.
pub mod realm;
/// Schema definitions.
pub mod schema;
/// Token handling.
pub mod token;
/// Validation utilities.
pub mod validation;
pub use token::store::JwtStore;

/// Macro to implement AsRef<Keycloak> for storage types.
#[macro_export]
macro_rules! keycloak {
    ($storage:ty) => {
        impl AsRef<qm::keycloak::Keycloak> for $storage {
            fn as_ref(&self) -> &qm::keycloak::Keycloak {
                &self.inner.keycloak
            }
        }
    };
}
