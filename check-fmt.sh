#!/usr/bin/env bash
set -eu

CARGO_BIN=$(expr "$(which cargo)" '|' "$HOME/.cargo/bin/cargo");

"${CARGO_BIN}" fmt -- --check
