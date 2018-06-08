FROM ubuntu:18.04 as build

RUN apt update
RUN apt install curl build-essential libssl-dev pkg-config software-properties-common -y
RUN add-apt-repository ppa:ethereum/ethereum
RUN apt install solc -y

RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly-2018-05-05-x86_64 -y
ENV PATH=/root/.cargo/bin:$PATH
ENV SOLC_BIN=solc
WORKDIR /source
ENV SRC_FOLDERS="bitcoin_rpc ethereum_htlc trading_service bitcoin_wallet ethereum_wallet ganache_rust_web3 bitcoin_htlc common_types exchange_service jsonrpc fake_treasury_service trading_client"
RUN for folder in $SRC_FOLDERS; do mkdir $folder; mkdir $folder/src; touch $folder/src/lib.rs; done
COPY bitcoin_rpc/Cargo.toml /source/bitcoin_rpc/Cargo.toml
COPY ethereum_htlc/Cargo.toml /source/ethereum_htlc/Cargo.toml
COPY trading_service/Cargo.toml /source/trading_service/Cargo.toml
COPY bitcoin_wallet/Cargo.toml /source/bitcoin_wallet/Cargo.toml
COPY ethereum_wallet/Cargo.toml /source/ethereum_wallet/Cargo.toml
COPY ganache_rust_web3/Cargo.toml /source/ganache_rust_web3/Cargo.toml
COPY bitcoin_htlc/Cargo.toml /source/bitcoin_htlc/Cargo.toml
COPY common_types/Cargo.toml /source/common_types/Cargo.toml
COPY exchange_service/Cargo.toml /source/exchange_service/Cargo.toml
COPY jsonrpc/Cargo.toml /source/jsonrpc/Cargo.toml
COPY fake_treasury_service/Cargo.toml /source/fake_treasury_service/Cargo.toml
COPY trading_client/Cargo.toml /source/trading_client/Cargo.toml
COPY Cargo.toml /source/Cargo.toml
COPY Cargo.lock /source/Cargo.lock
RUN cargo fetch
COPY . /source
RUN cargo build --release
WORKDIR /source/target/release