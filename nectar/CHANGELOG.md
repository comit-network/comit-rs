# Changelog `nectar`

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

**This release includes breaking changes to the DB schema. You will need to delete your database before upgrading.**

### Fixed

-   Correct a bug that would reset the bitcoin transient key index and the active peers when starting nectar.

### Added

-   New command to archive swaps: `nectar archive-swap <swap id>`.
    The command should only be used while nectar is stopped as it updates the database.
    Archiving a swap stops nectar to resume its automated execution, `create-transaction` command can then be used to recover funds.
-   New command to migrate the database to a new format;
    Use `nectar migrate-db status` to check if migration is needed;
    If so, backup your data and then execute `nectar migrate-db run` to proceed with the migration.
-   Add an optional `fund_amount` parameter to the `create-transaction` command.
    This allows users to redeem/refund `hbit` HTLCs that were funded with amounts different from what as agreed through the orderbook.

### Changed

-   Only store transaction IDs instead of full transactions in the database.
    This is a breaking change.

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
