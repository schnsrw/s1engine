# Docker

## Quick Start

```bash
docker run -p 8787:8787 rudra/editor
```

Open `http://localhost:8787` in your browser.

## Docker Compose

```yaml
version: '3.8'
services:
  editor:
    build: .
    ports:
      - "8787:8787"
    volumes:
      - ./data:/data
    environment:
      - NODE_ENV=production
```

## Build from Source

```bash
docker build -t s1-editor .
docker run -p 8787:8787 s1-editor
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8787` | HTTP port |
| `NODE_ENV` | `development` | Environment mode |

## Health Check

```bash
curl http://localhost:8787/health
# {"status":"ok"}
```
