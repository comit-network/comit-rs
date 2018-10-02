#!/bin/bash
set -e;

if test "$DEBUG"; then
    exec 4>&1;
else
    exec 4>/dev/null;
fi

exec 3>&1;
## Define functions from here

function setup() {
    ########
    #### Env variables to run the end-to-end test

    export ETH_HTLC_ADDRESS="0xa00f2cac7bad9285ecfd59e8860f5b2d8622e099"
    export ALICE_COMIT_NODE_URL=$ALICE_COMIT_NODE_URL
    export BOB_COMIT_NODE_URL=$BOB_COMIT_NODE_URL

    cli="$PROJECT_ROOT/target/debug/comit_node_client"
    curl="curl -s"

    symbol_param="--symbol=ETH-BTC"
    eth_amount=10.0
    btc_amount=1.0

    # Watch the btc bob redeem address
    debug "Adding BTC_BOB_REDEEM_ADDRESS to wallet";
    $curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
    "{\
        \"jsonrpc\": \"1.0\",\
        \"id\":\"curltest\",\
        \"method\": \"importaddress\",\
        \"params\":\
            [\
                \"${BTC_BOB_REDEEM_ADDRESS}\",\
                \"htlc\"\
            ]\
    }" \
    -H 'content-type: text/plain;' $BITCOIN_RPC_URL > /dev/null

    info "System is ready!"
}

function debug() {
    printf '%s\n' "$*" >&4;
}

function info() {
    printf '%s\n' "$*" >&3;
}

function exec_cmd() {
    printf '\e[32m%s\e[0m\n' "$*" >&3;
    $@
}

function info_blue() {
    printf '\e[34m%s\e[0m\n' "$*" >&3;
}

function send_swap_request() {
    COMIT_NODE_URL=$ALICE_COMIT_NODE_URL exec_cmd "$cli swap $@";
}

function alice_wait_swap_status() {
    id=$1
    desired_status=$2;
    cmd="$cli swap status $id";
    while ! { COMIT_NODE_URL=$ALICE_COMIT_NODE_URL exec_cmd $cmd | awk "NR==1 && !/status: $desired_status/{ exit 1; } { print; }"; } do
        sleep 1;
    done
}

function generate_blocks() {
    ## Generate blocks to confirm the transaction
    debug $(
        $curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
          "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"generate\", \"params\": [ 6 ]}" -H 'content-type: text/plain;' $BITCOIN_RPC_URL
    )

}
function fund_htlc() {
    btc_htlc_address=$1;
    ## Bitcoin RPC call
    output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
    "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"sendtoaddress\", \"params\": [ \"${btc_htlc_address}\", ${btc_amount}]}" -H 'content-type: text/plain;' $BITCOIN_RPC_URL)

    ## Get funding tx id
    htlc_funding_tx=$(echo $output | sed -E 's/^..result.:.([a-z0-9]+).,.error.*$/\1/')

    generate_blocks;

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

    info "HTLC successfully funded - BTC payment was made."
}

function notify_bob_comit_node_btc_htlc_funded() {
    id=$1;

    result=$($curl --data-binary "{\"transaction_id\": \"${htlc_funding_tx}\",\"vout\": ${htlc_funding_tx_vout}}" -H 'Content-Type: application/json' $(echo ${BOB_COMIT_NODE_URL} | sed 's/0$/2/' )/ledger/trades/ETH-BTC/$id/buy-order-htlc-funded )

    info_blue "Notified bob about alice's BTC payment (Alice funded BTC HTLC)."
}

function notify_alice_comit_node_eth_htlc_funded() {
    id=$1;

    result=$($curl --data-binary "{\"contract_address\": \"${ETH_HTLC_ADDRESS}\"}" -H 'Content-Type: application/json' $(echo ${ALICE_COMIT_NODE_URL} | sed 's/0$/2/')/ledger/trades/ETH-BTC/$id/buy-order-contract-deployed)

    debug $result;
    info_blue "Notified alice about bob's ETH payment (Bob funded ETH HTLC)."
}

