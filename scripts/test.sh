#!/usr/bin/env bash
set -euo pipefail

# Run s1engine tests
# Usage: ./scripts/test.sh [crate-name]
# Examples:
#   ./scripts/test.sh              # Run all tests
#   ./scripts/test.sh s1-format-docx  # Run only docx tests
#   ./scripts/test.sh s1engine-wasm   # Run WASM tests

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_DIR"

if [[ $# -eq 0 ]]; then
    echo "Running all workspace tests..."
    echo "================================"
    cargo test --workspace
    echo ""
    echo "Running clippy..."
    cargo clippy --workspace -- -D warnings
    echo ""
    echo "Checking formatting..."
    cargo fmt --all -- --check
    echo ""
    echo "All checks passed!"
else
    echo "Running tests for: $1"
    echo "================================"
    cargo test -p "$1"
fi
