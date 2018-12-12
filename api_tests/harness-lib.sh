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

function run_test() {

    log "Running $1";

    export NVM_SH=$([ -e $NVM_DIR/nvm.sh ] && echo "$NVM_DIR/nvm.sh" || echo /usr/local/opt/nvm/nvm.sh );
    . "$NVM_SH"
    nvm use 11 || (nvm install 11 && nvm use 11);

    cd "${PROJECT_ROOT}/api_tests";
    yarn install;
    npm test "$1" || { [ "$CAT_LOGS" ] && (cd "$LOG_DIR"; tail -n +1 *.log); exit 1; }
}
