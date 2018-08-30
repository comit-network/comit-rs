set -e;
export PROJECT_ROOT=$(git rev-parse --show-toplevel)

if test "$LOG_NODES"; then
    exec 4>&1
else
    exec 4>/dev/null
fi

if test "$LOG_SERVICES"; then
    if [ "$LOG_SERVICES" = "1" ]; then
        exec 3>&1;
    else
        exec 3>"$LOG_SERVICES";
    fi
else
    exec 3>/dev/null;
fi

function log {
    echo "$@" >&2;
}

END(){
    log "KILLING docker containers";
    (
        cd $PROJECT_ROOT/run_environments/regtest;
        docker-compose rm -sfv;
    );

    for pid in "$FAKE_TREASURY_PID" "$EXCHANGE_SERVICE_PID" "$TRADING_SERVICE_PID"; do
        if test "$pid" && ps "$pid" >/dev/null; then
            echo "KILLING $pid";
            kill "$pid" 2>/dev/null;
        fi
    done
}

trap 'END' EXIT;


function start_target() {
    name=$1;
    log "Starting $name";
    # Logs prefixes the service name in front of its logs
    "${PROJECT_ROOT}/target/debug/$name" 2>&1 | sed  "s/^/$name: / " >&3 &
    # returns the PID of the process
    jobs -p
}

function activate_segwit() {
    log "Generating enough blocks to activate segwit";
    curl -s --user $BITCOIN_RPC_USERNAME:$BITCOIN_RPC_PASSWORD --data-binary \
          "{\"jsonrpc\": \"1.0\",\"id\":\"curltest\",\"method\":\"generate\", \"params\": [ 432 ]}" -H 'content-type: text/plain;' $BITCOIN_RPC_URL  > /dev/null
}


function setup() {
    log "Starting up ...";

    #### Env variable to run all services
    set -a;
    source ${PROJECT_ROOT}/run_environments/common.env
    source ${PROJECT_ROOT}/run_environments/regtest/network.env
    source ${PROJECT_ROOT}/run_environments/regtest/regtest.env
    set +a;

    #### Start all services
    (
        cd $PROJECT_ROOT/run_environments/regtest;
        docker-compose up -d ethereum bitcoin >&4 2>&4;
    );

    sleep 5;

    activate_segwit;

    FAKE_TREASURY_PORT=8020
    EXCHANGE_PORT=8010
    TRADING_SERVICE_PORT=8000

    FAKE_TREASURY_PID=$(
        export ROCKET_ADDRESS=0.0.0.0 \
               ROCKET_PORT=$FAKE_TREASURY_PORT \
               RUST_LOG=info,fake_treasury_service=debug \
               RATE=0.1;

        start_target "fake_treasury_service";
    );

    EXCHANGE_SERVICE_PID=$(
        export BITCOIN_RPC_URL=http://localhost:18443 \
               ETHEREUM_NODE_ENDPOINT=http://localhost:8545 \
               TREASURY_SERVICE_URL=http://localhost:$FAKE_TREASURY_PORT \
               ROCKET_ADDRESS=0.0.0.0 \
               ROCKET_PORT=$EXCHANGE_PORT \
               RUST_LOG=info,exchange_service=debug,bitcoin_htlc=debug \
               RUST_BACKTRACE=1 \
               BITCOIN_SATOSHI_PER_KB=50;

        start_target "exchange_service";
    );


    TRADING_SERVICE_PID=$(
        export  ROCKET_ADDRESS=0.0.0.0 \
                RUST_LOG=info,fake_treasury_service=debug \
                EXCHANGE_SERVICE_URL=http://localhost:$EXCHANGE_PORT \
                ROCKET_PORT=$TRADING_SERVICE_PORT \
                RATE=0.1;

        start_target "trading_service";
    );
}

test "$@" || { log "ERROR: The harness requires a file to run!"; exit 1; }

setup;

sleep 2;

"$@"
