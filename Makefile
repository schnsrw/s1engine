# Rudra Office — Build & Development

CARGO := cargo
WASM_PACK := wasm-pack
WASM_CRATE := ffi/wasm
WASM_OUT := web/pkg

.PHONY: help build test clippy fmt check wasm wasm-release server relay web web-dist clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ─── Rust Engine ──────────────────────────────────────

build: ## Build all crates (debug)
	$(CARGO) build --workspace

test: ## Run all tests
	$(CARGO) test --workspace --exclude s1engine-wasm --exclude s1engine-c
	@echo "Bridge + fidelity tests:"
	$(CARGO) test -p s1engine --test wasm_bridge --test fidelity_validation

clippy: ## Run clippy lints
	$(CARGO) clippy --workspace -- -D warnings

fmt: ## Check formatting
	$(CARGO) fmt --all --check

check: clippy fmt test ## Run all checks

# ─── WASM ─────────────────────────────────────────────

wasm: ## Build WASM (dev mode) → web/pkg/
	$(WASM_PACK) build $(WASM_CRATE) --target web --dev --out-dir ../../$(WASM_OUT)

wasm-release: ## Build WASM (release mode) → web/pkg/
	$(WASM_PACK) build $(WASM_CRATE) --target web --out-dir ../../$(WASM_OUT)

# ─── Web Editor ──────────────────────────────────────

web: wasm ## Build WASM and prepare web app
	@echo "Web app ready at web/"
	@echo "Run: make server  or  python3 -m http.server 8080 --directory web"

web-dist: wasm ## Build packaged web dist → web/dist/
	node scripts/build-web-dist.js
	@echo "Production web app ready at web/dist/"
	@echo "Run: python3 -m http.server 8080 --directory web/dist"

# ─── Server ───────────────────────────────────────────

server: ## Run the Axum API server (serves web/ by default)
	$(CARGO) run -p s1-server

relay: ## Run the WebSocket relay
	node scripts/relay.js

# ─── Docker ───────────────────────────────────────────

docker-build: ## Build Docker image
	docker build -t rudra-office .

docker-run: ## Run Docker container
	docker run -p 8787:8787 rudra-office

# ─── Clean ────────────────────────────────────────────

clean: ## Clean build artifacts
	$(CARGO) clean
	rm -rf $(WASM_OUT)/*.wasm
