# Self-Hosting

This guide covers deploying Rudra Office on your own infrastructure. The server ships as a single Rust binary (or Docker image) that serves the editor UI, REST API, and WebSocket collaboration -- no reverse proxy or sidecar process required for basic deployments.

---

## Requirements

### Minimum Hardware

| Component | Minimum | Recommended |
|---|---|---|
| CPU | 1 vCPU | 2+ vCPUs |
| RAM | 512 MB | 2 GB |
| Disk | 1 GB | 10 GB+ (depends on document volume) |
| Network | 1 Mbps | 100 Mbps |

The server is single-process and multi-threaded (Tokio async runtime). Each active collaboration room holds its operation log in memory. Plan approximately 1 MB of RAM per active room with moderate edit history.

### Software

| Dependency | Version | Notes |
|---|---|---|
| Docker | 20.10+ | For container deployments. Docker Compose v2 recommended. |
| Linux | Any modern distro | The container image uses `debian:bookworm-slim`. |
| curl | Any | Used by the Docker health check. |

For bare-metal deployments without Docker, you need Rust 1.88+ and Node.js 20+ to build from source.

---

## Quick Deploy

### Docker Compose (recommended)

Create a `docker-compose.yml`:

```yaml
services:
  s1-server:
    image: rudra/server:latest
    ports:
      - "8080:8080"
    environment:
      - S1_PORT=8080
      - S1_STORAGE=local
      - S1_DATA_DIR=/data
      - S1_STATIC_DIR=/app/public
      - S1_ADMIN_USER=admin
      - S1_ADMIN_PASS=changeme
      - RUST_LOG=s1_server=info,tower_http=info
    volumes:
      - s1-data:/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 15s

volumes:
  s1-data:
```

Start the server:

```bash
docker compose up -d
```

Open `http://localhost:8080` in your browser. The admin panel is at `http://localhost:8080/admin/dashboard`.

### With AI Sidecar

To include the AI writing assistant (CPU-based, no GPU required):

```yaml
services:
  s1-server:
    image: rudra/server:latest
    ports:
      - "8080:8080"
    environment:
      - S1_PORT=8080
      - S1_STORAGE=local
      - S1_DATA_DIR=/data
      - S1_ADMIN_USER=admin
      - S1_ADMIN_PASS=changeme
    volumes:
      - s1-data:/data
    restart: unless-stopped

  rudra-ai:
    build:
      context: ./ai
      dockerfile: Dockerfile
    ports:
      - "8081:8081"
    deploy:
      resources:
        limits:
          cpus: "4"
          memory: "4G"
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
  s1-data:
```

Set the client configuration to point to the AI sidecar:

```javascript
window.S1_CONFIG = {
  aiUrl: 'http://localhost:8081',
  enableAI: true,
};
```

### Build from Source

```bash
# Clone the repository
git clone https://github.com/example/doc-engine.git
cd doc-engine

# Build the unified image (server + editor + WASM)
docker build -f Dockerfile.unified -t rudra/server .

# Run
docker run -d \
  -p 8080:8080 \
  -v rudra-data:/data \
  -e S1_ADMIN_USER=admin \
  -e S1_ADMIN_PASS=changeme \
  rudra/server
```

---

## Production Checklist

Before exposing Rudra Office to users, verify the following:

### Security

- [ ] Set a strong `S1_JWT_SECRET` (at least 32 random bytes, base64-encoded).
- [ ] Set `S1_REQUIRE_JWT_EXP=true` to reject tokens without expiry.
- [ ] Set `S1_AUTH_ENABLED=true` if the instance is not behind an authenticating proxy.
- [ ] Set `S1_ALLOW_ANONYMOUS=false` to prevent unauthenticated access.
- [ ] Change `S1_ADMIN_USER` and `S1_ADMIN_PASS` from their defaults.
- [ ] Set `S1_CALLBACK_SECRET` if using integration callbacks, so the host app can verify request authenticity.
- [ ] Terminate TLS at the reverse proxy (or load balancer). Never expose plain HTTP to the internet.

### Reliability

