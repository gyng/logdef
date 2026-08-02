# Understory — Systems

> Grown one milestone at a time. Each sprint writes **only its own** section, spec-first,
> before any code. If the code and this document disagree, that's a bug in one of them —
> fix both in the same commit.
>
> Whole-game context lives in `v2-plan.md`. Cross-cutting rules live in `DECISIONS.md`.
> Every number cited here is defined in `BALANCE.md` and lives in `assets/data/balance.ron`.

---

## M0 — The Stride *(chassis)*

**Sprint question:** does the walking tower feel alive on one screen?

**Scope:** the deterministic chassis and one legible slice of life on top of it — a tower
striding over streaming terrain, at a speed the player controls, with one crew member
hauling bamboo up the stairs to a mill.

**Non-goals (M1+):** charge/energy, dayparts, elevators and dumbwaiters, enemies, crew
needs, regions and route forks, T2 chains, art.

---

### 0.1 Units and arithmetic

No `f32` or `f64` appears anywhere in `GameState`, in any system, or in any command. This
is closed at M0 while the codebase is small; see `DECISIONS.md` §1.

| Quantity | Type | Unit |
|---|---|---|
| Item counts, costs | `i64` | whole items |
| Floor index | `u8` | floors from the ground, 0-based |
| Slot index | `u8` | slots from the left edge, 0-based |
| Sub-cell position, rates | `Fx` (`i32`) | Q8.8 — `FX_ONE = 256` is 1.0 |
| World distance | `i64` | Q8.8 **paces** |
| Durations | `u32` | ticks |

`Fx` is Q8.8 signed fixed point over `i32`. Multiplication and division widen through
`i64` and truncate toward zero, so the result is the same on every target. `Fx` never
converts to float inside the crate; `Fx::to_f32` exists solely for snapshot serialisation
at the presentation boundary.

The simulation runs at a fixed **30 Hz**. Wall-clock pacing is the engine's problem, not
the simulation's: `GameEngine::frame` accumulates elapsed microseconds against
`TICK_US = 33_333`, multiplies by the current speed, and runs up to
`MAX_TICKS_PER_FRAME = 12` ticks. That accumulator lives on the engine, never in state,
and never influences the outcome of a tick.

### 0.2 Randomness

Three named streams, all seeded from the run seed and all part of `GameState`:

| Stream | Feeds | May affect the economy |
|---|---|---|
| `world` | terrain bands, terrain features | yes |
| `sim` | anything the economy rolls for | yes |
| `cosmetic` | presentation-only jitter (crew idle phase) | **never** |

The firewall is a *drawing* rule, not a type rule: no system may read `cosmetic` and write
anything that another system reads. Adding a bark in M4 must not shift a single crate.
M0 uses `cosmetic` for exactly one thing — a per-crew `fidget` value handed to the
renderer at spawn — which is enough to prove the separation holds.

### 0.3 Tick order

Fixed, and load-bearing for determinism. Do not reorder without a migration of every
golden replay.

1. **stride** — advance world distance; stream terrain bands and features ahead of the
   tower; prune what is behind.
2. **intake** — intake rooms harvest from the band the tower currently occupies into
   their own output stack.
3. **production** — crafting rooms advance, consume inputs, emit outputs.
4. **haul** — crew advance their state machines, then idle crew claim tasks.

`state.tick` increments at the end. Haul runs last so crew react to the buffers this tick
actually produced.

### 0.4 World

```
World {
  distance: i64,            // Q8.8 paces walked since the run began
  stride_speed: Fx,         // paces per tick; a constant in M0, a throttle in M3
  bands: Vec<TerrainBand>,  // sorted by start, non-overlapping, covers the window
  features: Vec<Feature>,   // sorted by at
  generated_to: i64,        // paces the generator has produced through
}
```

