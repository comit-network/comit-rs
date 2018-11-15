# TenX SWAP

Trustless, easy trading through atomic swaps.

## Structure

The repository contains two main folders: `vendor` and `application`.

### Vendor

Contains crates that provides some kind of general functionality that is not specific to the domain of atomic swaps. Crates defined in here MUST NOT depend on crates in `application`. They may be separated out of the repository at some point (and possibly released on crates.io).

### Application

Contains crates specific to our application. Can depend on libraries located in `vendor`.

## Setup build environment

- Install `rustup`: `curl https://sh.rustup.rs -sSf | sh`
- Install `cargo-make`: `cargo install cargo-make`
- Install `docker` & `docker-compose`
- Install `nvm`

## Testing

- `cargo make` runs the whole test suite including integration tests but not end-to-end.
- `cargo make all` also runs the whole test suite, including end-to-end tests. 
- `cargo make e2e` only runs end-to-end tests.

## Configuration

Put a [`default.toml`](application/comit_node/config/default.toml) config file into `~/.config/comit_node` or set `COMIT_NODE_CONFIG_PATH` to wherever the config file is located.  

## Contributing

Contributions are welcome, please see [CONTRIBUTING](CONTRIBUTING.md) for more details
