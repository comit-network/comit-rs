#!/usr/bin/env bash

set -e;
export PROJECT_ROOT=$(git rev-parse --show-toplevel)
source "$PROJECT_ROOT/api_tests/harness-lib.sh"

TEST_PATH="$1";
export TEST_PATH=$(cd ${TEST_PATH} && pwd); # Convert to absolute path

if [[ -z "${TEST_PATH}" ]] || [[ ! -d "${TEST_PATH}" ]]
then
    log "Path to test directory needs to be passed.";
    exit 1;
fi

DIR=${TEST_PATH%*/} # Removes trailing slash
DIR=${DIR##*/} # Extract child dir

ALPHA=${DIR%_*}
ALPHA_CHAIN=${ALPHA%-*}

BETA=${DIR#*_}
BETA_CHAIN=${BETA%-*}

CHAINS="${ALPHA_CHAIN} ${BETA_CHAIN}" # Extract chain name before and after underscore

cd "$PROJECT_ROOT/api_tests";

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
        docker-compose rm -sfv ${CHAINS};
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

    export BITCOIN_RPC_URL="http://$BITCOIN_RPC_HOST:$BITCOIN_RPC_PORT";
    #### Start all services
    (
        cd ./regtest;
        log "Starting up docker containers";
        docker-compose up -d ${CHAINS};
        if test -d "$LOG_DIR"; then
            log_file="$LOG_DIR/docker-compose.log";
            docker-compose logs --tail=all >$log_file;
        fi
    );

    sleep 10;

    export BOB_CONFIG_FILE=./regtest/bob/default.toml;
    export BOB_COMIT_NODE_HOST=127.0.0.1;
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
        export LEDGER_QUERY_SERVICE_CONFIG_PATH=./regtest/ledger_query_service;
        export ETHEREUM_POLLING_TIME_SEC=1;
        export RUST_LOG=debug;

        start_target "ledger_query_service" "LQS";
    );
}

test "$*" || { log "ERROR: The harness requires to test to run!"; exit 1; }

setup;

debug "Bitcoin RPC url: $BITCOIN_RPC_URL";
debug "Ethereum node url: $ETHEREUM_NODE_ENDPOINT";
activate_segwit;

generate_btc_blocks_every 5;
sleep 2;

log "Run test";
set +x;
export NVM_SH=$([ -e $NVM_DIR/nvm.sh ] && echo "$NVM_DIR/nvm.sh" || echo /usr/local/opt/nvm/nvm.sh );
. "$NVM_SH"
nvm use;
npm test "${TEST_PATH}/test.js";
