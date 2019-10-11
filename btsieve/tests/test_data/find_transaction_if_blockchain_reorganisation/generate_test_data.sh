#!/bin/bash
set -e

source "../lib.sh"

# This script was used to conveniently generate the test data for the test
# "find_transaction_if_blockchain_reorganisation" located in
# "../../bitcoin_missing_blocks.rs".
# If the test changes, you can modify this script accordingly and run it again
# to generate different test data.

# Strategy followed to cause a blockchain reorganisation:
#
# 1. Start a bitcoin node inside a docker container with a volume.
# 2. Generate some blocks.
# 3. Stop the container and save the state by copying it to the file system.
# 4. Restart the container and generate blocks.
# 5. Stop the container again and overwrite its current state with the state
#    saved in step 3.
# 6. Restart the container and generate blocks. These will be different to
#    the ones generated in step 4.

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

# Created transaction will be included in the next generated block
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
