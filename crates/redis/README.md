<div align="center">

# Quick Microservices Redis - `qm-redis`

<samp>utilities to work with the Redis database</samp>

---

[GitHub repository](https://github.com/hd-gmbh-dev/quick-microservice-rs/tree/main/crates/redis)
⏺
[Cargo package](https://crates.io/crates/qm-redis)
⏺
[Docs](https://docs.rs/qm-redis/latest)

[![github.com - quick-microservice-rs](https://img.shields.io/github/v/release/hd-gmbh-dev/quick-microservice-rs?label=%20&logo=github)](https://github.com/hd-gmbh-dev/quick-microservice-rs/releases/latest)
[![crates.io - qm-redis](https://img.shields.io/crates/v/qm-redis?label=%20&logo=rust)](https://crates.io/crates/qm-redis)\
[![github.com - workflow - build](https://img.shields.io/github/actions/workflow/status/hd-gmbh-dev/quick-microservice-rs/build.yaml)](https://github.com/hd-gmbh-dev/quick-microservice-rs/actions/workflows/build.yaml)

</div>

---

## Description

With this crate it is easy to get a Redis configuration with the most common settings.
It also provides common helpers to handle locks and use workers with queues.

## Usage

```rust
let redis_config = qm::redis::RedisConfig::new()?;
```

The `Config` is populated with environment variables. By default, all variables with the prefix
`REDIS_` are considered.

The prefix can be changed by using a builder pattern.

```rust
let example_config = qm::redis::RedisConfig::builder().with_prefix("REDIS_").build()?;
```

## Variables and Defaults

These variables are available and are set with the following defaults.

| variable     | struct field | default                  |
| ------------ | ------------ | ------------------------ |
| MONGODB_HOST | host         | "127.0.0.1"              |
| MONGODB_PORT | port         | 6379                     |
|              | address      | `redis://{host}:{port}/` |
