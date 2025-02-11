<div align="center">

# Quick Microservices PostgreSQL - `qm-pg`

<samp>utilities to work with the PostgreSQL database</samp>

---

[GitHub repository](https://github.com/hd-gmbh-dev/quick-microservice-rs/tree/main/crates/pg)
⏺
[Cargo package](https://crates.io/crates/qm-pg)
⏺
[Docs](https://docs.rs/qm-pg/latest)

[![github.com - quick-microservice-rs](https://img.shields.io/github/v/release/hd-gmbh-dev/quick-microservice-rs?label=%20&logo=github)](https://github.com/hd-gmbh-dev/quick-microservice-rs/releases/latest)
[![crates.io - qm-pg](https://img.shields.io/crates/v/qm-pg?label=%20&logo=rust)](https://crates.io/crates/qm-pg)\
[![github.com - workflow - build](https://img.shields.io/github/actions/workflow/status/hd-gmbh-dev/quick-microservice-rs/build.yaml)](https://github.com/hd-gmbh-dev/quick-microservice-rs/actions/workflows/build.yaml)

</div>

---

## Description

With this crate it is easy to get a PostgreSQL configuration with the most common settings.

## Usage

```rust
let pgdb = qm::pg::DB::new("example", &qm::pg::DbConfig::new()?).await?;
```

The `Config` is populated with environment variables. By default, all variables with the prefix
`PG_` are considered.

The prefix can be changed by using a builder pattern.

```rust
let example_config = qm::pg::DbConfig::builder().with_prefix("EXAMPLE_").build()?;
```

## Variables and Defaults

These variables are available and are set with the following defaults.

| variable           | struct field    | default                                                                                                                                                                                                                                          |
| ------------------ | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| PG_HOST            | host            | "127.0.0.1"                                                                                                                                                                                                                                      |
| PG_PORT            | port            | 27017                                                                                                                                                                                                                                            |
| PG_MAX_CONNECTIONS | max_connections | 32                                                                                                                                                                                                                                               |
| PG_MIN_CONNECTIONS | min_connections | 0                                                                                                                                                                                                                                                |
| PG_ACQUIRE_TIMEOUT | acquire_timeout | 30                                                                                                                                                                                                                                               |
| PG_IDLE_TIMEOUT    | idle_timeout    | `10 * 60`                                                                                                                                                                                                                                        |
| PG_MAX_LIFETIME    | max_lifetime    | `30 * 60`                                                                                                                                                                                                                                        |
| PG_USERNAME        | username        |                                                                                                                                                                                                                                                  |
| PG_PASSWORD        | password        |                                                                                                                                                                                                                                                  |
| PG_DATABASE        | database        |                                                                                                                                                                                                                                                  |
| PG_ROOT_USERNAME   | root_username   |                                                                                                                                                                                                                                                  |
| PG_ROOT_PASSWORD   | root_password   |                                                                                                                                                                                                                                                  |
| PG_ROOT_DATABASE   | root_database   |                                                                                                                                                                                                                                                  |
| PG_SHARDED         | sharded         | false                                                                                                                                                                                                                                            |
|                    | address         | With credentials: `postgresql://{username}:{password}@{host}:{port}/`;<br> With username: `postgresql://{username}@{host}:{port}/`;<br> Without: `postgresql://{host}:{port}/`;<br> If provided, `database` will be appended                     |
|                    | root_address    | With credentials: `postgresql://{root_username}:{root_password}@{host}:{port}/`;<br> With username: `postgresql://{root_username}@{host}:{port}/`;<br> Without: `postgresql://{host}:{port}/`;<br> If provided, `root_database` will be appended |
