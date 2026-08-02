# Agent Guidelines — Understory

This document guides AI agents (and human engineers) working on Understory. It covers
project conventions, software practices, and pillar-specific guidance for the systems that
exist now or are coming soon.

---

## I. Project Context

### What this is

Understory is a single-player roguelike: SimTower's cross-section, Factorio's chains, and
tower defence's waves, fused into one walking garden-tower in a solarpunk jungle. You build
the supply chain that feeds and lights the tower, defend it in real time while the chain
keeps running, and keep walking — there is no separate build phase and no separate combat
phase. Rust/WASM deterministic core + React/TypeScript UI. Desktop browser first (itch.io).

v1 ("Supply Line," an ARPG with a hero, weapons, and a prep/combat phase split) is archived
on `main`. This is `v2`, on the `v2` branch. Don't carry v1 vocabulary — hero, weapons,
companions, encounters, prep/combat phases, chapters, ticks-as-currency — into new code or
docs; it describes a different game.

### Doc hierarchy

Before implementing anything, in override order:

1. **`docs/DECISIONS.md`** — cross-cutting engineering rules; overrides everything else
2. **`docs/SYSTEMS.md`** — what's actually built, grown one milestone at a time; if it isn't
   in here (or isn't in the current milestone's section), don't assume it exists
3. **`docs/v2-plan.md`** — the locked whole-game plan; sprint briefs, milestone scope,
   what's coming and when
4. **`docs/DESIGN.md`** — the distilled design argument; useful for *why*, not a spec

`docs/BALANCE.md` holds every tuning constant with a provenance grade
(`DESIGNED`/`PLAYTESTED`) — check there before hardcoding a number a designer might want to
tune, and add a graded row for any new constant (`DECISIONS.md` §7 explains why a test
enforces this).

### Architecture in one paragraph

Rust owns all game state and simulation, at a fixed 30 Hz tick. React owns UI chrome and
input collection; a custom WebGL2 renderer (not React) owns the per-frame tower
cross-section and terrain (`DECISIONS.md` §10). They talk to Rust through wasm-bindgen. The
full simulation state never crosses the bridge — one `view()` call per frame returns a
compact `ViewSnapshot`, and one `catalog()` call at startup returns content definitions.
Every mutation is a `GameCommand`, validated before it applies. `GameState` is the single
source of truth, contains no floating point, and hashes identically on native and wasm for
the same seed and command stream — see `DECISIONS.md` §1–§5 for the determinism machinery
this all rests on.

### What's built, and what isn't

As of this writing, M0 ("The Stride") is the current milestone: the deterministic chassis,
a tower striding over streaming terrain, stairs-only vertical transport, and one crew
member hauling bamboo to a mill. No charge/energy, no elevators or dumbwaiters, no enemies,
no crew needs, no regions. `docs/SYSTEMS.md` is the exact, current boundary of what exists
— read its non-goals section for a milestone before assuming a system is live.

### Project structure

```
Cargo.toml                  # workspace root: crates/core, crates/bridge
crates/
  core/                     # understory-core: GameState, systems, RNG, content, replay
    src/
      lib.rs                # module map + the four rules that don't bend
      fx.rs                 # Q8.8 fixed point (Fx) — see DECISIONS.md §1
      rng.rs                # named RNG streams + the cosmetic firewall — §2
      ids.rs                # interned content indices + runtime instance IDs
      content.rs            # RON content pack: load, intern, hash, validate
      state.rs              # GameState root
      state/
        world.rs            # World, TerrainBand, Feature — the streaming terrain
        tower.rs            # Tower, Floor, Room, Shaft, Stack, Shelf
        crew.rs             # Crew, CrewState, HaulTask — the porter state machine
      command.rs             # GameCommand, CommandResult, CommandError
      engine.rs               # GameEngine: frame/step/send/view, thin by design
      engine/
        commands.rs           # command validation + application — §4
      systems.rs               # tick() — the fixed system order
      systems/
        stride.rs              # tower movement, terrain streaming
        intake.rs               # harvesting the terrain underfoot
        production.rs           # crafting rooms
        haul.rs                  # crew state machine + task assignment/scoring
      snapshot.rs                # presentation boundary — the only place Fx::to_f32 runs
      replay.rs                  # Replay, Recorder, hash_state, embedded golden fixture
      tests.rs, tests/            # tests grouped by topic (determinism, haul, replay, ...)
    examples/
      record_golden.rs            # regenerate assets/replays/golden.json
  bridge/                         # understory-bridge: wasm-bindgen entry points (cdylib)
    src/lib.rs
assets/
  data/                           # RON content pack: balance.ron, items/, rooms/, terrain/
  replays/golden.json          # the embedded native/wasm parity fixture
web/                              # React/TypeScript frontend (Vite)
  src/
  e2e/smoke.spec.ts               # Playwright smoke test
docs/
  v2-plan.md, DESIGN.md, SYSTEMS.md, DECISIONS.md, BALANCE.md
```

