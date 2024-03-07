use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::extract::Extension;
use axum::http::header::HeaderMap;
use axum::http::header::AUTHORIZATION;
use qm_role::AuthContainer;

mod config;
pub use config::Config as ServerConfig;

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
