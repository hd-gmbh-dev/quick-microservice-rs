<div align="center">

# Quick Microservices MongoDB - `qm-mongodb`

<samp>utilities to work with the MongoDB database</samp>

---

[GitHub repository](https://github.com/hd-gmbh-dev/quick-microservice-rs/tree/main/crates/mongodb)
⏺
[Cargo package](https://crates.io/crates/qm-mongodb)
⏺
[Docs](https://docs.rs/qm-mongodb/latest)

[![github.com - quick-microservice-rs](https://img.shields.io/github/v/release/hd-gmbh-dev/quick-microservice-rs?label=%20&logo=github)](https://github.com/hd-gmbh-dev/quick-microservice-rs/releases/latest)
[![crates.io - qm-mongodb](https://img.shields.io/crates/v/qm-mongodb?label=%20&logo=rust)](https://crates.io/crates/qm-mongodb)\
[![github.com - workflow - build](https://img.shields.io/github/actions/workflow/status/hd-gmbh-dev/quick-microservice-rs/build.yaml)](https://github.com/hd-gmbh-dev/quick-microservice-rs/actions/workflows/build.yaml)

</div>

---

## Description

With this crate it is easy to get a MongoDB configuration with the most common settings.
It also provides common helpers for collections and indexes and provide a convenient way to share
clients for database access.

## Usage

```rust
let mongodb = qm::mongodb::DB::new("example", &qm::mongodb::DbConfig::new()?).await?;
```

The `Config` is populated with environment variables. By default, all variables with the prefix
`MONGODB_` are considered.

The prefix can be changed by using a builder pattern.

```rust
let example_config = qm::mongodb::DbConfig::builder().with_prefix("EXAMPLE_").build()?;
```

## Variables and Defaults

These variables are available and are set with the following defaults.

| variable              | struct field  | default                                                                                                                                             |
| --------------------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| MONGODB_HOST          | host          | "127.0.0.1"                                                                                                                                         |
| MONGODB_PORT          | port          | 27017                                                                                                                                               |
| MONGODB_USERNAME      | username      |                                                                                                                                                     |
| MONGODB_PASSWORD      | password      |                                                                                                                                                     |
| MONGODB_DATABASE      | database      | "test"                                                                                                                                              |
| MONGODB_ROOT_USERNAME | root_username |                                                                                                                                                     |
| MONGODB_ROOT_PASSWORD | root_password |                                                                                                                                                     |
| MONGODB_ROOT_DATABASE | root_database | "admin"                                                                                                                                             |
| MONGODB_SHARDED       | sharded       | false                                                                                                                                               |
|                       | address       | With credentials: `mongodb://{username}:{password}@{host}:{port}/{database}`;<br> Without: `mongodb://{host}:{port}/{database}`                     |
|                       | root_address  | With credentials: `mongodb://{root_username}:{root_password}@{host}:{port}/{root_database}`;<br> Without: `mongodb://{host}:{port}/{root_database}` |
