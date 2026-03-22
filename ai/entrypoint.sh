#!/bin/sh
set -e

MODEL_PATH="${MODEL_DIR}/${MODEL_FILE}"

# Download model on first run if not already present
if [ ! -f "$MODEL_PATH" ]; then
    echo "[rudra-ai] Downloading model (this only happens once)..."
    echo "[rudra-ai] URL: ${MODEL_URL}"
    mkdir -p "${MODEL_DIR}"
    curl -L --progress-bar -o "${MODEL_PATH}.tmp" "$MODEL_URL"
    mv "${MODEL_PATH}.tmp" "$MODEL_PATH"
    echo "[rudra-ai] Model downloaded successfully."
else
    echo "[rudra-ai] Model already present at ${MODEL_PATH}"
fi

echo "[rudra-ai] Starting llama-server..."
exec /usr/local/bin/llama-server \
    -m "$MODEL_PATH" \
    -c 4096 \
    -np 4 \
    -cb \
    --host 0.0.0.0 \
    --port 8081
