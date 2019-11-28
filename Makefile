RUSTUP = rustup

TOOLCHAIN = $(shell cat rust-toolchain)
CARGO = $(RUSTUP) run --install $(TOOLCHAIN) cargo --color always

NIGHTLY_TOOLCHAIN = "nightly-2019-07-31"
CARGO_NIGHTLY = $(RUSTUP) run --install $(NIGHTLY_TOOLCHAIN) cargo --color always

GIT_HOOKS_PATH = ".githooks"
GIT_HOOKS = $(wildcard $(GIT_HOOKS_PATH)/*)

default: init_git_hooks build format

init_git_hooks: $(GIT_HOOKS)
	git config core.hooksPath $(GIT_HOOKS_PATH)

install_rust:
	$(RUSTUP) toolchain list | grep -q $(TOOLCHAIN) || $(RUSTUP) install $(TOOLCHAIN)

install_rust_nightly:
	$(RUSTUP) toolchain list | grep -q $(NIGHTLY_TOOLCHAIN) || $(RUSTUP) install $(NIGHTLY_TOOLCHAIN)

## Dev environment

install_clippy: install_rust
	$(RUSTUP) component list --installed --toolchain $(TOOLCHAIN) | grep -q clippy || $(RUSTUP) component add clippy --toolchain $(TOOLCHAIN)

install_rustfmt: install_rust_nightly
	$(RUSTUP) component list --installed --toolchain $(NIGHTLY_TOOLCHAIN) | grep -q rustfmt || $(RUSTUP) component add rustfmt --toolchain $(NIGHTLY_TOOLCHAIN)

install_tomlfmt: install_rust
	$(CARGO) --list | grep -q tomlfmt || $(CARGO) install cargo-tomlfmt

yarn_install:
	(cd ./api_tests; yarn install)

## User install

install:
	$(CARGO) install --force --path .

clean:
	$(CARGO) clean

## Development tasks

all: format build clippy test doc e2e_scripts

format: install_rustfmt install_tomlfmt yarn_install
	$(CARGO_NIGHTLY) fmt
	$(CARGO) tomlfmt -p Cargo.toml
	(cd ./api_tests; yarn run fix)

ci: check_format doc clippy test build e2e

build:
	$(CARGO) build --all --all-targets $(BUILD_ARGS)

clippy: install_clippy
	$(CARGO) clippy \
	    --all-targets \
	    -- \
	    -W clippy::cast_possible_truncation \
	    -W clippy::cast_sign_loss \
	    -W clippy::fallible_impl_from \
	    -W clippy::cast_precision_loss \
	    -W clippy::cast_possible_wrap \
	    -W clippy::print_stdout \
	    -W clippy::dbg_macro \
	    -D warnings

test:
	$(CARGO) test --all

doc:
	$(CARGO) doc

check_format: check_rust_format check_toml_format check_ts_format

check_rust_format: install_rustfmt
	$(CARGO_NIGHTLY) fmt -- --check

check_toml_format: install_tomlfmt
	$(CARGO) tomlfmt -d -p Cargo.toml

check_ts_format: yarn_install
	(cd ./api_tests; yarn run check)

e2e: build yarn_install
	(cd ./api_tests; yarn test)
