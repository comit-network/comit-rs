#!/usr/bin/env bash

set -ev;

END(){
    docker-compose down
}

trap 'END' EXIT;

test -e ./node_modules || npm install newman

docker-compose up -d

sleep 5

./node_modules/.bin/newman run https://www.getpostman.com/collections/c866f49fd436d0b9ffcf --no-color
