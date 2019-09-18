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
RUN cargo build -p cnd --release

FROM debian:buster

RUN apt-get update && \
    apt-get install -y \
    tini \
    libsqlite3-dev \
 && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash cnd
USER cnd

COPY --from=builder /src/target/release/cnd /usr/local/bin

EXPOSE 9939
EXPOSE 8000

ENTRYPOINT ["tini"]
CMD ["cnd"]
