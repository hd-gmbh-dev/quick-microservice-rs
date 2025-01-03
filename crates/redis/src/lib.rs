pub use deadpool_redis::redis;
use deadpool_redis::Runtime;
use std::sync::Arc;
mod config;
pub mod lock;
pub mod work_queue;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use redis::AsyncCommands;
use redis::RedisResult;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::sync::RwLock;
use tokio::task::LocalSet;
use work_queue::Item;
use work_queue::KeyPrefix;
use work_queue::WorkQueue;

pub use crate::config::Config as RedisConfig;
use crate::lock::Lock;

pub struct Inner {
    config: RedisConfig,
    client: redis::Client,
    pool: deadpool_redis::Pool,
}

#[derive(Clone)]
pub struct Redis {
    inner: Arc<Inner>,
}

impl AsRef<deadpool_redis::Pool> for Redis {
    fn as_ref(&self) -> &deadpool_redis::Pool {
        &self.inner.pool
    }
}

impl Redis {
    pub fn new() -> anyhow::Result<Self> {
        let config = RedisConfig::builder().build()?;
        let client = redis::Client::open(config.address())?;
        let redis_cfg = deadpool_redis::Config::from_url(config.address());
        let pool = redis_cfg.create_pool(Some(Runtime::Tokio1))?;
        Ok(Self {
            inner: Arc::new(Inner {
                config,
                client,
                pool,
            }),
        })
    }

    pub fn config(&self) -> &RedisConfig {
        &self.inner.config
    }

    pub fn client(&self) -> &redis::Client {
        &self.inner.client
    }

    pub fn pool(&self) -> Arc<deadpool_redis::Pool> {
        Arc::new(self.inner.pool.clone())
    }

    pub async fn connect(&self) -> Result<deadpool_redis::Connection, deadpool_redis::PoolError> {
        self.inner.pool.get().await
    }

    pub async fn cleanup(&self) -> anyhow::Result<()> {
        let mut con = self.connect().await?;
        let _: redis::Value = redis::cmd("FLUSHALL").query_async(&mut con).await?;
        Ok(())
    }

    pub async fn lock(
        &self,
        key: &str,
        ttl: usize,
        retry_count: u32,
        retry_delay: u32,
    ) -> Result<Lock, lock::Error> {
        let mut con = self.connect().await?;
        lock::lock(&mut con, key, ttl, retry_count, retry_delay).await
    }

    pub async fn unlock(&self, key: &str, lock_id: &str) -> Result<i64, lock::Error> {
        let mut con = self.connect().await?;
        lock::unlock(&mut con, key, lock_id).await
    }
}

/// Runs async function exclusively using Redis lock.
///
/// Lock will be released even if async block fails.
///
/// # Errors
///
/// This function will return an error if either `f` call triggers exception, or lock failure.
/// Panic in async call will not release lock, but it will be released after timeout.
pub async fn mutex_run<S, O, E, F>(lock_name: S, redis: &Redis, f: F) -> Result<O, E>
where
    S: AsRef<str>,
    F: std::future::Future<Output = Result<O, E>>,
    E: From<self::lock::Error>,
{
    let lock = redis.lock(lock_name.as_ref(), 5000, 20, 250).await?;

    let result = f.await;

    redis.unlock(lock_name.as_ref(), &lock.id).await?;

    result
}

#[macro_export]
macro_rules! redis {
    ($storage:ty) => {
        impl AsRef<qm::redis::Redis> for $storage {
            fn as_ref(&self) -> &qm::redis::Redis {
                &self.inner.redis
            }
        }
    };
}

pub type RunningWorkers =
    FuturesUnordered<Pin<Box<dyn Future<Output = String> + Send + Sync + 'static>>>;

pub type ExecItemFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>>;

pub struct WorkerContext<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    ctx: Ctx,
    pub worker_id: usize,
    pub queue: Arc<WorkQueue>,
    pub client: Arc<redis::Client>,
    pub item: Item,
}

impl<Ctx> WorkerContext<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    pub fn ctx(&self) -> &Ctx {
        &self.ctx
    }
    pub async fn complete(&self) -> anyhow::Result<()> {
        let mut con = self.client.get_multiplexed_async_connection().await?;
        self.queue.complete(&mut con, &self.item).await?;
        Ok(())
    }
}

async fn add(
    is_running: Arc<AtomicBool>,
    instances: Arc<RwLock<Option<RunningWorkers>>>,
    fut: Pin<Box<dyn Future<Output = String> + Send + Sync + 'static>>,
) {
    if !is_running.load(Ordering::SeqCst) {
        return;
    }
    instances.write().await.as_mut().unwrap().push(fut);
}

#[async_trait::async_trait]
pub trait Work<Ctx, T>: Send + Sync
where
    Ctx: Clone + Send + Sync + 'static,
    T: DeserializeOwned + Send + Sync,
{
    async fn run(&self, ctx: WorkerContext<Ctx>, item: T) -> anyhow::Result<()>;
}