- [ ] Use a persistent volume for `/data` (Docker named volume or host mount).
- [ ] Enable the Docker health check (included in the default Dockerfile).
- [ ] Set `restart: unless-stopped` (or `always`) in your Compose file.
- [ ] Configure log rotation (Docker logging driver or external collector).

### Performance

- [ ] Set `RUST_LOG=s1_server=info,tower_http=warn` to reduce log volume.
- [ ] Set appropriate resource limits in Docker (`cpus`, `memory`).
- [ ] Monitor the `/admin/api/health` endpoint for memory and session counts.

---

## Architecture Options

### Single Node

The simplest deployment. One container runs the entire stack:

```
Internet --> [Reverse Proxy (TLS)] --> rudra-server:8080
                                           |
                                           +--> /        (editor UI)
                                           +--> /api/v1/ (REST API)
                                           +--> /ws/     (WebSocket)
                                           +--> /admin/  (admin panel)
```

Suitable for teams of up to ~50 concurrent editors. The server uses Tokio's multi-threaded runtime, so it scales well on a single machine with multiple cores.

### Multi-Node (Horizontal Scaling)

For larger deployments, run multiple rudra-server instances behind a load balancer with sticky sessions:

```
Internet --> [Load Balancer (TLS + sticky sessions)]
                 |          |          |
                 v          v          v
             server-1   server-2   server-3
                 |          |          |
                 +----------+----------+
                            |
                     [Shared Storage]
                     (S3 / PostgreSQL)
```

Requirements for multi-node:

- **Sticky sessions** -- WebSocket connections must be routed to the same backend for the duration of the editing session. Use cookie-based or IP-based affinity.
- **Shared storage** -- All nodes must share the same storage backend (S3 or a shared filesystem). In-memory and local storage are not suitable for multi-node.
- **Collaboration rooms are per-node** -- Each server instance manages its own set of rooms. Editors connected to different nodes for the same document will not see each other's changes in real time. For full cross-node collaboration, place a shared message broker (Redis Pub/Sub or NATS) in front of the room manager. This is a planned feature.

---

## Reverse Proxy

### nginx

```nginx
upstream rudra {
    server 127.0.0.1:8080;
}

server {
    listen 443 ssl http2;
    server_name docs.example.com;

    ssl_certificate     /etc/ssl/certs/docs.example.com.crt;
    ssl_certificate_key /etc/ssl/private/docs.example.com.key;

    # Security headers
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # Max upload size (match S1 max_upload_size)
    client_max_body_size 64m;

    # Proxy all traffic to rudra-server
    location / {
        proxy_pass http://rudra;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # WebSocket upgrade for collaboration
    location /ws/ {
        proxy_pass http://rudra;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;

        # Longer timeouts for WebSocket connections
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
    }
}

# HTTP to HTTPS redirect
server {
    listen 80;
    server_name docs.example.com;
    return 301 https://$host$request_uri;
}
```

### Traefik

Using Docker labels with Traefik v2:

```yaml
services:
  s1-server:
    image: rudra/server:latest
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.rudra.rule=Host(`docs.example.com`)"
      - "traefik.http.routers.rudra.tls=true"
      - "traefik.http.routers.rudra.tls.certresolver=letsencrypt"
      - "traefik.http.services.rudra.loadbalancer.server.port=8080"
      # Sticky sessions for WebSocket affinity
      - "traefik.http.services.rudra.loadbalancer.sticky.cookie=true"
      - "traefik.http.services.rudra.loadbalancer.sticky.cookie.name=rudra_backend"
    environment:
      - S1_PORT=8080
      - S1_STORAGE=local
      - S1_DATA_DIR=/data
    volumes:
      - s1-data:/data
```

### Caddy

```
docs.example.com {
    reverse_proxy localhost:8080

    @websocket {
        header Connection *Upgrade*
        header Upgrade    websocket
    }
    reverse_proxy @websocket localhost:8080
}
```

Caddy handles TLS automatically via Let's Encrypt.

---

## Backup and Restore

### Local Storage

When using `S1_STORAGE=local`, documents are stored in the `S1_DATA_DIR` directory as flat files:

