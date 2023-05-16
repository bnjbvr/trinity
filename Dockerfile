# Build image.

FROM rust:1.68 AS builder
LABEL maintainer="Benjamin Bouvier <public@benj.me>"

RUN mkdir -p /build/modules

COPY ./Cargo.* /build/
COPY ./src /build/src
COPY ./wit /build/wit
COPY ./modules /build/modules/

# Compile the host.
WORKDIR /build/src
RUN cargo build --release

# Install the pinned version of cargo-component.
WORKDIR /build/modules
RUN ./install-cargo-component.sh && \
    rustup component add rustfmt
RUN cargo component build --target=wasm32-unknown-unknown --release

# Actual image.
FROM debian:bullseye-slim

RUN apt-get update && \
    apt-get install -y ca-certificates && \
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
