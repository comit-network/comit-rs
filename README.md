<a href="https://comit.network">
<img src="logo.svg" height="120px" alt="COMIT logo" />
</a>

---

[COMIT](https://comit.network) is an open protocol facilitating cross-blockchain applications.
For example, with [COMIT](https://comit.network) you can exchange Bitcoin for Ether or any ERC20 token directly with another person.

This repository contains the implementation of the comit-network daemon (`cnd`), which is the reference implementation of the protocol written in Rust.

If you wish to do an atomic swap on your machine or to integrate COMIT into an application (e.g. a DEX) please take a look at the [Getting Started section](https://comit.network/docs/getting-started/create-comit-app/) of the COMIT documentation.
If you have any questions, feel free to [reach out to the team in our Gitter chat](https://gitter.im/comit-network/community)!

![GitHub Action CI on dev](https://github.com/comit-network/comit-rs/workflows/CI/badge.svg?branch=dev)
[![Safety Dance](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Bors enabled](https://bors.tech/images/badge_small.svg)](https://app.bors.tech/repositories/20717)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Matrix chat](https://img.shields.io/badge/chat-on%20matrix-brightgreen?style=flat&logo=matrix)](https://app.element.io/#/room/#comit:matrix.org)

# comit-rs

The Rust reference implementation of the COMIT protocol (comit-rs) implements atomic swaps using constructs like Hash Time-Locked Contracts (HTLCs) to keep your funds safe at any time.

## Structure

This repository is a cargo workspace:

- `cnd`: implementation of the comit-network daemon

`cnd` is [released](https://github.com/comit-network/comit-rs/releases) as binary.

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

1. Install `docker`,
2. Install `node` (check the version required in api_tests/package.json) & `yarn`,
3. Run `make` in the root folder of the repository, this will install various crates & tools such as clippy.

## Testing

- `make test` is just a wrapper around `cargo test --all`
- `make e2e` will run all the end-to-end tests

To run individual end-to-end tests, use `yarn` inside the `api_tests` folder:
- `yarn run test`: run all tests
- `yarn run test <directory>`: run all tests in the directory
- `yarn run test <path to test file>`: run all tests in this test file, supports shell glob on the path
- `yarn run fix`: run prettier and linter to fix format
- `yarn run check`: run tsc (to check validity of TypeScript code) and verify format

## Tor

If you would like to maintain anonymity while using cnd for atomic swaps we support running cnd over Tor.
You will need to configure an onion service (previously hidden service), virtual port can be anything but cnd expects the local port to be 9939.
```
HiddenServiceDir /var/lib/tor/hidden_service/
HiddenServicePort 9939 127.0.0.1:9939
```
After starting Tor for the first time get the onion service address from your local file system (e.g. `/var/lib/tor/hidden_service/hostname`) and add it to your cnd config file.
```
[network]
listen = ["/onion3/vww6ybal4bd7szmgncyruucpgfkqahzddi37ktceo3ah7ngmcopnpyyd:9939"]
```
All cnd traffic will now be routed over the Tor network.

## Contributing

Contributions are welcome, please visit [CONTRIBUTING](CONTRIBUTING.md) for more details.

If you have any question please [reach out to the team in our Gitter chat](https://gitter.im/comit-network/community)!

## License

This project is licensed under the terms of the [GNU GENERAL PUBLIC LICENSE v3](LICENSE.md).
