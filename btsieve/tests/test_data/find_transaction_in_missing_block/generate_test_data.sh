#!/bin/bash
set -e

source "../lib.sh"

# Clean up generated files from previous run
rm -f "./block1.hex" "./block2.hex" "./block3.hex"

docker_run

generate_101_blocks

generate_block "./block1.hex"

create_transaction "./address" "./transaction.hex"
generate_block "./block2.hex"

generate_block "./block3.hex"

docker_stop
docker_rm

exit 0
