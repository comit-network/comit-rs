# Changelog `cnd`

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `create-transaction` sub-command: Create a signed transactions for redeeming or refunding `hbit` protocols.
- `print-secret` sub-command: Prints the secret of a swap IF the node acts in the role of Alice for this swap.

### Changed

- Change log level configuration format from capitalised (e.g. "Debug") to lowercase (e.g. "debug").

### Removed

- Support for Lightning-based swaps.
- Endpoints for directly creating swaps.
  Users are encouraged to migrate to the orderbook-based API or write their own daemon based on the `comit` lib if they need more fine-grained control.
- The "refund" action from the REST API.
  `cnd` will no longer recommend when to refund a specific swap.
  By default, all orders are created in the role of `Alice` and hence there is no risk in losing funds.
  If necessary, users are encouraged to use the `create-transaction` and `print-secret` CLI commands to obtain the necessary transactions / data to perform a manual refund.

## [cnd-0.9.0] - 2020-10-12

### Added

- A set of API endpoints that expose the decentralized orderbook functionality.
- Ability to configure the Bitcoin fee rate with a static value for the redeem and refund transactions.
- Disallow unknown keys in the config file.
  Previously, unknown configuration keys would just be ignored.
  `cnd` will now refuse to startup if the configuration file contains unknown keys.

### Changed

- **Breaking Change** Remove support for RFC003 swaps
- **Breaking Change** Config directory for MacOS changed from `/Users/<user>/Library/Preferences/comit/` to `/Users/<user>/Library/Application Support/comit/`.
- Update the expected network times to calculate the expiries: We expect Bitcoin's transactions to be included within 6 blocks and Ethereum's within 30 blocks.
- Use CypherBlock Bitcoin fee estimation service for redeem and refund transactions.

## [0.8.0] - 2020-06-12

### Fixed

- Fix windows build.

### Changed

- **Breaking Change**: Rename `parity` to `geth` in the configuration file as we are only testing the software with Geth as part of the CI.

### Added

- Support for `herc20-halbit`, `halbit-herc20`, `hbit-herc20` and `herc20-hbit` swaps.

## [0.7.3] - 2020-04-14

### Fixed

- Deserialization problem of bitcoind's response: Bitcoind returns as chain/network either `main`,`test` or `regtest`. This fix was manually tested against bitcoind (0.17, 0.18 and 0.19).

### Changed

- Ensure that lnd parameters are defaulted if not present.

## [0.7.2] - 2020-03-26

## [0.7.1] - 2020-03-12

### Fixed

- The release workflow for GitHub Actions was unfortunately broken.

### Added

- Linting of GitHub Actions workflows to prevent silly mistakes in the future.

## [0.7.0] - 2020-03-12

### Added

- Added a step during initialisation to test whether the chain/network id's of the Ethereum/Bitcoin nodes that cnd has connected to matches the chain/network id's specified in the config. Cnd will abort if the config and the actual chain/network id does not match. Cnd will log a warning if it cannot make a request to the nodes.

### Changed

- **Breaking config changes**: cnd config has changed. Bitcoin and Ethereum has 2 optional fields specifically for the connector (i.e. bitcoind and parity). If provided, the network (for bitcoin) and chain_id (for ethereum) are mandatory. If the url was not provided, a default aiming at localhost will be derived. If no connectors were provided, defaults will be provided. For a full example config run: `cnd --dump-config`.

## [0.6.0] - 2020-02-13

### Fixed

- Ensure that failed Ethereum transactions are ignored during a swap.

### Changed

- Upgrade `blockchain-contracts` crate to 0.3.1. Ether and Erc20 HTLCs now use `revert` to fail the Ethereum transaction if the redeem or refund are unsuccessful.

## [0.5.1] - 2020-01-21

### Added

- Return Siren document containing peer ID, listen addresses and links to `/swaps` and `/swaps/rfc003` on `GET /` with the Accept request HTTP header set to `application/vnd.siren+json`.

### Changed

