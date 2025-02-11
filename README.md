<div align="center">

# Quick Microservices - `qm`

<samp>utilities to create quick microservices in Rust</samp>

---

[GitHub repository](https://github.com/hd-gmbh-dev/quick-microservice-rs)
⏺
[Cargo package](https://crates.io/crates/qm)
⏺
[Docs](https://docs.rs/qm/latest)

[![github.com - quick-microservice-rs](https://img.shields.io/github/v/release/hd-gmbh-dev/quick-microservice-rs?label=%20&logo=github)](https://github.com/hd-gmbh-dev/quick-microservice-rs/releases/latest)
[![crates.io - qm](https://img.shields.io/crates/v/qm?label=%20&logo=rust)](https://crates.io/crates/qm)\
[![github.com - workflow - build](https://img.shields.io/github/actions/workflow/status/hd-gmbh-dev/quick-microservice-rs/build.yaml)](https://github.com/hd-gmbh-dev/quick-microservice-rs/actions/workflows/build.yaml)

</div>

---

## Feature flags

### default

There are no default features. Every feature provided must be enabled explicitly.

### entity

Implements the opinionated concept of an `Entity`. A way to handle database models with predefined
constrains to describe permissions and ownership.
Uses the crates [`sea-orm`](https://crates.io/crates/sea-orm) and
[`sqlx`](https://crates.io/crates/sqlx).

Integrates with the features `keycloak`, `mongodb`, `redis`, `role` and `nats`.

### kafka

Provides an easy way to configure and set up a connection to a Kafka server.
Configuration can be done with environment variables. The default prefix is `KAFKA_`.
Uses the crate [`rdkafka`](https://crates.io/crates/rdkafka).

Also provides opinionated topics and producers for Kafka events.

### keycloak

Provides an easy way to configure and set up a connection to a Keycloak server.
Configuration can be done with environment variables. The default prefix is `KEYCLOAK_`.
Uses the crate [`keycloak`](https://crates.io/crates/keycloak).

Also provides an opinionated configuration template and some helper functions.

### mongodb

Provides an easy way to configure and set up a database connection for a MongoDB database.
Configuration can be done with environment variables. The default prefix is `MONGODB_`.
Uses the crate [`mongodb`](https://crates.io/crates/mongodb).

### nats

`todo!()`

### pg

Provides an easy way to configure and set up a database connection for a PostgreSQL database.
Configuration can be done with environment variables. The default prefix is `PG_`.
Uses the crate [`sqlx`](https://crates.io/crates/sqlx).

### redis

Provides an easy way to configure and set up a database connection for a Redis database.
Configuration can be done with environment variables. The default prefix is `REDIS_`.
Uses the crates [`redis`](https://crates.io/crates/redis) and
[`deadpool-redis`](https://crates.io/crates/deadpool-redis).

Also provides helper to handle worker queues and locking with Redis mechanisms.

### role

`todo!()`

### role-build

`todo!()`

### s3

`todo!()`

### server

Provides an easy way to get a server configuration.
Configuration can be done with environment variables. The default prefix is `SERVER_`.

Also provides a `graphql_handler`.

The handler requires a `qm_role::AuthContainer`, so the feature `role` must be activated.
Because the use of the handler is optional, the dependency is **not** automatically included.

### utils

`todo!()`
