# TenX SWAP

Trustless, easy trading through atomic swaps.

## Structure

The system consists of three parts:

- `exchange_service`
- `trading_service`
- `treasury_service` (this project only provides a fake)

## Setup

- Install `rustup`: `curl https://sh.rustup.rs -sSf | sh`
- Run `setup.sh` to install the necessary toolchains
- Install `docker` & `docker-compose`
- Use cargo as you know it

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
