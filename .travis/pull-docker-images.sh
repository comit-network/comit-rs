#!/usr/bin/env bash

# We use several docker images in our build through testcontainers.
# Because those tests run in parallel, we pull the images here so that we only download them once.

docker pull parity/parity:v1.11.11
docker pull coblox/bitcoin-core:0.16.1-r2
docker pull ruimarinho/bitcoin-core:0.17.0-alpine
