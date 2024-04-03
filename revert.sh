#!/bin/bash

cargo sqlx migrate revert --source crates/customer/migrations/keycloak --ignore-missing --target-version 0
cargo sqlx migrate revert --source crates/customer/migrations/customer --ignore-missing --target-version 0