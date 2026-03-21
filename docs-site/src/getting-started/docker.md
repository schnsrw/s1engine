# Docker Deployment

This guide covers deploying Rudra Office using Docker, from a quick one-liner to a
full production stack with persistent storage, collaboration, and an AI writing assistant.

---

## Quick Start

Run the all-in-one image to get a working editor in under a minute:

```bash
docker run -d --name rudra -p 8080:8080 rudra/server:latest
```

Open [http://localhost:8080](http://localhost:8080) in your browser. The editor, REST API,
and WebSocket collaboration server are all served from a single container on a single port.

| Endpoint | URL |
|----------|-----|
| Editor UI | `http://localhost:8080/` |
| REST API | `http://localhost:8080/api/v1/` |
| WebSocket collab | `ws://localhost:8080/ws/edit/{file_id}` |
| Admin panel | `http://localhost:8080/admin/dashboard` |
| Health check | `http://localhost:8080/health` |

To persist documents across container restarts, mount a data volume:

```bash
docker run -d --name rudra \
  -p 8080:8080 \
  -v rudra-data:/data \
  rudra/server:latest
```

---

## Docker Images

Rudra Office publishes three image variants to Docker Hub.

### rudra/server (Unified)

The recommended image for most deployments. A single Rust binary that serves the
editor frontend, REST API, and WebSocket collaboration -- all on one port.

```bash
docker run -p 8080:8080 rudra/server
```

- **Base:** `debian:bookworm-slim`
- **Size:** ~40 MB (stripped Rust binary + static assets)
- **Ports:** 8080

### rudra/editor (Static Frontend Only)

A lightweight image that serves only the editor UI as static files via nginx.
Use this when you run the API server separately or want to serve the frontend
from a CDN.

```bash
docker run -p 80:80 rudra/editor
```

- **Base:** `nginx:alpine`
- **Size:** ~15 MB
- **Ports:** 80

### rudra/all-in-one (Server + AI Sidecar)

Includes the unified server and an AI writing assistant powered by a local LLM
(Qwen2.5-3B via llama.cpp). Suitable for air-gapped environments where cloud
AI APIs are not available.

```bash
docker compose -f docker-compose.yml up -d
```

- **AI model:** Qwen2.5-3B-Instruct (Q4_K_M quantization, ~2 GB)
- **Minimum RAM:** 6 GB (4 GB for AI, 2 GB for server)

---

## Image Tags

All image variants follow the same tagging convention:

| Tag | Description | Example |
|-----|-------------|---------|
| `latest` | Latest stable release | `rudra/server:latest` |
| `X.Y.Z` | Specific version (immutable) | `rudra/server:1.0.2` |
| `X.Y` | Latest patch within a minor release | `rudra/server:1.0` |
| `X` | Latest minor within a major release | `rudra/server:1` |
| `edge` | Latest commit on `main` (unstable) | `rudra/server:edge` |

For production deployments, always pin to a specific `X.Y.Z` tag.

---

## Docker Compose

### Minimal (Single Container)

For development and small teams. All state is stored on the local filesystem.

```yaml
services:
  rudra:
    image: rudra/server:1.0.2
    ports:
      - "8080:8080"
    environment:
      - S1_STORAGE=local
      - S1_DATA_DIR=/data
      - S1_ADMIN_USER=admin
      - S1_ADMIN_PASS=${ADMIN_PASSWORD:-changeme}
    volumes:
      - rudra-data:/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 15s

volumes:
  rudra-data:
```

### Production (Full Stack)

A production-grade deployment with PostgreSQL for document metadata, MinIO for
S3-compatible document storage, Redis for multi-instance collaboration routing,
and an optional AI sidecar.

```yaml
services:
  # ── Rudra Server ─────────────────────────────────────
  rudra-server:
    image: rudra/server:1.0.2
    ports:
      - "${S1_PORT:-8080}:8080"
    environment:
      S1_PORT: "8080"
      S1_STORAGE: "s3"
      S1_DATA_DIR: "/data"
      S1_STATIC_DIR: "/app/public"
      S1_STORAGE_POSTGRES_URL: "postgresql://rudra:${POSTGRES_PASSWORD}@postgres:5432/rudra"
      S1_STORAGE_S3_ENDPOINT: "http://minio:9000"
      S1_STORAGE_S3_BUCKET: "rudra-documents"
      S1_STORAGE_S3_ACCESS_KEY: "${MINIO_ROOT_USER:-minioadmin}"
      S1_STORAGE_S3_SECRET_KEY: "${MINIO_ROOT_PASSWORD:-minioadmin}"
      S1_AUTH_ENABLED: "true"
      S1_JWT_SECRET: "${JWT_SECRET}"
      S1_REQUIRE_JWT_EXP: "true"
      S1_CALLBACK_SECRET: "${CALLBACK_SECRET}"
      S1_ALLOW_ANONYMOUS: "false"
      S1_COLLAB_ENABLED: "true"
      S1_COLLAB_REDIS_URL: "redis://redis:6379"
      S1_ADMIN_USER: "${ADMIN_USER:-admin}"
      S1_ADMIN_PASS: "${ADMIN_PASSWORD}"
      RUST_LOG: "s1_server=info,tower_http=info"
    volumes:
      - rudra-data:/data
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_started
      minio:
        condition: service_started
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 15s
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: "2"

  # ── PostgreSQL ───────────────────────────────────────
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: rudra
      POSTGRES_USER: rudra
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    volumes:
      - pgdata:/var/lib/postgresql/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U rudra"]
      interval: 10s
      timeout: 5s
      retries: 5

  # ── Redis ────────────────────────────────────────────
  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes --maxmemory 128mb --maxmemory-policy allkeys-lru
    volumes:
      - redis-data:/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

  # ── MinIO (S3-compatible storage) ────────────────────
  minio:
    image: minio/minio:latest
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: ${MINIO_ROOT_USER:-minioadmin}
      MINIO_ROOT_PASSWORD: ${MINIO_ROOT_PASSWORD:-minioadmin}
    ports:
      - "9001:9001"   # MinIO Console (optional, remove in production)
    volumes:
      - minio-data:/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "mc", "ready", "local"]
      interval: 30s
      timeout: 5s
      retries: 3

  # ── AI Sidecar (optional) ────────────────────────────
  rudra-ai:
    image: rudra/ai:latest
    build:
      context: ./ai
      dockerfile: Dockerfile
    ports:
      - "${AI_PORT:-8081}:8081"
    deploy:
      resources:
        limits:
          cpus: "${AI_CPUS:-4}"
          memory: "${AI_MEMORY:-4G}"
        reservations:
          memory: "2G"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-sf", "http://localhost:8081/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s

volumes:
  rudra-data:
  pgdata:
  redis-data:
  minio-data:
```

Create a `.env` file alongside `docker-compose.yml`:

```bash
# .env
JWT_SECRET=your-secret-key-min-32-chars-long-here
CALLBACK_SECRET=your-callback-signing-secret
ADMIN_USER=admin
ADMIN_PASSWORD=a-strong-admin-password
POSTGRES_PASSWORD=a-strong-postgres-password
MINIO_ROOT_USER=minioadmin
MINIO_ROOT_PASSWORD=a-strong-minio-password
```

Start the stack:

```bash
docker compose up -d
```

---

## Environment Variables

### Server Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `S1_PORT` | `8080` | TCP port the server listens on. |
| `S1_STORAGE` | `local` | Storage backend. One of `local`, `memory`, or `s3`. |
| `S1_DATA_DIR` | `/data` | Directory for local file storage. Only used when `S1_STORAGE=local`. |
| `S1_STATIC_DIR` | `/app/public` | Directory containing the editor static files (HTML, JS, CSS, WASM). |
| `RUST_LOG` | `s1_server=info` | Log level filter. Uses [`tracing-subscriber` EnvFilter syntax](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html). |

### Storage Backends

| Variable | Default | Description |
|----------|---------|-------------|
| `S1_STORAGE_POSTGRES_URL` | -- | PostgreSQL connection string for document metadata. Example: `postgresql://user:pass@host:5432/dbname`. |
| `S1_STORAGE_S3_ENDPOINT` | -- | S3-compatible endpoint URL. Example: `http://minio:9000` or `https://s3.amazonaws.com`. |
| `S1_STORAGE_S3_BUCKET` | -- | S3 bucket name for document storage. |
| `S1_STORAGE_S3_ACCESS_KEY` | -- | S3 access key ID. |
| `S1_STORAGE_S3_SECRET_KEY` | -- | S3 secret access key. |
| `S1_STORAGE_S3_REGION` | `us-east-1` | S3 region (required by some providers). |

### Authentication

| Variable | Default | Description |
|----------|---------|-------------|
| `S1_AUTH_ENABLED` | `false` | Enable JWT and API key authentication. When `false`, all endpoints are public. |
| `S1_JWT_SECRET` | -- | HMAC-SHA256 secret for validating JWT tokens. Must be set when `S1_AUTH_ENABLED=true`. Also used for integration mode (`/edit?token=`). |
| `S1_REQUIRE_JWT_EXP` | `false` | Reject JWT tokens that lack an `exp` (expiry) claim. Recommended for production. |
| `S1_ALLOW_ANONYMOUS` | `true` | Allow unauthenticated requests with read-only (Viewer) access. Set to `false` in production. |
| `S1_CALLBACK_SECRET` | -- | HMAC-SHA256 key for signing callback request bodies. When set, outgoing callbacks include an `X-S1-Signature: sha256=...` header for verification. |

### Collaboration

| Variable | Default | Description |
|----------|---------|-------------|
| `S1_COLLAB_ENABLED` | `true` | Enable real-time collaborative editing via WebSocket. |
| `S1_COLLAB_REDIS_URL` | -- | Redis URL for cross-instance collab room routing. Required when running multiple server instances. |

### Admin Panel

| Variable | Default | Description |
|----------|---------|-------------|
| `S1_ADMIN_USER` | `admin` | Username for the admin panel at `/admin/dashboard`. |
| `S1_ADMIN_PASS` | `admin` | Password for the admin panel. **Change this in production.** |

### AI Sidecar

| Variable | Default | Description |
|----------|---------|-------------|
| `AI_PORT` | `8081` | Port for the AI sidecar HTTP API. |
| `AI_CPUS` | `4` | CPU core limit for the AI container. |
| `AI_MEMORY` | `4G` | Memory limit for the AI container. |

---

## Volumes and Data Persistence

The following volumes should be persisted to avoid data loss:

| Volume | Container Path | Purpose |
|--------|---------------|---------|
| `rudra-data` | `/data` | Document files and metadata (when `S1_STORAGE=local`). |
| `pgdata` | `/var/lib/postgresql/data` | PostgreSQL database. |
| `redis-data` | `/data` | Redis AOF persistence (collaboration state). |
| `minio-data` | `/data` | MinIO object storage (when `S1_STORAGE=s3` with MinIO). |

Back up the `rudra-data` and `pgdata` volumes regularly. For S3-backed
deployments, documents are stored in the S3 bucket and only metadata lives in
PostgreSQL.

### Backup Example

```bash
# Stop the stack gracefully
docker compose stop

# Back up PostgreSQL
docker run --rm -v pgdata:/data -v $(pwd):/backup \
  alpine tar czf /backup/pgdata-backup.tar.gz -C /data .

# Back up local document storage
docker run --rm -v rudra-data:/data -v $(pwd):/backup \
  alpine tar czf /backup/rudra-data-backup.tar.gz -C /data .

# Restart
docker compose start
```

---

## Health Checks

The server exposes a health endpoint at `/health`:

```bash
curl http://localhost:8080/health
```

Response:

```json
{
  "status": "ok",
  "version": "1.0.2"
}
```

The admin panel provides a more detailed health endpoint at `/admin/api/health`
(requires admin authentication):

```json
{
  "status": "ok",
  "uptime_secs": 86400,
  "memory_mb": 48.3,
  "active_sessions": 5,
  "active_rooms": 3,
  "total_editors": 12,
  "pid": 1
}
```

Docker Compose health checks are configured to poll `/health` every 30 seconds
with a 15-second start period, 5-second timeout, and 3 retries. Dependent
services wait for the health check to pass before starting.

---

## Build from Source

### Prerequisites

- Docker 20.10+ with BuildKit enabled
- At least 4 GB of free disk space (Rust compilation is resource-intensive)
- At least 4 GB of RAM available to the Docker daemon

### Unified Image (Recommended)

The `Dockerfile.unified` performs a multi-stage build:

1. **Stage 1 (rust-builder):** Compiles the `s1-server` Rust binary and the WASM
   bindings using `wasm-pack`.
2. **Stage 2 (web-builder):** Installs npm dependencies and builds the editor
   frontend with Vite.
3. **Stage 3 (runtime):** Copies the stripped server binary (~15 MB) and the
   built static assets into a minimal `debian:bookworm-slim` image.

```bash
docker build -f Dockerfile.unified -t rudra/server:local .
docker run -p 8080:8080 rudra/server:local
```

### Server Only (No Frontend)

If you serve the editor frontend separately (e.g., from a CDN), build just the
server:

```bash
docker build -f server/Dockerfile -t rudra/server-api:local .
docker run -p 8080:8080 rudra/server-api:local
```

### Build Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `RUST_VERSION` | `1.88` | Rust toolchain version used in the builder stage. |
| `NODE_VERSION` | `20` | Node.js version used for the frontend build. |

### Build Caching

The Dockerfiles are optimized for layer caching. Cargo dependency manifests are
copied before source code so that changing a source file does not re-download
all crates. On a warm cache, incremental rebuilds take approximately 2-3 minutes
instead of 10-15 minutes for a full build.

---

## SSL/TLS with a Reverse Proxy

In production, terminate TLS at a reverse proxy in front of the Rudra container.
Do not expose port 8080 directly to the internet.

### nginx

```nginx
server {
    listen 443 ssl http2;
    server_name docs.example.com;

    ssl_certificate     /etc/ssl/certs/docs.example.com.pem;
    ssl_certificate_key /etc/ssl/private/docs.example.com.key;

    # Security headers
    add_header Strict-Transport-Security "max-age=63072000; includeSubDomains" always;
    add_header X-Content-Type-Options    nosniff;
    add_header X-Frame-Options           SAMEORIGIN;

    # Max upload size (match S1 server limit)
    client_max_body_size 64M;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host              $host;
        proxy_set_header X-Real-IP         $remote_addr;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # WebSocket upgrade for collaborative editing
    location /ws/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade    $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host       $host;
        proxy_set_header X-Real-IP  $remote_addr;
        proxy_read_timeout 86400s;
        proxy_send_timeout 86400s;
    }
}

server {
    listen 80;
    server_name docs.example.com;
    return 301 https://$host$request_uri;
}
```

### Traefik (Docker Labels)

Add these labels to the `rudra-server` service in `docker-compose.yml`:

```yaml
rudra-server:
  image: rudra/server:1.0.2
  labels:
    - "traefik.enable=true"
    - "traefik.http.routers.rudra.rule=Host(`docs.example.com`)"
    - "traefik.http.routers.rudra.entrypoints=websecure"
    - "traefik.http.routers.rudra.tls.certresolver=letsencrypt"
    - "traefik.http.services.rudra.loadbalancer.server.port=8080"
    # WebSocket support is automatic in Traefik
```

### Caddy

```
docs.example.com {
    reverse_proxy rudra-server:8080
}
```

Caddy automatically provisions and renews TLS certificates from Let's Encrypt.

---

## Scaling

### Horizontal Scaling with Redis

To run multiple server instances behind a load balancer, configure Redis for
collaboration room routing. Redis ensures that CRDT operations from any server
instance are broadcast to all peers in the same editing room, regardless of
which instance they are connected to.

```yaml
services:
  rudra-1:
    image: rudra/server:1.0.2
    environment:
      S1_COLLAB_ENABLED: "true"
      S1_COLLAB_REDIS_URL: "redis://redis:6379"
      S1_STORAGE: "s3"
      # ... other env vars
    deploy:
      replicas: 3

  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
```

### Load Balancer Requirements

When running multiple instances, the load balancer must support:

- **WebSocket connections** -- use sticky sessions or connection-based routing
  so that a WebSocket upgrade request reaches the same backend for the entire
  session lifetime.
- **Health check forwarding** -- route health probes to `/health` on port 8080.

Example for HAProxy:

```
backend rudra_backend
    balance roundrobin
    option httpchk GET /health
    http-check expect status 200
    # Sticky sessions for WebSocket
    stick-table type string len 64 size 100k expire 30m
    stick on req.cook(s1_session_id)
    server rudra1 rudra-1:8080 check
    server rudra2 rudra-2:8080 check
    server rudra3 rudra-3:8080 check
```

### Autoscaling Guidelines

| Metric | Scale Up When | Scale Down When |
|--------|---------------|-----------------|
| CPU usage | > 70% for 5 minutes | < 30% for 10 minutes |
| Memory usage | > 80% | < 40% |
| Active WebSocket connections | > 500 per instance | < 100 per instance |
| Request latency (p95) | > 500 ms | < 100 ms |

---

## Resource Limits

### Recommended Resources

| Component | CPU | Memory | Disk |
|-----------|-----|--------|------|
| rudra/server | 1-2 cores | 256-512 MB | 1 GB (binary + static files) |
| PostgreSQL | 1 core | 256-512 MB | Depends on document count |
| Redis | 0.5 core | 128 MB | Minimal (in-memory state) |
| MinIO | 1 core | 512 MB | Depends on total document size |
| AI sidecar | 4 cores | 4 GB | 3 GB (model weights) |

### Docker Resource Limits

Apply resource constraints in your Compose file to prevent a single container
from consuming all host resources:

```yaml
services:
  rudra-server:
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: "2"
        reservations:
          memory: 128M
          cpus: "0.5"
```

For the AI sidecar, allocate at least 4 GB of memory. The LLM model is loaded
entirely into RAM at startup:

```yaml
  rudra-ai:
    deploy:
      resources:
        limits:
          memory: 4G
          cpus: "4"
        reservations:
          memory: 2G
```

### Memory Sizing

The server uses approximately:

- **Base process:** 20-40 MB
- **Per active editing session:** 1-5 MB (depending on document size)
- **Per WebSocket connection:** ~64 KB

For 100 concurrent editors working on 20 documents, expect roughly 150-250 MB
of server memory usage.

---

## Configuration File

As an alternative to environment variables, the server reads a `s1.toml`
configuration file from the working directory at startup:

```toml
port = 8080
storage = "local"
data_dir = "/data"
max_upload_size = 67108864  # 64 MB
```

Mount this file into the container:

```bash
docker run -p 8080:8080 \
  -v ./s1.toml:/app/s1.toml \
  -v rudra-data:/data \
  rudra/server:1.0.2
```

Environment variables take precedence over `s1.toml` values.

---

## Logging

The server uses structured logging via `tracing`. Control verbosity with the
`RUST_LOG` environment variable:

```bash
# Default: info level for the server and tower-http
RUST_LOG=s1_server=info,tower_http=info

# Debug mode (verbose, includes request/response details)
RUST_LOG=s1_server=debug,tower_http=debug

# Trace mode (extremely verbose, includes CRDT operations)
RUST_LOG=s1_server=trace

# Quiet mode (warnings and errors only)
RUST_LOG=s1_server=warn
```

Logs are written to stdout in plain text format, suitable for collection by
Docker logging drivers, Fluentd, Loki, or any other log aggregator.

---

## Troubleshooting

### Container Exits Immediately

Check the logs:

```bash
docker logs rudra
```

Common causes:

- Port 8080 already in use on the host. Change the host port: `-p 9090:8080`.
- The `/data` volume directory has incorrect permissions.

### WebSocket Connections Fail

- Verify that your reverse proxy passes WebSocket upgrade headers. See the
  [SSL/TLS section](#ssltls-with-a-reverse-proxy) for correct nginx and Traefik
  configuration.
- Check that the `Connection: Upgrade` and `Upgrade: websocket` headers reach
  the server.

### Admin Panel Returns 403

- `S1_ADMIN_USER` and `S1_ADMIN_PASS` must both be set and non-empty.
- Sessions expire after 1 hour. Log in again.

### JWT Authentication Rejected

- Ensure `S1_JWT_SECRET` matches the secret used to sign tokens.
- If `S1_REQUIRE_JWT_EXP=true`, all tokens must include an `exp` claim.
- Check that the token has not expired (`exp` must be in the future).

### High Memory Usage

- Check the number of active editing sessions in the admin panel
  (`/admin/dashboard`).
- Large documents (> 10 MB) consume proportionally more memory during editing.
- Set memory limits with `deploy.resources.limits.memory` in Compose to prevent
  unbounded growth.
