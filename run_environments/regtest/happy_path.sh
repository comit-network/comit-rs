#!/bin/bash
set -e;


OUTPUT=/dev/null

if [ "$1" = "--interactive" ]
then
    IS_INTERACTIVE=1
fi

if test "$DEBUG"; then
    OUTPUT=/dev/stdout
fi

## Define functions from here

function setup() {
    ########
    #### Env variables to run the end-to-end test

    export ETH_HTLC_ADDRESS="0xa00f2cac7bad9285ecfd59e8860f5b2d8622e099"

    cli="$PROJECT_ROOT/target/debug/trading_client"
    curl="curl -s"

    symbol_param="--symbol=ETH-BTC"
    eth_amount=10

    # Watch the btc exchange redeem address
    debug "Adding BTC_EXCHANGE_REDEEM_ADDRESS to wallet";
    $curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
    "{\
        \"jsonrpc\": \"1.0\",\
        \"id\":\"curltest\",\
        \"method\": \"importaddress\",\
        \"params\":\
            [\
                \"${BTC_EXCHANGE_REDEEM_ADDRESS}\",\
                \"htlc\"\
            ]\
    }" \
    -H 'content-type: text/plain;' $BITCOIN_RPC_URL > /dev/null

    echo "System is ready!"
}

function debug() {
    printf '%s\n' "$*" > $OUTPUT;
}

function step() {
    { test "$IS_INTERACTIVE" && read; } || true;
}

function print_green() {
    printf '\e[32m%s\e[0m\n' "$1"
}

function print_blue() {
    printf '\e[34m%s\e[0m\n' "$1"
}

function new_offer() {
    ## Offer
    cmd="$cli offer ${symbol_param} --amount=${eth_amount} buy"
    print_green "$cmd"
    output=$($cmd)
    echo "$output"

    ## get UID
    uid=$(echo "$output" | head -n1 | grep "Trade id" |sed 's/^.* Trade id: \(.*\) .*$/\1/')
    # echo "--> Trade id: ${uid}"
}

function new_order() {

    cmd="$cli order ${symbol_param} --uid=${uid} --refund-address=${client_refund_address} --success-address=${client_success_address}"
    print_green "$cmd"
    output=$($cmd)
    echo "$output"

    ## Get BTC HTLC address
    btc_htlc_address=$(echo "$output" | grep "^bcrt1")
    # echo "--> BTC HTLC: ${btc_htlc_address}"

    ## Get BTC amount
    btc_amount=$(echo "$output" | grep "Please send" | sed -E 's/^Please send ([0-9.]+) BTC.*$/\1/')
    # echo "--> BTC amount: ${btc_amount}"
}

function generate_blocks() {

    ## Generate blocks to confirm the transaction
    $curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
    "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"generate\", \"params\": [ 6 ]}" -H 'content-type: text/plain;' $BITCOIN_RPC_URL

}
function fund_htlc() {

    ## Bitcoin RPC call
    output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
    "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"sendtoaddress\", \"params\": [ \"${btc_htlc_address}\", ${btc_amount}]}" -H 'content-type: text/plain;' $BITCOIN_RPC_URL)

    ## Get funding tx id
    htlc_funding_tx=$(echo $output | sed -E 's/^..result.:.([a-z0-9]+).,.error.*$/\1/')

    generate_blocks 2>&1 /dev/null

    ## Get raw funding tx
    output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
    "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"getrawtransaction\", \"params\": [ \"${htlc_funding_tx}\" ]}" \
    -H 'content-type: text/plain;' $BITCOIN_RPC_URL)

    raw_funding_tx=$(echo $output | sed -E 's/^..result.:.([a-z0-9]+).,.error.*$/\1/')

    ## Decode raw funding tx
    output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
    "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"decoderawtransaction\", \"params\": [ \"${raw_funding_tx}\" ]}"\
     -H 'content-type: text/plain;' $BITCOIN_RPC_URL)

    ## Getting the vout which pays the BTC HTLC
    htlc_funding_tx_vout=$(echo $output | jq .result.vout | jq ".[] | select(.scriptPubKey.addresses[0] == \"${btc_htlc_address}\")"|jq .n)

    echo "HTLC successfully funded - BTC payment was made."
}

