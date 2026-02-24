#![deny(missing_docs)]

//! MongoDB connection and utilities.
//!
//! This crate provides MongoDB connection management with automatic user creation,
//! sharding support, and collection management utilities.
//!
//! ## Features
//!
//! - **Connection Management**: Connect to MongoDB with automatic user creation
//! - **Sharding Support**: Enable sharding for collections with hashed shard keys
//! - **Collection Management**: Create collections with indexes and sharding
//! - **Configuration**: Environment-based configuration with prefix support
//!
//! ## Usage
//!
//! \```ignore
//! use qm_mongodb::{DbConfig, DB};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = DbConfig::new()?;
//!     let db = DB::new("my-app", &config).await?;
//!     let collection = db.get().collection::<Document>("my_collection");
//!     Ok(())
//! }
//! \```
//!
//! ## Environment Variables
//!
//! | Variable | Description | Default |
//! |----------|-------------|---------|
//! | `MONGODB_HOST` | MongoDB host | `127.0.0.1` |
//! | `MONGODB_PORT` | MongoDB port | `27017` |
//! | `MONGODB_USERNAME` | Database username | (none) |
//! | `MONGODB_PASSWORD` | Database password | (none) |
//! | `MONGODB_DATABASE` | Database name | `test` |
//! | `MONGODB_ROOT_USERNAME` | Admin username | (none) |
//! | `MONGODB_ROOT_PASSWORD` | Admin password | (none) |
//! | `MONGODB_ROOT_DATABASE` | Admin database | `admin` |
//! | `MONGODB_SHARDED` | Enable sharding | `false` |

pub use mongodb::*;

mod config;
mod db;

/// MongoDB configuration.
///
/// Loads configuration from environment variables with support for prefixes.
/// See module-level documentation for available environment variables.
pub use crate::config::Config as DbConfig;
pub use crate::db::{insert_always_opts, parse_vec, DB};
