use qm::{
    customer::{cache::Cache, worker::CleanupProducer},
    kafka::producer::Producer,
    keycloak::{JwtStore, Keycloak},
    mongodb::DB,
    redis::Redis,
    server::ServerConfig,
};
use std::sync::Arc;

struct Inner {
    server_config: ServerConfig,
    keycloak: Keycloak,
    jwt_store: JwtStore,
    db: DB,
    redis: Redis,
    cache: Cache,
    mutation_event_producer: Producer,
    cleanup_task_producer: CleanupProducer,
}

#[derive(Clone)]
pub struct Storage {
    inner: Arc<Inner>,
}

qm::mongodb::db!(Storage);
qm::keycloak::keycloak!(Storage);
qm::redis::redis!(Storage);
qm::customer::mutation_event_producer!(Storage);
qm::customer::cleanup_task_producer!(Storage);
qm::customer::storage!(Storage);
qm::customer::cache!(Storage);

impl Storage {
    pub async fn new() -> anyhow::Result<Self> {
        let server_config = ServerConfig::new()?;
        let db =
            qm::mongodb::DB::new(server_config.app_name(), &qm::mongodb::DbConfig::new()?).await?;
        let keycloak = qm::keycloak::Keycloak::new().await?;
        let jwt_store = JwtStore::new(keycloak.config());
        let redis = Redis::new()?;
        let cache = Cache::new("qm-example", keycloak.config().realm()).await?;
        let mutation_event_producer = Producer::new()?;
        let cleanup_task_producer = CleanupProducer::new(redis.pool());
        let result = Self {
            inner: Arc::new(Inner {
                server_config,
                keycloak,
                jwt_store,
                db,
                redis,
                cache,
                mutation_event_producer,
                cleanup_task_producer,
            }),
        };
        result
            .cache()
            .reload_all(result.keycloak(), &result)
            .await?;
        Ok(result)
    }

    pub fn server_config(&self) -> &ServerConfig {
        &self.inner.server_config
    }
    pub fn keycloak(&self) -> &Keycloak {
        &self.inner.keycloak
    }
    pub fn jwt_store(&self) -> &JwtStore {
        &self.inner.jwt_store
    }
    fn cache(&self) -> &qm::customer::cache::Cache {
        &self.inner.cache
    }
}
