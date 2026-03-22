# Rudra AI Sidecar — CPU-optimized LLM inference via llama.cpp
# Model: Qwen2.5-3B-Instruct (Q4_K_M quantization)
# Exposes OpenAI-compatible API on port 8081

FROM ghcr.io/ggml-org/llama.cpp:server AS runtime

# Model configuration
ENV MODEL_REPO=Qwen/Qwen2.5-3B-Instruct-GGUF
ENV MODEL_FILE=qwen2.5-3b-instruct-q4_k_m.gguf
ENV MODEL_DIR=/models

# Model is baked into the image at build time for instant startup.
# For smaller images, mount a volume instead:
#   docker run -v /path/to/models:/models rudra-ai
# and download the model separately:
#   huggingface-cli download Qwen/Qwen2.5-3B-Instruct-GGUF qwen2.5-3b-instruct-q4_k_m.gguf --local-dir /path/to/models

# Download model at build time for faster startup
# Using huggingface-cli to fetch the GGUF directly
RUN apt-get update && apt-get install -y --no-install-recommends \
      python3 python3-pip curl && \
    pip3 install --no-cache-dir --break-system-packages huggingface-hub && \
    mkdir -p ${MODEL_DIR} && \
    huggingface-cli download ${MODEL_REPO} ${MODEL_FILE} \
      --local-dir ${MODEL_DIR} --local-dir-use-symlinks False && \
    apt-get purge -y python3 python3-pip && apt-get autoremove -y && \
    rm -rf /var/lib/apt/lists/* /root/.cache

EXPOSE 8081

# llama-server flags optimized for CPU throughput:
#   -ngl 0        : no GPU layers (pure CPU)
#   -c 4096       : context window (enough for doc tasks)
#   -np 4         : 4 parallel request slots for throughput
#   -cb            : continuous batching for concurrent requests
#   -t auto       : auto-detect optimal thread count
#   --host 0.0.0.0 : listen on all interfaces
#   --port 8081
ENTRYPOINT ["llama-server", \
    "-m", "/models/qwen2.5-3b-instruct-q4_k_m.gguf", \
    "-ngl", "0", \
    "-c", "4096", \
    "-np", "4", \
    "-cb", \
    "-t", "auto", \
    "--host", "0.0.0.0", \
    "--port", "8081", \
    "--cors-allow-origin", "*"]
