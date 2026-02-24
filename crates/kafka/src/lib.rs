#![deny(missing_docs)]

//! Kafka producer and topic management utilities.
//!
//! This crate provides utilities for producing events to Kafka topics and
//! managing Kafka topics and consumer groups.
//!
//! ## Features
//!
//! - **Event Production**: Produce structured events to Kafka topics
//! - **Topic Management**: Create, delete, and ensure topics exist
//! - **Consumer Group Management**: List and clean up consumer groups
//! - **Configuration**: Environment-based configuration
//!
//! ## Usage
//!
//! \```ignore
//! use qm_kafka::{Producer, Config, EventNs, EventType};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let producer = Producer::new()?;
//!     producer.create_event(
//!         &EventNs::User,
//!         "user",
//!         "created",
//!         serde_json::json!({"id": "123", "name": "John"})
//!     ).await?;
//!     Ok(())
//! }
//! \```
//!
//! ## Environment Variables
//!
//! | Variable | Description | Default |
//! |----------|-------------|---------|
//! | `KAFKA_HOST` | Kafka broker host | `127.0.0.1` |
//! | `KAFKA_PORT` | Kafka broker port | `9092` |
//! | `KAFKA_ADDRESS` | Full broker address | `<host>:<port>` |
//! | `KAFKA_TOPIC_MUTATION_EVENTS` | Mutation events topic | `qm_mutation_events` |
//! | `KAFKA_CONSUMER_GROUP_MUTATION_EVENTS_PREFIX` | Consumer group prefix | `qm_consumer_group` |

/// Configuration module.
pub mod config;
/// Producer module for creating Kafka events.
pub mod producer;
/// Topics module for managing Kafka topics.
pub mod topics;
