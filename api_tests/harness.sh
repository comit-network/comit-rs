#!/usr/bin/env bash

set -e;
export PROJECT_ROOT=$(git rev-parse --show-toplevel)
cd "$PROJECT_ROOT/api_tests";
curl="curl -s"

if test "$LOG_DIR"; then
    mkdir -p "$LOG_DIR"
fi

function log {
    echo "$*" >&2;
}

function debug {
    if test "$DEBUG"; then
        echo "$*" >&2;
    fi
}

END(){
    set +e;
    for pid in "$BOB_COMIT_NODE_PID" "$ALICE_COMIT_NODE_PID" "$LQS_PID" "$BTC_BLOCKLOOP_PID"; do
        if test "$pid" && ps "$pid" >/dev/null; then
            echo "KILLING $pid";
            kill "$pid" 2>/dev/null;
            # Here if one of the above is a job is doesn't print out an annoying "Terminated" line to stderr
            wait "$pid" 2>/dev/null;
        fi
    done
    log "KILLING docker containers";
    (
        cd regtest;
        docker-compose rm -sfv bitcoin ethereum;
    );
}

trap 'END' EXIT;


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

function setup() {

    #### Env variable to run all services
    set -a;
    source ./regtest/regtest.env
    set +a;

    export BITCOIN_RPC_URL="http://$BITCOIN_RPC_HOST:$BITCOIN_RPC_PORT";
    #### Start all services
    (
        cd ./regtest;
        log "Starting up docker containers"
        docker-compose up -d bitcoin ethereum
        if test -d "$LOG_DIR"; then
            log_file="$LOG_DIR/docker-compose.log"
            docker-compose logs --tail=all >$log_file
        fi
    );

    sleep 5;

    export BOB_CONFIG_FILE=./regtest/bob/default.toml;
    BOB_COMIT_NODE_PID=$(
        export RUST_BACKTRACE=1 \
               COMIT_NODE_CONFIG_PATH=./regtest/bob;
        start_target "comit_node" "Bob";
    );

    export ALICE_COMIT_NODE_HOST=127.0.0.1;
    export ALICE_CONFIG_FILE=./regtest/alice/default.toml;
    ALICE_COMIT_NODE_PID=$(
        export COMIT_NODE_CONFIG_PATH=./regtest/alice;
        start_target "comit_node" "Alice";
    );

    LQS_PID=$(
        export BITCOIN_ZMQ_ENDPOINT=tcp://127.0.0.1:28332;
        export ETHEREUM_WEB3_ENDPOINT=$ETHEREUM_NODE_ENDPOINT;
        export ETHEREUM_POLLING_TIME_SEC=1
        export RUST_LOG=warn,web3=debug,ledger_query_service=trace;

        start_target "ledger_query_service" "LQS";
    );
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

function fund_ethereum_address() {
    export ETH_FUNDED_PRIVATE_KEY=a2312b03bb78b43ca1deed87b3d23e86a171d791e3377a743b19ff29f1605991;
    export ETH_FUNDED_ADDRESS=0x10c4109152e265fdf646c6251a8b19922e7a4b71
    export ETH_FUNDED_PUBLIC_KEY=03561a3d81579418d69fdd052c5839d4881829b75bb2813676a4ee7f99d9fb1a6b;
    # perl -Mbigint -E 'my $val = ((shift) * 1_000_000_000_000_000_000); say $val->as_hex' 20
    export ETH_FUNDED_AMOUNT=0x1158e460913d00000 # 20 ethereum

    debug "Funding $ETH_FUNDED_ADDRESS with $ETH_FUNDED_AMOUNT ETH";

    parity_dev_account=0x00a329c0648769a73afac7f9381e08fb43dbea72

    debug $(
        $curl --data-binary "{\
          \"jsonrpc\":\"2.0\",\
          \"method\":\"personal_sendTransaction\",\
          \"params\":[\
            {\
              \"from\": \"$parity_dev_account\",\
              \"to\": \"${ETH_FUNDED_ADDRESS}\",\
              \"value\": \"$ETH_FUNDED_AMOUNT\"\
            },\
            \"\"\
          ],\
          \"id\":1\
         }" \
          -H 'Content-Type: application/json' ${ETHEREUM_NODE_ENDPOINT}
    )
}

test "$*" || { log "ERROR: The harness requires to test to run!"; exit 1; }

setup;

debug "Bitcoin RPC url: $BITCOIN_RPC_URL";
debug "Ethereum node url: $ETHEREUM_NODE_ENDPOINT";
activate_segwit;

fund_bitcoin_address;
fund_ethereum_address;
generate_btc_blocks_every 5;
sleep 2;

npm test "$@";
