#![deny(missing_docs)]

//! Server configuration and utilities.
//!
//! This crate provides server configuration and GraphQL handler utilities
//! for building microservices with Axum and async-graphql.
//!
//! ## Features
//!
//! - **GraphQL Handler**: Axum handler for async-graphql with auth injection
//! - **Server Config**: Environment-based server configuration
//! - **Auth Integration**: Automatic auth header processing
//!
//! ## Usage
//!
//! \```ignore
//! use qm_server::{graphql_handler, ServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = ServerConfig::new()?;
//!     // Build and run Axum server with graphql_handler
//!     Ok(())
//! }
//! \```
//!
//! ## Environment Variables
//!
//! | Variable | Description | Default |
//! |----------|-------------|---------|
//! | `SERVER_HOST` | Server host | `127.0.0.1` |
//! | `SERVER_PORT` | Server port | `3000` |
//! | `SERVER_APP_NAME` | Application name | `quick-microservice` |

use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::extract::Extension;
use axum::http::header::HeaderMap;
use axum::http::header::AUTHORIZATION;
use qm_role::AuthContainer;

mod config;
pub use config::Config as ServerConfig;

/// GraphQL request handler for Axum.
///
/// Extracts the Authorization header and injects it into the GraphQL request
/// context. The schema extension type `A` is used for authentication.
pub async fn graphql_handler<A, Q, M, S>(
    schema: Extension<async_graphql::Schema<Q, M, S>>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse
where
    A: Send + Sync + 'static,
    Q: async_graphql::ObjectType + Send + Sync + 'static,
    M: async_graphql::ObjectType + async_graphql::ContainerType + Send + Sync + 'static,
    S: async_graphql::SubscriptionType + Send + Sync + 'static,
{
    let mut req = req.into_inner();
    if let Some(auth_header) = headers.get(AUTHORIZATION).map(AuthContainer::<A>::from) {
        req = req.data(auth_header);
    } else {
        req = req.data(AuthContainer::<A>::default());
    }
    schema.execute(req).await.into()
}
