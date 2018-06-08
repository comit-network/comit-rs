FROM ubuntu:18.04 as build

RUN apt update
RUN apt install curl build-essential libssl-dev pkg-config software-properties-common -y
RUN add-apt-repository ppa:ethereum/ethereum
RUN apt install solc -y

RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly-2018-05-05-x86_64 -y
ENV PATH=/root/.cargo/bin:$PATH
ENV SOLC_BIN=solc
WORKDIR /source
COPY dependency_cache /source
COPY Cargo.lock /source
RUN cargo build --release
RUN rm -rf src
COPY . /source
RUN cargo build --release
WORKDIR /source/target/release