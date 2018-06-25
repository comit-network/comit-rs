#!/usr/bin/env bash
set -eu

CARGO_BIN=$(expr "$(which cargo)" '|' "$HOME/.cargo/bin/cargo");

${CARGO_BIN} +stable-2018-05-10 fmt -- --write-mode diff
