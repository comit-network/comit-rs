#!/bin/bash
set -eu

cargo +stable-2018-03-29 fmt -- --write-mode diff
