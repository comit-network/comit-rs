#!/bin/bash
set -e

source "../lib.sh"

# This directory is created by docker as root
sudo rm -rf /tmp/bitcoin

# Clean up generated files from previous run
rm_file_if_exists "./query_genesis_block.hex"
rm_file_if_exists "./stale_block.hex"
rm_file_if_exists "./transaction_block.hex"
rm_file_if_exists "./transaction.hex"
rm_file_if_exists "./address"

docker_run

generate_101_blocks

generate_block "./query_genesis_block.hex"

docker_stop

sudo cp -r /tmp/bitcoin /tmp/bitcoin-101

docker_start

create_transaction "./address" "./transaction.hex"

generate_block "./transaction_block.hex"

docker_stop

sudo rm -rf /tmp/bitcoin
sudo mv /tmp/bitcoin-101 /tmp/bitcoin

docker_start

generate_block "./stale_block.hex"

docker_stop
docker_rm

exit 0
