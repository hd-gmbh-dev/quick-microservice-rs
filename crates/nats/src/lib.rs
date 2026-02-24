#![deny(missing_docs)]

//! NATS JetStream integration for building distributed microservices.
//!
//! This crate provides utilities for connecting to NATS with JetStream support,
//! enabling event-driven architectures with distributed locking and sequencing capabilities.
//!
//! ## Features
//!
//! - **Event Publishing**: Stream events to NATS JetStream with structured subject paths
//! - **Distributed Locks**: Acquire and manage distributed locks across services
//! - **Sequence Generation**: Generate unique, monotonically increasing sequences
//! - **System Consumers**: Create durable pull consumers for event processing
//! - **Configuration**: Environment-based configuration with sensible defaults
//!
//! ## Quick Start
//!
//! ```ignore
//! use qm_nats::{Config, Nats};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::new()?;
//!     let nats = Nats::new(config).await?;
//!
//!     // Create a publisher
//!     let publisher = nats.publisher().await?;
//!     publisher.publish("subject.here", &"hello").await?;
//!
//!     // Or use distributed locks
//!     let locks = nats.distributed_locks().await?;
//!     let lock_manager = locks.sys_locks().await?;
//!     let result = lock_manager.run_locked("my-resource", async {
//!         // Critical section
//!         Ok::<_, std::convert::Infalloid>(42)
//!     }).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Environment Variables
//!
//! | Variable | Description | Default |
//! |----------|-------------|---------|
//! | `NATS_HOST` | NATS server host | `127.0.0.1` |
//! | `NATS_PORT` | NATS server port | `4222` |
//! | `NATS_APP_NAME` | Application name | `edd-service-rs` |
//! | `NATS_SYS_LOCKS` | Key-value bucket for locks | `SYS_LOCKS` |
//! | `NATS_EVENTS_STREAM_NAME` | JetStream stream for events | `EVENTS` |
//! | `NATS_EVENTS_STREAM_SUBJECT` | Subject pattern for events | `ev.>` |

use std::{
    collections::HashSet,
    error::Error,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use async_nats::{
    jetstream::{
        self,
        consumer::PullConsumer,
        context::{
            CreateKeyValueErrorKind, CreateStreamError, GetStreamErrorKind, KeyValueErrorKind,
        },
        kv::{self, Operation, Store},
        stream::ConsumerError,
        Context,
    },
    subject::ToSubject,
    Client, ConnectError, ConnectErrorKind,
};
use futures::{StreamExt, TryStreamExt};
use tokio::task::JoinHandle;

pub use async_nats;

/// Subject module for event subject path generation.
pub mod subject;

/// Configuration for NATS JetStream connection.
///
/// Loads configuration from environment variables with sensible defaults.
/// See module-level documentation for available environment variables.
#[derive(Clone, serde::Deserialize)]
pub struct Config {
    app_name: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    #[serde(skip)]
    address: Option<String>,
    sys_locks: Option<String>,
    events_stream_name: Option<String>,
    events_stream_subject: Option<String>,
}

impl Config {
    /// Creates a new Config from environment variables with default NATS_ prefix.
    pub fn new() -> envy::Result<Self> {
        ConfigBuilder::default().build()
    }

    /// Creates a new ConfigBuilder for custom configuration.
    pub fn builder<'a>() -> ConfigBuilder<'a> {
        ConfigBuilder::default()
    }

    /// Returns the NATS server address.
    pub fn address(&self) -> &str {
        self.address.as_deref().unwrap()
    }

    /// Returns the NATS server port.
    pub fn port(&self) -> u16 {
        self.port.unwrap_or(3000)
    }

    /// Returns the key-value bucket name for system locks.
    pub fn sys_locks(&self) -> &str {
        self.sys_locks.as_deref().unwrap_or("SYS_LOCKS")
    }

    /// Returns the JetStream stream name for events.
    pub fn events_stream_name(&self) -> &str {
        self.events_stream_name.as_deref().unwrap_or("EVENTS")
    }

    /// Returns the subject pattern for events stream.
    pub fn events_stream_subject(&self) -> &str {
        self.events_stream_subject.as_deref().unwrap_or("ev.>")
    }
}

/// Builder for creating Config with custom settings.
#[derive(Default)]
pub struct ConfigBuilder<'a> {
    prefix: Option<&'a str>,
}

