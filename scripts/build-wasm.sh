#!/usr/bin/env bash
set -euo pipefail

# Build s1engine WASM bindings
# Usage: ./scripts/build-wasm.sh [--dev]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
WASM_CRATE="$PROJECT_DIR/ffi/wasm"
OUT_DIR="$PROJECT_DIR/demo/pkg"

MODE="--release"
if [[ "${1:-}" == "--dev" ]]; then
    MODE="--dev"
    echo "Building WASM (debug mode)..."
else
    echo "Building WASM (release mode)..."
fi

# Check prerequisites
if ! command -v wasm-pack &> /dev/null; then
    echo "Error: wasm-pack not found. Install with: cargo install wasm-pack"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Install Rust from https://rustup.rs"
    exit 1
fi

# Check for wasm32 target
if ! rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
    echo "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Build
if [[ "$MODE" == "--dev" ]]; then
    wasm-pack build "$WASM_CRATE" --target web --dev --out-dir "../../demo/pkg"
else
    wasm-pack build "$WASM_CRATE" --target web --out-dir "../../demo/pkg"
fi

echo ""
echo "WASM output: $OUT_DIR/"
ls -lh "$OUT_DIR/s1engine_wasm_bg.wasm" 2>/dev/null || true
echo ""
echo "Done. Start the demo with: make demo-only"
