//! # keycloak
//!
//! `keycloak` is a crate for management everything that has to do with
//! Keycloak. Keycloak is used for authentication and authorization.
//!
//! In the development environment, Keycloak can be found in a Kubernetes
//! at <https://keycloak.qm.local>.
//! If Keycloak is missing, you need to run
//! `k8s/helm-charts/skaffold/skaffold.infra-1-base.yaml`.
//!
//! Default username/password: `admin`/`Admin123`
mod client;

pub use client::*;
pub mod config;
pub mod realm;
pub mod schema;
pub mod token;
pub mod validation;
pub use token::store::JwtStore;

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
