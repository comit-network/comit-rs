# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- All the code since the dawn of comit-rs.
- Check if btsieve's version matches the expected version, on every request.
- Ping btsieve on cnd startup, checking for presence and version compatibility.

### Changed
- Move config files to standard location based on platform (OSX, Windows, Linux).
- Align implementation with RFC-002 to use the decision header instead of status codes

[Unreleased]: https://github.com/comit-network/comit-rs/compare/1625533e04119e8496b14d5e18786f150b4fce4d...HEAD
