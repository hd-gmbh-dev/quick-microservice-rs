#![deny(missing_docs)]

use redis::AsyncCommands;
use redis::RedisError;
use redis::SetOptions;
use redis::Value as RedisValue;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Error reasons for lock acquisition.
pub mod error {
    /// Reasons why a lock cannot be acquired.
    #[derive(thiserror::Error, Clone, Debug, PartialEq)]
    pub enum CanNotGetLockReason {
        /// Lock is currently held by another process.
        #[error("lock is busy")]
        LockIsBussy,
        /// Lock is still busy after exhausting retries.
        #[error("lock is still busy with count: {retry_count} and delay {retry_delay}")]
        LockIsStillBusy {
            /// Number of retry attempts made.
            retry_count: u32,
            /// Delay between retries in milliseconds.
            retry_delay: u32
        },
    }
}

/// Error type for lock operations.
#[derive(thiserror::Error, Clone, Debug, PartialEq)]
pub enum Error {
    /// Connection pool error.
    #[error("pool error {0}")]
    PoolError(String),
    /// Redis error.
    #[error("redis error {0}")]
    RedisError(String),
    /// Could not acquire lock.
    #[error("{0}")]
    CanNotGetLock(
        /// The reason why the lock could not be acquired.
        error::CanNotGetLockReason,
    ),
}

impl From<RedisError> for Error {
    fn from(value: RedisError) -> Self {
        Self::RedisError(value.to_string())
    }
}

impl From<deadpool_redis::PoolError> for Error {
    fn from(value: deadpool_redis::PoolError) -> Self {
        Self::PoolError(value.to_string())
    }
}

const UNLOCK_SCRIPT: &str = r#"
  if redis.call("get", KEYS[1]) == ARGV[1] then
    return redis.call("del", KEYS[1])
  else
    return 0
  end
"#;

/// A distributed lock handle.
#[derive(Debug)]
pub struct Lock {
    /// Unique ID of the lock.
    pub id: String,
}

/// Attempts to acquire a lock without retries.
pub async fn try_lock<C: AsyncCommands, T: AsRef<str>>(
    db: &mut C,
    key: T,
    ttl: usize,
) -> Result<Lock, Error> {
    let id = Uuid::new_v4().to_string();
    let options = SetOptions::default()
        .with_expiration(redis::SetExpiry::PX(ttl as u64))
        .conditional_set(redis::ExistenceCheck::NX);
    let result = db.set_options(key.as_ref(), &id, options).await?;

    match result {
        RedisValue::Okay => Ok(Lock { id }),
        _ => Err(Error::CanNotGetLock(
            error::CanNotGetLockReason::LockIsBussy,
        )),
    }
}

/// Acquires a distributed lock with retries.
pub async fn lock<C: AsyncCommands, T>(
    db: &mut C,
    key: T,
    ttl: usize,
    retry_count: u32,
    retry_delay: u32,
) -> Result<Lock, Error>
where
    T: AsRef<str>,
{
    for _ in 0..retry_count {
        let lock_result = try_lock(db, key.as_ref(), ttl).await;
        match lock_result {
            Ok(lock) => return Ok(lock),
            Err(Error::RedisError(error)) => return Err(Error::RedisError(error)),
            Err(Error::PoolError(error)) => return Err(Error::PoolError(error)),
            Err(Error::CanNotGetLock(_)) => {
                sleep(Duration::from_millis(u64::from(retry_delay))).await;
                continue;
            }
        };
    }

    Err(Error::CanNotGetLock(
        error::CanNotGetLockReason::LockIsStillBusy {
            retry_count,
            retry_delay,
        },
    ))
}

/// Releases a distributed lock.
pub async fn unlock<C: AsyncCommands, K, V>(db: &mut C, key: K, lock_id: V) -> Result<i64, Error>
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    let result: RedisValue = redis::Script::new(UNLOCK_SCRIPT)
        .key(key.as_ref())
        .arg(lock_id.as_ref())
        .invoke_async(db)
        .await?;

    match result {
        RedisValue::Int(remove_count) => Ok(remove_count),
        _ => Ok(0),
    }
}
