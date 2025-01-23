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
    }, subject::ToSubject, Client, ConnectError, ConnectErrorKind
};
use futures::{StreamExt, TryStreamExt};
use tokio::task::JoinHandle;

pub use async_nats;

pub mod subject;

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
    pub fn new() -> envy::Result<Self> {
        ConfigBuilder::default().build()
    }

    pub fn builder<'a>() -> ConfigBuilder<'a> {
        ConfigBuilder::default()
    }

    pub fn address(&self) -> &str {
        self.address.as_deref().unwrap()
    }

    pub fn port(&self) -> u16 {
        self.port.unwrap_or(3000)
    }
    pub fn sys_locks(&self) -> &str {
        self.sys_locks.as_deref().unwrap_or("SYS_LOCKS")
    }
    pub fn events_stream_name(&self) -> &str {
        self.events_stream_name.as_deref().unwrap_or("EVENTS")
    }
    pub fn events_stream_subject(&self) -> &str {
        self.events_stream_subject.as_deref().unwrap_or("ev.>")
    }
}

#[derive(Default)]
pub struct ConfigBuilder<'a> {
    prefix: Option<&'a str>,
}

impl<'a> ConfigBuilder<'a> {
    pub fn with_prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

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

pub struct Inner {
    client: Client,
    config: Config,
}

#[derive(Clone)]
pub struct Nats {
    inner: Arc<Inner>,
}

impl Nats {
    pub async fn new(config: Config) -> Result<Self, ConnectError> {
        let client = async_nats::ConnectOptions::new()
            .max_reconnects(Some(1))
            .connect(config.address())
            .await?;
        Ok(Self {
            inner: Arc::new(Inner { client, config }),
        })
    }

    pub async fn publisher(&self) -> Result<Publisher, CreateStreamError> {
        let ctx = jetstream::new(self.inner.client.clone());
        let p = Publisher { ctx };
        p.init(&self.inner.config).await?;
        Ok(p)
    }

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

    pub async fn distributed_locks(&self) -> Result<DistributedLocks, DistributedLocksError> {
        let ctx = jetstream::new(self.inner.client.clone());
        DistributedLocks::new(ctx, &self.inner.config).await
    }

    pub fn sequence_manager(&self) -> SequenceManager {
        let ctx = jetstream::new(self.inner.client.clone());
        SequenceManager { ctx }
    }
}


pub trait EventToSubject<M> {
    fn event_to_subject(&self) -> async_nats::Subject;
}

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

#[derive(thiserror::Error, Debug)]
pub enum DistributedLocksError {
    #[error(transparent)]
    Connect(#[from] async_nats::error::Error<ConnectErrorKind>),
    #[error(transparent)]
    CreateKeyValue(#[from] async_nats::error::Error<CreateKeyValueErrorKind>),
    #[error(transparent)]
    KeyValue(#[from] async_nats::error::Error<KeyValueErrorKind>),
}

#[derive(Clone)]
pub struct DistributedLocks {
    ctx: Context,
    sys_locks: String,
}

impl DistributedLocks {
    async fn new(ctx: Context, config: &Config) -> Result<Self, DistributedLocksError> {
        let lm = DistributedLocks { ctx, sys_locks: config.sys_locks().to_string() };
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

    pub async fn sys_locks(&self) -> anyhow::Result<LockManager> {
        let kv = self.ctx.get_key_value(&self.sys_locks).await?;
        Ok(LockManager { kv: Arc::new(kv) })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LockManagerError {
    #[error(transparent)]
    CreateKeyValue(#[from] async_nats::error::Error<CreateKeyValueErrorKind>),
    #[error(transparent)]
    KeyValue(#[from] async_nats::error::Error<KeyValueErrorKind>),
    #[error(transparent)]
    Watch(#[from] async_nats::error::Error<kv::WatchErrorKind>),
    #[error("unable to lock resource after {0:?}")]
    OutOfRetries(std::time::Duration),
}

#[derive(thiserror::Error, Debug)]
pub enum SequenceManagerError {
    #[error(transparent)]
    Connect(#[from] async_nats::error::Error<ConnectErrorKind>),
    #[error(transparent)]
    CreateKeyValue(#[from] async_nats::error::Error<CreateKeyValueErrorKind>),
    #[error(transparent)]
    KeyValue(#[from] async_nats::error::Error<KeyValueErrorKind>),
    #[error(transparent)]
    Put(#[from] async_nats::error::Error<async_nats::jetstream::kv::PutErrorKind>),
    #[error(transparent)]
    Entry(#[from] async_nats::error::Error<async_nats::jetstream::kv::EntryErrorKind>),
}

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

    pub async fn increment(&self, prefix: &str, id: i64) -> Result<i64, SequenceManagerError> {
        let bucket = format!("sm-{prefix}");
        let store = self.get(&bucket).await?;
        let e = store.put("id", id.to_be_bytes().to_vec().into()).await?;
        Ok(e as i64)
    }
}

pub struct LockManager {
    kv: Arc<Store>,
}

impl LockManager {
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
                if result.is_err() {
                    let err = result.unwrap_err();
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

#[derive(Debug, PartialEq, Eq)]
pub enum LockState {
    Registering,
    Registered,
}

#[derive(Debug)]
pub struct Lock {
    name: String,
    revision: Arc<AtomicU64>,
    jh: JoinHandle<anyhow::Result<()>>,
}

