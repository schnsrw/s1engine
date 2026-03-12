# s1engine — Build & Development Makefile

CARGO := cargo
WASM_PACK := wasm-pack
WASM_CRATE := ffi/wasm
WASM_OUT := demo/pkg
DEMO_PORT := 8080

.PHONY: help build test clippy fmt check wasm wasm-release demo clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ─── Rust ────────────────────────────────────────────────────────────

build: ## Build all crates (debug)
	$(CARGO) build --workspace

build-release: ## Build all crates (release)
	$(CARGO) build --workspace --release

test: ## Run all tests
	$(CARGO) test --workspace

test-docx: ## Run s1-format-docx tests only
	$(CARGO) test -p s1-format-docx

test-wasm: ## Run WASM binding tests
	$(CARGO) test -p s1engine-wasm

clippy: ## Run clippy linter
	$(CARGO) clippy --workspace -- -D warnings

fmt: ## Format code
	$(CARGO) fmt --all

fmt-check: ## Check formatting (CI)
	$(CARGO) fmt --all -- --check

check: fmt-check clippy test ## Run all checks (fmt + clippy + tests)

# ─── WASM ────────────────────────────────────────────────────────────

wasm: ## Build WASM bindings (debug, fast)
	$(WASM_PACK) build $(WASM_CRATE) --target web --dev --out-dir ../../$(WASM_OUT)

wasm-release: ## Build WASM bindings (release, optimized)
	$(WASM_PACK) build $(WASM_CRATE) --target web --out-dir ../../$(WASM_OUT)

# ─── Demo ────────────────────────────────────────────────────────────

demo: wasm-release ## Build WASM and start demo server
	@echo "Demo ready at http://localhost:$(DEMO_PORT)"
	@cd demo && python3 -m http.server $(DEMO_PORT)

demo-only: ## Start demo server (without rebuilding WASM)
	@echo "Demo at http://localhost:$(DEMO_PORT)"
	@cd demo && python3 -m http.server $(DEMO_PORT)

# ─── Clean ───────────────────────────────────────────────────────────

clean: ## Clean build artifacts
	$(CARGO) clean
	rm -rf $(WASM_OUT)
