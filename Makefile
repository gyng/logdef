.PHONY: check lint fmt fmt-check test build wasm dev clean

# Run all checks (format + lint + test)
check: fmt-check lint test
	@echo "All checks passed."

# Lint everything
lint:
	cargo clippy --all-targets -- -D warnings
	cd web && npm run check

# Format everything (write)
fmt:
	cargo fmt --all
	cd web && npm run format

# Check formatting without writing
fmt-check:
	cargo fmt --all --check
	cd web && npm run format:check

# Run all tests
test:
	cargo test

# Build WASM bridge (release, optimized)
wasm:
	wasm-pack build crates/bridge --target web --out-dir ../../web/pkg

# Build everything
build: wasm
	cargo build
	cd web && npm run build

# Dev: build WASM then start Vite dev server
dev: wasm
	cd web && npm run dev

# Clean build artifacts
clean:
	cargo clean
	rm -rf web/dist web/pkg web/node_modules/.vite
