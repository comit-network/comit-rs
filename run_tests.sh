#!/bin/sh
set -ev;

END(){
    if test "$docker_id"; then
        echo "KILLING bitcoin rpc docker $docker_id";
        docker rm -f $docker_id;
    fi
}

trap 'END' EXIT;

requires_bitcoin_rpc="bitcoin_rpc bitcoin_wallet";

bitcoin_rpc_integration_tests(){
    export BITCOIN_RPC_URL="http://localhost:18443"
    export BITCOIN_RPC_USERNAME="bitcoin"
    export BITCOIN_RPC_PASSWORD="54pLR_f7-G6is32LP-7nbhzZSbJs_2zSATtZV_r05yg="

    docker_id="$(sh .docker/bitcoind-regtest)";
    sleep_for=10
    echo "sleeping for $sleep_for while bitcoind starts";
    sleep $sleep_for;

    cmd="cargo test --jobs 1";

    for package in $requires_bitcoin_rpc; do
        cmd="$cmd --package $package"
    done
    $cmd;
}

unit_tests(){
    cargo test --all --exclude $requires_bitcoin_rpc;
}

unit_tests;
bitcoin_rpc_integration_tests;