async fn run_recovery_worker<Ctx, T>(
    client: Arc<redis::Client>,
    is_running: Arc<AtomicBool>,
    worker: Arc<AsyncWorker<Ctx, T>>,
) -> anyhow::Result<()>
where
    Ctx: Clone + Send + Sync + 'static,
    T: DeserializeOwned + Send + Sync,
{
    tracing::info!("start {} worker recovery", worker.prefix);
    let mut con = client.get_multiplexed_async_connection().await?;
    loop {
        if !is_running.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
        worker.recover(&mut con).await?;
    }
    Ok(())
}

async fn run_worker_queue<Ctx, T>(
    ctx: Ctx,
    client: Arc<redis::Client>,
    is_running: Arc<AtomicBool>,
    worker: Arc<AsyncWorker<Ctx, T>>,
    worker_id: usize,
) -> anyhow::Result<()>
where
    Ctx: Clone + Send + Sync + 'static,
    T: DeserializeOwned + Send + Sync,
{
    tracing::info!("start {} worker #{worker_id} queue", worker.prefix);
    let request_queue = Arc::new(WorkQueue::new(KeyPrefix::new(worker.prefix.clone())));
    let mut con = client.get_multiplexed_async_connection().await?;
    loop {
        if !is_running.load(Ordering::SeqCst) {
            break;
        }
        if let Some(item) = request_queue
            .lease(
                &mut con,
                Some(Duration::from_secs(worker.timeout)),
                Duration::from_secs(worker.lease_duration),
            )
            .await?
        {
            if item.data.is_empty() {
                tracing::info!("item is empty");
                request_queue.complete(&mut con, &item).await?;
                continue;
            }
            if let Ok(request) = serde_json::from_slice::<T>(&item.data).inspect_err(|_| {
                tracing::error!(
                    "invalid request item on worker {} #{worker_id} Item: {}",
                    worker.prefix,
                    String::from_utf8_lossy(&item.data)
                );
            }) {
                if let Some(work) = worker.work.as_ref() {
                    work.run(
                        WorkerContext {
                            ctx: ctx.clone(),
                            worker_id,
                            queue: request_queue.clone(),
                            client: client.clone(),
                            item: Item {
                                id: item.id.clone(),
                                data: Box::new([]),
                            },
                        },
                        request,
                    )
                    .await?;
                }
            } else {
                request_queue.complete(&mut con, &item).await?;
            }
        }
    }
    Ok(())
}

struct WorkerInner {
    client: Arc<redis::Client>,
    instances: Arc<RwLock<Option<RunningWorkers>>>,
    is_running: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct Workers {
    inner: Arc<WorkerInner>,
}

impl Workers {
    pub fn new(config: &RedisConfig) -> RedisResult<Self> {
        let client = Arc::new(redis::Client::open(config.address())?);
        Ok(Self::new_with_client(client))
    }

    pub fn new_with_client(client: Arc<redis::Client>) -> Self {
        Self {
            inner: Arc::new(WorkerInner {
                client,
                instances: Arc::new(RwLock::new(Some(RunningWorkers::default()))),
                is_running: Arc::new(AtomicBool::new(true)),
            }),
        }
    }

    pub async fn start<Ctx, T>(&self, ctx: Ctx, worker: AsyncWorker<Ctx, T>) -> anyhow::Result<()>
    where
        Ctx: Clone + Send + Sync + 'static,
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let worker = Arc::new(worker);
        let mut con = self.inner.client.get_multiplexed_async_connection().await?;
        worker.recover(&mut con).await?;
        {
            let instances = self.inner.instances.clone();
            let client = self.inner.client.clone();
            let worker = worker.clone();
            let _th = std::thread::spawn(move || {
                let rt = Builder::new_current_thread().enable_all().build().unwrap();
                let local = LocalSet::new();
                local.spawn_local(async move {
                    let fut_worker = worker.clone();
                    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
                    let is_running = Arc::new(AtomicBool::new(true));
                    let is_fut_running = is_running.clone();
                    add(
                        is_running.clone(),
                        instances,
                        Box::pin(async move {
                            let worker = fut_worker.clone();
                            tracing::info!("stopping {} recovery", worker.prefix);
                            is_fut_running.store(false, Ordering::SeqCst);
                            rx.await.ok();
                            " recovery".to_string()
                        }),
                    )
                    .await;
                    if let Err(err) = run_recovery_worker(client, is_running, worker).await {
                        tracing::error!("{err:#?}");
                        std::process::exit(1);
                    }
                    tx.send(()).ok();
                });
                rt.block_on(local);
            });
        }
        for worker_id in 0..worker.num_workers {
            let worker = worker.clone();
            let client = self.inner.client.clone();
            let ctx = ctx.clone();
            let instances = self.inner.instances.clone();
            let _th = std::thread::spawn(move || {
                let rt = Builder::new_current_thread().enable_all().build().unwrap();
                let local = LocalSet::new();
                local.spawn_local(async move {
                    let fut_worker = worker.clone();
                    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
                    let is_running = Arc::new(AtomicBool::new(true));
                    let is_fut_running = is_running.clone();
                    add(
                        is_running.clone(),
                        instances,
                        Box::pin(async move {
                            let worker = fut_worker.clone();
                            tracing::info!("stopping {} #{worker_id}", worker.prefix);
                            is_fut_running.store(false, Ordering::SeqCst);
                            rx.await.ok();
                            format!("{} worker #{worker_id}", fut_worker.prefix)
                        }),
                    )
                    .await;
                    if let Err(err) =
                        run_worker_queue(ctx.clone(), client, is_running, worker, worker_id).await
                    {
                        tracing::error!("{err:#?}");
                        std::process::exit(1);
                    }
                    tx.send(()).ok();
                });
                rt.block_on(local);
            });
        }
        Ok(())
    }

