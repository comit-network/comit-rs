# Changelog `nectar`

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

**This release includes changes to the DB schema, we recommend to backup your data directory before upgrading.**

### Added

-   New command to archive swaps: `nectar archive-swap <swap id>`.
    The command should only be used while nectar is stopped as it updates the database.
    Archiving a swap stops nectar to resume its automated execution, `create-transaction` command can then be used to recover funds.

## [nectar-0.1.0] - 2020-10-20

### Added

-   Ability to configure strategies to be used for Bitcoin fees and Ethereum Gas Price resolution.
    See `./sample-config.toml` for more details.
-   Disallow unknown keys in the config file.
    Previously, unknown configuration keys would just be ignored.
    `nectar` will now refuse to startup if the configuration file contains unknown keys.

### Changed

-   Update the expected network times to calculate the expiries: We expect Bitcoin's transactions to be included within 6 blocks and Ethereum's within 30 blocks.
-   By default, use bitcoind's `estimatesmartfee` feature to estimate Bitcoin fees.
    For Ethereum, Eth Gas Station API is used.
-   Change log level configuration format from capitalised (e.g. "Debug") to lowercase (e.g. "debug").

[Unreleased]: https://github.com/comit-network/comit-rs/compare/nectar-0.1.0...HEAD

[nectar-0.1.0]: https://github.com/comit-network/comit-rs/compare/b4ad16d63579c542a3885d57f0522b445cfa8bae...nectar-0.1.0
