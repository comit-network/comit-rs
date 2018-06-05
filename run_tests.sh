#!/bin/sh
set -ev;

END(){
    if test "${bitcoin_docker_id}${ganache_docker_id}"; then
        echo "KILLING docker containers $bitcoin_docker_id $ganache_docker_id";
        docker rm -f $bitcoin_docker_id $ganache_docker_id;
    fi
}

trap 'END' EXIT;

requires_bitcoin_rpc="bitcoin_rpc bitcoin_wallet";
requires_ganache="ethereum_htlc ethereum_wallet ganache_rust_web3";

bitcoin_rpc_integration_tests(){
    export BITCOIN_RPC_URL="http://localhost:18443"
    export BITCOIN_RPC_USERNAME="bitcoin"
    export BITCOIN_RPC_PASSWORD="54pLR_f7-G6is32LP-7nbhzZSbJs_2zSATtZV_r05yg="

    bitcoin_docker_id="$(sh .docker/bitcoind-regtest)";
    sleep_for=10
    echo "sleeping for $sleep_for while bitcoind starts";
    sleep $sleep_for;

    cmd="cargo test";

    for package in $requires_bitcoin_rpc; do
        cmd="$cmd --package $package"
    done
    $cmd;
}

ganache_integration_tests(){

    export GANACHE_ENDPOINT="http://localhost:8545"

    ganache_docker_id="$(sh .docker/ganache)";
    sleep_for=10
    echo "sleeping for $sleep_for while ganache starts";
    sleep $sleep_for;

    cmd="cargo test";

    for package in $requires_ganache; do
        cmd="$cmd --package $package"
    done
    $cmd;
}

unit_tests(){

    cmd="cargo test --all";

    for package in $requires_bitcoin_rpc $requires_ganache; do
        cmd="$cmd --exclude $package"
    done

    $cmd
}

export RUST_TEST_THREADS=1;

unit_tests;
bitcoin_rpc_integration_tests;
ganache_integration_tests;
