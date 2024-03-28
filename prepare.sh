#!/bin/bash

cd crates/pg
cargo sqlx prepare
cd ../../
cd crates/customer
cargo sqlx prepare
cd ../../

# cargo set-version --workspace $1
# cargo build

# git add .
# git commit -m "build: prepare release v$1"
# git push

# git tag v$1
# git push -u origin v$1

# cargo publish -p qm-utils-derive --allow-dirty
# cargo publish -p qm-utils --allow-dirty
# cargo publish -p qm-role-build --allow-dirty
# cargo publish -p qm-role --allow-dirty
# cargo publish -p qm-redis --allow-dirty
# cargo publish -p qm-pg --allow-dirty
# cargo publish -p qm-mongodb --allow-dirty
# cargo publish -p qm-s3 --allow-dirty
# cargo publish -p qm-kafka --allow-dirty
# cargo publish -p qm-keycloak --allow-dirty
# cargo publish -p qm-entity-derive --allow-dirty
# cargo publish -p qm-entity --allow-dirty
# cargo publish -p qm-customer --allow-dirty
# cargo publish -p qm-server --allow-dirty
# cargo publish -p qm --allow-dirty