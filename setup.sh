#!/usr/bin/env bash

PROJECT_RUST_VERSION=nightly-2018-06-24
RUSTFMT_RUST_VERSION=stable-2018-05-10

rustup toolchain install ${PROJECT_RUST_VERSION}
rustup toolchain install ${RUSTFMT_RUST_VERSION}

rustup component add rustfmt-preview --toolchain=${RUSTFMT_RUST_VERSION}

echo ${PROJECT_RUST_VERSION} > rust-toolchain

ln -sf $(pwd)/check-fmt.sh .git/hooks/pre-commit
