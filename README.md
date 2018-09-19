# TenX SWAP

Trustless, easy trading through atomic swaps.

## Structure

The repository contains two main folders: `vendor` and `application`.

### Vendor

Contains crates that provides some kind of general functionality that is not specific to the domain of atomic swaps. Crates defined in here MUST NOT depend on crates in `application`. They may be separated out of the repository at some point (and possibly released on crates.io).

### Application

Contains crates specific to our application. Can depend on libraries located in `vendor`.

## Setup

- Install `rustup`: `curl https://sh.rustup.rs -sSf | sh`
- Run `setup.sh` to install the necessary toolchains
- Install `docker` & `docker-compose`
- Use cargo as you know it

### Configuration

Cryptocurrency keys and addresses needs to be passed as environment variables.
Please note, `0x` prefix is never needed.
The following variables need to be set:
* `COMIT_NODE_CONFIG_PATH` (the path to a folder containing COMIT Node config files, examples can be found in ./application/comit_node/config)

IF you wish to run the tests, you need to save this values in Docker env_file format (VAR=VAL) in several files.
- regtest.env: to run systemstests/happy_path.sh
Save these files in the same folder (let's say ~/swap_env) and set the path in `$SWAP_ENV`:
`export SWAP_ENV=$HOME/swap_env`

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

Contributions are welcome, please see [CONTRIBUTING](CONTRIBUTING.md) for more details
