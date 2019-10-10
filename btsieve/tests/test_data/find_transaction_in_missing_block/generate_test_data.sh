#!/bin/bash
set -e

source "../lib.sh"

# Clean up generated files from previous run
rm_file_if_exists "./block1.hex"
rm_file_if_exists "./block2.hex"
rm_file_if_exists "./block3.hex"

docker_run

generate_101_blocks

generate_block "./block1.hex"
generate_block "./block2.hex"

create_transaction "./address" "./transaction.hex"
generate_block "./block3.hex"

docker_stop
docker_rm

exit 0