```
/data/
  {doc-id}.bin           # Document bytes
  {doc-id}.meta.json     # Metadata sidecar (JSON)
  {doc-id}_ops.json      # Collaboration operation log (if room was active)
```

#### Backup

```bash
# Stop the server to ensure consistency (or accept point-in-time)
docker compose stop s1-server

# Create a compressed archive
tar czf rudra-backup-$(date +%Y%m%d).tar.gz /data/

# Restart
docker compose start s1-server
```

For zero-downtime backups, copy the data directory while the server is running. The files are append-only from the server's perspective; individual file reads are atomic.

#### Restore

```bash
docker compose stop s1-server
rm -rf /data/*
tar xzf rudra-backup-20260322.tar.gz -C /
docker compose start s1-server
```

### S3-Compatible Storage

When using S3 storage, backups are handled by the storage provider's own mechanisms:

```bash
# Cross-region replication (AWS)
aws s3 sync s3://rudra-docs s3://rudra-docs-backup --region us-west-2

# MinIO mirror
mc mirror minio/rudra-docs backup/rudra-docs
```

### PostgreSQL (planned)

For deployments using PostgreSQL as the metadata store:

```bash
# Dump
pg_dump -h localhost -U rudra -d rudra_docs > rudra-$(date +%Y%m%d).sql

# Restore
psql -h localhost -U rudra -d rudra_docs < rudra-20260322.sql
```

Schedule automated dumps with `cron` or use continuous archiving with WAL-G for point-in-time recovery.

---

## Monitoring

### Health Endpoints

| Endpoint | Auth Required | Description |
|---|---|---|
| `GET /health` | No | Basic health check. Returns `{"status":"ok","version":"..."}`. Use this for load balancer probes and Docker health checks. |
| `GET /admin/api/health` | Yes (admin) | Detailed health metrics including uptime, memory usage, active sessions, active rooms, total editors, and PID. |
| `GET /admin/api/stats` | Yes (admin) | Server statistics: uptime, session count, editor count, memory. |
| `GET /api/v1/info` | No | Server version, engine version, and supported format list. |

### Prometheus Metrics (planned)

A `/metrics` endpoint exposing Prometheus-compatible metrics is planned. In the meantime, scrape the `/admin/api/health` JSON endpoint with a custom exporter or use the health check for alerting.

### Key Metrics to Monitor

| Metric | Source | Alert Threshold |
|---|---|---|
| HTTP response status | Reverse proxy logs | Error rate > 1% |
| WebSocket connections | `/admin/api/stats` (`total_editors`) | Approaching connection limit |
| Memory usage | `/admin/api/health` (`memory_mb`) | > 80% of container limit |
| Active sessions | `/admin/api/stats` (`active_sessions`) | Unexpected drop to 0 |
| Uptime | `/admin/api/health` (`uptime_secs`) | Reset indicates crash/restart |
| Disk usage | Host monitoring | > 85% of volume capacity |

### Logging

The server uses the `tracing` crate with structured logging. Control verbosity with the `RUST_LOG` environment variable:

```bash
# Production (recommended)
RUST_LOG=s1_server=info,tower_http=warn

# Debugging collaboration issues
RUST_LOG=s1_server=debug,tower_http=info

# Trace-level for development
RUST_LOG=s1_server=trace,tower_http=trace
```

Logs are written to stdout in plain text format. Use your container runtime's log driver to forward them to a log aggregator (Loki, Elasticsearch, CloudWatch).

### Client Error Reporting

The editor sends client-side errors to `POST /api/v1/errors`. These are stored in an in-memory ring buffer (capacity: 100) and visible in the admin panel at `/admin/api/errors`:

```bash
curl -s http://localhost:8080/admin/api/errors?limit=10 | jq .
```

---

## Upgrades

### Rolling Updates (Docker Compose)

```bash
# Pull the new image
docker compose pull s1-server

# Recreate with zero downtime (if using restart: always)
docker compose up -d --no-deps s1-server
```

Active WebSocket connections will be dropped during the restart. Clients reconnect automatically (the editor has built-in reconnection logic with exponential backoff).

### Pre-Upgrade Checklist

