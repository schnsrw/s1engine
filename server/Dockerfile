# s1-server — API Server Docker Image
#
# Usage:
#   docker build -t s1engine/server -f server/Dockerfile .
#   docker run -p 8080:8080 -v ./data:/data s1engine/server
#
# Environment variables:
#   S1_PORT=8080          Port to listen on
#   S1_STORAGE=local      Storage backend (local or memory)
#   S1_DATA_DIR=/data     Data directory for local storage

FROM rust:1.88-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY ffi/ ffi/
COPY server/ server/

# Build only the server binary in release mode
RUN cargo build --release -p s1-server

# ─── Runtime Stage ────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/s1-server /usr/local/bin/s1-server

# Create data directory
RUN mkdir -p /data

ENV S1_PORT=8080
ENV S1_STORAGE=local
ENV S1_DATA_DIR=/data

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["s1-server"]
