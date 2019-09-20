FROM rust:1.37-buster as builder

WORKDIR /src
COPY ./btsieve ./btsieve
COPY ./internal ./internal
COPY ./cnd ./cnd
COPY ./libp2p-comit ./libp2p-comit
COPY ./blockchain_contracts ./blockchain_contracts
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

RUN cargo check
RUN cargo build -p btsieve --release

FROM debian:buster

RUN apt-get update && \
    apt-get install -y \
    tini \
 && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash btsieve
USER btsieve

COPY --from=builder /src/target/release/btsieve /usr/local/bin
COPY .docker/btsieve/btsieve.toml /home/btsieve/.config/comit/btsieve.toml

EXPOSE 8181

ENTRYPOINT ["tini"]
CMD ["btsieve"]
