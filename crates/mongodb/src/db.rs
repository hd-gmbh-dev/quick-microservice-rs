use futures::stream::StreamExt;
use mongodb::bson::doc;
use mongodb::bson::Document;
use mongodb::options::{FindOneAndUpdateOptions, IndexOptions};
use mongodb::{options::ClientOptions, Client, ClientSession, Collection, Database, IndexModel};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config as MongoDbConfig;

async fn collections(client: &Client, database: &str) -> mongodb::error::Result<Arc<[Arc<str>]>> {
    Ok(client
        .database(database)
        .list_collection_names(None)
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
        log::info!("'{app_name}' -> connects to mongodb '{}'", cfg.database());
        let mut client_options = ClientOptions::parse(cfg.root_address()).await?;
        client_options.app_name = Some(app_name.to_string());
        let admin = Client::with_options(client_options)?;
        let collections = RwLock::new(collections(&admin, cfg.database()).await?);
        if collections.read().await.is_empty() {
            admin
                .database(cfg.database())
                .create_collection("counters", None)
                .await?;
        }
        let db_users = mongodb::bson::from_document::<DbUsers>(
            admin
                .database(cfg.database())
                .run_command(
                    doc! {
                        "usersInfo": [{
                            "db": cfg.database(),
                            "user": cfg.username(),
                        }],
                        "showPrivileges": false,
                        "showCredentials": false,
                    },
                    None,
                )
                .await?,
        )
        .ok();
        if !db_users
            .map(|u| {
                u.users
                    .iter()
                    .any(|u: &DbUser| u.db == cfg.database() && u.user == cfg.username())
            })
            .unwrap_or(false)
        {
            log::info!("{app_name} -> create db {}", cfg.database());
            admin
                .database(cfg.database())
                .run_command(
                    doc! {
                        "createUser": cfg.username(),
                        "pwd": cfg.password(),
                        "roles": [
                            {
                                "role": "readWrite",
                                "db": cfg.database(),
                            }
                        ]
                    },
                    None,
                )
                .await?;
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
        self.inner.client.start_session(None).await
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
                .run_command(
                    doc! {
                        "enableSharding": cfg.database()
                    },
                    None,
                )
                .await?;
        }
        for col in self.inner.collections.read().await.as_ref().iter() {
            log::debug!("found collection: {}", col);
        }
        Ok(())
    }

    pub fn counters<T>(&self) -> Collection<T> {
        self.get().collection::<T>("counters")
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
            self.get().create_collection(name, None).await.ok();
            self.get()
                .collection::<()>(name)
                .create_index(
                    IndexModel::builder().keys(doc! { shard_key: 1 }).build(),
                    None,
                )
                .await?;
            if self.is_sharded() {
                self.get_admin()
                    .run_command(
                        doc! {
                            "shardCollection": &format!("{}.{}", self.inner.db_name, name),
                            "key": { shard_key: "hashed" },
                        },
                        None,
                    )
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
            self.get().create_collection(name, None).await?;
            for index in indexes {
                self.get()
                    .collection::<()>(name)
                    .create_index(
                        IndexModel::builder()
                            .keys(index.0)
                            .options(IndexOptions::builder().unique(index.1).build())
                            .build(),
                        None,
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
            .list_collection_names(None)
            .await?
        {
            if &collection != "api_jwt_secrets" {
                self.inner
                    .admin
                    .database(self.db_name())
                    .collection::<Document>(&collection)
                    .delete_many(doc! {}, None)
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
                        log::error!("Error while parsing MongoDB document: {e:#?}");
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
