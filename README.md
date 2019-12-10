# COMIT-rs

[![CircleCI](https://circleci.com/gh/comit-network/comit-rs.svg?style=svg)](https://circleci.com/gh/comit-network/comit-rs)
[![Safety Dance](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Bors enabled](https://bors.tech/images/badge_small.svg)](https://app.bors.tech/repositories/20717)
[![Gitter chat](https://badges.gitter.im/gitterHQ/gitter.png)](https://gitter.im/comit-network/community)

COMIT is an open protocol facilitating trustless cross-blockchain applications.
This is a reference implementation for the COMIT protocol. 

## DISCLAIMER: This is not mainnet ready!

- Extensive testing on mainnet from our side still remains to be done
- Several features for achieving production-ready robustness are still under works
- Once we consider comit-rs production ready, we will release version 1.0.0

## Structure

This repository is a cargo workspace:

- `cnd`: implementation of the comit-network daemon
- `libp2p-comit`: implementation of the comit messaging protocol on top of libp2p

## Setup build environment

All you need is ~love~ rust: `curl https://sh.rustup.rs -sSf | sh` 

## Build & Run

1. `make install`
2. startup bitcoin node with `-regtest` and `-rest`
3. startup ethereum node with JSON-RPC interface exposed at `localhost:8545`
4. startup cnd: `cnd` (or `./target/release/cnd` if you do not have the `~/.cargo/bin` folder in your `$PATH`)

Keep in mind that in order to do a swap locally you will need to start two instances of cnd.
Please see `cnd --help` for help with command line options.

## Setup testing/dev environment

1. Install `docker`
2. Install `node` (check the version required in api_tests/package.json) & `yarn`
3. Run `make` in the root folder of the repository, this will install various crates & tools such as clippy
   
## Testing

- `make test` is just a wrapper around `cargo test --all`
- `make e2e` will run all the end-to-end tests

To run individual end-to-end tests, use `yarn` inside the `api_tests` folder:
- `yarn run test`: run all tests
- `yarn run test <directory>`: run all tests in the directory
- `yarn run test <path to test file>`: run all tests in this test file, supports shell glob on the path
- `yarn run fix`: run prettier and linter to fix format
- `yarn run check`: run tsc (to check validity of TypeScript code) and verify format

## Contributing

Contributions are welcome, please visit [CONTRIBUTING](CONTRIBUTING.md) for more details.

If you have any question please [reach out to the team in our Gitter chat](https://gitter.im/comit-network/community)!

## License

This project is licensed under the terms of the [GNU GENERAL PUBLIC LICENSE v3](LICENSE.md).
