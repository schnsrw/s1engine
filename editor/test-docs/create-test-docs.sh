#!/usr/bin/env bash
#
# Build and run the create_test_doc binary to generate test DOCX files.
#
# Usage:
#   ./create-test-docs.sh          # output goes to editor/test-docs/
#   ./create-test-docs.sh /tmp/out # output goes to /tmp/out/
#
set -euo pipefail

# Ensure cargo is on PATH
if ! command -v cargo &>/dev/null; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CRATE_DIR="$SCRIPT_DIR/create_test_doc"
OUT_DIR="${1:-$SCRIPT_DIR}"

echo "Building create_test_doc..."
cargo build --release --manifest-path "$CRATE_DIR/Cargo.toml"

echo "Generating test documents into $OUT_DIR ..."
cargo run --release --manifest-path "$CRATE_DIR/Cargo.toml" -- "$OUT_DIR"

echo ""
echo "Generated files:"
ls -lh "$OUT_DIR"/*.docx 2>/dev/null || echo "  (none found)"
