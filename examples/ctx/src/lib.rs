use qm::{
    customer::{
        cache::CacheDB,
        context::{CustomerDB, KeycloakDB},
        worker::CleanupProducer,
    },
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
    keycloak_db: qm::pg::DB,
    customer_db: qm::pg::DB,
    db: DB,
    redis: Redis,
    // cache: Cache,
    cache_db: CacheDB,
    mutation_event_producer: Producer,
    cleanup_task_producer: CleanupProducer,
}

#[derive(Clone)]
pub struct Storage {
    inner: Arc<Inner>,
}

impl KeycloakDB for Storage {
    fn keycloak_db(&self) -> &qm::pg::DB {
        &self.inner.keycloak_db
    }
}

impl CustomerDB for Storage {
    fn customer_db(&self) -> &qm::pg::DB {
        &self.inner.customer_db
    }
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
        let keycloak_db = qm::pg::DB::new(
            server_config.app_name(),
            &qm::pg::DbConfig::builder()
                .with_prefix("KEYCLOAK_DB_")
                .build()?,
        )
        .await?;
        let customer_db = qm::pg::DB::new(
            server_config.app_name(),
            &qm::pg::DbConfig::builder()
                .with_prefix("CUSTOMER_DB_")
                .build()?,
        )
        .await?;
        let keycloak = qm::keycloak::Keycloak::new().await?;
        let cache_db = CacheDB::new(
            &customer_db,
            &keycloak_db,
            keycloak.config().realm(),
            keycloak.config().realm_admin_username(),
        )
        .await?;
        let jwt_store = JwtStore::new(keycloak.config());
        let redis = Redis::new()?;
        // let cache = Cache::new("qm-example", keycloak.config().realm()).await?;
        let mutation_event_producer = Producer::new()?;
        let cleanup_task_producer = CleanupProducer::new(redis.pool());
        let result = Self {
            inner: Arc::new(Inner {
                server_config,
                keycloak,
                jwt_store,
                keycloak_db,
                customer_db,
                db,
                redis,
                // cache,
                cache_db,
                mutation_event_producer,
                cleanup_task_producer,
            }),
        };
        // result
        //     .cache()
        //     .reload_all(result.keycloak(), &result)
        //     .await?;
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
    // fn cache(&self) -> &qm::customer::cache::Cache {
    //     &self.inner.cache
    // }
}
