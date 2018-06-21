#!/usr/bin/env bash
set -eu

if [ -z "$(which cargo)" ] ; then
    export PATH=$PATH:$HOME/.cargo/bin
fi

cargo +stable-2018-05-10 fmt -- --write-mode diff
