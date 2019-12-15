# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
- Write all diagnostics and log messages to stderr.

## [0.5.0] - 2019-12-06

### Added
- Added persistent storage to cnd, now we save swaps to an Sqlite database when they are requested and accepted.

## [0.4.0] - 2019-11-26

### Changed
- **Breaking (HTTP+COMIT API):** Change the identity for the Bitcoin Ledger from a public key hash to a public key. This change impacts the HTTP and the COMIT API of cnd.
- **Breaking (COMIT API):**  Replace Ethereum `network` with Ethereum `chain_id`
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

[Unreleased]: https://github.com/comit-network/comit-rs/compare/0.5.0...HEAD
[0.5.0]: https://github.com/comit-network/comit-rs/compare/0.4.0...0.5.0
[0.4.0]: https://github.com/comit-network/comit-rs/compare/0.3.0...0.4.0
[0.3.0]: https://github.com/comit-network/comit-rs/compare/0.2.1...0.3.0
[0.2.1]: https://github.com/comit-network/comit-rs/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/comit-network/comit-rs/compare/b2dd02a7f93dc82f5cc9fd4b6eaaf54de1459ff6...40116c3e8a9f57a213661917b8cc057e1db60755
[0.1.0]: https://github.com/comit-network/comit-rs/compare/1625533e04119e8496b14d5e18786f150b4fce4d...b2dd02a7f93dc82f5cc9fd4b6eaaf54de1459ff6
