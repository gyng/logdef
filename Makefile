.PHONY: check lint fmt fmt-check test build clean

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

# Build everything
build:
	cargo build
	cd web && npm run build

# Clean build artifacts
clean:
	cargo clean
	rm -rf web/dist web/node_modules/.vite
