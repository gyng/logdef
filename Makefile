.PHONY: check lint fmt fmt-check test e2e build build-fast build-checked wasm wasm-dev dev dev-stop clean bench-core bench-scenario bench-pipeline timings-build timings-test timings-all

# Run all checks (format + lint + test)
check: fmt-check lint test
	@echo "All checks passed."

# Lint everything
lint:
	@set -e; \
	cargo clippy --all-targets -- -D warnings & \
	rust_pid=$$!; \
	(cd web && npm run check:code) & \
	web_pid=$$!; \
	wait $$rust_pid; \
	wait $$web_pid

# Format everything (write)
fmt:
	cargo fmt --all
	cd web && npm run format

# Check formatting without writing
fmt-check:
	@set -e; \
	cargo fmt --all --check & \
	rust_pid=$$!; \
	(cd web && npm run format:check) & \
	web_pid=$$!; \
	wait $$rust_pid; \
	wait $$web_pid

# Run all tests
test:
	cargo test

# End-to-end smoke test (requires WASM built).
# Stops any stale vite first so playwright can start its own clean
# dev server without colliding on port 3000.
e2e: wasm dev-stop
	cd web && npm run e2e

# Build WASM bridge (release, optimized)
wasm:
	wasm-pack build crates/bridge --target web --out-dir ../../web/pkg

# Build WASM bridge quickly for local iteration (no wasm-opt)
wasm-dev:
	wasm-pack build --dev crates/bridge --target web --out-dir ../../web/pkg

# Build everything for local iteration (fast path; typecheck lives in make check)
build: wasm
	cargo build --workspace --exclude supply-line-bridge
	cd web && npm run build

# Fastest local iteration path: dev wasm + native build + vite build
build-fast: wasm-dev
	cd web && npm run build

# Build everything with an explicit frontend typecheck step
build-checked: wasm
	cargo build --workspace --exclude supply-line-bridge
	cd web && npm run build:checked

# Dev: build WASM then start Vite dev server
dev: wasm-dev
	cd web && npm run dev

# Kill any vite dev server and its wrappers. Two-step:
# 1. `fuser -k 3000/tcp` kills whatever is holding the TCP listener,
#    which is the most reliable way to free the port.
# 2. Sweep any stray vite wrappers by PID (from pgrep), not by pkill
#    pattern — pkill -f "node.*vite" would match its own recipe shell
#    line (which contains the literal pattern string) and kill make
#    itself. Using pgrep + a bracket trick in the pattern ([v]ite) so
#    the pgrep process's own argv doesn't match its own pattern.
# Safe to call even if nothing is running — every step swallows errors.
dev-stop:
	@if command -v fuser >/dev/null 2>&1; then \
		fuser -k 3000/tcp 2>/dev/null || true; \
	elif command -v lsof >/dev/null 2>&1; then \
		pid=$$(lsof -ti :3000 2>/dev/null); \
		if [ -n "$$pid" ]; then kill -9 $$pid || true; fi; \
	fi
	@pids=$$(pgrep -f "node.*[v]ite" 2>/dev/null); \
	if [ -n "$$pids" ]; then kill -9 $$pids 2>/dev/null || true; fi
	@echo "Port 3000 is now free; any stray vite processes terminated."

# Rust microbenchmarks
bench-core:
	cargo run --release -p supply-line-core --example engine_bench

# Representative combat scenario benchmark
bench-scenario:
	cargo run --release -p supply-line-core --example encounter_scenario_bench

# Pipeline timing harness (set REPEATS=3 to average)
bench-pipeline:
	./scripts/bench-pipeline.sh $${REPEATS:-1}

# Cargo compile timing reports
timings-build:
	./scripts/cargo-timings.sh build

timings-test:
	./scripts/cargo-timings.sh test

timings-all: timings-build timings-test

# Clean build artifacts
clean:
	cargo clean
	rm -rf web/dist web/pkg web/node_modules/.vite
