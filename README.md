# COMIT-rs

[![Build Status](https://travis-ci.com/comit-network/comit-rs.svg?branch=master)](https://travis-ci.com/comit-network/comit-rs)
[![Mergify Status](https://gh.mergify.io/badges/comit-network/comit-rs.png?style=small)](https://mergify.io)
[![Waffle.io - Columns and their card count](https://badge.waffle.io/comit-network/comit-rs.svg?columns=all)](https://waffle.io/comit-network/comit-rs)

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
2. Install SSL libraries
   - Ubuntu/Debian: `apt install libssl-dev`
   - Mac ([Homebrew](https://brew.sh/)) `brew install openssl`
3. Install libzmq:
   - Ubuntu/Debian: `apt install libzmq3-dev`
   - Mac ([Homebrew](https://brew.sh/)) `brew install zeromq`
4. Install `docker` & `docker-compose`
5. Install `nvm`
6. Install `cargo-make`: `cargo install cargo-make`
7. Run `cargo make` in the root folder of the repository, this will install various crates & tools such as rustfmt & clippy

## Testing

- `cargo make` runs the whole test suite including integration tests but not end-to-end.
- `cargo make all` also runs the whole test suite, including end-to-end tests.
- `cargo make format` to format Rust code
- `cargo make js-format` to format JavaScript code
- `cargo make lqs` to run Ledger Query Service
- `cargo make dry` to run COMIT node dry tests
- `cargo make e2e` to run COMIT node end-to-end tests
- `cargo make e2e *btc*` to run COMIT node end-to-end tests with `btc` in the folder name (supports shell glob)


## Configuration

Put a [`default.toml`](application/comit_node/config/default.toml) config file into `~/.config/comit_node` or set `COMIT_NODE_CONFIG_PATH` to wherever the config file is located.  

## Contributing

Contributions are welcome, please visit [CONTRIBUTING](CONTRIBUTING.md) for more details.

## License

This project is licensed under the terms of the [GNU GENERAL PUBLIC LICENSE v3](LICENSE.md).
