#!/bin/bash
set -ev;

END(){
    if test "${docker_ids}"; then
        echo "KILLING docker containers";
        for id in ${docker_ids}
            do docker rm -f ${id};
        done
    fi
}

trap 'END' EXIT;

#### Env variable to run all services

export RUST_TEST_THREADS=1;
export BITCOIN_RPC_URL="http://localhost:18443"
export BITCOIN_RPC_USERNAME="bitcoin"
export BITCOIN_RPC_PASSWORD="54pLR_f7-G6is32LP-7nbhzZSbJs_2zSATtZV_r05yg="
export ETHEREUM_NODE_ENDPOINT="http://localhost:8545"
export ETHEREUM_NETWORK_ID=42
export ETHEREUM_PRIVATE_KEY=3f92cbc79aa7e29c7c5f3525749fd7d90aa21938de096f1b78710befe6d8ef59

export TREASURY_SERVICE_URL=http://localhost:8020
export EXCHANGE_SERVICE_URL=http://localhost:8010
export TRADING_SERVICE_URL=http://localhost:8000

#### Start all services

docker-compose up -d

sleep_for=10
echo "sleeping for ${sleep_for}s while all start";
sleep $sleep_for;

docker_ids=$(docker-compose ps -q)

########

#### Env variables to run the end-to-end test

export ETH_HTLC_ADDRESS="0xa00f2cac7bad9285ecfd59e8860f5b2d8622e099"

cli="./target/debug/trading_client"
curl="curl -s"

symbol_param="--symbol=ETH-BTC"
eth_amount=100
client_refund_address="bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap"
client_success_address="0x03744e31a6b9e6c6f604ff5d8ce1caef1c7bb58c"
# For contract calling
client_sender_address="0x96984c3e77f38ed01d1c3d98f4bd7c8b11d51d7e"

#### Start End to end test

## Offer
cmd="$cli offer ${symbol_param} --amount=${eth_amount} buy"
echo "--> ${cmd}"
output=$($cmd)
echo "--> $output"

## get UID
uid=$(echo "$output" | head -n1 | grep "Trade id" |sed 's/^.* Trade id: \(.*\) .*$/\1/')
echo "--> Trade id: ${uid}"

## Order
cmd="$cli order ${symbol_param} --uid=${uid} --refund-address=${client_refund_address} --success-address=${client_success_address}"
echo "--> ${cmd}"
output=$($cmd)
echo "--> $output"

## Get BTC HTLC address
btc_htlc_address=$(echo "$output" | grep "^bcrt1")
echo "--> BTC HTLC: ${btc_htlc_address}"

## Get BTC amount
btc_amount=$(echo "$output" | grep "Please send" | sed -E 's/^Please send ([0-9]+) BTC.*$/\1/')
echo "--> BTC amount: ${btc_amount}"

## Generate funds and activate segwit
$curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
"{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"generate\", \"params\": [ 432 ]}" -H 'content-type: text/plain;' $BITCOIN_RPC_URL > /dev/null

## Bitcoin RPC call
output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
"{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"sendtoaddress\", \"params\": [ \"${btc_htlc_address}\", ${btc_amount}]}" -H 'content-type: text/plain;' $BITCOIN_RPC_URL)
echo "--> ${output} <--"

## Get funding tx id
htlc_funding_tx=$(echo $output | sed -E 's/^..result.:.([a-z0-9]+).,.error.*$/\1/')
echo "--> $htlc_funding_tx <--"

## Generate blocks to confirm the transaction
$curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
"{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"generate\", \"params\": [ 6 ]}" -H 'content-type: text/plain;' $BITCOIN_RPC_URL

## Get raw funding tx
output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
"{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"getrawtransaction\", \"params\": [ \"${htlc_funding_tx}\" ]}" \
-H 'content-type: text/plain;' $BITCOIN_RPC_URL)
raw_funding_tx=$(echo $output | sed -E 's/^..result.:.([a-z0-9]+).,.error.*$/\1/')
echo "--> $raw_funding_tx <--"

## Decode raw funding tx
output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
"{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"decoderawtransaction\", \"params\": [ \"${raw_funding_tx}\" ]}"\
 -H 'content-type: text/plain;' $BITCOIN_RPC_URL)
echo $output

## Getting the vout which pays the BTC HTLC
htlc_funding_tx_vout=$(echo $output | jq .result.vout | jq ".[] | select(.scriptPubKey.addresses[0] == \"${btc_htlc_address}\")"|jq .n)
echo "--> $htlc_funding_tx_vout <--"

## Tell exchange service that BTC HTLC was funded
$curl --data-binary "{\"transaction_id\": \"${htlc_funding_tx}\",\"vout\": ${htlc_funding_tx_vout}}" \
-H 'Content-Type: application/json' ${EXCHANGE_SERVICE_URL}/trades/ETH-BTC/${uid}/buy-order-htlc-funded && echo "--> Exchange-service poked successfully <--"

