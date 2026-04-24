# syntax=docker/dockerfile:1.7

FROM rust:slim-bookworm AS builder
WORKDIR /src

RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked \
 && cp target/release/anubot /usr/local/bin/anubot

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && mkdir -p /data

COPY --from=builder /usr/local/bin/anubot /usr/local/bin/anubot

WORKDIR /data
VOLUME ["/data"]
EXPOSE 8080
ENV RUST_LOG=info
CMD ["anubot"]
