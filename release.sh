#!/bin/bash

cargo set-version --workspace $1
cargo build

git add .
git commit -m "build: prepare release v$1"
git push

# git tag v$1
# git push -u origin v$1

cargo publish -p qm-utils-derive
echo 'wait 10 sec to publish qm-utils'
sleep 10
cargo publish -p qm-utils
echo 'wait 10 sec to publish qm-redis'
sleep 10
cargo publish -p qm-redis
echo 'wait 10 sec to publish qm-mongodb'
sleep 10
cargo publish -p qm-mongodb
echo 'wait 10 sec to publish qm-s3'
sleep 10
cargo publish -p qm-s3
echo 'wait 10 sec to publish qm-kafka'
sleep 10
cargo publish -p qm-kafka
echo 'wait 10 sec to publish qm-keycloak'
sleep 10
cargo publish -p qm-keycloak
echo 'wait 10 sec to publish  qmrole-build'
sleep 10
cargo publish -p qm-role-build
echo 'wait 10 sec to publish qm-role'
sleep 10
cargo publish -p qm-role
echo 'wait 10 sec to publish  qmentity-derive'
sleep 10
cargo publish -p qm-entity-derive
echo 'wait 10 sec to publish qm-entity'
sleep 10
cargo publish -p qm-entity
echo 'wait 10 sec to publish qm-customer'
sleep 10
cargo publish -p qm-customer
echo 'wait 10 sec to publish qm-server'
sleep 10
cargo publish -p qm-server
echo 'wait 10 sec topublish  qm'
sleep 10
cargo publish -p qm