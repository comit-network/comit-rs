#!/usr/bin/env bash

which toml || cargo install tomlcli

find vendor application -name Cargo.toml `# find all Cargo.toml files` | \
xargs -I '{}' -n 1 toml --nocolor '{}' package.name | `# extract package name` \
xargs -n 1 -t cargo clean -p `# pass package name to cargo clean`
