#!/usr/bin/env bash
# Sets GitHub repo description, topics, and homepage for discoverability.
# Usage: ./scripts/setup-github-repo.sh
# Requires: gh CLI authenticated

set -euo pipefail

REPO="Rudra-Office/Rudra-Editor"

echo "Setting repository description..."
gh repo edit "$REPO" \
  --description "Open-source document engine SDK in Rust — read, write, edit DOCX/ODT/PDF/Markdown with CRDT collaboration, WASM support, and a self-hosted web editor."

echo "Setting repository topics..."
gh repo edit "$REPO" \
  --add-topic rust \
  --add-topic document-engine \
  --add-topic docx \
  --add-topic odt \
  --add-topic pdf \
  --add-topic wasm \
  --add-topic webassembly \
  --add-topic crdt \
  --add-topic collaborative-editing \
  --add-topic document-editor \
  --add-topic document-conversion \
  --add-topic self-hosted \
  --add-topic text-processing \
  --add-topic document-processing \
  --add-topic markdown \
  --add-topic rust-library \
  --add-topic document-sdk \
  --add-topic pure-rust \
  --add-topic word-processor

echo "Setting homepage..."
gh repo edit "$REPO" --homepage "https://github.com/Rudra-Office/Rudra-Editor"

echo "Done! Verify at https://github.com/$REPO"