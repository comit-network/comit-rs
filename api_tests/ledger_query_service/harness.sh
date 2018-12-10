#!/usr/bin/env bash

set -e;
export PROJECT_ROOT=$(git rev-parse --show-toplevel)
source "$PROJECT_ROOT/api_tests/harness-lib.sh"

TEST_PATH="$1";
export TEST_PATH=$(cd ${TEST_PATH} && pwd); # Convert to absolute path

LOG_DIR="$TEST_PATH/log"

END(){
    set +e;
    for pid in "$LQS_PID"; do
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
        docker-compose rm -sfv btc eth;
    );
}

trap 'END' EXIT;

function setup() {
    mkdir -p "$LOG_DIR"
    rm -f "$LOG_DIR/*.log"

    #### Env variable to run all services
    set -a;
    source "$PROJECT_ROOT/api_tests/regtest/regtest.env"
    set +a;

    export ALICE_CONFIG_FILE="$PROJECT_ROOT/api_tests/regtest/alice/default.toml";

    export BITCOIN_RPC_URL="http://$BITCOIN_RPC_HOST:$BITCOIN_RPC_PORT";
    #### Start all services
    (
        log "Starting up docker containers";

        cd "$PROJECT_ROOT/api_tests/regtest";
        docker-compose up -d btc eth;
        docker-compose logs --tail=all > "$LOG_DIR/docker-compose.log"
    );

    sleep 6;

    LQS_PID=$(
        export LEDGER_QUERY_SERVICE_CONFIG_PATH="$PROJECT_ROOT/api_tests/regtest/ledger_query_service"
        export ETHEREUM_POLLING_TIME_SEC=1
        export RUST_LOG=trace;

        start_target "ledger_query_service" "LQS";
    );
}

test "$*" || { log "ERROR: The harness requires to test to run!"; exit 1; }

setup;

debug "Bitcoin RPC url: $BITCOIN_RPC_URL";
debug "Ethereum node url: $ETHEREUM_NODE_ENDPOINT";

activate_segwit;
sleep 2;

run_test "${TEST_PATH}/test.js";
