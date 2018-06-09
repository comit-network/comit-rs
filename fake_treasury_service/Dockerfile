FROM tenx-tech/swap:latest

COPY . /source/fake_treasury_service
WORKDIR /source/fake_treasury_service
RUN cargo fetch
RUN cargo build

WORKDIR /source/target/debug

CMD ./fake_treasury_service