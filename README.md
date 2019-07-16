# COMIT-rs

[![CircleCI](https://circleci.com/gh/comit-network/comit-rs.svg?style=svg)](https://circleci.com/gh/comit-network/comit-rs)

COMIT is an open protocol facilitating trustless cross-blockchain applications.
This is a reference implementation for the COMIT protocol. 

## WARNING - We do not recommend running COMIT on mainnet for now!!!

## Structure

The repository contains three main folders: `comit_node`, `btsieve`, and`vendor`.

`comit_node` and `btsieve` hold code for the primary binaries that make up the reference implementation.
Code in these can depend on libraries located in `vendor`.

### Vendor

Contains crates that provide general functionality that is not specific to the domain of atomic swaps. 
Crates defined in here MUST NOT depend on `comit_node` or `btsieve`.
They may be separated from the repository at some point (and possibly released on crates.io).

## Setup build environment

1. Install `rustup`: `curl https://sh.rustup.rs -sSf | sh`
2. Install libzmq:
   - Ubuntu/Debian: `apt install libzmq3-dev`
   - Mac ([Homebrew](https://brew.sh/)) `brew install zeromq`
3. Install OpenSSL:
   - Ubuntu/Debian: `apt install libssl-dev pkg-config`

## Build & Run

1. `cargo build`
2. Put a [(default)](btsieve/config/btsieve.toml) config file into `~/.config/comit/btsieve.toml` or pass `--config <config_file>`.
3. startup bitcoin node (port to be set according to btsieve configuration)
4. startup ethereum node (port to be set according to btsieve configuration)
5. startup btsieve: `target/debug/btsieve`
6. startup comit_node: `target/debug/comit_node`

If the `[web_gui]` section is specified in the configuration the current release of the user interface [comit-i](https://github.com/comit-network/comit-i) will be served once the comit node started up (served at `localhost:8080` as default).

In order to do a swap you will have to start two comit nodes. 

## Setup testing/dev environment

1. Install `docker` & `docker-compose`
2. Install `node` (check the version required in package.json) & `yarn`
3. Install Rust `nightly-2019-04-30`: `rustup install nightly-2019-04-30` (this one is only used for `rustfmt`)
4. Install `rustfmt` for `nightly-2019-04-30`: `rustup component add rustfmt --toolchain nightly-2019-04-30`
5. Install `cargo-make`: `cargo install cargo-make`
6. Run `cargo make` in the root folder of the repository, this will install various crates & tools such as clippy
   
## Testing

- `cargo make` runs the whole test suite including integration tests but not end-to-end.
- `cargo make all` also runs the whole test suite, including end-to-end tests.
- `cargo make format` to format Rust code
- `cargo make ts-fix` to format Typescript code
- `cargo make btsieve` to run btsieve tests
- `cargo make dry` to run COMIT node dry tests
- `cargo make api` to run all API tests
- `cargo make e2e` to run COMIT node end-to-end tests
- `cargo make e2e *btc*` to run COMIT node end-to-end tests with `btc` in the folder name (supports shell glob on the name)

Alternatively, you can run the end-to-end tests and TypeScript related actions using `yarn` (careful! It does not recompile Rust for you):
- `yarn run tests`: run all tests
- `yarn run <directory>`: run all tests in the directory
- `yarn run <path to test file>`: run all tests in this test file, supports shell glob on the path
- `yarn run fix`: run prettier and linter to fix format
- `yarn run chec`: run tsc (to check validity of TypeScript code) and verify format




## Contributing

Contributions are welcome, please visit [CONTRIBUTING](CONTRIBUTING.md) for more details.

## License

This project is licensed under the terms of the [GNU GENERAL PUBLIC LICENSE v3](LICENSE.md).
