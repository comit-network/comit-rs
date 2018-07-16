# TenX SWAP

Trustless, easy trading through atomic swaps.

## Structure

The repository contains two main folders: `vendor` and `application`.

### Vendor

Contains crates that provides some kind of general functionality that is not specific to the domain of atomic swaps. Crates defined in here MUST NOT depend on crates in `application`. They may be separated out of the repository at some point (and possibly released on crates.io).

### Application

Contains crates specific to our application. Can depend on stuff in `vendor`.

## Setup

- Install `rustup`: `curl https://sh.rustup.rs -sSf | sh`
- Run `setup.sh` to install the necessary toolchains
- Install `docker` & `docker-compose`
- Use cargo as you know it

### Configuration

Cryptocurrency keys and addresses needs to be passed as environment variables.
Please note, `0x` prefix is never needed.
The following variables need to be set:
* `ETHEREUM_NODE_ENDPOINT` (url to ethereum node)
* `ETHEREUM_PRIVATE_KEY` (used by exchange to deploy contract)
* `ETHEREUM_EXCHANGE_ADDRESS` (must be derived from ETHEREUM_PRIVATE_KEY)
* `EXCHANGE_REFUND_ADDRESS` (to receive ETH back in case of timeout)
* `BITCOIN_RPC_URL` (used by both)
* `BITCOIN_RPC_USERNAME` (used by both)
* `BITCOIN_RPC_PASSWORD` (used by both)
* `EXCHANGE_SUCCESS_ADDRESS` (used by exchange to receive BTC)
* `EXCHANGE_SUCCESS_PRIVATE_KEY` (used by exchange to redeem BTC)

IF you wish to run the tests, you need to save this values in Docker env_file format (VAR=VAL) in several files.
- regtest.env: to run systemstests/happy_path.sh
- testnet.env: to run scripts/testnet/*.sh
Save these files in the same folder (let's say ~/swap_env) and set the path in `$SWAP_ENV`:
`export SWAP_ENV=$HOME/swap_env`

The following variables are also needed to run automated tests:
* `client_refund_address` (BTC)
* `client_success_address` (ETH)
* `client_sender_address` (ETH, when redeem the ETH HTLC)

## Testing

- `run_tests.sh` runs the whole test suite including integration tests. 

## Running

The most convenient way to run the applications is through `docker-compose`.

- `create_docker_base_image.sh` will create you a base image that allows for fast, incremental rebuilds of the application docker images.
- Each application has its own `Dockerfile` that builds on top of this base image
- `docker-compose up` will run the whole system, ready to be tested.

### Under the hood

The base image caches the compilation of the binaries and all its dependencies. If you have the feeling that the caches kind of stalls and upon building the docker images, cargo has to rebuild too much stuff because for example, some dependencies changed since you built the base image, just rebuild it. The script will retag the new container and your "cache" is up to date again!

## Contributing

- Follow these commit guidelines: https://chris.beams.io/posts/git-commit/
- Always run [`cargo fmt`](https://github.com/rust-lang-nursery/rustfmt) as PART of your change. Do not format the code after you are done, as this makes the history useless (git blame will only show the formatting commit).
