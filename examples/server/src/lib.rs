use async_graphql::http::GraphiQLSource;
use axum::{
    extract::Extension,
    http::Method,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use tower_http::cors::{AllowOrigin, CorsLayer};

pub mod schema;

use qm_example_ctx::Storage;

const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
const INDEX: &str = constcat::concat!(
    "<html><h1>",
    CRATE_NAME,
    " ",
    CRATE_VERSION,
    "</h1><div>visit <a href=\"/api/graphql\">GraphQL Playground</a></div></html>"
);

pub async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/api/graphql").finish())
}

pub async fn index() -> impl IntoResponse {
    Html(INDEX)
}

async fn router(store: Storage) -> Router {
    let port = store.server_config().port();
    let schema = schema::SchemaBuilder::default().build(store);
    println!("GraphiQL IDE: http://localhost:{port}");
    Router::new()
        .route("/", get(index))
        .route(
            "/api/graphql",
            get(graphiql).post(
                qm::server::graphql_handler::<
                    qm_example_auth::Authorization,
                    schema::QueryRoot,
                    schema::MutationRoot,
                    async_graphql::EmptySubscription,
                >,
            ),
        )
        .layer(Extension(schema))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(|_, _| true))
                .allow_methods([Method::GET, Method::POST]),
        )
}

pub async fn start() -> anyhow::Result<()> {
    let store = Storage::new().await?;
    let address = store.server_config().address().to_string();
    let router = router(store).await;
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, router).await.unwrap();
    Ok(())
}
