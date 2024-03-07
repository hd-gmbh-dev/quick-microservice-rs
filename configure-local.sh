#!/bin/bash

export $(grep -v '^#' .env | xargs)
export SERVER_PORT=3000
export RUST_LOG=debug
cargo run -p qm-example-cli -- configure all
