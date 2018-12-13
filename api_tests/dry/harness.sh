#!/usr/bin/env bash

set -e;
export PROJECT_ROOT=$(git rev-parse --show-toplevel)
source "$PROJECT_ROOT/api_tests/harness-lib.sh"

GIVEN_TEST_PATH="$1";

if [[ -z "${GIVEN_TEST_PATH}" ]]
then
    log "Path to test needs to passed";
    exit 1;
fi

export TEST_PATH=$(cd ${GIVEN_TEST_PATH} && pwd); # Convert to absolute path
export LOG_DIR="$TEST_PATH/log"

END(){
    set +e;
    for pid in "$BOB_COMIT_NODE_PID" "$ALICE_COMIT_NODE_PID"; do
        if test "$pid" && ps "$pid" >/dev/null; then
            echo "KILLING $pid";
            kill "$pid" 2>/dev/null;
            # Here if one of the above is a job is doesn't print out an annoying "Terminated" line to stderr
            wait "$pid" 2>/dev/null;
        fi
    done
}

trap 'END' EXIT;

function setup() {
    if test "$LOG_DIR"; then
        mkdir -p "$LOG_DIR"
        rm -f "$LOG_DIR/*.log"
    fi

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
}


setup;
run_test "${TEST_PATH}/test.js";