## Tell trading service that ETH deployed
$curl --data-binary "{\"contract_address\": \"${ETH_HTLC_ADDRESS}\"}" \
-H 'Content-Type: application/json' ${TRADING_SERVICE_URL}/trades/ETH-BTC/${uid}/buy-order-contract-deployed && echo "--> Trading-service poked successfully <--"

## Get redeem details
output=$($cli redeem ${symbol_param} --uid=${uid})
secret=$(echo "$output" | tail -n1 |sed -E 's/^ethereum:.*bytes32=(.+)$/\1/')
echo "--> Secret: $secret <--"

## Save previous balance
output=$($curl --data-binary "{\
  \"jsonrpc\":\"2.0\",\
  \"method\":\"eth_getBalance\",\
  \"params\":[\
    \"${client_success_address}\",\
    \"latest\"\
  ],\
  \"id\":1\
}" \
-H 'Content-Type: application/json' ${ETHEREUM_NODE_ENDPOINT})
old_balance=$(echo $output|jq -r .result)
echo "--> Old ETH balance: $old_balance <--"
old_balance=$((16#${old_balance#0x}))
echo "--> Previous ETH balance of customer: $old_balance <--"

## Redeem the ETH
$curl --data-binary "{\
  \"jsonrpc\":\"2.0\",\
  \"method\":\"eth_sendTransaction\",\
  \"params\":[\
    {\
      \"from\": \"${client_sender_address}\",\
      \"to\": \"${ETH_HTLC_ADDRESS}\",\
      \"gas\": \"0x100000\",\
      \"gasPrice\": \"0x01\",\
      \"value\": \"0x0\",\
      \"data\": \"0x${secret}\"\
    }\
  ],\
  \"id\":1\
}" \
-H 'Content-Type: application/json' ${ETHEREUM_NODE_ENDPOINT} && echo -e "\n--> ETH redeemed successfully <--"

## Save new balance
output=$($curl --data-binary "{\
  \"jsonrpc\":\"2.0\",\
  \"method\":\"eth_getBalance\",\
  \"params\":[\
    \"${client_success_address}\",\
    \"latest\"\
  ],\
  \"id\":1\
}" \
-H 'Content-Type: application/json' ${ETHEREUM_NODE_ENDPOINT})
new_balance=$(echo $output|jq -r .result)
echo "--> New ETH balance: $new_balance <--"
new_balance=$((16#${new_balance#0x}))
echo "--> New ETH balance of customer: $new_balance <--"
echo $old_balance
echo $new_balance
if [ ${old_balance} -lt ${new_balance} ]
then
    echo "## ETH WAS redeemed ##"
else
    echo "## ETH was NOT redeemed ##"
    exit 1
fi

# Watch the pw2sh address
$curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
"{\
    \"jsonrpc\": \"1.0\",\
    \"id\":\"curltest\",\
    \"method\": \"importaddress\",\
    \"params\":\
        [\
            \"bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap\",\
            \"htlc\"\
        ]\
}" \
-H 'content-type: text/plain;' $BITCOIN_RPC_URL > /dev/null && echo "PW2SH address is now watched"

# Save BTC unspent outputs before redeem
output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
"{\
  \"jsonrpc\":\"1.0\",\
  \"id\":\"curltest\",\
  \"method\":\"listunspent\",\
  \"params\":\
  [\
    0,\
    9999999,\
    [\
      \"bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap\"\
    ]\
  ],\
  \"id\":1\
}" \
-H 'content-type: text/plain;' $BITCOIN_RPC_URL)
old_unspent=$(echo $output |jq .result)
old_unspent_num=$(echo $output | jq '.result | length')
echo -e "--> Total Unspent: $old_unspent_num <--"

# Poke exchange service to redeem BTC
$curl --data-binary "{\"secret\": \"${secret}\"}" \
-H 'Content-Type: application/json' ${EXCHANGE_SERVICE_URL}/trades/ETH-BTC/${uid}/buy-order-secret-revealed \
&& echo "--> Exchange-service poked successfully to redeem BTC <--"

## Generate blocks to confirm the transaction
$curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
"{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"generate\", \"params\": [ 6 ]}" -H 'content-type: text/plain;' $BITCOIN_RPC_URL 2> /dev/null

# Check BTC unspent outputs after redeem
output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
"{\
  \"jsonrpc\":\"1.0\",\
  \"id\":\"curltest\",\
  \"method\":\"listunspent\",\
  \"params\":\
  [\
    0,\
    9999999,\
    [\
      \"bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap\"\
    ]\
  ],\
  \"id\":1\
}" \
-H 'content-type: text/plain;' $BITCOIN_RPC_URL)
new_unspent=$(echo $output |jq .result)
new_unspent_num=$(echo $output | jq '.result | length')
echo -e "--> Total Unspent: $new_unspent_num <--"

if [ ${old_unspent_num} -lt ${new_unspent_num} ]
then
    echo "## BTC WAS redeemed ##"
else
    echo "## BTC was NOT redeemed ##"
    exit 1
fi