A **band** is a stretch of terrain with one character: `Canopy`, `Clearing`, or
`RuinField`. Bands are 200–600 paces long, drawn from the `world` stream, and never
repeat the previous kind twice in a row. In M0 a band's only mechanical effect is its
`bamboo_yield`, which scales intake — canopy is rich, ruin-field is barren. That single
opposition is the seed of "your route is your power mix".

**Features** are the trees, ferns, rocks and ruins scattered along a band. They are drawn
from the `world` stream (not `cosmetic`) because M3 turns ruins into berthing sites; a
feature's position must be a fact about the run, not about the frame.

Streaming keeps `[distance - REAR_PACES, distance + AHEAD_PACES]` populated and drops
anything fully behind that window, so the world is unbounded at constant memory.

### 0.5 Tower

The v1 one-building-per-floor cap is gone from the data model on day one.

```
Tower {
  floors: Vec<Floor>,       // index 0 = ground
  shafts: Vec<Shaft>,       // vertical transport
  next_room_id: u32,
}

Floor { index: u8, slots: u8, rooms: Vec<Room> }   // rooms sorted by slot, never overlapping

Room {
  id: RoomId,
  def: RoomIdx,             // interned index into the content pack
  slot: u8, width: u8,
  inputs: Vec<Stack>,       // one per recipe input, in def order
  output: Vec<Stack>,       // outputs and storeroom shelves both live here
  progress: u32,            // ticks accumulated toward the current craft
}

Stack { item: ItemIdx, count: i64, max: i64 }
```

A room occupies `[slot, slot + width)` on exactly one floor. A shaft occupies a single
slot column on **every** floor it spans. Placement validates against both, so a shaft
column is a permanent tax on floor width — which is the point.

M0 ships four rooms: `heartseed` (unique, pre-placed, the loss condition later),
`cutter_arm` (intake, ground floors only), `mill` (bamboo → poles), and `storeroom`
(shelves, no recipe).

**Shafts** in M0 are stairs only:

```
Shaft { id: ShaftId, kind: Stairs, low: u8, high: u8, slot: u8, capacity: u8, riders: u8 }
```

Built-in stairs span every floor at slot 0 with `capacity = 1`. One rider at a time is
deliberate: with two crew you can already see the queue form. The dumbwaiter and the
elevator car sim arrive in M1.

### 0.6 Crew

```
Crew {
  id: CrewId, name: String,
  floor_fx: Fx,             // Q8.8 floor coordinate — fractional while climbing
  slot_fx: Fx,              // Q8.8 slot coordinate — fractional while walking
  carrying: Option<Stack>,
  task: Option<HaulTask>,
  state: CrewState,
  wait_ticks: u32,          // consecutive ticks blocked; drives the stress tint
  fidget: u16,              // cosmetic-stream draw, renderer-only
}

CrewState = Idle
          | Walking { to_slot: u8 }
          | Boarding { shaft: ShaftId }          // at the column, waiting for capacity
          | Climbing { shaft: ShaftId, to_floor: u8 }
          | Loading  { ticks_left: u32 }
          | Unloading{ ticks_left: u32 }
```

Movement is **L-shaped**, as in v1: walk horizontally to a shaft column, climb, walk to
the destination slot. Each leg is one state. Walking advances `slot_fx` by `walk_speed`
per tick; climbing advances `floor_fx` by `climb_speed` per tick. Both are `Fx`, so
positions are exact integers under addition and the renderer gets smooth motion for free.

A crew member blocked at a shaft accumulates `wait_ticks`. That counter is the only
bottleneck instrument in the game — there is no dashboard, and there will not be one.

**Task selection.** An idle crew member scans every room output holding at least one item
and scores each candidate delivery:

```
score = priority * 1000 - travel_cost
travel_cost = 2 * (floors between crew and pickup, plus floors between pickup and dropoff)
            + (slots walked on both floors)
```

| Priority | Destination |
|---:|---|
| 3 | a room input stack for this item, below its max — feeding a live recipe beats everything |
| 2 | a storeroom shelf with space |

Ties break on the lowest `(floor, slot)`. No destination means no task: the item sits in
the outbox, the room fills, and the room goes quiet. That silence *is* the feedback.

