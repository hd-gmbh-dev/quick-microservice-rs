<div align="center">

# Quick Microservices Kafka - `qm-kafka`

<samp>utilities to work with Kafka events</samp>

---

[GitHub repository](https://github.com/hd-gmbh-dev/quick-microservice-rs/tree/main/crates/kafka)
⏺
[Cargo package](https://crates.io/crates/qm-kafka)
⏺
[Docs](https://docs.rs/qm-kafka/latest)

[![github.com - quick-microservice-rs](https://img.shields.io/github/v/release/hd-gmbh-dev/quick-microservice-rs?label=%20&logo=github)](https://github.com/hd-gmbh-dev/quick-microservice-rs/releases/latest)
[![crates.io - qm-kafka](https://img.shields.io/crates/v/qm-kafka?label=%20&logo=rust)](https://crates.io/crates/qm-kafka)\
[![github.com - workflow - build](https://img.shields.io/github/actions/workflow/status/hd-gmbh-dev/quick-microservice-rs/build.yaml)](https://github.com/hd-gmbh-dev/quick-microservice-rs/actions/workflows/build.yaml)

</div>

---

## Description

With this crate it is easy to get a Kafka configuration with the most common settings.
It also provides common helpers for topics and defines event and event namespaces commonly used with
the other features provided by this crate.

## Usage

```rust
let kafka_config = qm::kafka::config::Config::new()?;
```

The `Config` is populated with environment variables. By default, all variables with the prefix
`KAFKA_` are considered.

The prefix can be changed by using a builder pattern.

```rust
let example_config = qm::kafka::config::Config::builder().with_prefix("EXAMPLE_").build()?;
```

## Variables and Defaults

These variables are available and are set with the following defaults.

| variable                                    | struct field                          | default              |
| ------------------------------------------- | ------------------------------------- | -------------------- |
| KAFKA_HOST                                  | host                                  | "127.0.0.1"          |
| KAFKA_PORT                                  | port                                  | 9092                 |
|                                             | address                               | `{host}:{port}`      |
| KAFKA_TOPIC_MUTATION_EVENTS                 | topic_mutation_events                 | "qm_mutation_events" |
| KAFKA_CONSUMER_GROUP_MUTATION_EVENTS_PREFIX | consumer_group_mutation_events_prefix | "qm_consumer_group"  |
