# Rudra Office — Multi-stage Docker Build
#
# Builds WASM bindings, bundles the web editor, and runs the relay server.
#
# Usage:
#   docker build -t s1-editor .
#   docker run -p 8787:8787 s1-editor
#
# The resulting image serves:
#   - Static web editor at http://localhost:8787/
#   - WebSocket relay at ws://localhost:8787/
#   - Health check at http://localhost:8787/health

# ──────────────────────────────────────────────────────────
# Stage 1: Build WASM bindings from the Rust workspace
# ──────────────────────────────────────────────────────────
FROM rust:1.88-bookworm AS wasm-builder

# Install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# Copy dependency manifests first for layer caching.
# If only source files change, cargo won't re-download deps.
COPY Cargo.toml Cargo.lock ./

# Copy all crate manifests (creates the workspace structure so cargo can resolve)
COPY crates/s1-model/Cargo.toml crates/s1-model/Cargo.toml
COPY crates/s1-ops/Cargo.toml crates/s1-ops/Cargo.toml
COPY crates/s1-format-docx/Cargo.toml crates/s1-format-docx/Cargo.toml
COPY crates/s1-format-odt/Cargo.toml crates/s1-format-odt/Cargo.toml
COPY crates/s1-format-pdf/Cargo.toml crates/s1-format-pdf/Cargo.toml
COPY crates/s1-format-txt/Cargo.toml crates/s1-format-txt/Cargo.toml
COPY crates/s1-format-md/Cargo.toml crates/s1-format-md/Cargo.toml
COPY crates/s1-convert/Cargo.toml crates/s1-convert/Cargo.toml
COPY crates/s1-layout/Cargo.toml crates/s1-layout/Cargo.toml
COPY crates/s1-text/Cargo.toml crates/s1-text/Cargo.toml
COPY crates/s1-crdt/Cargo.toml crates/s1-crdt/Cargo.toml
COPY crates/s1engine/Cargo.toml crates/s1engine/Cargo.toml
COPY ffi/wasm/Cargo.toml ffi/wasm/Cargo.toml
COPY ffi/c/Cargo.toml ffi/c/Cargo.toml
COPY server/Cargo.toml server/Cargo.toml

# Create dummy source files so cargo can resolve the workspace and fetch deps
RUN for dir in \
      crates/s1-model/src \
      crates/s1-ops/src \
      crates/s1-format-docx/src \
      crates/s1-format-odt/src \
      crates/s1-format-pdf/src \
      crates/s1-format-txt/src \
      crates/s1-format-md/src \
      crates/s1-convert/src \
      crates/s1-layout/src \
      crates/s1-text/src \
      crates/s1-crdt/src \
      crates/s1engine/src \
      ffi/wasm/src \
      ffi/c/src; do \
    mkdir -p "$dir" && echo "" > "$dir/lib.rs"; \
    done && \
    mkdir -p server/src && echo "fn main() {}" > server/src/main.rs && \
    mkdir -p crates/s1engine/benches && \
    echo "fn main() {}" > crates/s1engine/benches/engine_bench.rs

# Pre-fetch and compile dependencies (cached unless Cargo.toml/Cargo.lock change)
RUN cargo fetch

# Now copy the real source code
COPY crates/ crates/
COPY ffi/ ffi/
COPY server/ server/

# Build WASM (release mode)
# Output goes to /app/wasm-pkg so the next stage can pick it up
RUN wasm-pack build ffi/wasm \
    --target web \
    --release \
    --out-dir /app/wasm-pkg

# ──────────────────────────────────────────────────────────
# Stage 2: Build the web editor with Vite
# ──────────────────────────────────────────────────────────
FROM node:20-bookworm-slim AS web-builder

WORKDIR /app/editor

# Install npm dependencies (cached unless package.json/lock change)
COPY editor/package.json editor/package-lock.json* ./
RUN npm ci --ignore-scripts 2>/dev/null || npm install

# Copy editor source
COPY editor/ .

# Copy WASM output from stage 1 into wasm-pkg/ (where main.js imports from)
COPY --from=wasm-builder /app/wasm-pkg ./wasm-pkg/

# Build production bundle
RUN npm run build

# ──────────────────────────────────────────────────────────
# Stage 3: Minimal runtime — Node.js relay + static files
# ──────────────────────────────────────────────────────────
FROM node:20-bookworm-slim AS runtime

# Install curl for healthcheck
RUN apt-get update && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the relay/static server
COPY scripts/relay.js ./relay.js

# Copy built web assets from stage 2
COPY --from=web-builder /app/editor/dist ./public/

# Create data directory for persistence
RUN mkdir -p /app/data

# Environment
ENV PORT=8787
ENV PUBLIC_DIR=/app/public
ENV DATA_DIR=/app/data
ENV NODE_ENV=production

EXPOSE 8787

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8787/health || exit 1

CMD ["node", "relay.js"]
