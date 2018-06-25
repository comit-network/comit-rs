#!/usr/bin/env bash
CARGO_BIN=$(which cargo) || true
set -eu

if [ -z "${CARGO_BIN}" ] ; then
    CARGO_BIN=$HOME/.cargo/bin/cargo
fi

${CARGO_BIN} +stable-2018-05-10 fmt -- --write-mode diff