1. **Back up data** -- Create a snapshot of the data volume before upgrading.
2. **Check release notes** -- Look for breaking changes in configuration or storage format.
3. **Test in staging** -- Run the new version against a copy of your data before deploying to production.

### Database Migrations (planned)

When PostgreSQL support is added, migrations will be handled automatically on server startup using an embedded migration runner. The server will refuse to start if it detects a schema version newer than what it supports (preventing accidental downgrades).

### Rollback

If an upgrade causes issues:

```bash
# Roll back to the previous image
docker compose down
docker compose up -d --pull never s1-server
```

If the data format has changed, restore from the backup taken before the upgrade.

---

## Security Hardening

### CORS

The server ships with a permissive CORS policy (`CorsLayer::permissive()`) for development convenience. In production, configure your reverse proxy to enforce a strict CORS policy:

```nginx
# nginx — restrict to your domain
add_header Access-Control-Allow-Origin "https://docs.example.com" always;
add_header Access-Control-Allow-Methods "GET, POST, PUT, DELETE, OPTIONS" always;
add_header Access-Control-Allow-Headers "Authorization, Content-Type, X-API-Key" always;
add_header Access-Control-Max-Age 86400 always;
```

### Rate Limiting

The admin login endpoint has built-in rate limiting (5 attempts per IP per 60 seconds). For API endpoints, add rate limiting at the reverse proxy layer:

```nginx
# nginx — rate limit API requests
limit_req_zone $binary_remote_addr zone=api:10m rate=100r/s;

location /api/ {
    limit_req zone=api burst=50 nodelay;
    proxy_pass http://rudra;
}
```

### SSRF Protection

The server blocks outbound HTTP requests to private and internal IP addresses. This protects against Server-Side Request Forgery when processing `download_url` and `callback_url` values from integration JWTs. The following are blocked:

- RFC 1918 private ranges
- Loopback addresses
- Link-local addresses
- Cloud metadata endpoints (`169.254.169.254`)
- Hostnames ending in `.local` or `.internal`

No additional configuration is needed; SSRF protection is always active.

### Content Security Policy

Add a strict CSP header at your reverse proxy:

```nginx
add_header Content-Security-Policy "
    default-src 'self';
    script-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net;
    style-src 'self' 'unsafe-inline' https://fonts.googleapis.com;
    font-src 'self' https://fonts.gstatic.com;
    img-src 'self' data: blob:;
    connect-src 'self' wss://docs.example.com;
" always;
```

Adjust the `connect-src` directive to match your domain and any AI sidecar URLs.

### File Upload Validation

The server validates all uploaded documents through the s1engine parser before accepting them. Malformed or unrecognized files are rejected with a `400 Bad Request`. The maximum upload size is 64 MB by default (configurable via `max_upload_size` in `s1.toml`).

### Network Isolation

In a Docker Compose deployment, isolate internal services from the public network:

```yaml
services:
  s1-server:
    networks:
      - frontend
      - backend

  rudra-ai:
    networks:
      - backend  # AI sidecar not exposed to internet

networks:
  frontend:
  backend:
    internal: true
```

---

## Troubleshooting

### Common Issues

**Editor loads but shows "Failed to load WASM"**

The static directory (`S1_STATIC_DIR`) does not contain the compiled WASM files. Verify the build completed successfully and that the `public/` directory contains `.wasm` files.

**WebSocket connections fail with 403**

If `S1_AUTH_ENABLED=true`, WebSocket connections may require authentication. Ensure the client passes credentials (JWT or API key) in the WebSocket URL query parameters or upgrade headers.

**"S1_JWT_SECRET not set" warning on startup**

Integration mode (`/edit?token=`) requires a JWT secret. Set `S1_JWT_SECRET` to a strong random string. This warning is informational if you are not using integration mode.

**Admin panel returns 403**

Both `S1_ADMIN_USER` and `S1_ADMIN_PASS` must be set as environment variables. If either is empty, the admin panel is disabled entirely.

**High memory usage**

Check the number of active rooms via `/admin/api/stats`. Each room with a large operation log consumes memory. The server truncates logs at 10,000 entries, but many concurrent rooms can still accumulate. Consider restarting stale sessions via the admin panel.
