#!/usr/bin/env bash

set -eu

cargo +stable-2018-05-10 fmt -- --write-mode diff