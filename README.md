# Supply Line

A walking-fortress roguelike where you build the supply chain inside the tower and defend its exterior in real time.

The project is browser-first: a Rust/WASM game core with a React/TypeScript frontend and a Rust-rendered game canvas. Desktop packaging is intended via Tauri later, but the architecture is optimized around the web runtime.

## Status

This repo is in active implementation. The design and architecture docs are relatively mature and are intended to drive the first playable version.

If you're starting work here, do not begin with the broad vision docs. Start with the canonical implementation docs:

1. [implementation-decisions.md](docs/foundation/implementation-decisions.md)
2. [v1-scope.md](docs/foundation/v1-scope.md)
3. [registries.md](docs/foundation/registries.md)
4. [balance-config.md](docs/foundation/balance-config.md)

Those files override the rest of the foundation set when there is any disagreement.

## Quick Start

Prerequisites:

- Rust toolchain
- Node.js 20+
- `wasm-pack`

```bash
# Install frontend dependencies
cd web && npm install && cd ..

# Run the standard checks
make check
```

## Common Commands

```bash
make check   # format check + lint + test
make fmt     # format Rust + TypeScript
make lint    # clippy + eslint + tsc + prettier
make test    # cargo test
make build   # cargo build + vite build
```

See [Makefile](Makefile) for the current command set.

## Project Structure

```text
crates/
  core/        GameState, commands, systems, deterministic simulation
  bridge/      wasm-bindgen bridge between Rust and the web app
  renderer/    Rust-side wgpu renderer for the game canvas
web/           React/TypeScript frontend
docs/
  foundation/  Product, design, architecture, and balance source docs
  plans/       Implementation and migration plans
.agents/       Project-local agent configuration/skills
```

## Architecture

- Rust owns the authoritative game state and fixed-timestep simulation.
- Rust also owns the game canvas renderer.
- React owns prep UI, overlays, input collection, and frame orchestration in the browser.
- The full `RenderSnapshot` stays in Rust; compact typed accessors cross the WASM bridge to React.
- Audio is handled in the JS layer via Web Audio API.

For the detailed version, read [software-architecture.md](docs/foundation/software-architecture.md) and [tech-performance.md](docs/foundation/tech-performance.md) after the canonical files above.

## Documentation

Start with [docs/foundation/README.md](docs/foundation/README.md) for the main document map.

Other useful docs:

- [docs/plans/plan-001-data-driven-config.md](docs/plans/plan-001-data-driven-config.md) — migration plan for data-driven balance/content
- [docs/foundation/debugging-bridge.md](docs/foundation/debugging-bridge.md) — guide for Rust/WASM bridge debugging

## Contributing

Project-specific working norms for coding agents and collaborators live in [AGENTS.md](AGENTS.md).
