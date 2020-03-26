FROM debian:buster

RUN apt-get update && \
    apt-get install -y \
    tini libssl-dev \
 && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash cnd
USER cnd

COPY ./target/release/cnd /usr/local/bin

EXPOSE 9939
EXPOSE 8000

# tini allows us to CTRL+C our container if it runs in the foreground
ENTRYPOINT ["tini"]
CMD ["cnd"]