impl<'a> ConfigBuilder<'a> {
    /// Sets a custom environment variable prefix.
    pub fn with_prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// Builds the Config from environment variables.
    pub fn build(self) -> envy::Result<Config> {
        let prefix = self.prefix.unwrap_or("NATS_");
        let mut cfg: Config = envy::prefixed(prefix).from_env()?;
        if cfg.app_name.is_none() {
            cfg.app_name = Some("edd-service-rs".into());
        }
        let host = cfg.host.as_deref().unwrap_or("127.0.0.1");
        let port = cfg.port.unwrap_or(4222);
        cfg.address = Some(format!("nats://{}:{}", host, port));
        Ok(cfg)
    }
}

/// Internal state for the Nats client.
pub struct Inner {
    client: Client,
    config: Config,
}

/// NATS JetStream client wrapper.
///
/// Provides high-level access to NATS JetStream features including
/// event publishing, distributed locking, and sequence management.
#[derive(Clone)]
pub struct Nats {
    inner: Arc<Inner>,
}

impl Nats {
    /// Creates a new Nats client and connects to the NATS server.
    pub async fn new(config: Config) -> Result<Self, ConnectError> {
        let client = async_nats::ConnectOptions::new()
            .max_reconnects(Some(1))
            .connect(config.address())
            .await?;
        Ok(Self {
            inner: Arc::new(Inner { client, config }),
        })
    }

    /// Returns a reference to the underlying NATS client.
    pub fn client(&self) -> &Client {
        &self.inner.client
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    /// Creates a new event publisher.
    pub async fn publisher(&self) -> Result<Publisher, CreateStreamError> {
        let ctx = jetstream::new(self.inner.client.clone());
        let p = Publisher { ctx };
        p.init(&self.inner.config).await?;
        Ok(p)
    }

    /// Creates a durable pull consumer with the given name.
    pub async fn sys_consumer(&self, name: String) -> Result<PullConsumer, ConsumerError> {
        let ctx = jetstream::new(self.inner.client.clone());
        ctx.create_consumer_on_stream(
            jetstream::consumer::pull::Config {
                durable_name: Some(name),
                ..Default::default()
            },
            self.inner.config.events_stream_name(),
        )
        .await
    }

    /// Creates a durable pull consumer with a filter subject.
    pub async fn sys_consumer_with_filter(
        &self,
        name: String,
        filter_subject: String,
    ) -> Result<PullConsumer, ConsumerError> {
        let ctx = jetstream::new(self.inner.client.clone());
        ctx.create_consumer_on_stream(
            jetstream::consumer::pull::Config {
                durable_name: Some(name),
                filter_subject,
                ..Default::default()
            },
            self.inner.config.events_stream_name(),
        )
        .await
    }

    /// Creates a durable pull consumer with multiple filter subjects.
    pub async fn sys_consumer_with_filters(
        &self,
        name: String,
        filter_subjects: Vec<String>,
    ) -> Result<PullConsumer, ConsumerError> {
        let ctx = jetstream::new(self.inner.client.clone());
        ctx.create_consumer_on_stream(
            jetstream::consumer::pull::Config {
                durable_name: Some(name),
                filter_subjects,
                ..Default::default()
            },
            self.inner.config.events_stream_name(),
        )
        .await
    }

    /// Creates a temporary pull consumer with a filter subject.
    pub async fn tmp_sys_consumer_with_filter(
        &self,
        filter_subject: String,
    ) -> Result<PullConsumer, ConsumerError> {
        let ctx = jetstream::new(self.inner.client.clone());
        ctx.create_consumer_on_stream(
            jetstream::consumer::pull::Config {
                filter_subject,
                deliver_policy: jetstream::consumer::DeliverPolicy::Last,
                ..Default::default()
            },
            self.inner.config.events_stream_name(),
        )
        .await
    }

    /// Creates a distributed locks manager.
    pub async fn distributed_locks(&self) -> Result<DistributedLocks, DistributedLocksError> {
        let ctx = jetstream::new(self.inner.client.clone());
        DistributedLocks::new(ctx, &self.inner.config).await
    }

    /// Creates a sequence manager.
    pub fn sequence_manager(&self) -> SequenceManager {
        let ctx = jetstream::new(self.inner.client.clone());
        SequenceManager { ctx }
    }
}

/// Trait for converting events to NATS subjects.
///
/// Implement this trait on your event types to enable automatic
/// subject path generation for event publishing.
pub trait EventToSubject<M> {
    /// Converts the event to a NATS subject.
    fn event_to_subject(&self) -> async_nats::Subject;
}

/// Event publisher for NATS JetStream.
///
/// Manages the events stream and provides methods to publish events
/// with structured subject paths.
pub struct Publisher {
    ctx: Context,
}

impl Publisher {
    async fn init(&self, config: &Config) -> Result<(), CreateStreamError> {
        let names: HashSet<String> = self.ctx.stream_names().try_collect().await?;
        if !names.contains(config.events_stream_name()) {
            self.ctx
                .create_stream(jetstream::stream::Config {
                    name: config.events_stream_name().to_string(),
                    subjects: vec![config.events_stream_subject().into()],
                    allow_direct: true,
                    deny_delete: true,
                    deny_purge: true,
                    ..Default::default()
                })
                .await?;
        }
        Ok(())
    }

