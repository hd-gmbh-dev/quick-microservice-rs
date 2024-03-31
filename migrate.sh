#!/bin/bash

cargo sqlx migrate run --source crates/customer/migrations/keycloak --ignore-missing
cargo sqlx migrate run --source crates/customer/migrations/customer --ignore-missing