- Write all diagnostics and log messages to stderr.

## [0.5.0] - 2019-12-06

### Added

- Added persistent storage to cnd, now we save swaps to an Sqlite database when they are requested and accepted.
- Use stdlib `SocketAddr` as base for HTTP API config option.

## [0.4.0] - 2019-11-26

### Changed

- **Breaking (HTTP+COMIT API):** Change the identity for the Bitcoin Ledger from a public key hash to a public key. This change impacts the HTTP and the COMIT API of cnd.
- **Breaking (COMIT API):** Replace Ethereum `network` with Ethereum `chain_id`
- cnd no longer automatically generates a config file, but instead simply defaults to what it would have written to the file on first startup.
- Make expiries optional when sending a swap request, with defaults:
  - 24 hours later for alpha ledger.
  - 12 hours later for beta ledger.
- **Breaking (Config file format):** Expand `http_api` section in config file to contain both the socket and CORS settings.

### Added

- Return Ethereum `chain_id` on the HTTP API.
- Support Ethereum `chain_id` in the Swap Request (HTTP API).
- Ability to set CORS allowed origins through the configuration file.
- Added command line option `--dump-config` to print the running configuration to stdout.

### Fixed

- Error responses now properly identify themselves as `application/problem+json`. They have been conforming to this format for a while already, we just never set the `Content-Type` header properly. From now on, applications can fully rely on the error format!

## [0.3.0] - 2019-10-02

### Changed

- Embed btsieve as a library inside cnd: From now on, you'll only need to run cnd to use COMIT.

## [0.2.1] - 2019-09-24

### Changed

- Use the same Swap ID to identify a swap for both parties.

## [0.2.0] - 2019-09-13

### Changed

- Statically link openssl in the release build to allow the binaries to be ran out-of-the-box on most Linux distros.
- Replace ZMQ by using bitcoind's HTTP API for retrieving bitcoin blocks.

## [0.1.0] - 2019-09-05

### Added

- All the code since the dawn of comit-rs.
- Check if btsieve's version matches the expected version, on every request.
- Ping btsieve on cnd startup, checking for presence and version compatibility.

### Changed

- Move config files to standard location based on platform (OSX, Windows, Linux).
- Align implementation with RFC-002 to use the decision header instead of status codes.

[Unreleased]: https://github.com/comit-network/comit-rs/compare/cnd-0.9.0...HEAD
[cnd-0.9.0]: https://github.com/comit-network/comit-rs/compare/0.8.0...cnd-0.9.0
[0.8.0]: https://github.com/comit-network/comit-rs/compare/0.7.3...0.8.0
[0.7.3]: https://github.com/comit-network/comit-rs/compare/0.7.2...0.7.3
[0.7.2]: https://github.com/comit-network/comit-rs/compare/0.7.1...0.7.2
[0.7.1]: https://github.com/comit-network/comit-rs/compare/0.7.0...0.7.1
[0.7.0]: https://github.com/comit-network/comit-rs/compare/0.6.0...0.7.0
[0.6.0]: https://github.com/comit-network/comit-rs/compare/0.5.1...0.6.0
[0.5.1]: https://github.com/comit-network/comit-rs/compare/0.5.0...0.5.1
[0.5.0]: https://github.com/comit-network/comit-rs/compare/0.4.0...0.5.0
[0.4.0]: https://github.com/comit-network/comit-rs/compare/0.3.0...0.4.0
[0.3.0]: https://github.com/comit-network/comit-rs/compare/0.2.1...0.3.0
[0.2.1]: https://github.com/comit-network/comit-rs/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/comit-network/comit-rs/compare/b2dd02a7f93dc82f5cc9fd4b6eaaf54de1459ff6...40116c3e8a9f57a213661917b8cc057e1db60755
[0.1.0]: https://github.com/comit-network/comit-rs/compare/1625533e04119e8496b14d5e18786f150b4fce4d...b2dd02a7f93dc82f5cc9fd4b6eaaf54de1459ff6
