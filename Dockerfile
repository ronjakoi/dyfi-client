FROM rust:1-slim AS builder
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get -qq update && \
    apt-get -qq -y upgrade

WORKDIR /workdir
COPY Cargo.lock Cargo.toml  ./
COPY start.sh ./
COPY src/ ./src
RUN cargo build --release -q

FROM debian:stable-slim
RUN apt-get -qq update && \
    apt-get -qq -y upgrade && \
    apt-get -qq -y install ca-certificates libssl1.1 && \
    rm -rf /var/lib/apt/lists/*
RUN useradd -r -U dyfi

WORKDIR /app

COPY --from=builder --chown=dyfi /workdir/target/release/dyfi-client ./
USER hyapi

ENTRYPOINT ["/app/start.sh"]
