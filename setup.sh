#!/bin/bash

export DATABASE_URL=postgres://keycloak:keycloak@localhost/keycloak
cargo sqlx migrate run --source crates/customer/migrations/keycloak --ignore-missing
cargo sqlx migrate run --source crates/customer/migrations/customer --ignore-missing

cargo sqlx prepare --workspace