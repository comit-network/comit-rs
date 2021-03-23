# Archive note

This repository has been archived because it is not actively maintained.
For more details about why, see the [blog post](https://comit.network/blog/2020/11/09/project-feast-closing-notes) about the project's closing.

However, this doesn't mean that we've stopped on working on COMIT!
To get on top of what is happening currently, join our [Matrix channel](https://matrix.to/#/#comit:matrix.org) or checkout some of our [other repositories](https://github.com/comit-network?q=&type=source&language=&sort=).

# Old readme

<a href="https://comit.network">
<img src="https://comit.network/img/comit-logo-black.svg" height="120px" alt="COMIT logo" />
</a>

---

[COMIT](https://comit.network) is an open protocol facilitating cross-blockchain applications.
For example, with [COMIT](https://comit.network) you can exchange Bitcoin for Ether or any ERC20 token directly with another person.

This repository contains the reference implementation of the protocol written in Rust.

![GitHub Action CI on dev](https://github.com/comit-network/comit-rs/workflows/CI/badge.svg?branch=dev)
[![Safety Dance](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Bors enabled](https://bors.tech/images/badge_small.svg)](https://app.bors.tech/repositories/20717)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Matrix chat](https://img.shields.io/badge/chat-on%20matrix-brightgreen?style=flat&logo=matrix)](https://app.element.io/#/room/#comit:matrix.org)

# comit-rs

The Rust reference implementation of the COMIT protocol (comit-rs) implements atomic swaps using constructs like Hash Time-Locked Contracts (HTLCs) to keep your funds safe at any time.

## Structure

This repository is a cargo workspace:

- `cnd`: a non-custodial COMIT network daemon, designed to be driven by a client with wallets (f.e. [Ambrosia](https://github.com/comit-network/ambrosia/))
- `nectar`: a custodial COMIT network daemon, designed for automated trades
- `comit`: a library implementing primitives of the COMIT protocol like libp2p-protocols, the decentralized orderbook and locking- as well as swap-protocols

`cnd` and `nectar` are [released](https://github.com/comit-network/comit-rs/releases) as binaries.

The `comit` library will be released to crates.io once its interface stabilizes.

## Setup build environment

All you need is ~love~ Rust: `curl https://sh.rustup.rs -sSf | sh`

### Build binaries

- `cargo build --release --package cnd`
- `cargo build --release --package nectar`

### Run binaries

Both, `cnd` and `nectar` require a connection to a Bitcoin and an Ethereum full node.
All config file options have sensible defaults but can also be overridden.
Run `cnd dump-config` or `nectar dump-config` for more information.

## Setup testing/dev environment

1. Install `docker`,
2. Install `node` (check the version required in tests/package.json) & `yarn`,
3. Run `make` in the root folder of the repository, this will install various crates & tools such as clippy.

## Testing

- `make test` is just a wrapper around `cargo test --all`
- `make e2e` will run all the end-to-end tests

To run individual end-to-end tests, use `yarn` inside the `tests` folder:

- `yarn run test`: run all tests
- `yarn run test <directory>`: run all tests in the directory
- `yarn run test <path to test file>`: run all tests in this test file, supports shell glob on the path
- `yarn run fix`: run prettier and linter to fix format
- `yarn run check`: run tsc (to check validity of TypeScript code) and verify format

## Cnd over Tor

If you would like to maintain anonymity while using `cnd` for atomic swaps we support running `cnd` over Tor.
You will need to configure an onion service (previously hidden service), virtual port can be anything but `cnd` expects the local port to be 9939.

```
HiddenServiceDir /var/lib/tor/hidden_service/
HiddenServicePort 9939 127.0.0.1:9939
```

After starting Tor for the first time get the onion service address from your local file system (e.g. `/var/lib/tor/hidden_service/hostname`) and add it to your cnd config file.

```
[network]
listen = ["/onion3/vww6ybal4bd7szmgncyruucpgfkqahzddi37ktceo3ah7ngmcopnpyyd:9939"]
```

All `cnd` traffic will now be routed over the Tor network.

## Contributing

Contributions are welcome, please visit [CONTRIBUTING](CONTRIBUTING.md) for more details.

If you have any question please [reach out to the team in our Matrix channel](https://app.element.io/#/room/#comit:matrix.org)!

## License

This project is licensed under the terms of the [GNU GENERAL PUBLIC LICENSE v3](LICENSE.md).
