# Rudra AI Sidecar — LLM inference via llama.cpp
# Model: Qwen2.5-3B-Instruct (Q4_K_M quantization)
# Exposes OpenAI-compatible API on port 8081

# ── Stage 1: Build llama-server statically ──
FROM debian:bookworm-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential cmake git ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN git clone --depth 1 https://github.com/ggerganov/llama.cpp.git /src
WORKDIR /src
RUN cmake -B build -DCMAKE_BUILD_TYPE=Release \
      -DGGML_NATIVE=OFF \
      -DBUILD_SHARED_LIBS=OFF \
      -DLLAMA_BUILD_TESTS=OFF \
      -DLLAMA_BUILD_EXAMPLES=OFF \
      -DLLAMA_BUILD_SERVER=ON && \
    cmake --build build --target llama-server -j$(nproc)

# ── Stage 2: Slim runtime ──
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
      curl ca-certificates libgomp1 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/build/bin/llama-server /usr/local/bin/llama-server
RUN chmod +x /usr/local/bin/llama-server

ENV MODEL_DIR=/models
ENV MODEL_FILE=qwen2.5-3b-instruct-q4_k_m.gguf
ENV MODEL_URL=https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf

EXPOSE 8081

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
