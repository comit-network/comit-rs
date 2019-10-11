#!/bin/bash
set -e

source "../lib.sh"

# This script was used to conveniently generate the test data for the test
# "find_transaction_in_missing_block_with_big_gap" located in "../../bitcoin_missing_blocks.rs".
# If the test changes, you can modify this script accordingly and run it again
# to generate different test data.

# Clean up generated files from previous run
rm -f "./block1.hex" "./block2_with_transaction.hex" "./block3.hex" "./block4.hex" "./block5.hex" "./block6.hex" "./block7.hex" "./block8.hex"

docker_run

generate_101_blocks

generate_block "./block1.hex"

# Created transaction will be included in the next generated block
create_transaction "./address" "./transaction.hex"
generate_block "./block2_with_transaction.hex"

generate_block "./block3.hex"
generate_block "./block4.hex"
generate_block "./block5.hex"
generate_block "./block6.hex"
generate_block "./block7.hex"
generate_block "./block8.hex"

docker_stop
docker_rm

exit 0