There is no renderer crate — the earlier wgpu stub was deleted. The renderer is a frontend
concern; see §VI.

---

## II. Dev Commands

```bash
# Unified (Makefile)
make check                  # fmt-check + lint + test — run before every PR
make fmt                    # auto-format Rust + TypeScript
make lint                   # clippy + frontend typecheck + lint
make test                   # cargo test
make build                  # wasm-pack + native cargo build + vite build
make wasm                   # build the WASM bridge only (wasm-pack, optimized)
make dev                    # build WASM (dev, fast) + start Vite dev server
make dev-stop                # kill any stale vite holding port 3000
make e2e                     # stop stale vite, rebuild wasm, run the Playwright smoke test

# Rust
cargo fmt --all              # format
cargo fmt --all --check      # verify formatting
cargo clippy --all-targets -- -D warnings   # lint; warnings are errors
cargo test                    # test the workspace

# Regenerate the golden replay fixture (crates/core/examples/record_golden.rs)
cargo run -p understory-core --example record_golden

# Mutation testing on the core crate (occasional, not part of make check — slow)
cargo mutants

# Frontend (from web/) — ts7 (tsgo) / oxfmt / oxlint, not tsc/prettier/eslint
npm run typecheck             # tsgo — type-checks without emitting
npm run lint                  # oxlint
npm run format                # oxfmt — write
```

See `DECISIONS.md` §10 for why the frontend toolchain is ts7/oxfmt/oxlint rather than the
tsc/prettier/eslint stack v1 used, and why the renderer is custom WebGL2 rather than SVG.

---

## III. Determinism Discipline

This is covered in full, with the reasoning and the tests that enforce it, in
`DECISIONS.md` §1–§6. In practice, day to day:

- **Never add an `f32`/`f64` to `GameState`, a system, or a `GameCommand`.** Use `Fx`
  (`crates/core/src/fx.rs`) for anything sub-integer. If you need a fractional constant in
  code, write `Fx::ratio(1, 3)`, never a float literal.
- **Never call `Fx::to_f32` outside `snapshot.rs`.** If a system needs to compare or scale
  a fixed-point value, do it in `Fx` arithmetic; conversion to float is a presentation-only
  operation.
- **Never introduce a `HashMap`/`HashSet`.** Clippy denies it (`disallowed_types` in the
  workspace lints) — use a `Vec` (sorted, if you need lookup) instead. If clippy is
  complaining about this, don't suppress it; restructure the data.
- **Don't reorder `systems::tick`** (stride → intake → production → haul) without
  understanding that it invalidates every golden replay. If a change requires reordering,
  that's a determinism-affecting change and needs a regenerated fixture plus a deliberate
  note about why, not a quiet fix.
- **A flaky test under `crates/core/src/tests/determinism.rs` is a P0**, not a retry. It
  means the same seed and commands produced two different outcomes, which is the property
  replays, saves, and seed sharing all depend on.
- **Validate fully before mutating in every command handler.** See `DECISIONS.md` §4. If
  you're not sure whether your handler can partially apply before failing, write the test
  from `a_rejected_command_changes_nothing` against it before you trust it.

---

## IV. Software Engineering Best Practices

### Test-driven development

- **Write the test first**, especially for simulation systems — the command pattern makes
  this natural: feed a `GameCommand` to a `GameEngine`, assert the resulting state or the
  rejection.
