#!/usr/bin/env bash

# Install sccache if it doesn't exist
which sccache || cargo install sccache

# Start sccache with limited cache size to avoid a constantly growing caches (and thus continuously slower builds)
SCCACHE_CACHE_SIZE=400M sccache --start-server

sccache --show-stats
