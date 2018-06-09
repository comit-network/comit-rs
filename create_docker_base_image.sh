#!/usr/bin/env bash

REVISION=$(git rev-parse --short HEAD)
IMAGE="tenx-tech/swap"

ID="${IMAGE}:${REVISION}"

docker build . -t $ID
docker tag $ID "${IMAGE}:latest"