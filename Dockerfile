# Build image.

FROM rust:1.89 AS builder
LABEL maintainer="Benjamin Bouvier <benjamin@bouvier.cc>"

RUN mkdir -p /build/modules

COPY ./Cargo.* /build/
COPY ./src /build/src
COPY ./wit /build/wit
COPY ./modules /build/modules/

# Compile the host.
WORKDIR /build/src
RUN cargo build --release

# Set up protoc.
ENV PROTOC_ZIP=protoc-23.0-linux-x86_64.zip

RUN curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v23.0/$PROTOC_ZIP && \
    unzip -o $PROTOC_ZIP -d /usr/local bin/protoc && \
    unzip -o $PROTOC_ZIP -d /usr/local 'include/*' && \
    chmod +x /usr/local/bin/protoc && \
    rm -f $PROTOC_ZIP

ENV PROTOC=/usr/local/bin/protoc

WORKDIR /build/modules
RUN make install-tools && \
    rustup component add rustfmt && \
    rustup target add wasm32-unknown-unknown
RUN make release

# Actual image.
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates sqlite3 && \
    rm -rf /var/lib/apt/lists/* && \
    update-ca-certificates && \
    mkdir -p /opt/trinity/data && \
    mkdir -p /opt/trinity/modules/target/wasm32-unknown-unknown/release

COPY --from=builder /build/target/release/trinity /opt/trinity/trinity
COPY --from=builder \
    /build/modules/target/wasm32-unknown-unknown/release/*.wasm \
    /opt/trinity/modules/target/wasm32-unknown-unknown/release

ENV MATRIX_STORE_PATH /opt/trinity/data/cache
ENV REDB_PATH /opt/trinity/data/db

VOLUME /opt/trinity/data

WORKDIR /opt/trinity
CMD /opt/trinity/trinity
