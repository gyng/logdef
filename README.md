# Understory

A walking garden-tower roguelike: SimTower's cross-section, Factorio's production chains,
and tower defence's waves, fused into one tower that walks through a solarpunk jungle. You
build the supply chain that feeds and lights the tower and defend it in real time while that
same chain keeps running — there is no separate build phase and no separate combat phase.

v1 ("Supply Line" — a walking-fortress ARPG with a hero, weapons, and a prep/combat phase
split) is archived on `main`. This is v2, in active development on the `v2` branch. v1
vocabulary (hero, weapons, companions, encounters, chapters) does not carry over; it
describes a different game.

## Stack

- **Simulation core:** Rust, compiled to WASM, deterministic — fixed 30 Hz tick, fixed-point
  math (no floats in game state), ordered iteration (no `HashMap`). See `docs/DECISIONS.md`
  for the rules and what breaks if they're violated.
- **Frontend:** React/TypeScript for UI chrome, with a custom WebGL2 renderer (not React,
  not wgpu) drawing the tower cross-section and streaming terrain every frame, independent
  of React's render cycle.

## Build and run

Prerequisites: a Rust toolchain, Node.js, and `wasm-pack`.

```bash
make check     # fmt-check + lint + test — run before every PR
make dev       # build WASM (dev, fast) + start the Vite dev server
make e2e       # stop any stale dev server, rebuild WASM, run the Playwright smoke test
make build     # wasm-pack (optimized) + native cargo build + vite build
```

See `Makefile` for the full target list (`fmt`, `lint`, `test`, `wasm`, `wasm-dev`,
`build-fast`, `build-checked`, `bench-core`, `bench-scenario`, `golden`, `mutants`, ...).

## Documentation

Start with `docs/v2-plan.md` for the whole-game plan. The doc set, in override order when
two of them disagree:

1. `docs/DECISIONS.md` — cross-cutting engineering rules (determinism, RNG streams, the
   command pattern, the replay format, content packs, balance provenance, tone, frontend
   stack) — overrides everything else
2. `docs/SYSTEMS.md` — what's actually built, grown one milestone at a time
3. `docs/v2-plan.md` — the locked whole-game plan: milestones, scope, what's coming and when
4. `docs/DESIGN.md` — the design argument behind the plan; useful for *why*, not a spec

`docs/BALANCE.md` grades every tuning constant `DESIGNED` or `PLAYTESTED`.

## Contributing

Working norms for coding agents and human collaborators live in `AGENTS.md`.
