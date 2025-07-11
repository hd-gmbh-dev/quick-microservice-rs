<div align="center">

# Quick Microservices Entity - `qm-entity`

<samp>utilities to work with the concept of an `Entity`</samp>

---

[GitHub repository](https://github.com/hd-gmbh-dev/quick-microservice-rs/tree/main/crates/entity)
⏺
[Cargo package](https://crates.io/crates/qm-entity)
⏺
[Docs](https://docs.rs/qm-entity/latest)

[![github.com - quick-microservice-rs](https://img.shields.io/github/v/release/hd-gmbh-dev/quick-microservice-rs?label=%20&logo=github)](https://github.com/hd-gmbh-dev/quick-microservice-rs/releases/latest)
[![crates.io - qm-entity](https://img.shields.io/crates/v/qm-entity?label=%20&logo=rust)](https://crates.io/crates/qm-entity)\
[![github.com - workflow - build](https://img.shields.io/github/actions/workflow/status/hd-gmbh-dev/quick-microservice-rs/build.yaml)](https://github.com/hd-gmbh-dev/quick-microservice-rs/actions/workflows/build.yaml)

</div>

---

## Description

The opinionated concept of `Entity` introduces a way to work with database objects, define ownership
and have access control with different levels.

## Features

- `serde-str`: add `serde` support based on `Display` and `FromStr` traits for `InfraContext` and `InfraContextId` types.