    pub async fn terminate(&self) -> anyhow::Result<()> {
        if !self.inner.is_running.load(Ordering::SeqCst) {
            anyhow::bail!("Workers already terminated");
        }
        let mut futs = self.inner.instances.write().await.take().unwrap();
        tracing::info!("try stopping {} workers", futs.len());
        while let Some(result) = futs.next().await {
            tracing::info!("stopped {}", result);
        }
        Ok(())
    }
}

pub struct Producer {
    client: Arc<deadpool_redis::Pool>,
    queue: WorkQueue,
}

impl Producer {
    pub fn new<S>(config: &RedisConfig, prefix: S) -> anyhow::Result<Self>
    where
        S: Into<String>,
    {
        let redis_cfg = deadpool_redis::Config::from_url(config.address());
        let redis = Arc::new(redis_cfg.create_pool(Some(Runtime::Tokio1))?);
        Ok(Self::new_with_client(redis, prefix))
    }

    pub fn new_with_client<S>(client: Arc<deadpool_redis::Pool>, prefix: S) -> Self
    where
        S: Into<String>,
    {
        let queue = WorkQueue::new(KeyPrefix::new(prefix.into()));
        Self { client, queue }
    }

    pub async fn add_item_with_connection<C, T>(&self, db: &mut C, data: &T) -> anyhow::Result<()>
    where
        C: AsyncCommands,
        T: Serialize,
    {
        let item = Item::from_json_data(data)?;
        self.queue.add_item(db, &item).await?;
        Ok(())
    }

    pub async fn add_item<T>(&self, data: &T) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let item = Item::from_json_data(data)?;
        let mut con = self.client.get().await?;
        self.queue.add_item(&mut con, &item).await?;
        Ok(())
    }
}

pub struct AsyncWorker<Ctx, T>
where
    Ctx: Clone + Send + Sync + 'static,
    T: DeserializeOwned + Send + Sync,
{
    prefix: String,
    num_workers: usize,
    timeout: u64,
    lease_duration: u64,
    recovery_key: String,
    recovery_queue: WorkQueue,
    work: Option<Box<dyn Work<Ctx, T>>>,
}

impl<Ctx, T> AsyncWorker<Ctx, T>
where
    Ctx: Clone + Send + Sync + 'static,
    T: DeserializeOwned + Send + Sync,
{
    pub fn new<S>(prefix: S) -> Self
    where
        S: Into<String>,
    {
        let prefix = prefix.into();
        let name = KeyPrefix::new(prefix.clone());
        Self {
            recovery_key: name.of(":clean"),
            recovery_queue: WorkQueue::new(name),
            timeout: 5,
            lease_duration: 60,
            num_workers: 1,
            prefix,
            work: None,
        }
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_lease_duration(mut self, lease_duration: u64) -> Self {
        self.lease_duration = lease_duration;
        self
    }

    pub fn with_num_workers(mut self, num_workers: usize) -> Self {
        self.num_workers = num_workers;
        self
    }

    pub fn producer(&self, client: Arc<deadpool_redis::Pool>) -> Producer {
        Producer {
            client,
            queue: WorkQueue::new(KeyPrefix::new(self.prefix.clone())),
        }
    }

    pub async fn recover<C: AsyncCommands>(&self, db: &mut C) -> anyhow::Result<()> {
        let l = lock::lock(db, &self.recovery_key, 3600, 36, 100).await?;
        self.recovery_queue.recover(db).await?;
        lock::unlock(db, &self.recovery_key, l.id).await?;
        Ok(())
    }

    pub fn run(mut self, work: impl Work<Ctx, T> + 'static) -> Self {
        self.work = Some(Box::new(work));
        self
    }
}
