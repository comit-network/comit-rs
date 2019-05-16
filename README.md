# COMIT-rs

[![Build Status](https://travis-ci.com/comit-network/comit-rs.svg?branch=master)](https://travis-ci.com/comit-network/comit-rs)

COMIT is an open protocol facilitating trustless cross-blockchain applications.
This is a reference implementation for the COMIT protocol. 

## WARNING - We do not recommend running COMIT on mainnet for now!!!

## Structure

The repository contains two main folders: `vendor` and `application`.

### Vendor

Contains crates that provide general functionality that is not specific to the domain of atomic swaps. 
Crates defined in here MUST NOT depend on crates in `application`. 
They may be separated from the repository at some point (and possibly released on crates.io).

### Application

Contains crates specific to our application. Can depend on libraries located in `vendor`.

## Setup build environment

1. Install `rustup`: `curl https://sh.rustup.rs -sSf | sh`
2. Install libzmq:
   - Ubuntu/Debian: `apt install libzmq3-dev`
   - Mac ([Homebrew](https://brew.sh/)) `brew install zeromq`
3. Install SSL libraries
   - Ubuntu/Debian: `apt install libssl-dev`
   - Mac ([Homebrew](https://brew.sh/)) `brew install openssl`
4. `solc` is currently needed to build (will be deprecated). 2 choices:
   - Install `docker`
   - OR install `solc`

## Build & Run

1. `cargo build` (do `export SOLC_BIN=/usr/bin/solc` if `solc` is installed locally)
2. Put a [`default.toml`](application/comit_node/config/default.toml) config file into `~/.config/comit_node`
   - or pass `--config <config_path>`
   - or set `COMIT_NODE_CONFIG_PATH=<config_path>`
    With `<config_path>` as the folder path to where the `default.toml` is located.
3. Put a [`default.toml`](application/btsieve/config/default.toml) config file into `~/.config/btsieve` or set `BTSIEVE_CONFIG_PATH` as folder path to where the `default.toml` is located
4. startup bitcoin node (port to be set according to btsieve configuration)
5. startup ethereum node (port to be set according to btsieve configuration)
6. startup btsieve: `cargo run --bin btsieve`
7. startup comit_node: `cargo run --bin comit_node`

If the `[web_gui]` section is specified in the configuration the current release of the user interface [comit-i](https://github.com/comit-network/comit-i) will be served once the comit node started up (served at `localhost:8080` as default).

In order to do a swap you will have to start two comit nodes. 

## Setup testing/dev environment

1. Install `docker` & `docker-compose`
2. Install `node` (check the version required in package.json) & `yarn`
3. Install `cargo-make`: `cargo install cargo-make`
4. Run `cargo make` in the root folder of the repository, this will install various crates & tools such as rustfmt & clippy
   

## Testing

- `cargo make` runs the whole test suite including integration tests but not end-to-end.
- `cargo make all` also runs the whole test suite, including end-to-end tests.
- `cargo make format` to format Rust code
- `cargo make js-format` to format JavaScript code
- `cargo make btsieve` to run btsieve tests
- `cargo make dry` to run COMIT node dry tests
- `cargo make api` to run all API tests
- `cargo make e2e` to run COMIT node end-to-end tests
- `cargo make e2e *btc*` to run COMIT node end-to-end tests with `btc` in the folder name (supports shell glob)




## Contributing

Contributions are welcome, please visit [CONTRIBUTING](CONTRIBUTING.md) for more details.

## License

This project is licensed under the terms of the [GNU GENERAL PUBLIC LICENSE v3](LICENSE.md).
