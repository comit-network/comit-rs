# COMIT-rs

COMIT is an open protocol facilitating trustless cross-blockchain applications.
This is a reference implementation for the COMIT protocol. 

## WARNING - Do not use this code in production or you will regret it!!!

## Structure

The repository contains two main folders: `vendor` and `application`.

### Vendor

Contains crates that provides general functionality that is not specific to the domain of atomic swaps. 
Crates defined in here MUST NOT depend on crates in `application`. 
They may be separated out of the repository at some point (and possibly released on crates.io).

### Application

Contains crates specific to our application. Can depend on libraries located in `vendor`.

## Setup

- Install `rustup`: `curl https://sh.rustup.rs -sSf | sh`
- Run `setup.sh` to install the necessary toolchains
- Install `docker` & `docker-compose` & `nvm` (needed for testing)
- Use cargo as you know it

### Configuration

The following variables need to be set:
* `COMIT_NODE_CONFIG_PATH` - the path to a folder containing COMIT Node config files
   * Examples can be found in `./application/comit_node/config`

IF you wish to run the tests, you need to set up the test environment accordingly. 
Examples can be found in: 
* Regtest: `api_test/e2e/regtest/regtest.env`

## Testing

- `cargo make` runs the whole test suite including integration tests but not end-to-end.
- `cargo make all` also runs the whole test suite, including end-to-end tests. 
- `cargo make e2e` only runs end-to-end tests.

## Running
### Do not use this code in production or you will regret it!!!
Hence, we do not provide running instructions for now!

## Contributing

Contributions are welcome, please see [CONTRIBUTING](CONTRIBUTING.md) for more details

## License

This project is licensed under the terms of the [GNU GENERAL PUBLIC LICENSE v3](LICENSE.md).
