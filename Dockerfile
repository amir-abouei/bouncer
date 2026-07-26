FROM rust:1-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release || true

COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    touch src/main.rs && cargo build --release && cp target/release/bouncer /app/bouncer

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/bouncer /app/bouncer

ENV LISTEN_ADDR=0.0.0.0:8080
ENV ALLOWLIST_PATH=/app/config/allowlist.toml
ENV RUST_LOG=info

EXPOSE 8080

ENTRYPOINT ["/app/bouncer"]
