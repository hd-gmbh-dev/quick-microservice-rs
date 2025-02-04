use futures::stream::StreamExt;
use mongodb::bson::doc;
use mongodb::bson::Document;
use mongodb::options::{FindOneAndUpdateOptions, IndexOptions};
use mongodb::{options::ClientOptions, Client, ClientSession, Database, IndexModel};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config as MongoDbConfig;

async fn collections(client: &Client, database: &str) -> mongodb::error::Result<Arc<[Arc<str>]>> {
    Ok(client
        .database(database)
        .list_collection_names()
        .await?
        .into_iter()
        .map(Arc::from)
        .collect())
}

struct Inner {
    db_name: Arc<str>,
    admin_db_name: Arc<str>,
    client: Client,
    admin: Client,
    is_sharded: bool,
    collections: RwLock<Arc<[Arc<str>]>>,
}

#[derive(serde::Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct DbUser {
    // #[serde(rename = "_id")]
    // id: String,
    // user_id: Uuid,
    user: String,
    db: String,
}

#[derive(serde::Deserialize)]
pub struct DbUsers {
    users: Vec<DbUser>,
}

#[derive(Clone)]
pub struct DB {
    inner: Arc<Inner>,
}

impl DB {
    pub async fn new(app_name: &str, cfg: &MongoDbConfig) -> mongodb::error::Result<Self> {
        tracing::info!("'{app_name}' -> connects to mongodb '{}'", cfg.database());
        let mut client_options = ClientOptions::parse(cfg.root_address()).await?;
        client_options.app_name = Some(app_name.to_string());
        let admin = Client::with_options(client_options)?;
        let collections = RwLock::new(collections(&admin, cfg.database()).await?);
        if let (Some(username), Some(password)) = (cfg.username(), cfg.password()) {
            let db_users = mongodb::bson::from_document::<DbUsers>(
                admin
                    .database(cfg.database())
                    .run_command(doc! {
                        "usersInfo": [{
                            "db": cfg.database(),
                            "user": username,
                        }],
                        "showPrivileges": false,
                        "showCredentials": false,
                    })
                    .await?,
            )
            .ok();
            if !db_users
                .map(|u| {
                    u.users
                        .iter()
                        .any(|u: &DbUser| u.db == cfg.database() && u.user == username)
                })
                .unwrap_or(false)
            {
                tracing::info!(
                    "{app_name} -> create user {} for db {}",
                    username,
                    cfg.database()
                );
                admin
                    .database(cfg.database())
                    .run_command(doc! {
                        "createUser": username,
                        "pwd": password,
                        "roles": [
                            {
                                "role": "readWrite",
                                "db": cfg.database(),
                            }
                        ]
                    })
                    .await?;
            }
        }
        let mut client_options = ClientOptions::parse(cfg.address()).await?;
        client_options.app_name = Some(app_name.to_string());
        let client = Client::with_options(client_options)?;
        let is_sharded = cfg.sharded();
        let db = Self {
            inner: Arc::new(Inner {
                db_name: Arc::from(cfg.database()),
                admin_db_name: Arc::from(cfg.root_database()),
                client,
                admin,
                is_sharded,
                collections,
            }),
        };
        db.setup(cfg).await?;
        Ok(db)
    }

    pub fn is_sharded(&self) -> bool {
        self.inner.is_sharded
    }

    pub async fn session(&self) -> mongodb::error::Result<ClientSession> {
        self.inner.client.start_session().await
    }

    pub fn get(&self) -> Database {
        self.inner.client.database(&self.inner.db_name)
    }

    pub fn get_admin(&self) -> Database {
        self.inner.admin.database(&self.inner.admin_db_name)
    }

    pub fn db_name(&self) -> &str {
        &self.inner.db_name
    }

    pub async fn setup<'a>(&'a self, cfg: &MongoDbConfig) -> mongodb::error::Result<()> {
        if self.is_sharded() {
            self.get_admin()
                .run_command(doc! {
                    "enableSharding": cfg.database()
                })
                .await?;
        }
        for col in self.inner.collections.read().await.as_ref().iter() {
            tracing::debug!("found collection: {}", col);
        }
        Ok(())
    }

    pub async fn collections(&self) -> Arc<[Arc<str>]> {
        self.inner.collections.read().await.clone()
    }

    pub async fn update_collections(&self) -> mongodb::error::Result<()> {
        *self.inner.collections.write().await =
            collections(&self.inner.client, self.db_name()).await?;
        Ok(())
    }

    pub async fn ensure_collection_with_sharding(
        &self,
        collections: &[String],
        name: &str,
        shard_key: &str,
    ) -> mongodb::error::Result<()> {
        if !collections.iter().any(|c| c == name) {
            self.get().create_collection(name).await.ok();
            self.get()
                .collection::<()>(name)
                .create_index(IndexModel::builder().keys(doc! { shard_key: 1 }).build())
                .await?;
            if self.is_sharded() {
                self.get_admin()
                    .run_command(doc! {
                        "shardCollection": &format!("{}.{}", self.inner.db_name, name),
                        "key": { shard_key: "hashed" },
                    })
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn ensure_collection_with_indexes(
        &self,
        collections: &[String],
        name: &str,
        indexes: Vec<(Document, bool)>,
    ) -> mongodb::error::Result<bool> {
        if !collections.iter().any(|c| c == name) {
            self.get().create_collection(name).await?;
            for index in indexes {
                self.get()
                    .collection::<()>(name)
                    .create_index(
                        IndexModel::builder()
                            .keys(index.0)
                            .options(IndexOptions::builder().unique(index.1).build())
                            .build(),
                    )
                    .await?;
            }
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn cleanup(&self) -> mongodb::error::Result<()> {
        for collection in self
            .inner
            .admin
            .database(self.db_name())
            .list_collection_names()
            .await?
        {
            if &collection != "api_jwt_secrets" {
                self.inner
                    .admin
                    .database(self.db_name())
                    .collection::<Document>(&collection)
                    .delete_many(doc! {})
                    .await?;
            }
        }
        Ok(())
    }
}

pub async fn parse_vec<T>(cursor: mongodb::Cursor<Document>) -> Vec<T>
where
    T: serde::de::DeserializeOwned,
{
    cursor
        .filter_map(|v| async {
            v.ok().and_then(|v| {
                mongodb::bson::from_document::<T>(v)
                    .map_err(|e| {
                        tracing::error!("Error while parsing MongoDB document: {e:#?}");
                        e
                    })
                    .ok()
                    .map(From::from)
            })
        })
        .collect()
        .await
}

pub fn insert_always_opts() -> Option<FindOneAndUpdateOptions> {
    let mut opts = FindOneAndUpdateOptions::default();
    opts.upsert = Some(true);
    Some(opts)
}

#[macro_export]
macro_rules! db {
    ($storage:ty) => {
        impl AsRef<qm::mongodb::DB> for $storage {
            fn as_ref(&self) -> &qm::mongodb::DB {
                &self.inner.db
            }
        }
    };
}