function notify_exchange_service_btc_htlc_funded() {

    result=$($curl --data-binary "{\"transaction_id\": \"${htlc_funding_tx}\",\"vout\": ${htlc_funding_tx_vout}}" -H 'Content-Type: application/json' ${EXCHANGE_SERVICE_URL}/trades/ETH-BTC/${uid}/buy-order-htlc-funded )

    echo $result > $OUTPUT

    print_blue "Notified exchange about trader's BTC payment (Trader funded BTC HTLC)."
}

function notify_trading_service_eth_htlc_funded() {

    result=$($curl --data-binary "{\"contract_address\": \"${ETH_HTLC_ADDRESS}\"}" -H 'Content-Type: application/json' ${TRADING_SERVICE_URL}/trades/ETH-BTC/${uid}/buy-order-contract-deployed)

    echo $result > $OUTPUT

    print_blue "Notified trader about exchange's ETH payment (Exchange funded ETH HTLC)."
}

function notify_exchange_service_eth_redeemed() {
    $curl -v --data-binary "{\"secret\": \"${secret}\"}" -H 'Content-Type: application/json' ${EXCHANGE_SERVICE_URL}/trades/ETH-BTC/${uid}/buy-order-secret-revealed 2> $OUTPUT

    print_blue "Notified exchange about revealed secret (Trader redeemed ETH funds)."
}
function get_redeem_details() {

    cmd="$cli redeem ${symbol_param} --uid=${uid}"

    print_green "$cmd"

    output=$($cmd)

    secret=$(echo "$output" | tail -n1 |sed -E 's/^ethereum:.*bytes32=(.+)$/\1/')

    echo "Secret: $secret"
}

function get_eth_balance() {

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

    echo $output|jq -r .result
}

function redeem_eth() {
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
    -H 'Content-Type: application/json' ${ETHEREUM_NODE_ENDPOINT} > $OUTPUT
}

function list_unspent_transactions() {
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
          \"${BTC_EXCHANGE_REDEEM_ADDRESS}\"\
        ]\
      ],\
      \"id\":1\
    }" \
    -H 'content-type: text/plain;' $BITCOIN_RPC_URL)

    echo $output
}

function hex_to_dec() {
    perl -Mbigint -E 'say hex(shift)' $1
}

function is_greater_than() {
    perl -Mbigint -E 'exit !(((shift) - (shift)) > 0) ? 0 : 1' $1 $2
}

function wei_to_eth() {
    perl -Mbigint -E 'say ((shift) / 1_000_000_000_000_000_000)' $1
}

#### Start End to end test

setup;

step;

new_offer;

step;

new_order;

step;

fund_htlc;

step;

notify_exchange_service_btc_htlc_funded;

step;

notify_trading_service_eth_htlc_funded;

step;

get_redeem_details;

old_balance=$(get_eth_balance)
echo "Previous ETH balance in HEX: $old_balance" > $OUTPUT

old_balance=$(hex_to_dec $old_balance)
old_balance=$(wei_to_eth $old_balance)

echo "Previous ETH balance: $old_balance"

step;

redeem_eth;

new_balance=$(get_eth_balance)
echo "New ETH balance in HEX: $new_balance" > $OUTPUT
new_balance=$(hex_to_dec $new_balance)
new_balance=$(wei_to_eth $new_balance)
echo "New ETH balance:      $new_balance"

if [ ${old_balance} -lt ${new_balance} ]
then
    echo "## ETH WAS redeemed ##" > $OUTPUT
else
    echo "## ETH was NOT redeemed ##" > $OUTPUT
    exit 1
fi

step;

output=$(list_unspent_transactions)
old_unspent=$(echo $output |jq .result)
echo "BTC: Old Unspent: $old_unspent" > $OUTPUT
old_unspent_num=$(echo $output | jq '.result | length')
echo "BTC: Total UTXOs before redeem: $old_unspent_num"

step;

# Poke exchange service to redeem BTC
notify_exchange_service_eth_redeemed;

generate_blocks 2>&1 /dev/null

# Check BTC unspent outputs after redeem
output=$(list_unspent_transactions)

new_unspent=$(echo $output |jq .result)
echo "BTC: New Unspent: $new_unspent" > $OUTPUT
new_unspent_num=$(echo $output | jq '.result | length')
echo "BTC: Total UTXOs after redeem: $new_unspent_num"
echo "BTC: Amount: $(echo $new_unspent | jq '.[0].amount')"

if [ ${old_unspent_num} -lt ${new_unspent_num} ]
then
    echo "## BTC WAS redeemed ##" > $OUTPUT
else
    echo "## BTC was NOT redeemed ##" > $OUTPUT
    exit 1
fi
