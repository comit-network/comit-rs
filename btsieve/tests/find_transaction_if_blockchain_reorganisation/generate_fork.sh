set -e

rm -rf /tmp/bitcoin

container=$(docker run -d -v /tmp/bitcoin:/root/.bitcoin coblox/bitcoin-core -regtest)
sleep 2

docker exec ${container} bitcoin-cli -regtest generate 101 > /dev/null

query_genesis=$(docker exec ${container} bitcoin-cli -regtest getbestblockhash)
docker exec ${container} bitcoin-cli -regtest getblock ${query_genesis} 0 > ./query_genesis_block.hex

docker stop ${container} > /dev/null

cp -r /tmp/bitcoin /tmp/bitcoin-101

docker start ${container} > /dev/null
sleep 2

address=$(docker exec ${container} bitcoin-cli -regtest getnewaddress)
echo ${address} > ./address
txid=$(docker exec ${container} bitcoin-cli -regtest sendtoaddress ${address} 1)

docker exec ${container} bitcoin-cli -regtest getrawtransaction ${txid} 0 > ./transaction.hex
docker exec ${container} bitcoin-cli -regtest generate 1 > /dev/null
block_with_tx=$(docker exec ${container} bitcoin-cli -regtest getbestblockhash)
docker exec ${container} bitcoin-cli -regtest getblock ${block_with_tx} 0 > ./transaction_block.hex

prev_blockhash_of_block_with_tx=$(docker exec ${container} bitcoin-cli -regtest getblock ${block_with_tx} 1 | jq .previousblockhash)

docker stop ${container} > /dev/null

rm -rf /tmp/bitcoin
mv /tmp/bitcoin-101 /tmp/bitcoin

docker start ${container} > /dev/null
sleep 2

docker exec ${container} bitcoin-cli -regtest generate 1 > /dev/null
stale_block=$(docker exec ${container} bitcoin-cli -regtest getbestblockhash)
docker exec ${container} bitcoin-cli -regtest getblock ${stale_block} 0 > ./stale_block.hex

prev_blockhash_of_stale_block=$(docker exec ${container} bitcoin-cli -regtest getblock ${stale_block} 1 | jq .previousblockhash)

test "${prev_blockhash_of_block_with_tx}"="${prev_blockhash_of_stale_block}"
