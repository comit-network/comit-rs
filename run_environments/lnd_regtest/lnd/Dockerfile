FROM golang:1.10-alpine as builder

LABEL maintainer.0="Philipp Hoenisch philipp@tenx.tech" \
  maintainer.1="Thomas Eizinger thomas@tenx.tech" \
  maintainer.2="Franck Royer franck.royer@tenx.tech" \
  maintainer.3="Lloyd Fournier lloyd.fournier@coblox.tech"

RUN apk add --no-cache \
    git \
    make

# Force Go to use the cgo based DNS resolver. This is required to ensure DNS
# queries required to connect to linked containers succeed.
ENV GODEBUG netdns=cgo

RUN go get -u github.com/golang/dep/cmd/dep
RUN go get -d github.com/lightningnetwork/lnd
RUN cd $GOPATH/src/github.com/lightningnetwork/lnd \
 && git checkout v0.4.2-beta \
 && make \
 && make install

# Start a new, final image to reduce size.
FROM alpine as final

# Expose lnd ports (server, rpc).
EXPOSE 9735 10009

# Copy the binaries and entrypoint from the builder image.
COPY --from=builder /go/bin/lncli /bin/
COPY --from=builder /go/bin/lnd /bin/

# Add bash.
RUN apk add --no-cache \
    bash

# Copy the entrypoint script.
COPY start-lnd.sh .
RUN chmod +x start-lnd.sh