function notify_bob_comit_node_eth_redeemed() {
    id=$1 secret=$2;
    debug "$($curl --data-binary "{\"secret\": \"$secret\"}" -H 'Content-Type: application/json' $(echo ${BOB_COMIT_NODE_URL} | sed 's/0$/2/' )/ledger/trades/ETH-BTC/${id}/buy-order-secret-revealed)";

    info_blue "Notified Bob about revealed secret (Alice redeemed ETH funds)."
}

function get_eth_balance() {
    address=$1
    output=$($curl --data-binary "{\
      \"jsonrpc\":\"2.0\",\
      \"method\":\"eth_getBalance\",\
      \"params\":[\
        \"$address\",\
        \"latest\"\
      ],\
      \"id\":1\
    }" \
    -H 'Content-Type: application/json' ${ETHEREUM_NODE_ENDPOINT})

    echo $output|jq -r .result
}

function redeem_eth() {
    address="$1" data="$2" gas=$(printf '%x' "$3");
    debug $(
        $curl --data-binary "{\
      \"jsonrpc\":\"2.0\",\
      \"method\":\"eth_sendTransaction\",\
      \"params\":[\
        {\
          \"from\": \"${alice_sender_address}\",\
          \"to\": \"${address}\",\
          \"gas\": \"0x100000\",\
          \"gasPrice\": \"0x01\",\
          \"value\": \"0x0\",\
          \"data\": \"0x${data}\"\
        }\
      ],\
      \"id\":1\
    }" \
       -H 'Content-Type: application/json' ${ETHEREUM_NODE_ENDPOINT}
    );
}

function list_unspent_transactions() {
    address=$1;
    $curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
    "{\
      \"jsonrpc\":\"1.0\",\
      \"id\":\"curltest\",\
      \"method\":\"listunspent\",\
      \"params\":\
      [\
        0,\
        9999999,\
        [\
          \"$address\"\
        ]\
      ],\
      \"id\":1\
    }" \
    -H 'content-type: text/plain;' $BITCOIN_RPC_URL | jq '.result'
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

id=$(send_swap_request "btc-eth" "$btc_amount" "$eth_amount" "$alice_refund_address" "$alice_success_address")

alice_htlc_address=$(alice_wait_swap_status "$id" "accepted" | sed  -n 's/funding_required: //p');

fund_htlc "$alice_htlc_address";

notify_bob_comit_node_btc_htlc_funded $id;

notify_alice_comit_node_eth_htlc_funded $id;

secret=$(
    ### REDEEM ETHEREUM
    redeemable=$(alice_wait_swap_status "$id" "redeemable")
    debug "REDEEM DETIALS"
    debug "$redeemable";
    debug "=======";
    contract_address=$(echo "$redeemable" | sed -n 's/contract_address: //p');
    secret=$(echo "$redeemable" | sed -n 's/data: //p');
    gas=$(echo "$redeemable" | sed -n 's/gas: //p');

    old_balance=$(wei_to_eth $(hex_to_dec $(get_eth_balance $alice_success_address)));
    info "Previous ETH balance: $old_balance";

    redeem_eth "$contract_address" "$secret" "$gas";

    new_balance=$(wei_to_eth $(hex_to_dec $(get_eth_balance $alice_success_address)));
    info "New ETH balance: $new_balance";

    if [ ${old_balance} -lt ${new_balance} ]
    then
        info_blue "## ETH WAS redeemed ##";
    else
        info "## ETH was NOT redeemed ##";
        exit 1
    fi
    echo $secret;
);

{
    ### REDEEM BITCOIN
    old_unspent=$(list_unspent_transactions $BTC_BOB_REDEEM_ADDRESS);
    debug "BTC Old Unspent: $old_unspent";
    old_unspent_num=$(echo "$old_unspent" | jq length);
    info "BTC: Total UTXOs before redeem: $old_unspent_num";

    notify_bob_comit_node_eth_redeemed $id $secret;
    generate_blocks;

    new_unspent=$(list_unspent_transactions $BTC_BOB_REDEEM_ADDRESS);
    debug "BTC New Unspent: $new_unspent";
    new_unspent_num=$(echo "$new_unspent" | jq length);
    info "BTC: Total UTXOs after redeem: $new_unspent_num";

    if [ ${old_unspent_num} -lt ${new_unspent_num} ]
    then
        info_blue "## BTC WAS redeemed ##";
    else
        info "## BTC was NOT redeemed ##";
        exit 1
    fi
}
