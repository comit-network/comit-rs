#!/usr/bin/env bash

function start {
    docker-compose up -d
}

function stop {
    docker-compose down
}

# Check if the function exists (bash specific)
if declare -f "$1" > /dev/null
then
  # call arguments
  "$@"
else
  # Show a helpful error
  echo "'$1' is not a known function name" >&2
  exit 1
fi