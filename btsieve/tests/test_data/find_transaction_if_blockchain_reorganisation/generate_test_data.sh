#!/bin/bash
set -e

source "../lib.sh"

# Clean up generated files from previous run
rm -f "./block1.hex" "./block1b_stale.hex" "./block2_with_transaction.hex" "./transaction.hex" "./address"

temp_dir=$(mktemp -d)
temp_dir_101=$(mktemp -d)

docker_run $temp_dir

generate_101_blocks

generate_block "./block1.hex"

docker_stop

cp -r $temp_dir $temp_dir_101

docker_start

create_transaction "./address" "./transaction.hex"
generate_block "./block2_with_transaction.hex"

docker_stop

rm -rf $temp_dir
mv $temp_dir_101 $temp_dir

docker_start

generate_block "./block1b_stale.hex"

docker_stop
docker_rm
rm -rf $temp_dir

exit 0
