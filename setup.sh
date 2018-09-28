#!/usr/bin/env bash

PROJECT_RUST_VERSION=nightly-2018-09-14

rustup toolchain install ${PROJECT_RUST_VERSION}

rustup component add rustfmt-preview --toolchain=${PROJECT_RUST_VERSION}

rustup component add clippy-preview --toolchain=${PROJECT_RUST_VERSION}

echo ${PROJECT_RUST_VERSION} > rust-toolchain

ln -sf $(pwd)/check-fmt.sh .git/hooks/pre-commit
