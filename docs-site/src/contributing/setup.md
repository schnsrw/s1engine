# Development Setup

## Prerequisites

- Rust 1.88+ (`rustup install stable`)
- wasm-pack (`cargo install wasm-pack`)
- Node.js 18+ and npm
- Git

## Clone & Build

```bash
git clone https://github.com/Rudra-Office/Rudra-Editor.git
cd s1engine

# Build and test all Rust crates
cargo build --workspace
cargo test --workspace

# Build WASM bindings
make wasm

# Start the editor dev server
cd editor && npm install && npm run dev
```

## Verify Setup

```bash
make check  # Runs fmt + clippy + all tests
```

## Project Structure

```
crates/           Rust library crates (model, ops, formats, etc.)
ffi/wasm/         WASM bindings
ffi/c/            C FFI bindings
editor/           Browser editor (vanilla JS)
server/           REST API server (Axum)
packages/         npm packages (SDK, editor, React, Vue, Web Component)
docs/             Internal documentation and trackers
docs-site/        Public documentation (mdBook)
scripts/          Build and utility scripts
examples/         Example projects
```
