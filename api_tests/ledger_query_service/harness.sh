#!/usr/bin/env bash

set -e;
export PROJECT_ROOT=$(git rev-parse --show-toplevel)
cd "$PROJECT_ROOT/api_tests";

source "$PROJECT_ROOT/api_tests/harness-lib.sh"

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
    if test "$LOG_DIR"; then
        mkdir -p "$LOG_DIR"
        rm -f "$LOG_DIR/*.log"
    fi

    #### Env variable to run all services
    set -a;
    source ./regtest/regtest.env
    set +a;

    export ALICE_CONFIG_FILE=./regtest/alice/e2e/default.toml;

    export BITCOIN_RPC_URL="http://$BITCOIN_RPC_HOST:$BITCOIN_RPC_PORT";
    #### Start all services
    (
        cd ./regtest;
        log "Starting up docker containers";
        docker-compose up -d btc eth;
        if test -d "$LOG_DIR"; then
            log_file="$LOG_DIR/docker-compose.log"
            # docker-compose logs --tail=all >$log_file
        fi
    );

    sleep 6;

    LQS_PID=$(
        export LEDGER_QUERY_SERVICE_CONFIG_PATH=./regtest/ledger_query_service
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

npm test "$@";