### 0.7 Intake and production

**Intake.** A `cutter_arm` accrues `yield_per_tick × band_yield` into an internal `Fx`
accumulator; each time the accumulator crosses `FX_ONE` it pushes one item into its output
stack and subtracts `FX_ONE`. A full output stack stalls the accumulator — the arm stops
visibly, and nothing is silently lost.

**Production.** A crafting room with every input stack at or above its per-craft amount
and room in its output advances `progress` by one tick. At `craft_ticks` it consumes the
inputs, emits `amount_per_craft` of each output (the field v1 declared and never read),
and resets. Missing inputs or a full outbox stalls at the current progress; partial work
survives the gap.

### 0.8 Commands

Every mutation is a `GameCommand`, validated before application, rejected with a typed
error, never silently dropped.

| Command | Effect | Rejects on |
|---|---|---|
| `SetSpeed { speed }` | `Paused` / `X1` / `X2` / `X4` | — |
| `BuildFloor` | append a floor, extend the stairs | floor cap, stock |
| `PlaceRoom { room, floor, slot }` | place a room from the content pack | unknown room, bad slot range, overlap, floor restriction, uniqueness, stock |
| `RemoveRoom { floor, slot }` | remove a room, refund nothing | no room there, room is unique |

Construction stock is drawn from **storeroom shelves**, not an abstract wallet: the chain
pays for the tower. The starting storeroom is pre-stocked so the first floor is buildable
before the mill has run.

`SetSpeed` lives in `GameState` rather than the engine so a save restores it and a replay
records it, but it has no effect on any individual tick.

### 0.9 Snapshots

One accessor per frame: `get_view()` returns the whole presentable state as one JSON
document. Chatty per-system accessors are a v1 mistake we are not repeating; the shape is
already grouped the way an SoA/worker bridge would want it (see `DECISIONS.md` §3).

```
ViewSnapshot {
  tick, speed, alpha,                 // alpha = fraction of a tick elapsed, for interpolation
  world:  { distance, bands[], features[] },
  tower:  { floors[{ index, slots, rooms[] }], shafts[] },
  crew:   [{ id, name, floor, slot, state, carrying, wait_ticks, fidget }],
  stock:  [{ item, count }],          // summed across storerooms, for the build affordability check
  stats:  { hauls, crafts },
}
```

`get_catalog()` returns the item and room definitions once at startup; it is static for
the lifetime of a content pack and must not be polled.

### 0.10 Replay

The replay is the save format, the regression fixture, and the seed-sharing format. There
is only one.

```
Replay {
  version: 1,
  seed: u64,
  content_hash: u64,        // XXH3 of the content pack bytes
  commands: [{ tick, cmd }],
  checkpoints: [{ tick, hash }],   // XXH3 of the serialised state
  final_tick: u64,
}
```

Semantics, exactly: commands stamped tick *T* apply **before** tick *T* runs. A checkpoint
at tick *T* is the hash of the state after *T* ticks have run, so checkpoint 0 is the
freshly seeded state. Checkpoints land every `CHECKPOINT_INTERVAL = 30` ticks.

Verification replays into a fresh engine with the same seed and compares every checkpoint,
reporting the first divergence by tick. A content-hash mismatch fails loudly rather than
replaying against different data.

`assets/replays/m0-golden.json` is embedded in the binary with `include_str!`, so the
native test and the browser both verify **the same bytes**. That is the native/wasm hash
parity gate; it runs in `cargo test` and again in the Playwright smoke test. Regenerate it
with `cargo run -p understory-core --example record_golden`.

### 0.11 Exit criteria

- [x] `make check` green; smoke test green end-to-end.
- [x] A recorded session replays bit-identically, verified natively **and** in wasm
      against the same embedded fixture.
- [x] The tower strides over streaming terrain at pause/1×/2×/4×, and one crew member
      hauls bamboo up the stairs to the mill without intervention.
