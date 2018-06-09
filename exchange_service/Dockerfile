FROM tenx-tech/swap:latest

COPY . /source/exchange_service
WORKDIR /source/exchange_service
RUN cargo fetch
RUN cargo build

WORKDIR /source/target/debug

CMD ./exchange_service