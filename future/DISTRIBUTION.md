# Distribution Strategy

## Overview

s1engine is distributed through multiple channels to reach every type of consumer: Rust developers, JavaScript/TypeScript developers, DevOps teams, and end users.

## Distribution Channels

```
┌─────────────────────────────────────────────────────────────────┐
│                      Distribution Matrix                        │
├──────────────┬──────────────┬───────────────┬──────────────────┤
│   Channel    │   Package    │   Audience     │   Use Case       │
├──────────────┼──────────────┼───────────────┼──────────────────┤
│ crates.io    │ s1engine     │ Rust devs      │ Native library   │
│ crates.io    │ s1-model     │ Rust devs      │ Doc model only   │
│ crates.io    │ s1-format-*  │ Rust devs      │ Format I/O only  │
│ crates.io    │ s1-server    │ Rust devs      │ API server       │
├──────────────┼──────────────┼───────────────┼──────────────────┤
│ npm          │ @s1engine/wasm│ JS devs       │ Raw WASM access  │
│ npm          │ @s1engine/sdk │ JS devs       │ Headless API     │
│ npm          │ @s1engine/editor│ JS devs     │ Embeddable UI    │
│ npm          │ @s1engine/react│ React devs   │ React component  │
│ npm          │ @s1engine/vue │ Vue devs      │ Vue component    │
│ npm          │ @s1engine/web-component│ All  │ Universal embed  │
├──────────────┼──────────────┼───────────────┼──────────────────┤
│ Docker Hub   │ s1engine/editor│ DevOps       │ Self-hosted UI   │
│ Docker Hub   │ s1engine/server│ DevOps       │ API server       │
│ Docker Hub   │ s1engine/all-in-one│DevOps    │ Full platform    │
├──────────────┼──────────────┼───────────────┼──────────────────┤
│ GitHub       │ s1engine     │ All            │ Source code      │
│ GitHub       │ Releases     │ All            │ Pre-built bins   │
├──────────────┼──────────────┼───────────────┼──────────────────┤
│ CDN          │ @s1engine/*  │ Quick start    │ No-build embed   │
└──────────────┴──────────────┴───────────────┴──────────────────┘
```

---

## 1. Rust Packages (crates.io)

### Publishing Order

Crates must be published in dependency order:

```
1. s1-model         (zero deps — publish first)
2. s1-ops           (depends: s1-model)
3. s1-text          (depends: s1-model)
4. s1-format-txt    (depends: s1-model)
5. s1-format-docx   (depends: s1-model)
6. s1-format-odt    (depends: s1-model)
7. s1-format-md     (depends: s1-model)
8. s1-layout        (depends: s1-model, s1-text)
9. s1-format-pdf    (depends: s1-model, s1-layout, s1-text)
10. s1-crdt         (depends: s1-model, s1-ops)
11. s1-convert      (depends: s1-format-docx, s1-format-odt)
12. s1engine        (depends: all above)
13. s1-server       (depends: s1engine, axum, tokio)
```

### Cargo.toml Metadata

Each crate's `Cargo.toml` must include:

```toml
[package]
name = "s1-model"
version = "1.0.0"
edition = "2021"
rust-version = "1.75"
license = "AGPL-3.0-or-later"
description = "Core document model for s1engine — document tree, nodes, attributes, styles"
repository = "https://github.com/schnsrw/s1engine"
homepage = "https://s1engine.dev"
documentation = "https://docs.rs/s1-model"
readme = "README.md"
keywords = ["document", "docx", "editor", "word-processing"]
categories = ["text-processing", "parser-implementations"]

[badges]
maintenance = { status = "actively-developed" }
```

### Feature Flags (s1engine facade)

```toml
[features]
default = ["docx", "odt", "txt"]
docx = ["s1-format-docx"]
odt = ["s1-format-odt"]
pdf = ["s1-format-pdf", "layout"]
txt = ["s1-format-txt"]
md = ["s1-format-md"]
layout = ["s1-layout", "s1-text"]
crdt = ["s1-crdt"]
convert = ["s1-convert", "docx", "odt"]
full = ["docx", "odt", "pdf", "txt", "md", "layout", "crdt", "convert"]
```

Consumers pick only what they need:

```toml
# Just DOCX parsing (minimal deps)
s1engine = { version = "1.0", features = ["docx"] }

# Full document processing
s1engine = { version = "1.0", features = ["full"] }

# PDF generation only
s1engine = { version = "1.0", features = ["docx", "pdf"] }
```

### Versioning