- **Test at the system boundary.** Command in, state (or `ViewSnapshot`) out. Don't unit
  test a private helper unless it holds genuinely complex logic in isolation (scoring,
  fixed-point math, interning).
- **Property-based testing for procedural generation.** Terrain band generation, and later
  region and encounter generation, should satisfy invariants (bands never repeat their
  predecessor's kind, the streaming window stays populated, no orphaned state) across many
  seeds, not just a handful of examples.
- **Determinism is testable — see §III.** Treat it as a first-class test category, not an
  afterthought bolted onto functional tests.
- **Snapshot tests for bridge output shape.** `snapshot.rs`'s `ViewSnapshot`/`CatalogSnapshot`
  are the bridge's public contract; a shape change there is a frontend-breaking change and
  should be deliberate.
- **Always run a smoke test before wrapping up.** If a change touches React, WASM, the
  bridge, or the frame loop, run the smoke path as part of verification, not just
  `cargo test`. A browser-only failure is a regression even when every Rust test passes.
- **Run the smoke test with `make e2e`, in the foreground.** `make e2e` stops any stale
  vite, rebuilds WASM, and lets Playwright own the dev server for the run. Do not run
  `npx playwright test` inside a background shell wrapper — if the wrapper dies (timeout,
  ctrl-c, tool abandonment), the vite child it spawned survives as a zombie on port 3000
  and poisons the next run. If you ever need to clear a rogue dev server, run `make dev-stop`.

### Code organization

- **Separation of concerns is structural, not aspirational.** The simulation crate
  (`understory-core`) knows nothing about rendering or React. The frontend renderer knows
  nothing about game logic. If you're importing across that boundary, stop — you're
  violating the architecture, not extending it.
- **Command pattern is non-negotiable.** No system, input handler, or snapshot builder
  mutates `GameState` directly. Every change is a validated `GameCommand`. This is what
  makes the game replayable and debuggable — see `DECISIONS.md` §4.
- **Prefer data over code.** Room recipes, terrain yields, crew rates — these live in
  `assets/data/*.ron` and are graded in `BALANCE.md`, not hardcoded. If you're writing a
  tuning number a designer might want to change, it belongs in data.
- **Ordered iteration everywhere.** `Vec`, sorted where lookup matters. See §III.
- **No floating-point surprises.** Identical operations in identical order, always through
  `Fx`. If determinism breaks, suspect floating point (or a `HashMap`) first.

### Error handling

- **Validate at the command boundary.** `engine::commands::apply` validates legality —
  enough stock? valid floor and slot range? not already occupied? — and rejects illegal
  commands with a typed `CommandError`. Never silently ignore a bad command.
- **Panic on impossible states.** If the simulation reaches a state that should be
  structurally impossible (a room with no matching category behavior, a negative stack
  count), panic with a descriptive message — these are bugs, not edge cases to paper over.
  `Content::load_embedded` panicking on an invalid shipped content pack is the model: a
  broken pack is a build error, not a runtime condition to degrade gracefully from.
- **Degrade gracefully in presentation.** If the renderer gets unexpected data, render a
  fallback rather than crash. If audio gets an unrecognized `SoundEvent`, drop it. The
  simulation is authoritative; presentation layers are resilient to being asked to draw
  something odd.

### Performance discipline

- **Profile before optimizing.** The frame budget is generous at cozy scale (8–14 floors,
  single-digit crew and room counts). Don't optimize speculatively.
- **Allocation-free hot paths in the tick.** `systems::tick` and its four systems should not
  allocate per tick where avoidable — reuse buffers, avoid `String` construction in a
  system.
- **Batch bridge calls.** One `view()` call per frame, not one accessor per subsystem. See
  `DECISIONS.md` §3.

### Version control

- **Small, focused commits.** One system per commit; don't mix a simulation change with a
  UI refactor.
- **Commit messages reference the design doc.** "Add elevator car dispatch
  (`docs/SYSTEMS.md` M1 §x.y)" makes review easier than a bare summary.
- **Balance changes get their own commits**, noting what motivated the change and the
  before/after values, since `BALANCE.md`'s reasoning column is meant to stay accurate.

---

## V. Rust/WASM Practices

- **`GameState` is the single source of truth.** If you're computing something derived,
  keep it out of `GameState` unless there's a specific reason it needs to be
  save/replay-stable, and document that reason.
- **Serialize everything.** `GameState` must be fully `serde`-serializable; that's the hash
  input for determinism (`replay::hash_state`) and the save format. A field that can't
  serialize doesn't belong in state.
- **No ECS framework.** The struct-of-arrays-ish shape (`Tower` owns `Floor`s, `Floor`s own
  `Room`s, `GameState` owns a flat `Vec<Crew>`) is deliberate at this entity count. Don't
  reach for `bevy_ecs`/`specs`/`legion`.
- **No threads.** WASM threading has browser compatibility costs not worth paying at this
  scale. Don't reach for `rayon` or `tokio` in `understory-core`.
- **Minimize bridge crossings.** Each `wasm_bindgen` call serializes. Batch commands where
  it makes sense; keep the per-frame call count at two (`frame`, `view`) plus whatever
  commands the player actually issued that frame.
- **String IDs in data, dense indices in the simulation.** Content is authored as
  `"room.mill"`; the tick loop only ever sees `RoomIdx`. See `DECISIONS.md` §6. If you find
  yourself comparing a `&str` inside a system, that string should have been interned at
  load.

---

## VI. Frontend Practices

- **React is chrome, not the renderer.** The tower cross-section and streaming terrain are
  drawn by a custom WebGL2 renderer (batched, with a DOM overlay for text) running its own
  frame loop, independent of React's render cycle — see `DECISIONS.md` §10 for why this is
  the plan from M0 rather than an SVG-first, WebGL-if-needed path. React owns panels, menus,
  and other UI chrome, and calls into the bridge for commands and snapshots.
- **No external state library.** No zustand, redux, or jotai. React state/context holds UI
  state only (panel open/closed, hover targets, the last fetched snapshot) — never
  authoritative game state. If you're building a parallel state tree that mirrors
  `GameState`, stop; call the bridge accessor and use the result.
- **Throttle to the sim's rate where it matters.** The sim ticks at 30 Hz; there's no reason
  to poll `view()` faster than the renderer's frame loop needs, and no reason to re-render
  React chrome on every WebGL frame.
- **No game logic in components.** Components render and dispatch commands. They don't
  compute production rates, validate placements, or run pathfinding — that's Rust's job.
- **ts7/oxfmt/oxlint, not tsc/prettier/eslint.** See §II for the commands and
  `DECISIONS.md` §10 for the reasoning.

---

## VII. Pillar Guidance

Covers the pillars that exist now (M0) or are coming soon (M1–M2). Combat, journey/region,
and meta-progression pillars aren't built yet — check `docs/SYSTEMS.md` before writing
guidance-driven code against a system that isn't there yet.

### Logistics & transport contention

The core bet of the whole game (`DESIGN.md` §2 insight 1) is that transport is shared, not
dedicated per-chain. Every new production room you place adds load to the same stairs (and,
from M1, the same dumbwaiters and elevator shafts) everyone else is already using.

- **Runner/crew pathfinding is greedy, not planned.** `systems/haul.rs` scores every
  candidate (item, destination) pair for each idle crew member — `priority * 1000 -
  travel_cost` — and picks the best. It doesn't plan a global schedule. This is deliberate:
  greedy, locally-reasoned behavior is what makes crew readable and predictable to a
  player watching the cross-section.
- **A shaft's capacity is the whole point, not a limitation to work around.** `Shaft.capacity`
  gating riders is what turns "add a chain" into "add load to shared infrastructure." Don't
  quietly raise default capacities to make queues go away — that removes the tension the
  design rests on.
- **Buffer overflow is visible, never silently discarded.** An intake accumulator or a
  production output that's full stalls in place (`intake.rs`, `production.rs`) rather than
  dropping the overflow. If you add a new production or intake room, its stall behavior
  should follow this pattern by construction, not by special-casing.
- **Nothing a crew member picks up is ever destroyed.** If a destination fills mid-trip,
  they get re-tasked; if there's nowhere, they hold it. Don't add a code path that drops
  carried items — it breaks a stated invariant in `haul.rs` and will show up as items
  vanishing, which reads as a bug, not a feature.
- **Test what v1 already proved out, extended.** Chain delivery across multiple rooms,
  stalls under a full buffer, capacity limits, slot collisions, and now shaft contention
  under multiple crew — see `crates/core/src/tests/haul.rs` and `tests/production.rs` for
  the existing shape to extend.

### Tower building

- **A floor holds many rooms.** `Floor.rooms: Vec<Room>` is unbounded by design — the
  interesting layout question (what shares a floor with what) has to be askable from the
  data model on day one. Don't reintroduce a one-room-per-floor constraint anywhere,
  including in UI affordances that only let the player picture one room per floor.
- **A shaft costs a slot column on every floor it spans.** `Tower::slot_range_blocked`
  checks both rooms and shaft columns. This is the mechanical expression of "vertical
  transport is the belt" (`DESIGN.md` pillar 2) — a shaft is a permanent tax on every
  floor's width, and that tax is deliberate, not a bug to optimize away.
- **Construction is paid from storeroom stock, not an abstract wallet.** `check_stock` /
  `spend` in `engine/commands.rs` draw from shelves the chain actually filled. If you add a
  new buildable, its cost should draw from the same stock pool — there's no separate
  currency to introduce.
- **Growing taller has a real cost beyond `floor_cost`.** From M1, a new top floor
  displaces the canopy sail deck (`v2-plan.md` §6.3). When that lands, don't let floor
  addition become a free action just because the ticks/materials are paid — the height
  cost is structural, not just economic.

### Procedural world streaming

- **Terrain is a stream, not a level.** `World::generate_ahead`/`prune_behind` keep a
  constant-size window around the tower — generate to `stream_ahead_paces`, drop anything
  before `distance - stream_behind_paces`. A run of any length costs the same memory; don't
  add a path that accumulates unbounded history "for the minimap" or similar without
  pruning it the same way.
- **Determinism applies to world generation too.** Bands and features are drawn from the
  `world` RNG stream (`DECISIONS.md` §2), not `cosmetic` — a feature's position has to be a
  fact about the run (M3 turns ruins into berthing sites) rather than a fact about which
  frame it happened to render on.
- **Anti-frustration constraints belong in the generator, invisibly.** `pick_band_kind`
  already guarantees no band repeats its predecessor's kind. As region/threat generation
  comes online, keep this pattern: hard constraints enforced in code and tested across many
  seeds, never a runtime check that the player can perceive as a rule.
- **Property-test generation, not just example-test it.** Once regions and route forks
  exist, the right test shape is "N seeds → every generated stretch satisfies invariant X,"
  matching how `content.rs`'s validation and `world.rs`'s band-repeat rule are already
  structured to be checkable mechanically rather than by eyeballing one seed.

### UI/UX — diegetic-first

Covered in full, with the enforceable version of the rule, in `DECISIONS.md` §8. In
practice: a stalled room renders quiet, a blocked crew member tints red past
`stress_ticks`, and neither of those facts gets a second, numeric representation as a
warning banner. Precision numbers are a hover-only layer, never the primary signal. If
you're building a UI element and reaching for a dashboard-style readout as the *first*
thing the player sees, look for the diegetic version first — a piece of the cross-section
that already changes state — before adding a new number to the screen.

### Audio

Not built yet as of M0 (`v2-plan.md` targets the audio pass at M4), but the intended shape
carries forward from v1's philosophy: production loops go silent when a room is starved,
not muted with a separate "problem" sound; `SoundEvent`s (`systems.rs`) are fire-and-forget
— emitted during a tick, consumed or dropped by the JS `AudioManager`, never read back into
the simulation. When audio work starts, keep that boundary: Rust decides *that* something
happened, JS decides whether and how it sounds.

### Narrative / crew tone

Crew are named individuals with jobs, not stat blocks (`DESIGN.md` §2 structural call 4).
When crew-facing content (barks, portraits, names) lands, keep it off the `sim` RNG stream
— use `cosmetic` (`DECISIONS.md` §2), so that adding or editing a bark can never perturb an
economic roll. Tone follows `DECISIONS.md` §8: solarpunk warmth, defenders not soldiers,
creatures defending territory rather than a target gallery to clear. If a piece of crew
dialogue or a creature description reads as militaristic, that's a tone bug worth flagging
even if it's mechanically inert.
