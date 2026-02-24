#![deny(missing_docs)]

use std::future::Future;
use std::time::Duration;

use deadpool_redis::redis::{self, AsyncCommands, RedisResult, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Key prefix for work queue keys.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KeyPrefix {
    prefix: String,
}

impl KeyPrefix {
    /// Creates a new KeyPrefix.
    pub fn new(prefix: String) -> KeyPrefix {
        KeyPrefix { prefix }
    }

    /// Returns the full key with the given name appended.
    pub fn of(&self, name: &str) -> String {
        let mut key = String::with_capacity(self.prefix.len() + name.len());
        key.push_str(&self.prefix);
        key.push_str(name);
        key
    }

    /// Creates a new KeyPrefix by appending the given name.
    pub fn and(&self, other: &str) -> KeyPrefix {
        KeyPrefix::new(self.of(other))
    }

    /// Appends a string to this prefix.
    pub fn concat(mut self, other: &str) -> KeyPrefix {
        self.prefix.push_str(other);
        self
    }
}

impl From<String> for KeyPrefix {
    fn from(prefix: String) -> KeyPrefix {
        KeyPrefix::new(prefix)
    }
}

impl From<&str> for KeyPrefix {
    fn from(prefix: &str) -> KeyPrefix {
        KeyPrefix::new(prefix.to_string())
    }
}

impl From<KeyPrefix> for String {
    fn from(key_prefix: KeyPrefix) -> String {
        key_prefix.prefix
    }
}

impl AsRef<str> for KeyPrefix {
    fn as_ref(&self) -> &str {
        &self.prefix
    }
}

/// A work item in the queue.
#[derive(Clone, Debug)]
pub struct Item {
    /// Unique ID of the item.
    pub id: String,
    /// Binary data of the item.
    pub data: Box<[u8]>,
}

impl Item {
    /// Creates a new Item with the given data.
    pub fn new(data: Box<[u8]>) -> Item {
        Item {
            data,
            id: Uuid::new_v4().to_string(),
        }
    }

    /// Creates a new Item from string data.
    pub fn from_string_data(data: String) -> Item {
        Item::new(data.into_bytes().into_boxed_slice())
    }

    /// Creates a new Item from JSON-serializable data.
    pub fn from_json_data<T: Serialize>(data: &T) -> serde_json::Result<Item> {
        Ok(Item::new(serde_json::to_vec(data)?.into()))
    }

    /// Deserializes the item data as JSON.
    pub fn data_json<'a, T: Deserialize<'a>>(&'a self) -> serde_json::Result<T> {
        serde_json::from_slice(&self.data)
    }

    /// Deserializes the item data as JSON with static lifetime.
    pub fn data_json_static<T: for<'de> Deserialize<'de>>(&self) -> serde_json::Result<T> {
        serde_json::from_slice(&self.data)
    }
}

/// Work queue for managing async jobs.
pub struct WorkQueue {
    session: String,
    main_queue_key: String,
    processing_key: String,
    lease_key: KeyPrefix,
    item_data_key: KeyPrefix,
}

impl WorkQueue {
    /// Creates a new WorkQueue with the given name.
    pub fn new(name: KeyPrefix) -> WorkQueue {
        WorkQueue {
            session: Uuid::new_v4().to_string(),
            main_queue_key: name.of(":queue"),
            processing_key: name.of(":processing"),
            lease_key: name.and(":leased_by_session:"),
            item_data_key: name.and(":item:"),
        }
    }

    /// Recovers items from a previous run.
    pub async fn recover<C: AsyncCommands>(&self, db: &mut C) -> RedisResult<()> {
        let processing: RedisResult<Value> = db.lrange(&self.processing_key, 0, -1).await;
        let mut pipeline = Box::new(redis::pipe());
        if let Ok(Value::Array(processing)) = processing {
            for v in processing {
                if let Value::SimpleString(item_id) = v {
                    let a: bool = db.exists(self.lease_key.of(&item_id)).await?;
                    let b: bool = db.exists(self.item_data_key.of(&item_id)).await?;
                    if !a && b {
                        tracing::info!("requeue '{}' -> item '{item_id}'", self.processing_key);
                        pipeline.lpush(&self.main_queue_key, &item_id);
                    }
                }
            }
        }
        pipeline.query_async(db).await
    }

    /// Adds an item to a pipeline.
    pub fn add_item_to_pipeline(&self, pipeline: &mut redis::Pipeline, item: &Item) {
        pipeline.set(self.item_data_key.of(&item.id), item.data.as_ref());
        pipeline.lpush(&self.main_queue_key, &item.id);
    }

    /// Adds an item to the queue.
    pub async fn add_item<C: AsyncCommands>(&self, db: &mut C, item: &Item) -> RedisResult<()> {
        let mut pipeline = Box::new(redis::pipe());
        self.add_item_to_pipeline(&mut pipeline, item);
        pipeline.query_async(db).await
    }

    /// Returns the length of the queue.
    pub fn queue_len<'a, C: AsyncCommands>(
        &'a self,
        db: &'a mut C,
    ) -> impl Future<Output = RedisResult<usize>> + 'a {
        db.llen(&self.main_queue_key)
    }

    /// Returns the number of items being processed.
    pub fn processing<'a, C: AsyncCommands>(
        &'a self,
        db: &'a mut C,
    ) -> impl Future<Output = RedisResult<usize>> + 'a {
        db.llen(&self.processing_key)
    }

    /// Leases an item from the queue.
    pub async fn lease<C: AsyncCommands>(
        &self,
        db: &mut C,
        timeout: Option<Duration>,
        lease_duration: Duration,
    ) -> RedisResult<Option<Item>> {
        let item_id: Option<String> = match timeout {
            Some(Duration::ZERO) => {
                db.lmove(
                    &self.main_queue_key,
                    &self.processing_key,
                    redis::Direction::Right,
                    redis::Direction::Left,
                )
                .await?
            }
            _ => {
                db.blmove(
                    &self.main_queue_key,
                    &self.processing_key,
                    redis::Direction::Right,
                    redis::Direction::Left,
                    timeout.map(|d| d.as_secs() as f64).unwrap_or(0f64),
                )
                .await?
            }
        };

        let item = match item_id {
            Some(item_id) => Item {
                data: db
                    .get::<_, Vec<u8>>(self.item_data_key.of(&item_id))
                    .await?
                    .into_boxed_slice(),
                id: item_id,
            },
            None => return Ok(None),
        };

        let _: () = db
            .set_ex(
                self.lease_key.of(&item.id),
                &self.session,
                lease_duration.as_secs(),
            )
            .await?;

        Ok(Some(item))
    }

    /// Marks an item as completed and removes it from the queue.
    pub async fn complete<C: AsyncCommands>(&self, db: &mut C, item: &Item) -> RedisResult<bool> {
        let removed: usize = db.lrem(&self.processing_key, 0, &item.id).await?;
        if removed == 0 {
            return Ok(false);
        }
        let _: () = redis::pipe()
            .del(self.item_data_key.of(&item.id))
            .del(self.lease_key.of(&item.id))
            .query_async(db)
            .await?;
        Ok(true)
    }
}