- Follow [Semantic Versioning](https://semver.org/) strictly
- All workspace crates share the same version number
- Use `cargo-release` for coordinated publishing
- Pre-1.0: `0.x.y` versions for beta period
- Post-1.0: careful about breaking changes

---

## 2. npm Packages

### Package Structure

```
packages/
├── wasm/
│   ├── package.json
│   ├── README.md
│   ├── dist/
│   │   ├── s1engine_wasm_bg.wasm
│   │   ├── s1engine_wasm.js
│   │   └── s1engine_wasm.d.ts
│   └── LICENSE
├── sdk/
│   ├── package.json
│   ├── README.md
│   ├── dist/
│   │   ├── index.js          (ESM)
│   │   ├── index.cjs         (CJS)
│   │   └── index.d.ts        (types)
│   └── LICENSE
├── editor/
│   ├── package.json
│   ├── README.md
│   ├── dist/
│   │   ├── index.js
│   │   ├── index.cjs
│   │   ├── index.d.ts
│   │   ├── style.css         (default styles)
│   │   └── themes/
│   │       ├── dark.css
│   │       └── minimal.css
│   └── LICENSE
├── react/
│   ├── package.json
│   ├── dist/
│   │   ├── index.js
│   │   ├── index.cjs
│   │   └── index.d.ts
│   └── LICENSE
├── vue/
│   ├── package.json
│   ├── dist/
│   │   ├── index.js
│   │   └── index.d.ts
│   └── LICENSE
└── web-component/
    ├── package.json
    ├── dist/
    │   └── s1-editor.js      (single file, self-contained)
    └── LICENSE
```

### package.json (example: @s1engine/editor)

```json
{
  "name": "@s1engine/editor",
  "version": "1.0.0",
  "description": "Embeddable document editor with DOCX, PDF, and collaboration support",
  "license": "AGPL-3.0-or-later",
  "type": "module",
  "main": "./dist/index.cjs",
  "module": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "exports": {
    ".": {
      "import": "./dist/index.js",
      "require": "./dist/index.cjs",
      "types": "./dist/index.d.ts"
    },
    "./style.css": "./dist/style.css",
    "./themes/dark.css": "./dist/themes/dark.css",
    "./themes/minimal.css": "./dist/themes/minimal.css"
  },
  "files": ["dist", "LICENSE", "README.md"],
  "repository": {
    "type": "git",
    "url": "https://github.com/schnsrw/s1engine",
    "directory": "packages/editor"
  },
  "homepage": "https://s1engine.dev",
  "keywords": [
    "document-editor", "docx", "pdf", "wysiwyg", "collaboration",
    "word-processor", "embeddable", "white-label"
  ],
  "peerDependencies": {
    "@s1engine/wasm": "^1.0.0"
  },
  "dependencies": {
    "@s1engine/sdk": "^1.0.0"
  },
  "sideEffects": ["*.css"]
}
```

### npm Publishing Automation

```yaml
# .github/workflows/npm-publish.yml
name: Publish npm packages
on:
  release:
    types: [published]

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # Build WASM first
      - name: Build WASM
        run: make wasm-release

      # Build all JS packages
      - name: Build packages
        run: |
          cd packages/sdk && npm ci && npm run build
          cd packages/editor && npm ci && npm run build
          cd packages/react && npm ci && npm run build
          cd packages/vue && npm ci && npm run build
          cd packages/web-component && npm ci && npm run build

      # Publish in order
      - name: Publish @s1engine/wasm
        run: cd packages/wasm && npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

      - name: Publish @s1engine/sdk
        run: cd packages/sdk && npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

      # ... etc for each package
```

---

## 3. Docker Images

### Image Variants

#### s1engine/editor (Static Editor)

```dockerfile
# Serves the editor as a static site
FROM nginx:alpine
COPY editor/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
```

**Usage:**
```bash
docker run -p 8080:80 s1engine/editor
```

#### s1engine/server (API Server)

```dockerfile
# Rust binary server
FROM debian:bookworm-slim
COPY --from=builder /app/s1-server /usr/local/bin/
EXPOSE 8080 8787
CMD ["s1-server"]
```

**Usage:**
```bash
docker run -p 8080:8080 \
  -e S1_STORAGE_BACKEND=s3 \
  -e S1_STORAGE_S3_BUCKET=my-docs \
  -e S1_AUTH_JWT_SECRET=mysecret \
  s1engine/server
```

#### s1engine/all-in-one (Server + Editor + Collab)

```dockerfile
# Everything in one container
# Editor (nginx) + Server (s1-server) + Collab (built into server)
```

**Usage:**
```bash
docker run -p 8080:8080 s1engine/all-in-one
# Editor at http://localhost:8080
# API at http://localhost:8080/api/v1
# WebSocket at ws://localhost:8080/ws/collab
```

### Docker Compose (Reference Deployment)

```yaml
version: '3.8'

services:
  editor:
    image: s1engine/editor
    ports:
      - "3000:80"
    depends_on:
      - server

  server:
    image: s1engine/server
    ports:
      - "8080:8080"
    environment:
      S1_STORAGE_BACKEND: postgres
      S1_STORAGE_POSTGRES_URL: postgresql://s1:password@postgres/s1engine
      S1_AUTH_JWT_SECRET: ${JWT_SECRET}
      S1_COLLAB_ENABLED: "true"
    depends_on:
      - postgres
      - redis

  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: s1engine
      POSTGRES_USER: s1
      POSTGRES_PASSWORD: password
    volumes:
      - pgdata:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    # Used for collab room routing in multi-node deployments

  minio:
    image: minio/minio
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    # Optional: S3-compatible storage

volumes:
  pgdata:
```

### Image Tags

```
s1engine/server:latest          # Latest stable release
s1engine/server:1.0.0           # Specific version
s1engine/server:1.0             # Latest patch in minor
s1engine/server:1               # Latest minor in major
s1engine/server:edge            # Latest commit on main (unstable)
```

---

## 4. GitHub Releases

### Release Artifacts

Each GitHub Release includes:

```
s1engine-v1.0.0/
├── s1-server-linux-amd64        # Server binary (Linux x86_64)
├── s1-server-linux-arm64        # Server binary (Linux ARM64)
├── s1-server-macos-amd64        # Server binary (macOS x86_64)
├── s1-server-macos-arm64        # Server binary (macOS ARM64)
├── s1-server-windows-amd64.exe  # Server binary (Windows x86_64)
├── s1engine-wasm.tar.gz         # WASM package (for manual use)
├── s1engine-editor.tar.gz       # Pre-built editor (static files)
├── checksums.txt                # SHA256 checksums
└── CHANGELOG.md                 # Release notes
```

### Release Process

1. Create git tag: `v1.0.0`
2. GitHub Actions builds all artifacts
3. Cross-compiles server binary for all platforms
4. Builds WASM package
5. Builds editor static files
6. Publishes to crates.io
7. Publishes to npm
8. Pushes Docker images
9. Creates GitHub Release with artifacts
10. Updates documentation site

---

## 5. CDN Distribution

For consumers who want to embed the editor without a bundler:

### jsDelivr

```html
<script type="module">
  import { S1Editor } from 'https://cdn.jsdelivr.net/npm/@s1engine/editor@1.0.0/dist/index.js'
</script>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@s1engine/editor@1.0.0/dist/style.css">
```

### unpkg

```html
<script type="module">
  import { S1Editor } from 'https://unpkg.com/@s1engine/editor@1.0.0/dist/index.js'
</script>
```

### Self-Hosted CDN

Consumers can mirror packages on their own CDN:

```bash
# Download and host on your CDN
npx @s1engine/editor --output ./cdn/s1engine/
# Upload ./cdn/s1engine/ to your CDN
```

---

## 6. Installation Matrix

Quick reference for how to install based on use case:

| I want to... | Install |
|--------------|---------|
| Embed an editor in React | `npm install @s1engine/react` |
| Embed an editor in Vue | `npm install @s1engine/vue` |
| Embed an editor in any framework | `npm install @s1engine/editor` |
| Use editor via Web Component | `npm install @s1engine/web-component` |
| Process documents in JS (no UI) | `npm install @s1engine/sdk` |
| Process documents in Rust | `cargo add s1engine` |
| Run a document API server | `docker run s1engine/server` |
| Self-host the editor | `docker run s1engine/all-in-one` |
| Convert DOCX to PDF (CLI) | Download binary from GitHub Releases |
| Quick prototype (no install) | CDN link in HTML |

---

## Version Synchronization

All packages share the same version number and are released together:

```
s1engine (Rust)         1.0.0
s1-model (Rust)         1.0.0
@s1engine/wasm (npm)    1.0.0
@s1engine/sdk (npm)     1.0.0
@s1engine/editor (npm)  1.0.0
@s1engine/react (npm)   1.0.0
s1engine/server (Docker) 1.0.0
```

This simplifies compatibility: if all packages are the same version, they're guaranteed to work together.

### Compatibility Policy

- **Major version**: Breaking API changes (rare, with migration guide)
- **Minor version**: New features, backward compatible
- **Patch version**: Bug fixes, security patches
- **WASM ↔ SDK**: Must match major.minor
- **SDK ↔ Editor**: Must match major.minor
- **Server ↔ Client**: Negotiated via protocol version (can differ by minor)
