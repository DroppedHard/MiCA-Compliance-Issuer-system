FROM rust:1.98-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
RUN cargo build --release --bins

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/target/release/crypto-asset-backend /usr/local/bin/crypto-asset-backend
COPY --from=builder /build/target/release/mock-bank /usr/local/bin/mock-bank
RUN mkdir -p /app/data
CMD ["crypto-asset-backend"]
