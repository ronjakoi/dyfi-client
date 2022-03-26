FROM rust:1-slim AS builder
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get -qq update && \
    apt-get -qq -y upgrade && \
	apt-get -qq -y install libssl-dev pkg-config

WORKDIR /workdir
COPY Cargo.lock Cargo.toml  ./
COPY src/ ./src
RUN rustc --version && cargo build --release
RUN cargo test --release

FROM debian:stable-slim
RUN apt-get -qq update && \
    apt-get -qq -y upgrade && \
    apt-get -qq -y install ca-certificates libssl1.1 && \
    rm -rf /var/lib/apt/lists/*
RUN useradd -r -U dyfi

WORKDIR /app

COPY --from=builder /workdir/target/release/dyfi-client ./
USER dyfi

ENTRYPOINT ["/app/dyfi-client"]
