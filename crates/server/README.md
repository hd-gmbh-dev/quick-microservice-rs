<div align="center">

# Quick Microservices Server - `qm-server`

<samp>helper to configure a simple HTTP server</samp>

---

[GitHub repository](https://github.com/hd-gmbh-dev/quick-microservice-rs/tree/main/crates/server)
⏺
[Cargo package](https://crates.io/crates/qm-server)
⏺
[Docs](https://docs.rs/qm-server/latest)

[![github.com - quick-microservice-rs](https://img.shields.io/github/v/release/hd-gmbh-dev/quick-microservice-rs?label=%20&logo=github)](https://github.com/hd-gmbh-dev/quick-microservice-rs/releases/latest)
[![crates.io - qm-server](https://img.shields.io/crates/v/qm-server?label=%20&logo=rust)](https://crates.io/crates/qm-server)\
[![github.com - workflow - build](https://img.shields.io/github/actions/workflow/status/hd-gmbh-dev/quick-microservice-rs/build.yaml)](https://github.com/hd-gmbh-dev/quick-microservice-rs/actions/workflows/build.yaml)

</div>

---

## Description

With this crate it is easy to get a server configuration with the most common server settings.

## Usage

```rust
let server_config = qm::server::ServerConfig::new()?;
```

The `Config` is populated with environment variables. By default, all variables with the prefix
`SERVER_` are considered.

The prefix can be changed by using a builder pattern.

```rust
let example_config = qm::server::ServerConfig::builder().with_prefix("EXAMPLE_").build()?;
```

## Variables and Defaults

These variables are available and are set with the following defaults.

| variable        | struct field | default              |
| --------------- | ------------ | -------------------- |
| SERVER_APP_NAME | app_name     | "quick-microservice" |
| SERVER_HOST     | host         | "127.0.0.1"          |
| SERVER_PORT     | port         | 3000                 |
|                 | address      | `{host}:{port}`      |
