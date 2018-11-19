#!/usr/bin/env bash

curl="curl -s"

function log {
    echo "$*" >&2;
}

function debug {
    if test "$DEBUG"; then
        echo "$*" >&2;
    fi
}

function start_target() {
    name=$1;
    log_prefixed=$name-$2
    log "Starting $log_prefixed";
    log_file="/dev/null";

    if test "$LOG_DIR"; then
        log_file="$LOG_DIR/$(printf '%s.log' $2)";
        log "Logging $log_prefixed to $log_file";
    fi

    "${PROJECT_ROOT}/target/debug/$name" >"$log_file" 2>&1 &
    echo $!
}

function generate_blocks() {
    debug "Generating $1 blocks";
    $curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
          "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"generate\", \"params\": [ $1 ]}" -H 'content-type: text/plain;' "$BITCOIN_RPC_URL" >/dev/null
}

function generate_btc_blocks_every() {
    debug "Generating blocks every $1 seconds"
    {
        while true; do
            generate_blocks 1;
            sleep "$1";
        done;
    } & BTC_BLOCKLOOP_PID=$!;
}

function activate_segwit() {
    debug "Generating enough blocks to activate segwit";
    count=0;
    tries=3
    while [ "$((count+=1))" -le "$tries" ] && ! generate_blocks 432; do
        sleep 5;
        if [ "$count" = "$tries" ]; then
            log "Segwit activation failed so far trying one last time with verbose output:";
            $curl -vvv --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
                  "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"generate\", \"params\": [ 432 ]}" -H 'content-type: text/plain;' "$BITCOIN_RPC_URL";
        else
            debug "Failed to generate segwit blocks. Trying again $count/$tries";
        fi
    done
}

function fund_bitcoin_address() {
    export BTC_FUNDED_PRIVATE_KEY=KxDGGfKJ15GkDEUuaZwM2eCd46vm9Dg4CiTBYD5A7tKMeu8utePH;
    export BTC_FUNDED_PUBLIC_KEY=03deeb9ed34ff51e5388873f4671373bc6e87c45566c79d52f08af1a974893a40f;
    export BTC_FUNDED_ADDRESS=bcrt1qd6msadw56awmjgsm9843kzgs7cth9q48cxvahx;
    export BTC_FUNDED_AMOUNT=5;
    debug "Funding $BTC_FUNDED_ADDRESS with $BTC_FUNDED_AMOUNT BTC";

    output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
                   "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"sendtoaddress\", \"params\": [ \"${BTC_FUNDED_ADDRESS}\", $BTC_FUNDED_AMOUNT]}" -H 'content-type: text/plain;' $BITCOIN_RPC_URL);

    funding_tx=$(echo $output | sed -E 's/^..result.:.([a-z0-9]+).,.error.*$/\1/');
    generate_blocks 1;

    output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
                   "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"getrawtransaction\", \"params\": [ \"${funding_tx}\" ]}" \
                   -H 'content-type: text/plain;' $BITCOIN_RPC_URL);

    raw_funding_tx=$(echo $output | sed -E 's/^..result.:.([a-z0-9]+).,.error.*$/\1/')

    output=$($curl --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
                   "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"decoderawtransaction\", \"params\": [ \"${raw_funding_tx}\" ]}"\
                   -H 'content-type: text/plain;' $BITCOIN_RPC_URL);

    funding_tx_vout=$(echo $output | jq .result.vout | jq ".[] | select(.scriptPubKey.addresses[0] == \"${BTC_FUNDED_ADDRESS}\")"|jq .n);

    debug "$BTC_FUNDED_AMOUNT BTC was funded to $BTC_FUNDED_ADDRESS at tx $funding_tx at vout $funding_tx_vout";

    export BTC_FUNDED_TX=$funding_tx;
    export BTC_FUNDED_VOUT=$funding_tx_vout;
}
