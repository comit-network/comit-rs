FROM tenx-tech/swap:latest

COPY . /source/comit_node
WORKDIR /source/comit_node
RUN cargo fetch
RUN cargo build

WORKDIR /source/target/debug

CMD ./comit_node