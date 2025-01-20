#!/bin/bash

cd crates/pg
cargo sqlx prepare
cd ../../
