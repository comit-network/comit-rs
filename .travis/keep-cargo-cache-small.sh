#!/usr/bin/env bash

# The cargo registry cache grows continuously over time, making our build take longer and longer because it is cached on S3.
# This command removes everything that is older than 30 days, thus keeping only very recent libraries.

find $HOME/.cargo/registry/src $HOME/.cargo/registry/cache -mindepth 2 -type d -ctime 30 -exec rm -rf "{}" \;
