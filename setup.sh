#!/usr/bin/env bash

PROJECT_RUST_VERSION=nightly-2018-05-05
RUSTFMT_RUST_VERSION=stable-2018-05-10

rustup toolchain install ${PROJECT_RUST_VERSION}
rustup toolchain install ${RUSTFMT_RUST_VERSION}

rustup component add rustfmt-preview --toolchain=${RUSTFMT_RUST_VERSION}

rustup override set ${PROJECT_RUST_VERSION}

ln -sf $(pwd)/check-rust-fmt.sh .git/hooks/pre-commit