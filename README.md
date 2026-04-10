# Supply Line

A walking-fortress roguelike: build the supply chain inside, defend the exterior in real-time.

Rust/WASM game core + React/TypeScript frontend. Browser-first (itch.io), desktop via Tauri.

## Quick start

```bash
# Prerequisites: Rust toolchain, Node.js 20+, wasm-pack

# Install frontend dependencies
cd web && npm install && cd ..

# Check everything compiles
make check
```

## Dev commands

```bash
make check       # format check + lint + test (run before every PR)
make fmt          # auto-format Rust + TypeScript
make lint         # clippy + eslint + tsc + prettier
make test         # cargo test
make build        # cargo build + vite build
```

See the [Makefile](Makefile) for individual commands.

## Project structure

```
crates/
  core/           Simulation: GameState, GameCommand, systems, RNG
  bridge/         wasm-bindgen entry points (WASM ↔ JS)
  renderer/       wgpu game canvas
web/              React/TypeScript frontend (Vite)
  src/
    styles/       CSS tokens + global styles
    bridge/       TS types + WASM bridge interface
    hooks/        React hooks (useGameCommand, etc.)
    audio/        AudioManager (Web Audio API)
    components/   Atomic design: atoms → molecules → organisms → pages
docs/foundation/  Design docs
```

## Architecture

Rust owns all game state and simulation (30hz fixed tick). React owns UI, input collection, and frame orchestration. They communicate through wasm-bindgen. All mutations flow through `GameCommand` variants. GameState is the single source of truth.

```
React (input + UI) → GameCommand → Rust (simulation) → typed snapshots → React (render)
                                                      → RenderSnapshot → wgpu (canvas)
                                                      → SoundEvents → Web Audio API
```

## Docs

Start with the [foundation docs README](docs/foundation/README.md). The override chain:

1. **[implementation-decisions.md](docs/foundation/implementation-decisions.md)** — overrides everything
2. **[v1-scope.md](docs/foundation/v1-scope.md)** — what ships
3. **[registries.md](docs/foundation/registries.md)** — canonical names and stats
4. **[balance-config.md](docs/foundation/balance-config.md)** — all tuning numbers

For contributor guidelines, see [AGENTS.md](AGENTS.md).
