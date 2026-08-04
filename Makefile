.PHONY: check instruments lint fmt fmt-check test e2e build build-fast build-checked wasm wasm-dev dev dev-stop clean release release-check bench-pipeline timings-build timings-test timings-all golden mutants

# Run all checks (format + lint + test)
check: fmt-check lint test
	@echo "All checks passed."

# Run every measuring instrument, in the order they answer questions.
#
# **Not part of `check`, and that is why three of them were dead.**
# Clippy's `--all-targets` compiles the examples, so a type error is
# caught; nothing runs them, so a *runtime* failure is invisible until
# somebody asks a question. In one session this hid: `throughput.rs`
# panicking on startup since M5 gated the elevator on rope; the plating
# comparison in `siege_run.rs` counting a total whose maximum was the
# thing under test, for the fourth time; and that harness's whole
# "battery + darts" tower building nothing at all, because a dart
# battery costs rope the pressure tower has none of and no one checked
# the return value. M2's exit criterion rested on that comparison and it
# had never once run.
#
# They stay out of `check` because journey.rs alone is minutes. Run this
# after touching balance.ron, a room, a creature, or anything a
# BALANCE.md row cites — and read the output rather than the exit code,
# since an instrument that measures the wrong thing exits zero.
instruments:
	@set -e; \
	for x in throughput charge needs haulcycle chain worldrate siege_run journey; do \
		echo "=== $$x ==="; \
		cargo run --release -q -p understory-core --example $$x; \
	done

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
	cargo build --workspace --exclude understory-bridge
	cd web && npm run build

# Fastest local iteration path: dev wasm + native build + vite build
build-fast: wasm-dev
	cd web && npm run build

# The release cut: an itch.io-ready zip.
#
# Optimised WASM (not the dev path — wasm-opt is worth the wait for a
# quarter-megabyte payload), a typechecked frontend build, and the whole
# of `web/dist` zipped with `index.html` at the root, which is the shape
# itch unpacks. Nothing about this is clever; it exists so that cutting a
# build is one command rather than a remembered sequence, and so the
# thing uploaded is the thing that was tested.
release: check wasm
	cd web && npm run build
	rm -f understory-web.zip
	cd web/dist && zip -qr ../../understory-web.zip .
	@echo "wrote understory-web.zip — upload to itch as an HTML5 game,"
	@echo "with index.html as the entry point and 'fullscreen' enabled."

# Open the built bundle the way itch will and play it for a moment.
#
# `npm run build` succeeding proves the bundler was happy and nothing
# else. The failure this exists to catch is a bundle that 404s its own
# WASM and shows a blank canvas — which every other check in the project
# passes, because every other check runs against the dev server.
release-check: release
	cd web && (npx vite preview --port 4173 --strictPort &) && sleep 3 && 		npx playwright test release.spec.ts --reporter=line

# Build everything with an explicit frontend typecheck step
build-checked: wasm
	cargo build --workspace --exclude understory-bridge
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

# **`bench-core` and `bench-scenario` are gone**, and were dead. They ran
# `engine_bench` and `encounter_scenario_bench`, two v1 examples deleted
# with the rest of that game — the second one is named after `encounter`,
# which `AGENTS.md` lists as vocabulary that describes a different game.
# Nothing had failed, because a Makefile target pointing at a missing
# example only fails when somebody types it.
#
# The frame budget has not been the constraint at any point through M5
# (`v2-plan.md` §11 open #3), so nothing replaces them. If profiling is
# ever needed, write the harness for the question at hand rather than
# reviving a target named after a system that no longer exists.

# Pipeline timing harness (set REPEATS=3 to average)
bench-pipeline:
	./scripts/bench-pipeline.sh $${REPEATS:-1}

# Cargo compile timing reports
timings-build:
	./scripts/cargo-timings.sh build

timings-test:
	./scripts/cargo-timings.sh test

timings-all: timings-build timings-test

# Regenerate the golden determinism fixture. Run this whenever a
# deliberate sim change alters expected output; commit the
# regenerated fixture in the same PR as the change that caused it,
# so a diff always explains itself.
golden:
	cargo run -p understory-core --example record_golden
	@echo "Golden fixture regenerated — review the diff and rebuild before committing."

# Mutation testing for the core crate. Slow (recompiles + reruns
# tests per mutant) — not part of `make check`, run it deliberately.
# Requires `cargo install cargo-mutants` (not a workspace dependency).
mutants:
	cargo mutants -p understory-core

# Clean build artifacts
clean:
	cargo clean
	rm -rf web/dist web/pkg web/node_modules/.vite
