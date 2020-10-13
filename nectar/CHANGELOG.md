# Changelog `nectar`

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

-   Ability to configure strategies to be used for Bitcoin fees and Ethereum Gas Price resolution.
    See `./sample-config.toml` for more details.
-   Disallow unknown keys in the config file.
Previously, unknown configuration keys would just be ignored.
`nectar` will now refuse to startup if the configuration file contains unknown keys.

### Changed

-   Update the expected network times to calculate the expiries: We expect Bitcoin's transactions to be included within 6 blocks and Ethereum's within 30 blocks.
-   By default, use bitcoind's `estimatsmartfee` feature to estimate Bitcoin fees.
For Ethereum, Eth Gas Station API is used.
-   Change log level configuration format from capitalised (e.g. "Debug") to lowercase (e.g. "debug").

[Unreleased]: https://github.com/comit-network/comit-rs/compare/d5d26e42a2d8dd026ae94f2c8a9e0bd80ab81133...HEAD
