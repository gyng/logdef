# Understory

**[Play it in a browser](https://gyng.github.io/logdef/)** — the `v2` branch, deployed to
GitHub Pages by `.github/workflows/pages.yml` on every push. It is the same bundle
`make release` zips for itch.io.

A walking garden-tower roguelike: SimTower's cross-section, Factorio's production chains,
and tower defence's waves, fused into one tower that walks through a solarpunk jungle. You
build the supply chain that feeds and lights the tower and defend it in real time while that
same chain keeps running — there is no separate build phase and no separate combat phase.

v1 ("Supply Line" — a walking-fortress ARPG with a hero, weapons, and a prep/combat phase
split) is archived on `main`. This is v2, in active development on the `v2` branch. v1
vocabulary (hero, weapons, companions, encounters, chapters) does not carry over; it
describes a different game.

## State

A run is playable start to finish: from the first pace to an arrival at the Refugia, 31–36
minutes at 1×. M0–M4 and M6 are shipped and M5 is most of the way there — `docs/SYSTEMS.md`
is the exact, current boundary of what exists, so read the milestone section for a system
before assuming it is live.

What is missing is the difficulty pass. Every constant in `docs/BALANCE.md` is graded
`MEASURED` — an instrument confirms the effect the constant exists to produce, and the limit
of that measurement is written into the row — and **none is `PLAYTESTED`**, which that file
defines as somebody having played with it *and with neighbouring values*. The distinction is
load-bearing and the gap is a person's work rather than an agent's; `docs/PLAYTEST.md` is
what to do and what to write down.

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

Alongside them: `docs/BALANCE.md` grades every tuning constant `DESIGNED`, `MEASURED` or
`PLAYTESTED` and records the limit of what each measurement showed; `docs/PLAYTEST.md` is
the open criteria that need a person at the keyboard; `docs/ART.md` is the asset handoff;
`docs/RENDERER.md` covers what is drawn procedurally, what was tried and reverted, and the
measured frame budget.

The measuring instruments live in `crates/core/examples` and answer the design questions the
tests cannot. `make check` deliberately does not run them — `make instruments` does. Read
their output rather than their exit code: an instrument measuring the wrong thing exits zero,
and several have.

## Contributing

Working norms for coding agents and human collaborators live in `AGENTS.md`.
