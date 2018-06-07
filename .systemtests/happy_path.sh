#!/usr/bin/env bash

set -ev;

END(){
    docker-compose down
}

trap 'END' EXIT;

docker-compose up -d

sleep 5

newman run https://www.getpostman.com/collections/c866f49fd436d0b9ffcf