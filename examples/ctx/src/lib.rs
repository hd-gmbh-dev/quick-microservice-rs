use qm::{
    customer::{cache::Cache, context::InMemoryCache},
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
}

#[derive(Clone)]
pub struct Storage {
    inner: Arc<Inner>,
}

qm::mongodb::db!(Storage);
qm::keycloak::keycloak!(Storage);
qm::redis::redis!(Storage);
qm::customer::storage!(Storage);

impl Storage {
    pub async fn new() -> anyhow::Result<Self> {
        let server_config = ServerConfig::new()?;
        let db =
            qm::mongodb::DB::new(server_config.app_name(), &qm::mongodb::DbConfig::new()?).await?;
        let keycloak = qm::keycloak::Keycloak::new().await?;
        let jwt_store = JwtStore::new(keycloak.config());
        let redis = Redis::new()?;
        let cache = Cache::new("qm-example", &keycloak, &db).await?;
        Ok(Self {
            inner: Arc::new(Inner {
                server_config,
                keycloak,
                jwt_store,
                db,
                redis,
                cache,
            }),
        })
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
}

impl InMemoryCache for Storage {
    fn cache(&self) -> Option<&qm::customer::cache::Cache> {
        Some(&self.inner.cache)
    }
}
