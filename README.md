# COMIT-rs

[![CircleCI](https://circleci.com/gh/comit-network/comit-rs.svg?style=svg)](https://circleci.com/gh/comit-network/comit-rs)
[![Safety Dance](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

COMIT is an open protocol facilitating trustless cross-blockchain applications.
This is a reference implementation for the COMIT protocol. 

## DISCLAIMER: This is not mainnet ready!

- Extensive testing on mainnet from our side still remains to be done
- Several features for achieving production-ready robustness are still under works
- Once we consider comit-rs production ready, we will release version 1.0.0

## Structure

The repository contains several main folders: `cnd`, `btsieve`, `blockchain_contracts` and `internal`.

### Root crates

Crates at the root of the repository hold primary binaries and libraries:
- `cnd`: the COMIT Node Daemon binary.
- `btsieve`: the binary that sifts through Blockchain Transactions and blocks from the network.
- `blockchain_contracts`: the library that contains the HTLCs, to be migrated to its own repo once some dependencies are sorted.

### Internal

Contains crates that provide general functionality that is not specific to the domain of atomic swaps. 
Crates defined in here MUST NOT depend on any root crate.
They need to be cleaned-up/sorted out to either be removed, contributed back to a popular library or extracted into their own repository.
See [#626](https://github.com/comit-network/comit-rs/issues/626) for tracking.

## Setup build environment

All you need is ~love~ rust: `curl https://sh.rustup.rs -sSf | sh` 

## Build & Run

1. `cargo build`
2. Put a [(default)](btsieve/config/btsieve.toml) config file into `~/.config/comit/btsieve.toml` or pass `--config <config_file>`.
3. startup bitcoin node (port to be set according to btsieve configuration)
4. startup ethereum node (port to be set according to btsieve configuration)
5. startup btsieve: `target/debug/btsieve`
6. startup cnd: `target/debug/cnd`

If the `[web_gui]` section is specified in the configuration file the current release of the user interface [comit-i](https://github.com/comit-network/comit-i) will be served once cnd is started up (served at `localhost:8080` by default).

Keep in mind that in order to do a swap locally you will need to start two instances of cnd and at least one instance of btsieve. 

## Setup testing/dev environment

1. Install `docker` & `docker-compose`
2. Install `node` (check the version required in api_tests/package.json) & `yarn`
3. Install Rust `nightly-2019-07-31`: `rustup install nightly-2019-07-31` (this one is only used for `rustfmt`)
4. Install `rustfmt` for `nightly-2019-07-31`: `rustup component add rustfmt --toolchain nightly-2019-07-31`
5. Install `cargo-make`: `cargo install cargo-make`
6. Run `cargo make` in the root folder of the repository, this will install various crates & tools such as clippy
   
## Testing

- `cargo make` runs the whole test suite including integration tests but not end-to-end.
- `cargo make all` also runs the whole test suite, including end-to-end tests.
- `cargo make format` to format Rust code
- `cargo make ts-fix` to format Typescript code
- `cargo make btsieve` to run btsieve tests
- `cargo make dry` to run cnd dry tests
- `cargo make api` to run all API tests
- `cargo make e2e` to run cnd end-to-end tests
- `cargo make e2e *btc*` to run cnd end-to-end tests with `btc` in the folder name (supports shell glob on the name)

Alternatively, you can run the end-to-end tests and TypeScript related actions using `yarn` (careful! It does not recompile Rust for you):
- `yarn run test`: run all tests
- `yarn run test <directory>`: run all tests in the directory
- `yarn run test <path to test file>`: run all tests in this test file, supports shell glob on the path
- `yarn run fix`: run prettier and linter to fix format
- `yarn run check`: run tsc (to check validity of TypeScript code) and verify format




## Contributing

Contributions are welcome, please visit [CONTRIBUTING](CONTRIBUTING.md) for more details.

## License

This project is licensed under the terms of the [GNU GENERAL PUBLIC LICENSE v3](LICENSE.md).
