#!/bin/bash

set -e

export SQLX_OFFLINE=true

cargo set-version --workspace --bump patch
VERSION=`cargo pkgid | cut -d "@" -f2`
cargo build

git add .
git commit -m "build: prepare release v${VERSION}"
git push

git tag v${VERSION}
git push --tag
