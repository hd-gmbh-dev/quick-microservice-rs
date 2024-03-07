#!/bin/bash

cargo set-version --workspace $1
cargo build

git add .
git commit -m "build: prepare release v$1"
git push
# git tag v$1
# git push -u origin v$1

pnpm publish --recursive --access public --no-git-checks

cargo publish -p qm-mongodb
cargo publish -p qm-keycloak
cargo publish -p qm-redis
cargo publish -p qm-kafka
cargo publish -p qm-s3
cargo publish -p qm-role
cargo publish -p qm-role-build
cargo publish -p qm-entity
cargo publish -p qm-entity-derive
cargo publish -p qm-server
cargo publish -p qm-utils
cargo publish -p qm-utils-derive
cargo publish -p qm
