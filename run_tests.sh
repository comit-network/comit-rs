#!/bin/sh
set -ev;

END(){
    if test "${ganache_docker_id}"; then
        echo "KILLING docker containers $ganache_docker_id";
        docker rm -f $ganache_docker_id;
    fi
}

trap 'END' EXIT;


export RUST_TEST_THREADS=1;
export GANACHE_ENDPOINT="http://localhost:8545"
export ETHEREUM_NETWORK_ID=42

ganache_docker_id="$(sh .blockchain_nodes/ganache)";

sleep_for=10
echo "sleeping for $sleep_for while ganache starts";
sleep $sleep_for;

cargo test --all