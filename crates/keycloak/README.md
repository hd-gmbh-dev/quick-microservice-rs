<div align="center">

# Quick Microservices Keycloak - `qm-keycloak`

<samp>utilities to work with a Keycloak server and managing authentication and authorization</samp>

---

[GitHub repository](https://github.com/hd-gmbh-dev/quick-microservice-rs/tree/main/crates/keycloak)
⏺
[Cargo package](https://crates.io/crates/qm-keycloak)
⏺
[Docs](https://docs.rs/qm-keycloak/latest)

[![github.com - quick-microservice-rs](https://img.shields.io/github/v/release/hd-gmbh-dev/quick-microservice-rs?label=%20&logo=github)](https://github.com/hd-gmbh-dev/quick-microservice-rs/releases/latest)
[![crates.io - qm-keycloak](https://img.shields.io/crates/v/qm-keycloak?label=%20&logo=rust)](https://crates.io/crates/qm-keycloak)\
[![github.com - workflow - build](https://img.shields.io/github/actions/workflow/status/hd-gmbh-dev/quick-microservice-rs/build.yaml)](https://github.com/hd-gmbh-dev/quick-microservice-rs/actions/workflows/build.yaml)

</div>

---

## Description

With this crate it is easy to get a Keycloak configuration with the most common settings.
It also provides token management and session handling as well as Keycloak configuration validation.

## Usage

```rust
let keycloak_config = qm::keycloak::KeycloakConfig::new()?;
```

The `Config` is populated with environment variables. By default, all variables with the prefix
`KEYCLOAK_` are considered.

The prefix can be changed by using a builder pattern.

```rust
let example_config = qm::keycloak::KeycloakConfig::builder().with_prefix("EXAMPLE_").build()?;
```

## Variables and Defaults

These variables are available and are set with the following defaults.

| variable                             | struct field                | default                         |
| ------------------------------------ | --------------------------- | ------------------------------- |
| KEYCLOAK_REALM                       | realm                       | "rmp"                           |
| KEYCLOAK_USERNAME                    | username                    | "admin"                         |
| KEYCLOAK_PASSWORD                    | password                    | "admin"                         |
| KEYCLOAK_THEME                       | theme                       | "qm"                            |
| KEYCLOAK_EMAIL_THEME                 | email_theme                 | "qm"                            |
| KEYCLOAK_REALM_ADMIN_EMAIL           | realm_admin_email           | "admin@test.local"              |
| KEYCLOAK_REALM_ADMIN_USERNAME        | realm_admin_username        | "admin"                         |
| KEYCLOAK_REALM_ADMIN_PASSWORD        | realm_admin_password        | "Admin123!"                     |
| KEYCLOAK_PORT                        | port                        | 42210                           |
| KEYCLOAK_HOST                        | host                        | "127.0.0.1"                     |
| KEYCLOAK_ADDRESS                     | address                     | `http://{host}:{port}/`         |
| KEYCLOAK_PUBLIC_URL                  | public_url                  | "http://127.0.0.1:80"           |
| KEYCLOAK_SMTP_REPLY_TO_DISPLAY_NAME  | smtp_reply_to_display_name  |                                 |
| KEYCLOAK_SMTP_STARTTLS               | smtp_starttls               | false                           |
| KEYCLOAK_SMTP_PORT                   | smtp_port                   | 1025                            |
| KEYCLOAK_SMTP_HOST                   | smtp_host                   | "smtp"                          |
| KEYCLOAK_SMTP_REPLY_TO               | smtp_reply_to               |                                 |
| KEYCLOAK_SMTP_FROM                   | smtp_from                   | "noreply@test.local"            |
| KEYCLOAK_SMTP_FROM_DISPLAY_NAME      | smtp_from_display_name      |                                 |
| KEYCLOAK_SMTP_SSL                    | smtp_ssl                    | false                           |
| KEYCLOAK_BROWSER_FLOW                | browser_flow                | "browser"                       |
| KEYCLOAK_AUTHENTICATOR_EMAIL_SUBJECT | authenticator_email_subject | "Temporary Authentication Code" |
