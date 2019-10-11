#!/bin/bash
set -e

source "../lib.sh"

# This directory is created by docker as root
sudo rm -rf /tmp/bitcoin

# Clean up generated files from previous run
rm -f "./block1.hex" "./block2.hex" "./block3.hex" "./block4.hex" "./block4b_stale.hex" "./block5_with_transaction.hex" "./transaction.hex" "./address"

docker_run

generate_101_blocks

generate_block "./block1.hex"
generate_block "./block2.hex"
generate_block "./block3.hex"
generate_block "./block4.hex"

docker_stop

sudo cp -r /tmp/bitcoin /tmp/bitcoin-101

docker_start

create_transaction "./address" "./transaction.hex"

generate_block "./block5_with_transaction.hex"

docker_stop

sudo rm -rf /tmp/bitcoin
sudo mv /tmp/bitcoin-101 /tmp/bitcoin

docker_start

generate_block "./block4b_stale.hex"

docker_stop
docker_rm

exit 0
