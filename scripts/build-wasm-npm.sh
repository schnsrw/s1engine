#!/usr/bin/env bash
set -euo pipefail

# Build WASM npm package for @s1engine/wasm
# Usage: ./scripts/build-wasm-npm.sh [--release]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
OUT_DIR="$ROOT_DIR/packages/wasm/dist"

PROFILE="--dev"
if [[ "${1:-}" == "--release" ]]; then
  PROFILE="--release"
  echo "Building in release mode..."
else
  echo "Building in dev mode (use --release for production)..."
fi

# Step 1: Build WASM with wasm-pack
echo ">> wasm-pack build..."
cd "$ROOT_DIR"
wasm-pack build ffi/wasm --target web --out-dir "$OUT_DIR" $PROFILE

# Step 2: Copy license files
echo ">> Copying LICENSE files..."
cp "$ROOT_DIR/LICENSE-MIT" "$ROOT_DIR/packages/wasm/"
cp "$ROOT_DIR/LICENSE-APACHE" "$ROOT_DIR/packages/wasm/"

# Step 3: Clean up wasm-pack generated files we don't need
rm -f "$OUT_DIR/.gitignore"
rm -f "$OUT_DIR/package.json"  # We use our own package.json

echo ""
echo "Done! Package ready at: packages/wasm/"
echo "  npm pack --dry-run    # preview contents"
echo "  npm publish            # publish to npm"
