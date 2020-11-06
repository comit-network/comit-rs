## Setup all variables

RUSTUP = rustup

TOOLCHAIN = $(shell cat rust-toolchain)
CARGO = $(RUSTUP) run --install $(TOOLCHAIN) cargo --color always

GIT_HOOKS_PATH = ".githooks"
GIT_HOOKS = $(wildcard $(GIT_HOOKS_PATH)/*)

INSTALLED_TOOLCHAINS = $(shell $(RUSTUP) toolchain list)
INSTALLED_COMPONENTS = $(shell $(RUSTUP) component list --installed --toolchain $(TOOLCHAIN))

## Only recipe targets from here

# All our targets go into .PHONY because none of them actually create files
.PHONY: init_git_hooks default install_rust install_clippy install clean all ci build clippy test doc e2e check_format format lint ts_lint

default: init_git_hooks build format

init_git_hooks:
	git config core.hooksPath $(GIT_HOOKS_PATH)

## Dev environment

install_rust:
ifeq (,$(findstring $(TOOLCHAIN),$(INSTALLED_TOOLCHAINS)))
	$(RUSTUP) install $(TOOLCHAIN)
endif

install_clippy: install_rust
ifeq (,$(findstring clippy,$(INSTALLED_COMPONENTS)))
	$(RUSTUP) component add clippy --toolchain $(TOOLCHAIN)
endif

## User install

install:
	$(CARGO) install --force --path cnd

clean:
	$(CARGO) clean

## Development tasks

all: format build clippy test doc e2e

build:
	$(CARGO) build --workspace --all-targets $(BUILD_ARGS)

lint: clippy ts_lint

ts_lint:
	(cd ./tests; yarn install; yarn check)

clippy: install_clippy
	$(CARGO) clippy --all-targets -- -D warnings

e2e:
	$(CARGO) build -p cnd -p nectar $(BUILD_ARGS)
	(cd ./tests; yarn install; yarn test)

check_format:
	dprint check

format:
	dprint fmt
