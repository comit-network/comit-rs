FROM tenx-tech/swap:latest

COPY . /source/trading_service
WORKDIR /source/trading_service
RUN cargo fetch
RUN cargo build

WORKDIR /source/target/debug

CMD ./trading_service