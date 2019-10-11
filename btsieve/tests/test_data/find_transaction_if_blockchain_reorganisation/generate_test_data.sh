#!/bin/bash
set -e

source "../lib.sh"

# This directory is created by docker as root
sudo rm -rf /tmp/bitcoin

# Clean up generated files from previous run
rm -f "./block1.hex" "./block1b_stale.hex" "./block2_with_transaction.hex" "./transaction.hex" "./address"

docker_run

generate_101_blocks

generate_block "./block1.hex"

docker_stop

sudo cp -r /tmp/bitcoin /tmp/bitcoin-101

docker_start

create_transaction "./address" "./transaction.hex"

generate_block "./block2_with_transaction.hex"

docker_stop

sudo rm -rf /tmp/bitcoin
sudo mv /tmp/bitcoin-101 /tmp/bitcoin

docker_start

generate_block "./block1b_stale.hex"

docker_stop
docker_rm

exit 0