    /// Publishes an event to the given subject.
    pub async fn publish<S: ToSubject, P: ?Sized + serde::Serialize>(
        &self,
        subject: S,
        payload: &P,
    ) -> anyhow::Result<()> {
        self.ctx
            .publish(subject, serde_json::to_vec(payload)?.into())
            .await?;
        Ok(())
    }

    /// Publishes an event using the subject derived from the event type.
    pub async fn publish_event<S, M, P>(&self, subject: &S, payload: &P) -> anyhow::Result<()>
    where
        S: ?Sized + EventToSubject<M>,
        P: ?Sized + serde::Serialize,
    {
        self.ctx
            .publish(
                subject.event_to_subject(),
                serde_json::to_vec(payload)?.into(),
            )
            .await?;
        Ok(())
    }
}

impl AsRef<Context> for Publisher {
    fn as_ref(&self) -> &Context {
        &self.ctx
    }
}

/// Error type for distributed lock operations.
#[derive(thiserror::Error, Debug)]
pub enum DistributedLocksError {
    /// Error connecting to NATS.
    #[error(transparent)]
    Connect(#[from] async_nats::error::Error<ConnectErrorKind>),
    /// Error creating a key-value store.
    #[error(transparent)]
    CreateKeyValue(#[from] async_nats::error::Error<CreateKeyValueErrorKind>),
    /// Error accessing a key-value store.
    #[error(transparent)]
    KeyValue(#[from] async_nats::error::Error<KeyValueErrorKind>),
}

/// Distributed locks manager.
///
/// Manages a key-value store for distributed locking across services.
#[derive(Clone)]
pub struct DistributedLocks {
    ctx: Context,
    sys_locks: String,
}

impl DistributedLocks {
    async fn new(ctx: Context, config: &Config) -> Result<Self, DistributedLocksError> {
        let lm = DistributedLocks {
            ctx,
            sys_locks: config.sys_locks().to_string(),
        };
        if !lm.exists(config.sys_locks()).await? {
            lm.create(config.sys_locks(), 5).await?;
        }
        Ok(lm)
    }

    async fn create<T: Into<String>>(
        &self,
        name: T,
        max_age: u64,
    ) -> Result<Store, DistributedLocksError> {
        Ok(self
            .ctx
            .create_key_value(kv::Config {
                bucket: name.into(),
                max_age: std::time::Duration::from_secs(max_age),
                history: 1,
                ..Default::default()
            })
            .await?)
    }

    async fn exists<T: Into<String>>(&self, bucket: T) -> Result<bool, DistributedLocksError> {
        if let Err(err) = self.ctx.get_key_value(bucket).await {
            if err.kind() == KeyValueErrorKind::GetBucket {
                if let Some(src) = err.source() {
                    let err = src.downcast_ref::<async_nats::error::Error<GetStreamErrorKind>>();
                    if let Some(err) = err {
                        if let GetStreamErrorKind::JetStream(err) = err.kind() {
                            if err.code() == 404 {
                                return Ok(false);
                            }
                        }
                    }
                }
            }
            Err(err)?;
        }
        Ok(true)
    }

    /// Returns a lock manager for system locks.
    pub async fn sys_locks(&self) -> anyhow::Result<LockManager> {
        let kv = self.ctx.get_key_value(&self.sys_locks).await?;
        Ok(LockManager { kv: Arc::new(kv) })
    }
}

/// Error type for lock manager operations.
#[derive(thiserror::Error, Debug)]
pub enum LockManagerError {
    /// Error creating a key-value store.
    #[error(transparent)]
    CreateKeyValue(#[from] async_nats::error::Error<CreateKeyValueErrorKind>),
    /// Error accessing a key-value store.
    #[error(transparent)]
    KeyValue(#[from] async_nats::error::Error<KeyValueErrorKind>),
    /// Error watching key-value store changes.
    #[error(transparent)]
    Watch(#[from] async_nats::error::Error<kv::WatchErrorKind>),
    /// Unable to acquire lock after exhausting retries.
    #[error("unable to lock resource after {0:?}")]
    OutOfRetries(std::time::Duration),
}

/// Error type for sequence manager operations.
#[derive(thiserror::Error, Debug)]
pub enum SequenceManagerError {
    /// Error connecting to NATS.
    #[error(transparent)]
    Connect(#[from] async_nats::error::Error<ConnectErrorKind>),
    /// Error creating a key-value store.
    #[error(transparent)]
    CreateKeyValue(#[from] async_nats::error::Error<CreateKeyValueErrorKind>),
    /// Error accessing a key-value store.
    #[error(transparent)]
    KeyValue(#[from] async_nats::error::Error<KeyValueErrorKind>),
    /// Error putting a value to the key-value store.
    #[error(transparent)]
    Put(#[from] async_nats::error::Error<async_nats::jetstream::kv::PutErrorKind>),
    /// Error reading an entry from the key-value store.
    #[error(transparent)]
    Entry(#[from] async_nats::error::Error<async_nats::jetstream::kv::EntryErrorKind>),
}

/// Sequence manager for generating unique, monotonically increasing IDs.
///
/// Uses NATS JetStream key-value stores to maintain sequence counters
/// that can be used across distributed services.
pub struct SequenceManager {
    ctx: Context,
}

impl SequenceManager {
    async fn create<T: Into<String>>(&self, name: T) -> Result<Store, SequenceManagerError> {
        Ok(self
            .ctx
            .create_key_value(kv::Config {
                bucket: name.into(),
                ..Default::default()
            })
            .await?)
    }

    async fn exists<T: Into<String>>(&self, bucket: T) -> Result<bool, SequenceManagerError> {
        if let Err(err) = self.ctx.get_key_value(bucket).await {
            if err.kind() == KeyValueErrorKind::GetBucket {
                if let Some(src) = err.source() {
                    let err = src.downcast_ref::<async_nats::error::Error<GetStreamErrorKind>>();
                    if let Some(err) = err {
                        if let GetStreamErrorKind::JetStream(err) = err.kind() {
                            if err.code() == 404 {
                                return Ok(false);
                            }
                        }
                    }
                }
            }
            Err(err)?;
        }
        Ok(true)
    }

    async fn get<T: Into<String>>(&self, bucket: T) -> Result<Store, SequenceManagerError> {
        Ok(self.ctx.get_key_value(bucket).await?)
    }

    /// Gets the next sequence number for the given prefix, creating the bucket if needed.
    pub async fn next(&self, prefix: &str, id: i64) -> Result<i64, SequenceManagerError> {
        let bucket = format!("sm-{prefix}");
        if !self.exists(&bucket).await? {
            let store = self.create(&bucket).await?;
            let result = store.put("id", id.to_be_bytes().to_vec().into()).await?;
            Ok(result as i64)
        } else {
            let store = self.get(&bucket).await?;
            let e = store.entry("id").await?;
            if let Some(e) = e {
                Ok(e.revision as i64)
            } else {
                let result = store.put("id", id.to_be_bytes().to_vec().into()).await?;
                Ok(result as i64)
            }
        }
    }

    /// Increments the sequence number for the given prefix.
    pub async fn increment(&self, prefix: &str, id: i64) -> Result<i64, SequenceManagerError> {
        let bucket = format!("sm-{prefix}");
        let store = self.get(&bucket).await?;
        let e = store.put("id", id.to_be_bytes().to_vec().into()).await?;
        Ok(e as i64)
    }
}

/// Lock manager for acquiring and managing distributed locks.
///
/// Provides automatic lock acquisition and release with retry logic.
/// Locks are automatically refreshed and released when the critical
/// section completes or the lock holder crashes.
pub struct LockManager {
    kv: Arc<Store>,
}

impl LockManager {
    /// Runs the given future while holding a distributed lock.
    ///
    /// The lock is automatically acquired before the future runs and released
    /// when the future completes or the holder crashes.
    pub async fn run_locked<N, O, F, E>(&self, name: N, f: F) -> Result<O, E>
    where
        N: Into<String>,
        F: std::future::Future<Output = Result<O, E>>,
        E: From<LockManagerError>,
    {
        let lock = self.try_lock(name.into(), 3, 5).await?;
        let result = f.await;
        let w_kv = self.kv.clone();
        tokio::spawn(async move {
            if !lock.jh.is_finished() {
                lock.jh.abort();
                let result = lock.jh.await;
                if let Err(err) = result {
                    if !err.is_cancelled() {
                        tracing::error!("{err:#?}");
                    }
                }
            }
            w_kv.delete_expect_revision(lock.name, Some(lock.revision.load(Ordering::SeqCst)))
                .await
                .ok();
        });
        result
    }

    async fn try_lock(
        &self,
        name: String,
        timeout: u64,
        retries: usize,
    ) -> Result<Lock, LockManagerError> {
        let now = std::time::Instant::now();
        let max_retries = retries;
        let mut tries = 0;
        let revision = Arc::new(AtomicU64::new(0));
        let kv = &self.kv;
        loop {
            if tries >= max_retries {
                return Err(LockManagerError::OutOfRetries(now.elapsed()));
            }
            let v = kv.create(&name, "r".into()).await;
            if let Err(err) = v {
                if err.kind() == async_nats::jetstream::kv::CreateErrorKind::AlreadyExists {
                    tracing::debug!("seems to be locked already, {tries} try to watch for changes");
                    let mut w = kv.watch(&name).await?;
                    let f = async {
                        'inner: while let Some(m) = w.next().await {
                            if let Ok(e) = m {
                                if e.operation == Operation::Delete {
                                    tracing::debug!("retry because prev lock was deleted");
                                    break 'inner;
                                }
                            }
                        }
                    };
                    let t = async {
                        tokio::time::sleep(std::time::Duration::from_secs(timeout)).await;
                    };
                    let change = tokio::select! {
                        _ = f => true,
                        _ = t => false,
                    };
                    if !change {
                        tries += 1;
                    }
                }
            } else {
                let r = v.unwrap();
                revision.store(r, Ordering::SeqCst);
                tracing::debug!("got lock: '{name}'");
                break;
            }
        }
        let w_kv = self.kv.clone();
        let w_name = name.clone();
        let w_revision = revision.clone();

        let jh = tokio::spawn(async move {
            let mut run = 0;
            loop {
                run += 1;
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                tracing::debug!("refresh lock {w_name}");
                let result = w_kv
                    .update(&w_name, "u".into(), w_revision.load(Ordering::SeqCst))
                    .await;
                if let Err(err) = result {
                    tracing::error!("{err:#?}");
                    break;
                } else {
                    w_revision.store(result.unwrap(), Ordering::SeqCst);
                }
                if run >= 5 {
                    tracing::debug!("release lock after timeout");
                    break;
                }
            }
            anyhow::Ok(())
        });

        Ok(Lock { name, revision, jh })
    }
}

/// State of a distributed lock.
#[derive(Debug, PartialEq, Eq)]
pub enum LockState {
    /// Lock is being acquired.
    Registering,
    /// Lock has been acquired.
    Registered,
}

/// A distributed lock handle.
///
/// Represents an acquired lock. The lock is automatically released
/// when the handle is dropped or the holder crashes.
#[derive(Debug)]
pub struct Lock {
    name: String,
    revision: Arc<AtomicU64>,
    jh: JoinHandle<anyhow::Result<()>>,
}
