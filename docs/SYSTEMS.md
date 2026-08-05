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

> **Superseded by §1.6.** M1 inserted the clock, power, and transport systems and moved
> stride to the end. The four systems below still run in this relative order; the current
> full order is in the M1 section.

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

**Width is a balance lever, not a constant.** Rooms are one, two or three slots wide, and
which is a statement about the room:

| Slots | Rooms | Why |
|---|---|---|
| 3 | Heartseed, salvage rig | The heart takes the room a hearth takes. A rig is a boom long enough to reach into a ruin from the deck. The canopy sails were the third three-slot room and the widest thing the tower carried — roof-only, the cost of the charge economy stated in floor space rather than poles — and M6 cut them (§6.10). |
| 2 | Cutter arm, mill, burner, storeroom | The working middle. |
| 1 | Thornwright, cell bank, dart battery | A bench, a rack, and — deliberately the smallest thing in the pack — an emplacement. Defence competes for floor with the chain that pays for it, and at two slots it was a decision most towers declined. Cheap in space and expensive in ammo is the better trade. |

Everything was two wide until M3, which meant eight slots was always exactly four rooms and
the question this section exists to pose — what shares a floor with what — could not be
asked. The opening tower is now laid out to leave awkward gaps rather than tidy ones, with
exactly one three-wide hole in it (floor 1, where the rig wants to go) and none at all on
the ground floor, so salvage and harvest compete for the same scarce low deck.

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

> **Superseded at M3, and wrong as shipped.** `yield_per_tick` is a *fraction of an item*,
> and in Q8.8 a fraction that small has almost no resolution: `Fx::ratio(1, 90)` is `Fx(2)`,
> so an arm authored at 90 ticks ran at 128, and `Fx(2) × band_yield` was very nearly a
> no-op, so four authored terrain yields behaved as two. Intake now accumulates *effort*
> against a threshold, and against paces rather than ticks. See §3.6 and
> `intake::terrain_effort`. The stall behaviour described above is unchanged and still
> correct.

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

`assets/replays/golden.json` is embedded in the binary with `include_str!`, so the
native test and the browser both verify **the same bytes**. That is the native/wasm hash
parity gate; it runs in `cargo test` and again in the Playwright smoke test. Regenerate it
with `cargo run -p understory-core --example record_golden`.

### 0.11 Exit criteria

- [x] `make check` green; smoke test green end-to-end.
- [x] A recorded session replays bit-identically, verified natively **and** in wasm
      against the same embedded fixture.
- [x] The tower strides over streaming terrain at pause/1×/2×/4×, and one crew member
      hauls bamboo up the stairs to the mill without intervention.

---

## M1 — The Chain *(the whole bet)*

**Sprint question:** is elevator contention actually fun? Everything downstream assumes yes.

**Scope:** the three systems that make the rest of the game possible — a real chain
(bamboo → poles → darts), a real energy economy (charge), and real vertical transport (the
elevator car simulation, plus the dumbwaiter). This is the milestone the project is a bet
on. The elevator ships before the eleventh anything.

**Non-goals (M2+):** no enemies, no crew needs or shifts, no regions or route forks, no
T2 chains, no art pass.

---

### 1.1 The clock

A day is `ticks_per_day` long and repeats forever. Two things read it: the sun, and the
elevator's per-daypart programs.

**Dayparts** are content (`assets/data/dayparts/*.ron`): an id, a display name, and the
per-mille of the day at which it starts. Seven of them, from `predawn` to `night`. They
exist so a player can say "run the freight program at night" and so the UI has something to
name; the simulation only uses the index.

**The sun curve** is separate, and is balance rather than content: a list of
`(permille_of_day, sun_pct)` anchors, linearly interpolated in integers. Sun is a curve, not
a staircase, because a step change in charge income at a daypart boundary would read as a
bug.

```
sun_pct(tick)  = lerp over the anchor table at (tick_of_day * 1000 / ticks_per_day)
exposure_pct   = sun_pct * terrain.sun_pct / 100
```

### 1.2 Charge

**Charge is a stored flux, not a crate.** One pool, capacity summed from the cell banks in
the tower. It is never hauled, never sits in a stack, and never appears on a shelf.

**Income**

| Source | Rate |
|---|---|
| Canopy sails | `sail_charge_per_100_ticks × exposure_pct / 100`, **top floor only** |
| Burner | `burner_charge_per_100_ticks`, consuming `burner_bamboo_per_charge`; player-toggled |

Sails only generate on the top floor. Build a floor above them and they go dark — that is
the literal cost of height promised in `v2-plan.md` §6.3, and it is a placement decision the
player has to keep re-making as the tower grows. A shaded sail renders stalled like any
other quiet room; there is no warning popup.

The burner is the dirty fallback: it turns the contested material into power. From M2 its
smoke raises provocation, which is what stops it being a free answer.

**Draw**, in this fixed order each tick:

| Sink | When | Notes |
|---|---|---|
| Transport | a car moves a floor | `charge_per_floor`, per car |
| Production | a powered room advances a craft | `power_draw` per tick |
| Lighting | `exposure_pct` is below `night_light_threshold` | per floor |
| Striding | the tower is walking | `stride_charge_per_100_ticks` |

The order is the priority order: when the pool cannot cover everything, the sinks at the
bottom fail first. Striding is cut before the chain stalls, and the elevator freezing
mid-shaft only happens when things are genuinely dire — which is what makes it land as an
emergency rather than as noise.

A failed draw does not go into debt. The consumer simply does not act this tick: the car
holds position, the room holds its progress, the floor goes dark, the tower stands still.
`Power.brownout` is set whenever any draw failed, and it is what the cross-section reads to
dim the tower.

`SetStriding { walking }` lets the player halt to bank charge. The full stride throttle —
speed as a continuous economic dial — is M3; this is the boolean subset M1 needs for the
bank-or-burn decision to exist at all.

### 1.3 Vertical transport

Three kinds now, with `ShaftKind` deciding which of the mechanisms below applies.

| Kind | Carries | Autonomous | Charge | Span |
|---|---|---|---|---|
| Stairs | crew | — | free | whole tower |
| Dumbwaiter | items | yes | per trip | 2–3 floors |
| Elevator | crew (and what they carry) | no | per floor travelled | chosen floors |

**The elevator is the machine.** The dispatch model is ported from the trace-verified
SimTower behaviour in `phulin/tower-together` rather than invented (`v2-plan.md` §12), because
"how does an elevator decide where to go" is a solved problem with a specific, recognisable
feel, and getting it wrong would make the milestone's design question unanswerable.

```
Car { pos: Fx (fractional floor), dir: Up|Down|Idle, state, riders, stops }
CarState = Idle | Moving | Dwelling { ticks_left }
```

* **Fixed floor queues.** Each served floor holds a queue of crew waiting to travel, with
  the direction each wants. A queue is a fact about the floor, not about the car — which is
  what lets several cars share one shaft later without rewriting anything.
* **Bidirectional sweep.** A moving car continues in its current direction serving every
  stop and every same-direction call ahead of it, reverses when nothing remains ahead, and
  idles when nothing remains at all. This is the behaviour players recognise as "an
  elevator", including the part where it sails past you going the wrong way.
* **Dispatch threshold.** An idle car does not depart for a single caller immediately. It
  waits until either `dispatch_threshold` people are queued or the oldest call has waited
  `dispatch_max_wait` ticks. Batching is what creates the queue the player can see, and
  removing it would quietly remove the contention this milestone is testing.
* **Dwell.** A stop costs `dwell_base + dwell_per_unit × (boarding + alighting)` ticks.
  Loading is not free, and a busy floor is slow to leave.
* **Capacity, in units.** A crew member is one unit; carrying a load makes them two. That
  is the freight extension — crates board like passengers, at a different weight.
* **Programs, per daypart.** A `ShaftProgram` names the floors a car will serve and a
  priority (`Balanced`, `FreightFirst`, `CrewFirst`). One program per daypart, so a player
  can run a different pattern on the night shift.

**The dumbwaiter** is the inserter: item-only, autonomous, no crew involved. When idle it
looks across its spanned floors for the best (source, destination) pair using the same
priority the crew use — a hungry recipe outranks a shelf — loads a batch, travels, and
unloads. It serves any room on a floor it spans regardless of horizontal slot, which is
exactly why it is worth its slot column and its charge.

**Choosing a shaft.** Crew no longer take the first shaft that spans the trip; they
estimate. Stairs cost `climb_ticks_per_floor × floors`, plus a penalty when the shaft is at
capacity. An elevator costs an expected wait derived from where its car is and which way it
is pointing, plus travel and dwell. Add the horizontal walk to each column and take the
cheapest. The estimate does not have to be right — it has to be *deterministic* and roughly
sensible, so that a player who adds a shaft sees the crew start using it.

### 1.4 The chain

`bamboo → poles → darts`, across at least three floors.

| Room | Recipe | Notes |
|---|---|---|
| Cutter arm | terrain → bamboo | ground floors only |
| Mill | bamboo → poles | |
| Thornwright | poles → darts | **powered** — the first charge sink in production |
| Canopy sails | exposure → charge | top floor only |
| Burner | bamboo → charge | player-toggled |
| Cell bank | — | adds charge capacity |
| Storeroom | — | shelves |

Darts have no consumer until the dart batteries arrive in M2. That is a deliberate,
recorded exception to the rule in `DECISIONS.md` §9.1: the plan's M1 scope names the chain
explicitly, the storeroom does consume them as stock, and M2 is where they get eaten. If M2
slips, darts get cut rather than left sitting.

### 1.5 What is readable, and how

No new dashboards. Everything below is a change to what the cross-section already draws.

| State | Signal |
|---|---|
| Starved room | draws quiet — already true in M0 |
| Backed-up outbox | fill bar at maximum, room quiet |
| Queue at a shaft | crew tint toward red as `wait_ticks` climbs |
| Shaded sails | the sail room draws stalled |
| Brown-out | the tower's interior lighting drops out |
| Night | the sky darkens and lit floors glow |

### 1.6 Tick order, as of M1

> **Superseded by §2.8.** M2 inserted `siege` and `defence` between production and haul,
> and `repair` after haul. The eight systems below still run in this relative order; the
> current full order is in the M2 section.

Charge priority *is* tick order: consumers draw from a shared pool as they run, so who runs
first is who gets served when it is thin.

1. **clock** — advance the day.
2. **power income** — recompute capacity from the banks; collect from sails and burners.
3. **transport** — cars move. First claim on charge, because a car freezing mid-shaft
   should be the last thing that happens, not the first.
4. **intake** — harvest the band underfoot.
5. **production** — recipes advance, consume, emit. Powered rooms pay here.
6. **haul** — crew advance their legs, then idle crew claim work.
7. **lighting** — lamps, after dark.
8. **stride** — the tower walks if it can still afford to, and terrain streams in.

### 1.7 Exit criteria

- [x] A deliberately under-built tower visibly bottlenecks at the shaft, and adding a shaft
      visibly fixes throughput. **Measured**: +90% crafts, 56% fewer ticks queueing
      (`cargo run --release -p understory-core --example throughput`). Needed
      `starting_crew` 2 → 3 to be true at all.
- [x] A night with no banked charge browns out; a night with banked charge does not.
- [x] Golden replay regenerated and verified natively and in wasm; hash-parity runs in CI.
- [x] `make check` and the smoke suite green.

**Deferred out of M1:** per-daypart elevator programs exist in the data model, the command
layer, and the replay format, but have no UI — a player cannot yet change them without
issuing a command by hand. The night-shift program is the reason they exist, so this should
land alongside M4's shift rota if not before.

---

## M2 — The Siege *(the load test)*

**Sprint question:** does combat-as-logistics-stress produce drama without any aimed weapon?

**Scope:** enemies that damage infrastructure, emplacements that are fed by the chain, a
repair loop that competes with everything else for the same crew and the same poles, and a
provocation knob that ties all of it back to how you have been playing.

**Non-goals (M3+):** no feral wardens or ruin salvage, no enclaves, no regions, no meta.

### 2.1 The shape of the thing

Combat is not a mode. There is no phase change, no pause, no separate screen — that split
is precisely what made v1's most interesting moment structurally impossible. A wave is a
**demand spike on the circulation you already have**: darts to the batteries, repair crews
to the breach, on the same stairs the mill is using.

The player's verbs stay infrastructural: where a battery goes decides what it can reach,
whether the chain keeps it fed decides whether it fires at all, and triage — which repair,
if any, is worth committing a crew member to right now — falls out of what has been built
and how full the storeroom is, not a menu. Nothing in M2 adds an aimed weapon, a targeting
priority list, or a tower-level ability to fiddle with, and nothing should.

### 2.2 Enemies

They live on the terrain layer, approach the tower, and attack **infrastructure** rather
than a hit-point bar. Each type teaches one lesson, and a type without a lesson is clutter:

| Type | Lesson |
|---|---|
| skitters | ammo drain economics — cheap, numerous, and they make you count darts |
| canopy leapers | drop onto *upper* decks from overhanging trees, so height is exposure |
| root-borers | gnaw legs and shaft columns, so transport needs redundancy |
| night predators | nocturnal pressure — the reason you banked charge in M1 |

Content, not enum arms, following `ShaftDef`'s precedent: `assets/data/enemies/*.ron`,
interned like everything else.

Contact does not last forever. Once a creature reaches the tower it holds on for
`cling_ticks` — content on `EnemyDef`, tuned per type in `BALANCE.md` — and when that runs
out it lets go and is left behind, rather than being destroyed. The clock only runs while
the tower is actually striding (`GameState.strode`, not the player's `walking` intent — a
tower that cannot afford the charge to move shakes nothing off either), and it runs on
anything that has reached the tower, not only on a creature mid-bite: one that has run out
of things to chew and dropped back to circling is still counting down, so a tower stripped
to its Heartseed doesn't keep a wave orbiting it indefinitely. That makes `SetStriding`
(`command.rs`) a real answer to a wave that costs nothing in poles or darts — keeping the
legs moving is a legitimate way to survive one, and stopping to work mid-assault is a
genuine risk rather than a free action. Cling timers are tuned per creature rather than
uniform: a root-borer's is well short of the time it needs to sever a shaft alone, so one
borer on a walking tower gets a column partway down and loses its grip before finishing,
and it takes two overlapping borers, or a tower that stopped moving, to actually sever one.

A creature leaves the fight one of two ways, and the distinction is a fact the player can
see, not an implementation detail: `Dying` is shot down by an emplacement, `Leaving` is a
creature whose grip ran out. Both fade over `enemy_fade_ticks` rather than disappearing on
the tick they end — long enough at 1x, and still visible at 4x, that an outcome reads as
something that happened rather than something a counter reports after the fact. Only
`Dying` counts toward the `repelled` readout: walking away from something is not the same
as seeing it off, and per the tone guardrail in `DECISIONS.md` §8 the game should never
claim otherwise.

### 2.3 Damage as a state of the tower

Damage attaches to the things the player built, because that is what makes it legible:

* **Panels** — per floor. Breached panels let things inside.
* **Rooms** — a damaged room works slower; a destroyed one is gone, with its contents.
* **Shafts** — a severed shaft column splits the tower's circulation in two. This is the
  signature emergency, and `best_shaft` already routed around what did not span a trip, so
  the reroute falls out of the existing model rather than needing a special case:
  `tests::siege::a_severed_shaft_forces_a_live_reroute`,
  `a_severed_shaft_is_no_longer_a_route`,
  `a_severed_shaft_puts_everyone_on_it_back_on_their_feet`, and
  `a_tower_with_one_shaft_stalls_when_it_is_cut` (the case where there is nowhere to
  reroute to) all hold.

### 2.4 Emplacements

Rooms with a `defence` block: a dart battery on a balcony, a seed-bomb mortar on a deck.
They auto-fire at the nearest live target in range — a battery has no judgement of its own;
the player's judgement went into where they put it — and consume ammo from a **local
rack**, which is just an input stack, so feeding them is the haul system's existing job,
and a battery that runs dry does so for exactly the same reason a mill does.

That equivalence is the whole design. If emplacements get their own special supply
mechanism, combat stops being a load test and becomes a parallel game.

The chain behind that rack has its own arithmetic, and it is not free of tradeoffs. A
thornwright turns poles into darts three to the one. Its own craft timer runs a touch
slower than a mill's, which reads as "a thornwright can't outpace one mill" on paper — but
a mill's *realised* pole output is well short of its nominal rate once haul latency is
counted, so in practice a single thornwright reliably out-consumes a single mill's actual
output. A tower that wants a steady dart supply **and** poles left over for repair needs a
second mill, not a faster thornwright (`docs/BALANCE.md`, thornwright recipe). Shooting
stays the cheap side of that trade regardless: a skitter costs two darts to put down and
does about 48 hit points of damage over a full, unanswered cling — the better part of five
poles to mend (`repair_poles_per_10_hp`) — so putting one down is consistently cheaper than
letting it bite, provided the rack has darts in it at all.

### 2.5 Repair

Repair consumes poles and crew time. It is a chain sink like any other, and it
competes for the same three crew. Triage — letting a floor stay breached because the mill
matters more right now — is the interesting decision, so repair must never be automatic and
never free.

Crew do not chase scratches. A repair job is only started against damage worth at least one
shift (`repair_hp_per_shift`) of hit points, because a shift costs its poles whether it
mends twenty hit points or one — starting one on a mark that has lost four would throw poles
away for almost nothing. Below a shift's worth of damage, the mark simply stays on the
tower: the cross-section is still doing its job as the health readout, it just isn't a job
worth a crew member's time yet.

### 2.6 Provocation

One knob, raised by aggressive harvesting, burner smoke, and (from M3) salvaging ruins.
It feeds the threat table. Tone-safe by construction: the creatures defend their territory
and the tower is the thing passing through — see `DECISIONS.md` §8. Nothing in the UI
should frame this as a kill count.

### 2.7 Loss

The Heartseed is already placed, already unique, already undemolishable. M2 gives it hit
points and makes its destruction the end of the run.

### 2.8 Tick order, current

> **Supersedes §1.6.** M2 inserted `siege` and `defence` between production and haul, and
> `repair` after haul.

1. **clock** — advance the day.
2. **power income** — recompute capacity from the banks; collect from sails and burners.
3. **transport** — cars move. First claim on charge, because a car freezing mid-shaft
   should be the last thing that happens, not the first.
4. **intake** — harvest the band underfoot.
5. **production** — recipes advance, consume, emit. Powered rooms pay here.
6. **siege** — creatures approach and attack; provocation decays. Runs before defence so an
   emplacement fires at where a creature actually is this tick, not where it stood a tick
   ago.
7. **defence** — emplacements fire at what siege just moved.
8. **haul** — crew advance their legs, then idle crew claim work.
9. **repair** — crew already at damage put hit points back. Runs after haul because repair
   competes with hauling for the same crew, and hauling's claims on that crew are settled
   first.
10. **lighting** — lamps, after dark.
11. **stride** — the tower walks if it can still afford to, and terrain streams in.

### 2.9 Exit criteria

- [x] A severed shaft mid-assault forces a live reroute. **Confirmed**:
      `tests::siege::a_severed_shaft_forces_a_live_reroute`,
      `a_severed_shaft_is_no_longer_a_route`,
      `a_severed_shaft_puts_everyone_on_it_back_on_their_feet`, and
      `a_tower_with_one_shaft_stalls_when_it_is_cut` (the case where there is nowhere left
      to reroute to). The second half of this criterion — that the reroute is *legible*,
      that you can see why the crew changed route without opening a debug view — is a
      question you answer by looking, and a passing test cannot answer it. The renderer
      draws a severed column as two pieces sheared past each other with dust still falling
      out of it, but no captured still has yet caught one mid-run. Listed as deferred
      below rather than claimed here.
- [x] A brown-out night assault is survivable with banked charge and lethal without.
      **Confirmed**: `tests::power::a_banked_night_is_survivable_and_an_empty_one_is_not`
      (carried over from M1, still holds).
- [x] Does combat-as-logistics-stress produce drama without an aimed weapon? **Yes**,
      measured by the five-day, three-tower comparison in
      `cargo run -p understory-core --example siege_run`: `subsistence` (one cutter arm,
      never expands) is left alone, ending at full (1000‰) integrity with 220 poles banked
      and nothing left to spend them on; `greedy` (a second cutter arm and nothing else)
      degrades to 743‰, runs out of poles by day 2, and has stopped repairing entirely by
      day 4; `answered` (the second arm, plus a mill, a thornwright, and a dart battery)
      holds full (1000‰) integrity through two days of rising provocation, sees off 51
      creatures, and is still mending on day 5. Defence is a choice that pays for itself;
      expanding without it has a visible, mounting price.
- [x] Golden replay regenerated; hash parity green natively and in wasm. **Confirmed**: the
      fixture now runs 30,000 ticks and exercises the elevator, a thornwright, a
      demolition, a dart battery, and a wave with damage and repair
      (`crates/core/examples/record_golden.rs`).
- [x] `make check` and the smoke suite green — 166 Rust tests, 7 Playwright tests.

**Deferred out of M2:**

- **Most of the creature taxonomy is unexercised in play.** The pack defines four
  creatures, but `min_provocation` gates the canopy leaper at 330, the night prowler at
  200, and the root-borer at 500 (`docs/BALANCE.md`), and the measured five-day run above
  peaks around provocation 250. So the skitter, and barely the night prowler, are the only
  creatures a normal run has actually met; the leaper's "height is exposure" lesson and the
  borer's shaft-severing emergency are reachable only by a much louder or much longer run
  than the one measured here. They are tested in isolation (`tests/siege.rs`) but not
  balanced in situ. M5's full taxonomy pass is where this gets settled.
- **A severed shaft has not been looked at.** The reroute is tested and the renderer draws
  the break, but the screenshot harness (`web/e2e/capture.spec.ts`) has never caught one:
  it takes a root-borer, which is gated at provocation 500, and a captured run peaks around
  250. Until somebody has seen it, the legibility half of the first exit criterion above is
  a claim about code rather than about the game.
- **One emplacement, and no priority targeting.** M2's brief in `v2-plan.md` §9 asked for
  a dart battery *and* a seed-bomb mortar "with priority targeting." Only the battery is
  built, so "which defence to build" is not yet a decision — only "whether" — and
  `defence.rs` picks the nearest creature in range with no way for the player to say
  otherwise. The targeting half is a deliberate cut rather than an oversight: a priority
  list is the kind of menu `DECISIONS.md` §8 argues against, and placement already decides
  what an emplacement can reach. The second emplacement is not a cut, just undone, and
  belongs with M5's taxonomy pass where there is something for it to be good against.
- **Repair costs poles and crew time, not rope.** The brief said "poles+rope+crew". There
  is no rope item in the pack and nothing that would make one anything but a second tax on
  the same haul, so it was not authored — `v2-plan.md` §10's first process rule says no
  content type until a system consumes it.
- **Building under fire is neither slow nor exposed.** The brief asked for construction to
  cost something extra during an assault. It does not: `PlaceRoom` is as instant mid-wave
  as it is in the quiet. The pressure M2 does apply — that poles spent building are poles
  not spent mending — turned out to be sharp enough on its own in the measured runs, so
  this was left rather than stacked on top of an economy that was already too tight. If it
  comes back it should be revisited against the numbers, not added on principle.
- **A creature giving up sounds like a creature dying.** `EnemyState::Leaving` and
  `EnemyState::Dying` are distinct in the state and in the snapshot (§2.2, above), but the
  audio layer has no separate cue yet, so walking a wave off and shooting it down sound the
  same.
- **Per-daypart elevator programs still have no UI**, carried forward unchanged from M1's
  deferred note (§1.7, above).

---

## M3 — The Journey *(the run)*

**Sprint question:** does run pacing work, and does the route-is-your-power-mix tension
actually bite?

**Scope:** the run becomes a journey. Two regions with opposed terrain palettes, route
forks that split the way ahead, drowned ruins worth stopping to strip, the feral wardens
that guard them, one enclave a short way into the second region, and the walk/stop decision
made economically real by moving intake off the clock and onto the ground covered.

**Non-goals (M4+):** no region 3 and no Refugia arrival, no unlocks or meta-progression,
no art pass, no crew needs or shift rota, no per-region creature tables beyond a single
threat multiplier, no enclave repair.

### 3.1 The shape of the thing

A run is a walk through an ordered sequence of **regions**. Nothing about the world model
changes to accommodate that: terrain is still a stream, `World::generate_ahead` and
`prune_behind` still keep a constant-size window around the tower (§0.4), and the tower
still walks down one distance axis from zero. What M3 changes is *which* bands the
generator draws from — it asks the region the tower is in rather than a single pack-wide
weight table — and it hangs three things on that axis worth stopping for: a **fork**, a
**ruin**, and an **enclave**.

Distance is the run's clock, and the tower only advances while it is walking. That makes
stop-or-go the verb that actually spends the run, which is why M3's other half is making
that verb cost something in both directions (§3.6). Everything below is a reason to stop or
a reason not to.

There is no map screen, no node graph, and no travel mode. A region is a stretch of the
same axis with a different palette; a fork is a place on that axis where the world runs out
until you say which way it continues; an enclave is a place you can park next to. The
`v2-plan.md` §3 structural call — "continuous world, no node map" — survives intact, and
none of the three additions needs a second kind of space to live in.

### 3.2 Regions

A **region** is content: `assets/data/regions/*.ron`, interned like everything else
(`DECISIONS.md` §6).

```
RegionDef {
  id: String,                  // "region.deep_jungle"
  name: String,
  order: u8,                   // position in the journey, 0-based
  length_min_paces: i64,       // rolled once per run, from the world stream
  length_max_paces: i64,
  ruin_richness_min_pct: i64,  // likewise; scales what this region's ruins hold
  ruin_richness_max_pct: i64,
  palette: Vec<TerrainWeight>, // { terrain: String, weight: i64 }
  threat_pct: i64,             // multiplier on a wave's threat budget
  fork_interval_paces: i64,    // 0 for a region with no forks
  branches: Vec<BranchDef>,
  enclave: Option<EnclaveDef>, // where in the region, if anywhere, people live
}
```

Regions are traversed in order and sorted by `order` rather than by `id`, so `RegionIdx`
is both the interned index and the position in the journey. That is the second deliberate
exception to the sort-by-string-ID rule in `DECISIONS.md` §6, for the same reason dayparts
are the first: a list whose meaning is a sequence must be stored in that sequence, or the
index lies. Validation requires the `order` values to be a contiguous run from zero, with
no duplicates.

**Two things about a region are rolled from the seed, not authored.** They are the answer to
a real risk in this design: fork spacing, the enclave, and the palettes are all content, and
region length was going to be too — leaving two seeds to differ only in band order and where
the ferns are, which is unlikely to clear the exit criterion that two seeds feel meaningfully
different (§3.10). Rather than give up the predictable fork rhythm to buy variety, the
variety is bought structurally and cheaply:

* **Length**, drawn once per region from `[length_min_paces, length_max_paces]`. Fork
  positions stay on the fixed interval (§3.3), so what the roll changes is **how many
  decisions a region contains** — a long draw fits another fork, a short one does not —
  without forks becoming unpredictable *within* a run. The min/max pair mirrors
  `band_min_paces`/`band_max_paces`, which is how the pack already expresses "a length with
  a range."
* **Ruin richness**, a percentage drawn once per region and applied to what each of its
  ruins holds (§3.4). This is the roll the player will actually feel, because it changes
  whether stopping is worth it: one run's drowned city is picked over and grudging, the
  next one's is worth berthing at three times. It scales the amount rather than the count,
  so a poor city still has ruins in it — visibly near-empty ones, which reads as
  disappointment rather than as absence.

Both draw from `world`, never `cosmetic`: how long a region is and how much is left in it
are facts about the run, and two people sharing a seed must get the same journey
(`DECISIONS.md` §2).

**Every roll happens once, at run start.** `World` carries the result:

```
World {
  ...
  journey: Vec<RegionRoll>,    // { end: Paces, ruin_richness_pct: i64 }, one per region
  region: RegionIdx,
  region_start: Paces,
}
```

Rolling the whole journey up front rather than region by region on entry is what keeps the
generator simple. Because a branch is a palette override rather than a detour (§3.3), region
boundaries are still fixed absolute distances for the life of a run — they are just fixed by
the seed rather than by the content — so "which region is the tower in" and "which region is
the generator producing into" both stay pure functions of a distance. Rolling on entry would
mean the generator, which streams up to `stream_ahead_paces` past a boundary, had to produce
terrain for a region whose length had not been decided yet.

`region` and `region_start` are derivable from `distance` and `journey`, and are stored
anyway for one specific reason: crossing a boundary is an **event**, not just a fact. The
stride system compares the stored region against the region the tower's new distance falls
in, and on a mismatch it does the region's one-time setup — placing the enclave, resetting
the fork schedule. `region_start` keeps the fork arithmetic local and readable rather than a
running sum recomputed at each use.

**The two regions.**

| | Region 1 — Deep Jungle | Region 2 — The Drowned City |
|---|---|---|
| Character | canopy-heavy: biomass-rich, sun-poor | ruin-heavy: sun-rich, biomass-poor |
| Palette | canopy heavy, clearing moderate, ruin-field light | drowned street and ruin-field heavy, clearing light, canopy light |
| Ruins | scarce | common, and how much they hold is a seeded roll |
| `threat_pct` | 100 | higher |
| Enclave | none | one, a short way in |

The opposed sun and biomass rates `v2-plan.md` §9 asks M3 for **already exist**, per
terrain, from M0: `TerrainDef` carries `yield_pct` against `sun_pct`, and
`assets/data/terrain/canopy.ron` is 140 against 35 where `ruin_field.ron` is 50 against
130 — no band is allowed to be good at both, and
the field comment says so. What M3 adds is that opposition at *region* scale. In M0 and M1
the opposition was a texture: a band lasts 300–900 paces, roughly sixteen to fifty seconds
at 1×, so the tower crosses the whole spectrum several times an in-game day and the mix
averages out. A region lasts long enough that it does not average out. Walking into the
drowned city means the mill goes hungry and the banks fill for hours, not for a minute —
the journey itself changes the tower's power mix, and the player has to rebuild around it
rather than wait it out. That is the difference between "your route is your power mix" as a
sentence and as a decision.

Two consequences the palette has to respect:

* **A palette needs at least three kinds with positive weight.** `pick_band_kind` never
  repeats the previous band's kind. With a two-entry palette that rule degenerates into
  strict alternation — a perfectly regular ABABAB horizon, which reads as a bug. Validation
  rejects a palette with fewer than three positive weights, in the same spirit as the
  anti-frustration constraints already living invisibly inside the generator.
* **The drowned city needs a band of its own.** Re-weighting the three M0 kinds would make
  region 2 read as "region 1 with more ruins" rather than as somewhere else. M3 authors one
  new terrain, `terrain.drowned_street` — high sun, low yield, ruin-bearing — so the city
  has a face. Reusing `clearing` and `canopy` at low weight is what keeps the green
  reclaiming the concrete visible.

`TerrainDef.weight` is deleted. Its job — how often a kind comes up — now belongs to the
region palette, and leaving a pack-wide weight in place as a second knob doing the same job
would be one of the two numbers going stale. Validation gains the matching check: every
terrain kind must appear with positive weight in at least one region palette, because a
terrain no region can produce is content with no consumer (`DECISIONS.md` §9.1).

`threat_pct` multiplies the provocation-scaled threat budget in `siege::maybe_spawn_wave`,
after the affordability gate and before `base_threat`'s floor. One knob, not a table: M2
already deferred most of the creature taxonomy as unexercised in play (§2.9), and adding
per-region creature tables on top of creatures a run has never met would be authoring
content for a system that is not yet consuming what it has. Per-region threat tables belong
with M5's taxonomy pass.

### 3.3 Route forks

At `fork_interval_paces` intervals inside a region, the route splits in two.

Fork **spacing** is content, not seed: the first fork sits at `region_start +
fork_interval_paces`, the next one an interval further on, and so forth. Predictable
punctuation is a feature — the player should learn the rhythm of "another choice is coming"
without having to watch for it, and a fork that could arrive at any moment would be an
interruption rather than a beat.

A fork is skipped if it falls within `fork_edge_margin_paces` of either end of the region or
of the region's enclave, so a decision never lands on top of a boundary or a berth and
competes with it for the same stretch of horizon.

What the seed decides is therefore **how many** forks a region has and **what each one
offers**. Fork count falls out of the region's rolled length against the fixed interval
(§3.2): a long draw fits one more decision in than a short one. The two branch archetypes at
each fork are drawn from the `world` stream. Spacing stays regular; the number of beats and
the content of each one do not.

Each region authors a set of **branch archetypes**:

```
BranchDef {
  id: String,                  // "branch.canopy_passage"
  name: String,
  length_paces: i64,
  palette: Vec<TerrainWeight>,
  threat_pct: i64,
}
```

The two a fork draws are always distinct, and the choice between them is the choice. Taking
one overrides the region palette for
`length_paces` past the fork and multiplies the region's `threat_pct` by the branch's, then
the route rejoins the region. **A branch is a palette override, not a detour.** There is no
second distance axis, no route tree in state, and no rejoin arithmetic: the tower keeps
walking down the one axis it has always walked down, and for a stretch the terrain it walks
through is drawn from somewhere else. Everything downstream — streaming, pruning, region
boundaries, replay — is unchanged by construction.

**What the player is told before committing.** The fork card names each branch and lists
its two heaviest terrain kinds and a word for its threat (*quieter* / *as usual* /
*louder*), all derived from the branch's own palette and `threat_pct`. There is deliberately
no authored blurb: a hand-written line describing a branch can drift out of step with its
palette during tuning, and a game that misdescribes the only informed choice it asks the
player to make is worse than one that describes it drily. The description is generated from
the data it describes, so it cannot lie.

**The tower halts at a fork it has not been given an answer for.** New command:

| Command | Effect | Rejects on |
|---|---|---|
| `TakeFork { branch }` | commit to branch 0 or 1 of the pending fork | no fork pending, no such branch |

Three things make the halt right rather than arbitrary.

*It is what the world does, not a rule imposed on top of it.* The generator produces terrain
`stream_ahead_paces` in front of the tower, and past an unanswered fork there is nothing to
produce — the palette beyond depends on an answer that does not exist. So generation stops
at the fork line. The tower halts because the ground it would walk onto has not been decided
yet, and the renderer draws exactly that: the terrain strip ends, and the fork is the
horizon. The property test `the_tower_is_always_standing_somewhere` continues to hold,
because the tower never crosses into the ungenerated stretch.

*It costs the player the thing M3 has just made expensive.* The halt reuses the stop
machinery wholesale (§3.6): `strode` stays false, so nothing clinging to the tower loses its
grip (`DECISIONS.md` §11), the cutter arms harvest nothing, and the charge the legs would
have burned is banked. A fork reached in the middle of a wave is a genuine emergency, and a
fork reached with the banks nearly empty is a small mercy. That is the same trade every
other stop in the game presents, which is exactly why it needs no new machinery and no new
UI mode.

*It needs no modal dialogue and no pause.* The fork is visible from up to
`stream_ahead_paces` out — roughly fifty seconds at 1× — and `TakeFork` is legal from the
moment it appears. A player who answers early never stops at all. The halt is not the
decision; it is what happens when the decision is late. That distinction is what keeps the
fork from being a speed bump, and it is the reason to reject the obvious alternatives: a
modal pause would make the choice a mode (against `v2-plan.md` §3), and auto-picking a
branch would make it not a choice.

An answer may be replaced while the fork is still pending — the last `TakeFork` before the
tower reaches the fork line is the one that counts. Once the tower crosses, the branch is
committed and the fork is cleared. A player who never answers stands there indefinitely,
banking charge while the waves keep arriving on their own schedule; that is a legitimate,
bad outcome and needs no special handling.

**The halted states have to be visually distinct, and this is a renderer requirement rather
than a simulation one.** A tower the player stopped, a tower waiting at a fork, and a tower
that has reached the end of the world are the same silhouette with the same legs still and
the same `strode` false, and they are three completely different situations. The fork halt
in particular must read as *waiting for you* — the terrain strip ending at a fork in the
path, with two ways named — or it looks like the game has frozen, which is the one reading
that would make the whole argument above worthless. The arrival halt needs its own reading
too, since standing still at the far edge is the run being over rather than a decision
pending.

**Anything that drives the engine without a player has to answer forks.** `examples/siege_run.rs`
and `examples/throughput.rs` both step tens of thousands of ticks with no commands beyond a
shopping list, and a harness that walks into a fork and stops measures a parked tower with
total confidence — the exact failure mode `siege_run.rs`'s own comments record it having had
once already, when a room it thought it had built had in fact been refused. Both harnesses,
the golden-replay recorder, and the Playwright smoke test need a standing fork answer, and
that is a change to make in the same commit as the halt, not after the numbers come out
wrong.

```
World {
  ...
  fork: Option<PendingFork>,   // { at, branches: [BranchIdx; 2] }
  branch: Option<ActiveBranch>,// { def, from, to }
}
```

### 3.4 Berthing: ruins, the salvage rig, feral wardens

Some `Feature`s are **berthing sites** — drowned ruins with something left in them. The
world module has been ready for this since M0, and says so:

> Drawn from the **world** stream rather than the cosmetic one, because M3 turns ruins into
> berthing sites — where a ruin stands has to be a fact about the run, not about the frame.
> — `crates/core/src/state/world.rs`

That comment is the whole justification for the field's existence on the economically live
stream, and M3 is what cashes it. A ruin's position, and how much it holds, are facts about
the seed; two people sharing a seed pass the same ruins.

```
Feature {
  at, kind, scale, layer,
  salvage: i64,   // whole units of scrap left; 0 for ordinary scenery
  roused: bool,   // this ruin has already woken its wardens
}
```

`TerrainDef` names which of its `feature_kinds` are ruins and the range each holds, so the
canopy's ferns and the ruin-field's broken frames scatter through the same generator and
only the latter come out salvageable. Both the choice of which features are ruins and the
amount each holds are drawn from the `world` stream at scatter time, in
`World::scatter_features`, alongside position and scale — and the amount is then scaled by
the **ruin richness** rolled for the region the band belongs to (§3.2). A picked-over
drowned city and a generous one are the same ruins in the same places holding different
amounts, which is why the renderer draws `salvage` rather than just drawing a ruin: the
player can see from the strip which stops are worth making, and a lean region reads as
disappointment rather than as an empty map.

**The salvage rig** is a new intake room, `room.salvage_rig`, ground floors only. It works
exactly like a cutter arm except for where it draws from, and that difference is expressed
in the content rather than in a special case:

```
enum IntakeSource {
  Terrain { paces_per_item: i64 },
  Ruin    { ticks_per_item: u32, range_paces: i64 },
}
```

A `Terrain` source accrues per pace walked (§3.6). A `Ruin` source accrues per tick, and
only while the tower is stopped with a ruin inside `range_paces`. **Walking harvests
bamboo; stopping harvests scrap.** The two intakes are exact opposites, and the stop/go
decision is therefore also an intake-mix decision — which is the cleanest possible statement
of what M3 is for. A rig with a full outbox stalls in place like every other intake room
(§0.7), and the ruin keeps whatever it has not given up.

**Berthing is implicit.** There is no `Berth` command. A tower that is stopped with a
working rig in range of a ruin is berthing; a tower that walks on is not. Range is a
property of the rig (`range_paces`), the way a dart battery's reach is a property of the
battery — a longer-reaching rig is a thing M5 can author, and putting the number on the room
means the player reads it where they choose to build it. Stopping next to a ruin with no
salvage rig does nothing at all: nothing is extracted, nothing is roused, and there is no
error, because there is no command to reject. The empty space in the build menu is the
affordance.

**Feral wardens** are what the ruin has instead of a lock. The first tick a rig extracts
from a ruin rouses it: one wave, sized against how much the ruin held at that moment
(`warden_threat_per_100_salvage`, floored at a single warden), spawning out of the ruin
itself rather than at the usual `spawn_paces_ahead`, offset by `warden_wake_paces` so there
is a few seconds of warning between the ground moving and the first bite. `Feature.roused`
means a ruin wakes once and only once, so leaving and coming back is not an exploit.

Wardens are content, `assets/data/enemies/feral_warden.ron`, with one new field on
`EnemyDef`:

| Field | Value | Why |
|---|---|---|
| `wave_eligible` | `false` | Ordinary waves draw from every creature whose `min_provocation` the tower has passed. A warden is not summoned by attention; it is summoned by berthing, so it has to be excluded from that pool explicitly rather than fenced off with an out-of-range `min_provocation`. |

**This is the counterweight to the cling rule, and it is the best thing in the milestone.**
`DECISIONS.md` §11 makes a creature's grip count down only while the tower is actually
striding: walking shakes things off, so "keep the legs moving" is a real, free answer to a
wave, and stopping to work mid-assault is a real risk. Berthing is the one time the tower
*cannot* walk away — not because a new rule forbids it, but because walking away is what
ends the salvage. So the one place the game puts something worth stopping for is the one
place the existing escape hatch is closed, and it closes itself. No new mechanic produces
this; §11 already did, and M3 simply builds the room that makes it matter. A warden's grip
never runs out while you keep working, and the moment you decide the scrap is not worth it
you start walking and it does.

Salvaging also raises provocation, closing the forward reference M2 already recorded in
§2.6 ("aggressive harvesting, burner smoke, and (from M3) salvaging ruins"). The rate should
sit close to `provocation_per_100_harvested` rather than far above it: the wardens are the
price of a ruin, and charging a second, much louder price in provocation on top would make
salvage a thing nobody does twice.

The arithmetic the balance pass has to settle, stated here so whoever runs it knows what
the equation is: a ruin's scrap is worth some number of poles at the enclave (§3.5), the
wardens it rouses cost some number of darts to see off and some number of poles to mend
what they chew through, and salvage is only a decision if those two numbers are close
enough that the answer depends on the tower. Both sides — ruin size and the enclave's
exchange rate — move together and must be tuned together.

### 3.5 The enclave

One, standing a short way **into** region 2 rather than on the boundary. It is a settlement
the tower walks past: berthing works exactly as it does at a ruin — stop within range — and
the tower that keeps walking loses it, because there is no going back down the axis.

```
EnclaveDef {
  id: String,
  name: String,
  at_paces: i64,           // offset from the start of the owning region
  offers: Vec<OfferDef>,   // { give: (item, amount), take: (item, amount), stock: i64 }
  recruits: u8,
  recruit_cost: Vec<CostEntryDef>,
}
```

**Where it stands is a considered departure from a locked plan, so it is recorded here
rather than buried.** `v2-plan.md` §6.6 says "enclaves between regions," and the obvious
reading of that is the boundary. Spec'd that way, the enclave puts scrap's only consumer
*upstream* of the only region that produces much scrap — the drowned city — so everything
salvaged past it is dead weight, and the milestone's headline mechanic pays out only for the
handful of ruins region 1 scatters. Worse, it compounds: region 2's palette already halves
bamboo yield, per-pace intake (§3.6) means every minute spent berthing costs bamboo the
tower did not walk past, and `threat_pct` is higher — a tower that engages with salvage
arrives in the hard region poorer, louder, and with no restock ahead of it. That is a design
that punishes the player for playing the thing M3 is about.

A short way in fixes all of it and buys a pacing beat the boundary version did not have: you
cross into the hard region, work its edge, find out what its ruins are worth this run, and
*then* find people to trade with. `at_paces` is small relative to the region — far enough in
that arriving with something to trade is the normal case, near enough that the enclave still
provisions the bulk of the region rather than arriving after it matters. The spirit of
"between regions" survives; the letter does not.

Two things happen here.

**Trade** exchanges items at posted rates. Each offer has finite `stock`, so the enclave is
a windfall rather than an exchange to farm, and the rates are deliberately worse than the
chain's own: the enclave is where a tower that lacks a room buys its way around the gap
once, not a substitute for building the room. The offers that earn their place are scrap for
poles (the payoff for everything in §3.4, and the reason scrap exists at all in M3), surplus
bamboo for poles at a rate a mill beats comfortably, and poles for darts at a rate a
thornwright beats comfortably — so a tower heading deeper into the city without a
thornwright can still arm itself, and pays for the privilege.

**Recruit** adds one crew member for poles, up to a new `crew_cap` (a `CrewBalance` field;
M3's target is a little above the starting three, with the plan's cap of around eight left
for M4's shift rota to earn). Priced steeply, because `starting_crew` was the first constant
in the game to be graded `PLAYTESTED` and what it measured was that going from two crew to
three moved throughput by ninety percent (§1.7). A fourth pair of hands is the largest single
change a player can buy, and it should cost like it. The enclave offers exactly one. The new crew member takes the next name from the
placeholder list in `state.rs` by index — not by a roll — so recruiting perturbs no stream;
their `fidget` is drawn from `cosmetic`, as every crew member's is (`DECISIONS.md` §2).

| Command | Effect | Rejects on |
|---|---|---|
| `Trade { offer }` | take one offer, once | not berthed at an enclave, no such offer, offer exhausted, insufficient stock |
| `Recruit` | one crew member for poles | not berthed at an enclave, no recruits left, crew at cap, insufficient stock |

Both spend from and deliver to storeroom shelves through the existing `check_stock` /
`spend` path in `engine/commands.rs` — the chain pays for the enclave the same way it pays
for the tower. `v2-plan.md` §6.2 leaves room for trade tokens at enclaves; M3 declines to
introduce one. A currency that exists in exactly one place is a second economy with a single
customer, and item-for-item exchange keeps the shelves the only thing worth filling. If M5's
enclave economy wants a token, it can add one against several enclaves that use it.

The berth range for an enclave is a `JourneyBalance` field rather than a property of a room,
since docking at a settlement is not the salvage rig's job. That new balance section is where
`fork_edge_margin_paces` (§3.3) lives too — the handful of journey-wide constants that belong
to no single room or creature.

**What is unavoidably UI, honestly.** The berth is diegetic: the enclave is drawn on the
terrain strip, the tower parks beside it, and walking on ends it — no screen is entered and
nothing is paused. The transaction is not. Taking an offer is a button on a posted board,
and the goods appear on the shelves without a crew member carrying them. The genuinely
diegetic version — the enclave as a dock the crew haul to and from, with each accepted offer
becoming a haul job — was considered and cut for M3: it needs a `HaulDestination` outside the
tower and a leg of the crew state machine that walks off the edge of the cross-section, which
is a change to the system M1 was a bet on, spent on a single waystation. If it is worth
doing, M5's enclave economy is where there is enough enclave to justify it.

The tone guardrail applies to the board's copy as much as to anything else (`DECISIONS.md`
§8): the enclave is people who live here and the tower is passing through, so the offers
read as an exchange between neighbours, not as a merchant's inventory.

**Shell work, and what scrap is for.** The settlement will plate the tower's hull for
scrap: `ReinforceDef` on the enclave, a cost and a `panel_hp` figure and a number of times
they will do it. It is the only permanent upgrade in the game.

It exists because salvage had nowhere to go. Scrap's only other consumer is the trade board,
whose offers are finite and behind you the moment you walk on, so a run that berthed at the
drowned city's ruins banked metal it could not spend — recorded as a deferred hole when the
enclave moved into region 2, and closed here. Plating turns a city's worth of old metal into
hull, which is what it ought to become.

Three properties that matter more than the numbers:

* **It applies to floors that do not exist yet.** The bonus lives on `Tower.shell_bonus`, not
  on each floor, so a storey built afterwards arrives already plated. Otherwise growing
  taller would mean growing a soft spot, and the player would have to remember which floors
  had been done.
* **New material arrives as material.** `max` and `hp` both go up, so plating does not leave
  an undamaged tower reading as freshly damaged.
* **It is not a repair.** A panel already breached comes back plated and still breached.
  Shell work cannot be used to skip the repair loop (§2.5) it is meant to make survivable —
  which is also why enclave *repair*, listed in `v2-plan.md` §6.6, stays cut.

### 3.6 Walk and stop

`v2-plan.md` §9 calls this "walk/stop throttle economics." The interpretation taken,
recorded here because it is a deliberate narrowing of what §6.2 of that plan and
`BALANCE.md`'s `stride_paces_per_100_ticks` row both anticipated:

**`SetStriding` stays binary.** Walk or stop. No speed notches, no continuous dial. The plan
describes striding as "a player-set throttle," and a throttle implies a range — but a range
would be a new decision layered on top of an old one that does not yet cost anything, and
M3's job is to make the existing decision economically real, not to add a knob. Five speeds
over a choice that is nearly free is a worse game than two speeds over a choice where each
option gives up something the other has. If, after M3, walk-or-stop turns out to be *too*
coarse in play, notches are a small addition to a system that already prices motion; adding
them first would have priced nothing.

Four opposed forces bear on that one binary choice:

| Walking | Stopping |
|---|---|
| spends charge (`stride_charge_per_100_ticks`) | banks it |
| covers ground — distance is the run's clock | holds a berth: salvage, trade, recruit |
| harvests, because the cutter arms strip what they pass | harvests nothing at all |
| shakes off anything clinging (`DECISIONS.md` §11) | lets everything attached keep its grip |

Three of those four are already true. The fourth is the change that makes the decision bite.

**Intake becomes per-pace rather than per-tick.** `IntakeDef` for a `Terrain` source is
authored as `paces_per_item` instead of `ticks_per_item`, and `intake::run` accrues against
the ground actually covered rather than against the clock. A stopped tower harvests nothing.

This is not a new claim about the fiction; it is the fiction finally being true. The cutter
arm's own description already says so: *"Strips bamboo from the terrain as the tower walks
past it"* (`assets/data/rooms/cutter_arm.ron`). Per-tick intake meant a parked tower stripped
bamboo out of ground it had already stripped, indefinitely, which is the sort of thing that
is invisible until stopping becomes a thing players do on purpose — and M3 gives them three
reasons to.

**The shipped value is 78 paces, not the 54 a one-for-one conversion gives.** At
`stride_paces_per_100_ticks` of 60, the cutter arm's authored `ticks_per_item` of 90 is
`paces_per_item` of 54 — and the arm had never run at 90 ticks. Intake accrued a truncated
per-tick *fraction of an item*: `Fx::ratio(1, 90)` is `Fx(2)` in Q8.8, so the arm ran at 128
ticks, and every M2 measurement was taken against that figure rather than against the
content. Worse, scaling a fraction that small was very nearly a no-op — `Fx(2) * 1.40` is
also `Fx(2)` — so dense canopy and open clearing harvested at *identical* rates, and ruin
field and drowned street collapsed together too. Four authored terrain kinds behaved as two,
and the flagship contrast of the route being the power mix (`DESIGN.md` pillar 1) was not in
the simulation at all. The region palettes in §3.2 are built on exactly those differences,
so per-pace intake could not be shipped on top of it.

Rooms now accumulate *effort* against a threshold rather than a fraction of an item against
one, which keeps the precision where it is needed; `intake::terrain_effort`'s doc comment is
the authoritative account. 78 paces is 130 ticks at the shipped stride — a shade slower than
the mill's 120 ticks a pole, which is what the M2 economy was actually measured against.
Shipping 54 alongside the fix would have made the arm 33% faster than one mill can consume,
and because a shelf holds one kind, bamboo would claim every shelf, poles would have nowhere
to go, and the chain would deadlock with the storeroom three-quarters full — recoverable
only out of poles stuck inside the mill, which construction cannot draw from. See
`docs/BALANCE.md`'s `storeroom` row for that failure written up in full; it is reachable
today with a second arm and no second mill, and it will read as a bug to a player who does
not know why.

**This is the riskiest change in M3.** It revalues every constant the M2 balance run settled,
and it does so indirectly, through a chain that is long enough to be hard to reason about
from the desk: stopping cuts bamboo, which cuts poles, which cuts both repair and darts,
which moves the whole siege curve — and because provocation is driven by
`provocation_per_100_harvested`, a tower that stops is also quieter, so the difficulty dial
moves in the opposite direction at the same time. There is one genuinely new failure mode to
watch for: a tower too poor in charge to walk (`power::pay_for_stride` already stops it) now
also cannot harvest, so it cannot make poles, so it cannot build its way out — a brown-out
becomes a spiral rather than a bad night.

That spiral has a floor, and it is worth knowing where it is before treating this as a
reason not to make the change. `collect_solar` does not read `walking` or `strode`: the
sails fill whether or not the legs are running, so a tower stranded overnight is walking
again by mid-morning under its own power, having lost a night's harvest rather than the
run. The genuinely unrecoverable case is narrower — a night entered with no banked charge,
no bamboo to burn, and the tower already stopped — and it is reachable only after the
player has already spent everything twice over. Design that as a bad night with a hard
morning, not as a trap; if the re-measurement shows it landing more often than that, the
lever is `starting_charge` or the burner's fuel cost, not reverting per-pace intake.

**What was promised here, and what is true instead.** This passage originally said twice
that the always-walking case would be *arithmetically identical* to M2's, because 54 paces
is 90 ticks and 90 ticks was what the arm was authored at. That rested on a false premise:
the author did not know the rate was being truncated, and an identical *authored* number
would have been a 42% faster *actual* arm. What holds instead is the honest version of the
same guarantee — the always-walking case reproduces the rate M2 was **measured** at, because
the arm has been re-authored to state that rate out loud (78 paces, 130 ticks, against the
128 it was really running). The conversion is a measurement, not an arithmetic identity, and
that is a weaker claim, so the second bound matters more than it did: the terrain palettes in
§3.2 move the numbers on their own, and — now that `yield_pct` is doing anything at all —
they move them further than anyone had reason to expect.

Both bounds were re-measured together against
`cargo run --release -p understory-core --example siege_run`, and that harness is the reason
`provocation_per_100_harvested` came down from 300 to 240: with the yield multiplier finally
live and region 1's palette at 55% canopy, a subsistence tower gained about 13% a day and
started provoking on its own. `docs/BALANCE.md`'s five Siege rows —
`wave_interval_ticks`, `base_threat`, `provocation_per_100_harvested`,
`provocation_decay_per_100_ticks`, and `repair_poles_per_10_hp` — carry the re-measured
figures and stay `PLAYTESTED`; they were graded against this harness, and this harness has
been re-run. The shape it now reports is sharper than M2's rather than merely preserved:
`subsistence` ends whole with 180 poles banked, `greedy` is pole-broke from day 3 and slides
to 860‰, and `answered` holds 992‰, sees off 63 creatures **and** still banks 83 poles — so
defence visibly pays for itself instead of just slowing a decline.

### 3.7 The run

A run is a seed. It starts in region 1 at distance zero, with the tower, crew, and stock
`GameState::new` already builds, and it ends exactly two ways:

* **The Heartseed is lost.** M2's loss condition, unchanged (§2.7).
* **The far edge of region 2 is reached.** The generator has nothing past the last region,
  so the tower halts there the same way it halts at an unanswered fork — the world has run
  out — and `arrived` is set.

Arrival is presented as an arrival, not a victory: the same register as the elegy the
frontend already shows when the Heartseed goes, reporting where the tower got to rather than
grading it (`DECISIONS.md` §8).

**M3's finish line is a placeholder.** `v2-plan.md` §6.6 puts three regions between the
start and the Refugia; M5 adds the third and the actual arrival. Region 2 in M3 is
deliberately shorter than region 1 — it exists to prove the region machinery works with more
than one region in it and to give the drowned city's opposed palette somewhere to bite, not
to be a second full act.

The snapshot grows one group, `journey`, carrying the current region and how far through it
the tower is, any pending fork and its two branches, any active branch, whatever the tower
is berthed at, the enclave's remaining offers, and `arrived`. `FeatureView` grows `salvage`
so the renderer can draw a ruin that still has something in it differently from one that has
been stripped — a change to the bridge's public contract, and therefore a frontend change
made deliberately rather than discovered (`AGENTS.md` §IV).

**Opening values.** Collected here so whoever authors the content has one list rather than
nine paragraphs to mine. Every figure below is a **design target** — a first value with an
argument behind it, not a measurement. None of them is a `BALANCE.md` row until it has been
authored, and each gets a graded row in the same commit that authors it (`DECISIONS.md` §7).
All the arithmetic assumes the tower's current 0.6 paces per tick and 30 Hz.

| Thing | Target | The arithmetic |
|---|---|---|
| region 1 length | 52,000–68,000 | About 55 minutes of unbroken 1× walking at the midpoint (60,000 paces is 100,000 ticks, roughly seven in-game days), sized so a session that stops for a berth and a fork or two lands in the 45–60 minute window the exit criterion asks for. The ±8,000 spread is a little over half a fork interval, so most seeds differ by one fork and none by more than one. |
| region 2 length | 34,000–46,000 | Shorter on purpose: a placeholder act, not a second full one. |
| region ruin richness | 60–140% | Wide enough that a lean drowned city and a generous one are different propositions rather than different rounding. |
| region 1 palette | canopy 55 / clearing 30 / ruin-field 15 | Canopy-heavy, so the region reads as biomass-rich and sun-poor, with ruins scarce enough that a berth is an event. |
| region 2 palette | drowned street 40 / ruin-field 35 / clearing 15 / canopy 10 | Sun-rich and barren, with the jungle still visibly taking it back. Four kinds clears the three-kind minimum comfortably. |
| `terrain.drowned_street` | `yield_pct` 60, `sun_pct` 120 | Between clearing and ruin-field on both axes, so the city is poor rather than dead. |
| `threat_pct` | 100 / 150 | Region 2 is half again as dangerous for the same provocation. |
| `fork_interval_paces` | 15,000 | Three forks in region 1, about fourteen minutes apart at 1×. Punctuation, not a metronome. |
| `fork_edge_margin_paces` | 5,000 | Keeps the last fork clear of the enclave, so the two never compete for the same stretch of horizon. |
| branch `length_paces` | 6,000 | Roughly five and a half minutes at 1× — long enough to change what the chain is doing, short enough that a bad pick is not a lost region. |
| branch `threat_pct` | 80–130 | The quieter branch is meaningfully quieter; the louder one is not punishing on its own. |
| ruin `salvage` | 25–60 units, before richness | At the rig's rate, 50 seconds to two minutes of berthing at 100% richness. A commitment, not a top-up. |
| enclave `at_paces` | 8,000 into region 2 | About seven minutes of 1× walking past the boundary — one or two of the city's ruins, so the normal case is arriving with something to trade, while the remaining four-fifths of the region is still ahead of you to be provisioned for. |
| salvage rig | `Ruin { ticks_per_item: 60, range_paces: 60 }`, `buffer_max` 8, 8 poles, ground floors only | Two seconds a unit. `range_paces` 60 matches the dart battery's reach, which is the number the player already has a feel for. Priced above a mill and below a dumbwaiter. |
| cutter arm | `Terrain { paces_per_item: 78 }` | **Authored at 78, not the 54 this table first targeted.** 54 is the authored `ticks_per_item` of 90 converted at 0.6 paces/tick, but truncated intake had the arm running at 128 ticks, so 78 paces (130 ticks) is the rate the M2 economy was actually measured against. See §3.6. |
| feral warden | hp 300, damage 12, `attack_ticks` 60, speed 20, `cling_ticks` 2,400, `threat` 30, Ground, `wave_eligible: false` | A dart battery needs 800 ticks and 20 darts to put one down (300 hp against 15 damage every 40 ticks), and takes about 160 damage doing it — roughly 16 poles to mend. Toughest thing in the pack, slowest approach, and the longest grip: 80 seconds of walking to shake one that you have decided to leave. |
| `warden_threat_per_100_salvage` | 120 | A 25-unit ruin rouses one warden, a 50-unit ruin rouses two. The payout and the price are the same number, which is the whole point. |
| `warden_wake_paces` | 120 | About twenty seconds between the ground moving and the first bite, at a warden's own pace — the same order of warning `spawn_paces_ahead` gives an ordinary wave. |
| `provocation_per_100_salvaged` | 300 | The same three points per unit that cutting costs. The wardens are the price of a ruin; charging a much louder second price would make salvage a thing nobody does twice. |
| `enclave_berth_paces` | 80 | About a five-second window at 1×, under two at 4×. If that reads as fiddly in play, the answer is a larger number, not an approach-and-dock mechanic. |
| `crew_cap` | 6 | Twice the starting three, and short of the plan's eventual eight, which M4's shift rota should have to earn. |
| enclave offers | `4 scrap → 3 poles` ×20 · `10 bamboo → 4 poles` ×10 · `4 poles → 6 darts` ×12 | Every rate is worse than the chain's own: the mill turns bamboo into poles one for one, and the thornwright turns a pole into three darts. The enclave is where a tower without the room buys around the gap once. |
| enclave recruit | 30 poles, one only | A 40-unit ruin is 30 poles at the scrap rate — so one good berth on the city's edge is one pair of hands, which is the trade the walk into region 2 is arranged around. |

### 3.8 Tick order, current

> **Unchanged from §2.8.** M3 adds no system. Two existing systems do more, and one new
> value crosses between them.

1. **clock** — advance the day.
2. **power income** — recompute capacity from the banks; collect from sails and burners.
3. **transport** — cars move.
4. **intake** — harvest. Terrain-sourced rooms accrue against the ground covered by *last*
   tick's stride; ruin-sourced rooms accrue per tick while berthed, and the first extraction
   from a ruin rouses its wardens.
5. **production** — recipes advance, consume, emit. Powered rooms pay here.
6. **siege** — creatures approach and attack; provocation decays; wardens roused in step 4
   spawn here, through the ordinary spawn path.
7. **defence** — emplacements fire at what siege just moved.
8. **haul** — crew advance their legs, then idle crew claim work.
9. **repair** — crew already at damage put hit points back.
10. **lighting** — lamps, after dark.
11. **stride** — region crossings, the fork halt, the arrival halt, the distance advance,
    and terrain streaming.

**Why the new work lands where it does.** Region tracking, fork resolution, and arrival all
gate or follow the distance advance, and `stride` already owns distance, `generate_ahead`,
and `prune_behind` — so they extend a system in place rather than needing one of their own.
The three halt conditions (the player stopped, a fork is unanswered, the run is over)
collapse into a single predicate that `power::pay_for_stride` reads, so a halted tower pays
no charge for standing still whichever reason it is standing still for, and `strode` stays
false in every case. Salvage is an intake room, so it belongs in `intake`, which already
runs before `production` (nothing can consume scrap before it lands) and already reaches
across into siege to charge provocation for harvesting — the warden rousing follows exactly
that existing call shape, and because `intake` runs before `siege`, a roused warden spawns
on the same tick.

**The one ordering hazard.** Intake now depends on how far the tower moved, and stride runs
*last* — its position is not negotiable, because charge priority is tick order and walking
is the first thing a tower short of power gives up (§1.6). So `intake` at tick *T* reads the
motion of tick *T−1*. `GameState` gains `paces_last`, written by stride, read by intake. The
one-tick lag is deterministic and invisible at 30 Hz, and it is not a new pattern: `strode`
is already exactly this — "did the legs run last tick" — written by stride and read by
siege's cling logic. `paces_last` is its quantitative sibling. What must not happen is
someone moving stride earlier to make intake read the current tick; that invalidates every
golden replay and reorders the charge priority, to fix a lag nobody can perceive.

### 3.9 What M3 changes in code that already exists

Not a task list — a list of the places where existing code assumes something M3 stops being
true, collected so they are found before they are debugged.

**Written before any of it was built, and kept as written.** Almost all of it has now
landed, and the value of the list is no longer as a plan but as a record of which of these
were spotted in advance and which were not. Two were not, and both were found by something
running rather than by anyone reading: the golden recorder walked to the fork at 15,000
paces and stood there for the rest of its script, and a storeroom turned out to be a
one-way sink for anything without a `take_stock` consumer, so bamboo that reached a shelf
could never come off it again. The first is on this list; the second is not, and is exactly
the kind of thing a list like this is bad at catching.

- **`examples/siege_run.rs`, `examples/throughput.rs`, `examples/record_golden.rs`,
  `web/e2e/smoke.spec.ts`** — all drive the engine with no player, so all will walk into a
  fork and silently measure a parked tower (§3.3). Each needs a standing fork answer.
- **`power::pay_for_stride`** early-returns on `!state.walking`; the fork and arrival halts
  must go through the same predicate, or a halted tower pays charge for standing still.
- **`world::pick_band_kind`** reads `content.terrain_runtime[i].weight` pack-wide; the
  region or branch palette replaces it, and its no-repeat rule is what forces the
  three-kind palette minimum (§3.2).
- **`World::generate_ahead`** streams unconditionally; it now stops at an unanswered fork
  and at the end of the last region, and it needs the palette for the region covering
  `generated_to` rather than the one under the tower.
- **`Feature.kind`'s "Presentation only" comment** becomes false — features are
  economically live once ruins hold salvage. Legal only because they were already drawn
  from the `world` stream, which is the whole point of the M0 note quoted in §3.4.
- **`FeatureView` gains `salvage` and `ViewSnapshot` gains `journey`** — a change to the
  bridge's public contract, so `Chrome.tsx` needs the fork card and the enclave board, and
  the renderer needs the fork horizon, the ruin fill level, and three distinguishable
  halted states (§3.3).
- **`intake::run`** computes one `yield_mul` from the band underfoot and applies it to every
  intake room; a `Ruin` source must not be scaled by terrain it is not drawing from.
- **`IntakeDef.ticks_per_item` becomes the `IntakeSource` enum**, touching `RoomRuntime`,
  `content::validate`'s category wiring check, and `RoomInfo` in the catalog.
- **`TerrainDef.weight` is deleted**; `TerrainRuntime` loses the field and
  `content::validate`'s "no terrain band has a positive weight" check moves onto palettes.
- **`siege::maybe_spawn_wave`** filters eligibility on `min_provocation` alone; without
  `wave_eligible` wardens leak into ordinary waves.
- **`tests/balance_doc.rs` is bidirectional** — every new `balance.ron` field needs a graded
  `BALANCE.md` row in the same commit, and the content-constants group row needs rewording
  for `paces_per_item`.
- **`assets/replays/golden.json`** goes stale twice over, from per-pace intake and from the
  new palettes; regenerate, and extend the recording to cover the new commands.
- **`ids.rs`, `Content`, `command.rs`** gain `RegionIdx`/`BranchIdx`, a `regions` list, and
  three commands with their rejections.
- **`state.rs`'s `place_starting_crew`** picks names by index, not by a roll. Recruiting
  must keep doing that, or adding a crew member perturbs a stream (`DECISIONS.md` §2).

### 3.10 Exit criteria

- [x] **Two runs on the same seed are identical.** The golden fixture runs 40,571 ticks
      through a fork answer and a berth with a rousing, and verifies natively and in wasm
      against the same embedded bytes (`DECISIONS.md` §5). The enclave is deliberately *not*
      in it: reaching one means walking into region 2, which took the recording to 115,000
      ticks and 261 KB — a quarter of a megabyte embedded in the WASM bundle to cover two
      handlers that are pure state arithmetic. `Trade` and `Recruit` are covered by tests
      instead, and their survival through the replay format by
      `every_command_survives_the_replay_format`, whose match is exhaustive so a new command
      cannot be added without a decision about it.

      Backed by thirteen property tests over 300 seeds: rolled lengths inside their authored
      range, journeys identical for a seed across runs, no band repeating its predecessor,
      every band drawn from a palette that covers it, nothing generated past an unanswered
      fork, no fork crowding a region edge or the enclave, salvage only in ruins, and seeds
      differing in how many decisions a region asks.

      Two bugs those tests found that no example test would have: a band could **straddle
      the fork line**, meaning the generator had already drawn the far side from the near
      side's palette — running past a decision that had not been made, the one thing the
      halt depends on it never doing; and changing a fork answer kept that straddling band,
      so the rejected branch's ground survived the rejection.

- [x] **Two seeds feel meaningfully different.** `cargo run --release -p understory-core
      --example journey` plays region 1 across twelve seeds under one policy, then one seed
      under three, and puts the two kinds of variation side by side — because the criterion
      is a comparison, and a comparison belongs on screen rather than in somebody's head.

      **Measured 3 of 3.** Forks offered: 1 across seeds against 0 within one. Widest
      terrain band: 7 points against 1. Salvage in the region's path: 47,280 against 12,274
      — the richness roll is doing exactly what it was added for, so whether a city is worth
      stopping at is a fact about the seed rather than about the player.

      The instrument also settled a design question while being built. A tower that berths
      at a ruin and stays until it is empty **dies**, every time, even having bought a
      battery and a thornwright first — the Heartseed inside a day. That is not a bug: a
      berthed tower has `strode` false, so the wardens it woke never lose their grip, and
      the answer the game gives you is the one M2 built. The policy that *leaves* when it
      has been hurt enough, keeping whatever it already pulled out, reaches the boundary in
      64 minutes having salvaged 60 scrap off 28 ruins. §3.4's trade is real and it is
      played by walking away.

- [x] **A 45–60 minute session reaches the drowned city.** Measured across twelve seeds:
      **48 to 61 minutes** at 1×, median about 54. That is the half a harness can answer —
      it rules out the region being wrong by a factor. The other half, how long a person
      actually takes at the speeds they actually use with the stops they actually choose, is
      answered by playing it and writing the number down (`v2-plan.md` §10 rule 3), and has
      not been.

- [x] Golden replay regenerated; hash parity green natively and in wasm.

- [x] `make check` and the smoke suite green — 237 Rust tests, 7 Playwright.

**The sprint question — run pacing, and does route-is-your-power-mix actually bite?**

Pacing: yes, 48 to 61 minutes to the drowned city, median 54.

The power mix: **partly, and the half that does not show is worth naming.** Measured on one
seed with two policies that differ only in which branch they take at every fork — one always
the shadiest on offer, one always the most open (`Policy::Forager` and `Policy::Sunseeker`
in `examples/journey.rs`, both picking on the branch palette's own sunlight rather than on
its name):

| | shadiest route | most open route |
|---|---|---|
| mean exposure | 34% | 39% |
| canopy walked | 42% | 37% |
| ruin field walked | 15% | 27% |
| salvage in the path | 21,142 | 38,063 |

Five points of sunlight and eighty per cent more salvage, from nothing but the choice at
four forks. The sun half of the argument is live, and it comes with a second axis nobody
designed deliberately: the open route is also the ruin-rich one, so choosing sunlight is
also choosing more to stop for.

The biomass half does not show, and the reason is not the route. Both towers harvest
exactly 100 bamboo, because a tower that builds nothing has no consumer for the poles its
mill makes, so the shelves fill, the mill's outbox backs up, and the arm stalls — long
before the difference between a 140% band and a 50% one could accumulate into anything. The
yields *are* distinct in the simulation now (§3.6, and `every_authored_yield_is_a_different_harvest_rate`
guards it), but a starting tower cannot spend fast enough to feel them. That is a real
answer rather than a null result: the biomass axis needs somewhere for biomass to go, which
is what M4's meals and M5's tier-two chains are. Until then, the route is a power mix in
the literal sense — sunlight — and a salvage mix, but not yet a food one.

**Deferred out of M3:**

- **Region 3, the coast approach, and the Refugia arrival.** Named non-goals in
  `v2-plan.md` §9; region 2's far edge is a placeholder finish line and is labelled as one
  in §3.7.
- **Per-region creature tables.** `RegionDef` scales threat with a single multiplier and
  nothing else. M2 already recorded that most of the four-creature taxonomy is unexercised in
  play (§2.9); giving each region its own table would author selection rules for creatures a
  run has still never met. It belongs with M5's taxonomy pass, alongside the second
  emplacement.
- **Enclave repair.** `v2-plan.md` §6.6 lists enclaves as "trade, recruit, repair"; §9's M3
  scope lists "trade, recruit." Repair is left out deliberately rather than overlooked —
  M2's repair loop is a crew-time-and-poles decision (§2.5), and letting a waystation
  shortcut it would remove the triage pressure the loop exists to create.
- **The enclave as a place the crew haul to.** Reasoned about and cut in §3.5. Trade is a
  posted board and a button; the diegetic version needs a haul destination outside the tower.
- **A continuous stride throttle.** Reasoned about and cut in §3.6. `SetStriding` stays
  binary.
- **~~Scrap has exactly one consumer, and you pass it once.~~ Closed.** Shell work (§3.5)
  is the second consumer, and the one that scales with how hard a run salvaged. The original
  note is kept below because the reasoning that led to it is what produced the fix.
 Moving the enclave into region 2
  (§3.5) means most salvage now has somewhere to go, but the offers have finite stock and
  the enclave is behind you for the rest of the run, so scrap taken late banks against M5's
  sun-forge and its alloy. This is the same shape of deliberate exception M1 recorded for
  darts (§1.4) — the content rule is satisfied, since the enclave consumes scrap at runtime
  — reduced from a structural hole to a tail. If M5's forge slips, the honest fix is a
  second enclave later in the journey rather than leaving late ruins as decorative dead
  weight.

- **The siege balance is ungraded again, and this is the biggest thing M3 leaves behind.**
  Three correctness fixes in one milestone — per-pace intake, a yield multiplier that had
  been rounding to a no-op, and a storeroom that had been a one-way sink — each made the
  economy more forgiving, and together they dissolved the three-way shape M2's
  `PLAYTESTED` rows were tuned to produce. Re-measured, every plan finishes untouched with
  poles banked. `BALANCE.md`'s Siege section opens by saying so. Recovering it needs an
  instrument that keeps a tower genuinely constrained, which is a balance pass and not
  something to bolt onto the end of a systems milestone. First thing at M4.
- **The biomass half of "route is your power mix" is real but unfeelable.** The four terrain
  yields are genuinely distinct now and a test guards it, but a starting tower fills its
  shelves and stalls long before a 140% band and a 50% one add up to anything, so both a
  shade-seeking and a sun-seeking route harvest identically. The sun half measures clearly
  (§3.10). What the biomass half needs is somewhere for biomass to go — M4's meals, M5's
  tier-two chains — not a change to the routes.
- **Region 2 is barely exercised.** Everything measured routinely stops at the region-1
  boundary; only `a_run_can_be_played_from_the_first_pace_to_the_last` and the whole-way
  section of `examples/journey.rs` go further. The drowned city's palette, its higher
  `threat_pct` and the enclave in it have been walked through, and that is all — nobody has
  looked at how the second region *plays*.

### 3.11 Open questions

Things that genuinely cannot be settled without building them.

1. **Does the fork halt read as a decision or as a lurch?** The argument in §3.3 is that a
   player who sees the fork coming answers it early and never stops. Whether they actually
   see it — at 2×, with a chain to watch and a wave inbound — is a question for the
   cross-section, not for the spec. If they consistently do not, the fix is a louder approach
   in the terrain strip, not a modal pause.
2. **Is one ruin worth one warden fight?** §3.4 states the equation and both of its sides
   move together. Whether there is a setting where the answer is genuinely "it depends on the
   tower" rather than always yes or always no is the thing the balance pass has to find, and
   it may turn out that salvage needs a second payoff — something the tower does with scrap
   directly — to be a decision at all. Ruin richness (§3.2) helps here by making the answer
   differ between runs, but a mechanic that is worth it at 140% and never worth it at 60%
   is a mechanic that is off half the time, which is not the same as a decision.
3. **How bad a night is a brown-out, once harvesting stops with the legs?** §3.6 identifies
   the loop and its floor — the sails fill regardless of whether the tower is walking, so
   morning ends it. What the instrument has to find is the cost of that night in a run that
   was otherwise going well, and whether a competently-played tower ever pays it twice.
4. **How long is a region, in minutes?** The paces figure is arithmetic; the minutes figure
   depends on how much of a session is spent at 1× versus 4×, and how often people stop. The
   instrument narrows it, playing settles it.
5. **Should a branch be able to change *what* comes, not just how much?** Threat is one
   multiplier in M3. A branch that trades "quieter" for "different" — the ruin road that
   wakes machines, the canopy passage that drops leapers — is a better choice than a branch
   that trades quiet for loud, but it depends on M5's taxonomy landing first.

---

## M4 — The Home *(the tone)*

**Sprint question:** does it feel like a home reclaiming the world, or a spreadsheet with legs?

**Scope:** the tower stops being a machine and becomes somewhere people live. Crew get names,
faces and things to say; they get two needs — meals from a canteen chain, sleep in a bunk on a
shift rota the player sets — and both needs are demands on the circulation that already
exists. Then the two presentation passes: the flat-vector solarpunk-tropical art pass
`v2-plan.md` §0 scheduled here from the start, and the entire audio subsystem — which does not
exist at all, and which M1's brief already asked for and did not get ("starvation/stall/
brown-out all readable *and audible* — silence = broken").

**Non-goals (M5):** no region 3 and no Refugia arrival, no tier-two chains, no unlocks or
meta-progression, no second emplacement or full creature taxonomy, no enclave economy beyond
M3's single waystation, and no role-priority system — crew remain generalists who haul, mend,
eat and sleep, and the "haul / operate / gun / repair" priorities `v2-plan.md` §6.7 sketches
are deferred with a reason (§4.9).

### 4.1 The shape of the thing

**Nothing in M4 adds an economy.** It adds *needs*, and a need is not a new resource loop — it
is a new customer for the loops already running. Meals are bamboo the mill did not get; a
sleeping crew member is a pair of hands the stairs did not carry. Every number M4 introduces
is a claim on circulation, which is the one thing this game has always been about
(`DESIGN.md` §2 insight 1). If a system in this milestone needs its own supply mechanism, its
own screen, or its own currency, it has been designed wrong — the same test §2.4 applied to
emplacements applies here.

The second half is presentation, and it is not decoration. Every milestone so far has been
answered by a test or an instrument. M4's two exit criteria are a **screenshot** and a
**recording**, answered by looking and listening (§4.9), and that is a genuine change in how
this milestone gets judged: `make check` passing tells you nothing about whether M4 worked.

**The ordering inside the sprint matters, and it is not the order of the sections below.**
`SYSTEMS.md` §3.10 leaves the siege balance ungraded — three M3 correctness fixes dissolved
the three-way shape M2's `PLAYTESTED` rows were tuned to produce — and says the re-measurement
is the first thing at M4. It cannot be. Sleep takes something on the order of two-fifths of
the tower's crew-hours out of the economy (§4.4), and meals take a fifth of a mill's bamboo
draw (§4.3); re-measuring before those land means measuring an economy that is about to move
again. So: **build the needs, then re-measure once.** The balance pass is the end of M4, not
the beginning of it, and the harnesses it runs on need the changes in §4.8 before their
numbers mean anything.

### 4.2 Crew as named individuals

Crew are named individuals with jobs, not stat blocks (`DESIGN.md` §2 structural call 4).
Three things carry that: a name, a face, and something to say. None of the three is allowed
anywhere near the simulation.

**Names are content, chosen by index.** `assets/data/crew/names.ron` holds an ordered list,
longer than `crew_cap`, and `GameState::add_crew` takes the next one by index exactly as it
does today — the rule §3.5 established for the enclave's recruit, kept because a name drawn
from a stream would perturb the stream (`DECISIONS.md` §2). `Crew.name` stays a `String` in
state: it is already there, it is already in the hash, and interning it would buy nothing
mechanical. What that does mean is that the *order* of the name list is load-bearing for
replays, the same way the sort order of every other content list is (`DECISIONS.md` §6);
reordering the file is a simulation change, and editing the text of an existing entry changes
the content hash. Both are ordinary, and both are worth knowing before someone alphabetises
the file.

**Portraits and barks live entirely in the frontend.** This is stronger than the firewall rule
requires and the strongest available reading of it: rather than draw a bark line from
`cosmetic`, M4 puts no part of a bark in `GameState` at all. Rust already publishes everything
the frontend needs to know that something bark-worthy happened — the event list from `frame()`
and the per-frame `ViewSnapshot` (§4.6) — so the JS layer decides *whether*, *which line*, and
*when*, and the simulation cannot be perturbed by a bark because there is nothing to perturb.
`DECISIONS.md` §2 names a bark drawing from `sim` as the exact catastrophe to avoid; a bark
that does not draw at all cannot be that catastrophe on any future day either.

Where a bark needs a *stable* per-person choice — this crew member's face, and which of
several equivalent lines is hers — it derives from `CrewView.fidget`, the per-crew `u16`
already drawn from `cosmetic` at spawn and already in the snapshot for the renderer's idle
phase. One more consumer of an existing cosmetic draw costs nothing, adds no state, and is
stable for the life of a crew member and reproducible from a seed. `portrait = fidget % faces`
is the whole mechanism.

**What each bark is triggered by, and from which of the two inputs.** Barks are punctuation on
top of state, so they follow the same split as sound (§4.6):

| Bark occasion | Read from | Note |
|---|---|---|
| picked something up, set something down | `Pickup` / `Deliver` event | the most common, so the most heavily rate-limited |
| stuck in a queue | `CrewView.stressed` going true | the diegetic red tint already says it; the line is colour, not information |
| sat down to a meal | `MealServed` event | the warmest moment in the tower, and worth a line |
| going off shift | `ShiftChange` event | the handover is the tower's one daily ritual |
| woken early | `CrewView.asleep` going false without a `ShiftChange` | the surge lever (§4.4) should sound like an imposition |
| a wave on the horizon | `WaveArrives` event | defenders, not soldiers — see below |
| something gave up and left | `EnemyLeaves` event | never triumphant; §2.2's distinction is a tone rule as much as a mechanical one |
| a breach, a wreck, a severed column | `Breach` / `Wrecked` / `Severed` | concern for the tower, not for the enemy |

Tone follows `DECISIONS.md` §8 without exception. The crew are gardeners and porters who
defend a home they live in; they are not troops, they do not report kills, they do not banter
about ordnance, and nothing they say frames a creature as a target. A creature that lost its
grip and walked away has not been *beaten* — §2.2 already refuses to count it as repelled, and
a bark that celebrates it would undo that refusal in the one register the player actually
attends to. A line that reads as militaristic is a tone bug even though it is mechanically
inert, and the check is `AGENTS.md` §VII's: read it aloud and ask whether it belongs in
Nausicaä or in a shooter.

Barks are fire-and-forget in the same sense sound is: emitted by a situation, drawn on screen
or dropped, and never read back into anything. There is no bark log, no morale that barks feed,
and no relationship state — `v2-plan.md` §3's fourth structural call rules the last one out
explicitly.

### 4.3 Meals, and the kitchen chain

A **canteen** takes bamboo and makes meals. Meals are an ordinary item on ordinary shelves,
hauled by the same crew through the same shafts, subject to the same buffer stalls. There is no
food logistics layer; there is one more production room whose output happens to be eaten by
people instead of by a battery.

```
room.canteen — Production, 2 slots, 5 poles
  recipe: 2 bamboo -> 3 meals, craft_ticks 300
```

**The canteen is deliberately not powered.** Every other production room could take a
`power_draw` and the thornwright does, but a kitchen that goes dark in a brown-out and stops
feeding people turns one bad night into a spiral inside a spiral — and the "powered production
is a charge sink" lesson is already taught by a room the player chose to build for a reason
other than survival. The canteen stays free to run.

**`Crew.hunger` counts ticks since the last meal.** Not a fraction, not an `Fx`, not a
per-mille with an accumulator: a `u32` tick count, which is how this codebase already expresses
every duration (`DECISIONS.md` §1) and which makes the accrual exactly `hunger += 1` with no
rounding anywhere. It rises every tick, awake or asleep. A meal resets it to zero — a meal is a
meal, and a crew member who ate late does not carry the deficit forward.

Two thresholds, and the gap between them is the design:

| Threshold | Ticks | What happens |
|---|---:|---|
| `hungry_ticks` | 4,800 | they go and eat: an errand outranking a new haul, but never a load already in hand |
| `starving_ticks` | 7,200 | they work *slower* — the multiplier in §4.4 — and keep working |

**A fed tower never sees the penalty.** That is the point of two thresholds rather than one. If
going-to-eat and slowing-down were the same number, crew would trudge to the canteen every
time, and a working kitchen would read as a permanent tax; with a third of a day between them,
the slow-down appears only when the kitchen has actually failed — no bamboo, no canteen, or a
canteen nobody can reach. **Hunger is a supply problem.** It is invisible while the chain
works and it is the chain's failure that you feel, which is the same shape as every other
signal in this game.

At 14,400 ticks a day, `hungry_ticks` of 4,800 is three meals a day a person. At 2 bamboo a
meal that is **6 bamboo a day a crew member** — four crew is 24 a day, about a fifth of what a
fed mill draws (one bamboo per 120 ticks is 120 a day) and a fifth of what one cutter arm
brings in on region 1's palette. That ratio is the whole judgement: enough that a route rich
in biomass visibly feeds people better, not so much that meals displace poles as what bamboo
is *for*.

**Crew eat where the meals are, and the arrival machinery already exists.** Eating is not a
haul and it is not a new pathfinder. It is shaped exactly like a repair: go to a place, stand
there, spend ticks (§2.5, `haul::repair_leg`). A hungry crew member picks the nearest room
holding at least one meal — the canteen's own outbox, or any shelf a meal has been hauled to —
walks and climbs to it on the ordinary legs, and enters `CrewState::Eating`, which consumes one
meal on completion. If the meal is gone when they arrive, the errand clears and they look
again: the same failure path `CrewState::Loading` already has when somebody else got there
first, and for the same reason no reservation bookkeeping is added for it.

**Eating at the source is load-bearing, not a convenience.** `BALANCE.md`'s `storeroom` row
documents the sharpest emergent failure in the game — a shelf takes whichever item lands on it
first, so bamboo can claim every shelf and deadlock the chain — and explicitly hands the
question of what to do about it to M4. Meals make it worse: they are a fifth claimant on four
shelves per storeroom, alongside bamboo, poles, darts and scrap, and a shelf-starved tower
would now starve its people as well as its mill. Because crew can eat straight out of the
canteen's outbox, the meal chain survives a fully-claimed storeroom: distribution to shelves is
an *optimisation*, not a requirement. That optimisation is real and worth discovering — a
storeroom high in the tower becomes a pantry, and where the pantry is decides how far people
walk to eat — but nobody starves for want of a free shelf. The deadlock itself is still
unanswered; see §4.10.

**This is load-bearing well beyond M4, and it is why the canteen's rate is the number to watch.**
`SYSTEMS.md` §3.10 records that the biomass half of "your route is your power mix" is real in
the simulation and unfeelable in play: the four terrain yields are genuinely distinct and a
test guards it, but a starting tower has nothing to *do* with bamboo, so the shelves fill, the
mill's outbox backs up, the arm stalls, and a shade-seeking route and a sun-seeking one harvest
identically — measured at exactly 100 bamboo each, over a whole region. **Meals are the
consumer that fixes it.** They are a demand that scales with the crew rather than with shelf
space, they cannot be satisfied by stockpiling, and they run all day.

So whoever authors the canteen's rate owes one more thing: **re-run `examples/journey.rs`'s
shade-versus-sun comparison afterwards** — the `Policy::Forager` against `Policy::Sunseeker`
table in §3.10 — with a canteen and bunks in the shopping list, and check that harvest is no
longer identical across the two routes. If it still is, the canteen is too cheap to feed
people with, and the lever is the meal's bamboo cost, not the terrain yields. That measurement
is the single clearest test of whether M4 closed M3's biggest open finding.

### 4.4 Sleep, and the shift rota

**`Crew.shift` is `Day` or `Night`, and the player sets it.** Crew are awake when the current
daypart belongs to their shift and asleep when it does not. Sleeping crew do no work at all:
they take no tasks, advance no legs, mend nothing, and accumulate no `wait_ticks` — a sleeper
tinting red would make the only bottleneck instrument in the game lie (`DECISIONS.md` §8).

**Which dayparts belong to which shift is content.** `DaypartDef` gains a `shift` field, so the
handover is a designer's decision rather than a constant buried in a system, and the seven
dayparts already in the pack divide as:

| Shift | Dayparts | Per-mille | Ticks |
|---|---|---:|---:|
| Day | morning, midday, afternoon | 180–760 | 8,352 |
| Night | dusk, night, predawn, dawn | 760–180 (wrapping) | 6,048 |

Validation requires both bands to be non-empty and each to be one contiguous run modulo the
day, in the same spirit as the contiguous-`order` check on regions (§3.2): a rota with two
separate night stretches is not a rota, it is a bug in the content pack, and a broken pack is a
load error rather than a runtime condition (`AGENTS.md` §IV).

The bands are deliberately unequal, and the night band is deliberately the one that covers the
lamplit hours. Against the shipped sun curve, exposure drops below `night_light_threshold` at
about 835‰ and climbs back through it at about 150‰ — entirely inside the night shift. **The
night shift is the dark shift, exactly.** It is also the shorter one, and that asymmetry is its
compensation: a night worker is awake for 6,048 ticks against a day worker's 8,352, so staffing
the night costs more hands than it returns.

**`Crew.rested` is ticks of work left in them.** A `u32`, like hunger, counting down one a tick
while awake and up while asleep — `rest_gain_per_tick` in a bunk, and less on the floor.
Below `tired_ticks` they work slower, on the same multiplier hunger uses. The arithmetic is
arranged so that a bunked crew member on either shift wakes full:

| | ticks awake | ticks asleep | rest banked |
|---|---:|---:|---:|
| Day shift | 8,352 | 6,048 | 12,096 (capped at `rested_max_ticks` 8,640) |
| Night shift | 6,048 | 8,352 | capped |

**A day-shift crew member with a bunk flags at the end of every shift, and that is intended.**
`rested_max_ticks` of 8,640 against 8,352 ticks awake leaves almost no slack, so they cross
`tired_ticks` about eighty percent of the way through the day and work the last stretch of it
slowly. The tower visibly tires in the late afternoon and picks up at dawn. That is not a
balance oversight to tune out — it is the day having a shape, which is most of what "somewhere
people live" means on screen, and it costs nothing but the shape of two constants.

**Tiredness is a scheduling problem, the way hunger is a supply problem.** There is no mid-shift
nap: a crew member cannot fix being tired the way a hungry one can walk to the canteen, because
the only thing that refills `rested` is being off shift. So the two needs fail differently and
are read differently. Hunger says *your chain broke*; tiredness says *your rota is wrong, or
you have no beds*.

**No bunk means sleeping where they stand, and worse.** A crew member with nowhere to lie down
enters `CrewState::Sleeping` on the deck where they stopped — visible, and drawn as such — and
gains rest at `no_bunk_rest_gain` of 1 a tick instead of 2. The arithmetic is unforgiving and
exact: a day-shift sleeper on the floor banks 6,048 against the 8,352 they spend, a net loss of
2,304 a day, so they slide into permanent tiredness inside four days and never climb out.
Bunkless is survivable and visibly degrading, which is the right shape for a cost the player
can fix at any time for three poles.

```
room.bunk — Quarters, 2 slots, 3 poles, sleepers: 2
```

**A bunk is a new room category, because the pack's validation demands one.**
`content::validate` rejects any room whose category has no matching behaviour block
(`DECISIONS.md` §9.1), and a bunk has no recipe, no storage, no intake and no defence. So
`RoomCategory::Quarters` requires `quarters: Option<QuartersDef { sleepers: u8 }>`. Two
sleepers to a two-slot bunk is one slot a person — the tightest legible packing — which makes
quarters a floor tax that grows with the crew that pays for it. That cost is the point and
should not be quietly relieved; if it reads as too dear in play, the lever is `sleepers`, not
free beds.

**And here the rota pays for itself in floor space, which nobody designed and which falls
straight out of the model: beds are shared between shifts.** A bed is only occupied while its
sleeper is off shift, so a tower with everyone on the day shift needs one bed a head — three
crew, two bunks — while a tower at cap split four and four needs only four beds, also two
bunks, because the night watch is up while the day crew are in them. Eight crew unrota'd would
want four bunks, eight slots, more than a floor has left once the stairs have taken theirs.
Staffing the night halves the dormitory, and a player who works that out has found a real
reason to do it that has nothing to do with the prowler.

**Bunk occupancy is derived, never stored.** How many sleepers a bunk holds is counted by
scanning the crew whose errand names it — the same trick `haul::shaft_queues` already uses to
give one crew member a picture of the whole queue while the borrow checker only lets them see
themselves. `Room` gains no field, and there is no occupancy counter to get out of step with
reality. Beds are claimed in crew order, which is creation order, which is `CrewId` order, so
who gets the last bed is a pure function of state.

**The rota is a real decision, and both of its costs are already in the game.**

*Night cover.* The night prowler is `night_only` (`assets/data/enemies/`), so the hours a tower
is least able to answer a wave are precisely the hours something is out. A tower with everyone
on the day shift has nobody to run darts up to a battery or mend a breach between dusk and
dawn; a tower that staffs the night pays for that cover in daylight throughput, on the shift
where the chain actually flows. Neither answer is free and neither is wrong.

*Lamps, which are charge.* This connection needed a decision, because as shipped it is not
true: `power::lighting` buys light for the whole tower whenever exposure is below
`night_light_threshold`, regardless of whether anybody is awake in it. So a night shift adds no
charge cost, and "night operations need light" was a sentence rather than a mechanic. **The
call: lighting stays unconditional, and working in the dark joins hunger and tiredness on the
same slow-down multiplier.** A crew member working while `Power.lit` is false — a brown-out,
not merely a dark night — works at `dark_work_pct`. That makes a night shift's dependence on
charge sharp and immediate (a brown-out does not just dim the tower, it wastes the shift you
staffed) without adding a single new charge sink, and it strengthens M1's and M2's signature
emergency instead of relaxing it.

The alternative — gating lighting on somebody being awake — was considered and rejected. It
reads well and it is three lines, but the default rota is all-Day, so it would make every
night's lamps free for most towers and quietly relax the brown-out pressure `BALANCE.md`'s
power rows were measured against. Making the dark expensive by slowing the people in it costs
no constants and revalues nothing.

**The rota's one emergency verb is a surge, and it costs what it should.** Because awake means
"the current daypart belongs to my shift", setting a sleeping day-worker to `Night` in the
middle of the night wakes them immediately — unrested, on the slow multiplier — and come
morning they are off shift and will sleep through the day you needed them for. That is a real
all-hands lever with a real price, built out of nothing but the definition above. It is also
why **crew are never woken automatically.** An attack does not rouse a sleeper: if the
simulation woke people when things got bad, the rota would be decorative, and the interesting
decision — do I burn tomorrow morning to answer tonight — would be made by the game instead of
the player.

**Two invariants the state machine has to hold.** A crew member never falls asleep holding
something: going off shift stops them taking *new* work, and they head for a bunk once their
hands are empty, which is the same reasoning `assign_idle` already applies to a carrier who
would otherwise be pulled onto a repair. And an errand whose room is gone — a bunk demolished
under a sleeper, a canteen removed mid-meal — clears to `Idle` rather than spinning, exactly as
`CrewState::Boarding` already handles a shaft demolished out from under a queue.

**Priority order for an idle crew member**, extending the ladder in `haul::assign_idle`:

1. a task already under way — pick the trip back up rather than re-deciding it
2. a load in hand with somewhere to put it — finish the delivery; nothing carried is ever
   dropped
3. off shift — go to a bunk, or lie down where they are
4. past `hungry_ticks` — go and eat
5. damage worth a shift — mend it (§2.5)
6. a haul

Eating above mending is deliberate: a crew member past `starving_ticks` mends slowly too, and a
meal is 300 ticks against a repair shift's 80 plus the walk. Feeding them first is the cheaper
order.

**The multiplier, and the one trap in implementing it.** Three causes — starving, tired, working
unlit — compose multiplicatively into a `work_pct`, and that percentage scales **the duration of
an action, never the fixed-point step that advances it.**

```
work_pct      = 100, times each active penalty / 100, floored at 1
effective(t)  = t * 100 / work_pct        // integer, computed once per leg
```

This is not a stylistic preference. `haul::advance` currently steps position by
`Fx::ratio(1, walk_ticks_per_slot)`, and `Fx::ratio(1, 12)` is already `Fx(21)` — 1.6% off the
authored rate. Scaling *that* by a percentage is precisely the arithmetic that broke intake
before M3: `Fx::ratio(60, 1200)` truncates to `Fx(12)`, a 21-tick slot rather than the 20 the
constants describe, and the errors compound per penalty. `intake::terrain_effort`'s doc comment
is the authoritative account of what that class of mistake cost the game the first time — four
authored terrain yields behaving as two, and the flagship contrast of `DESIGN.md` pillar 1
absent from the simulation entirely. Scaling the tick count instead keeps a single integer
division, leaves the Fx precision exactly where it already is, and makes the penalties
inspectable as tick counts. `Loading`, `Unloading`, `Eating` and `Repairing` are trivially
exact, since they are already `ticks_left` counters.

**`work_pct` never exceeds 100.** A fed, rested crew member in a lit tower is the baseline, not
a buff — being cared for is normal and neglect is what costs you. A food that made people
*faster* would turn the crew into a throughput stat to optimise, which is the one thing
structural call 4 exists to prevent.

**Opening values.** Every figure in §4.3 and §4.4, collected so whoever authors the content has
one list rather than nine paragraphs to mine. **Each is a design target** — a first value with
an argument behind it, not a measurement — and none is a `BALANCE.md` row until it has been
authored, at which point it gets a graded row in the same commit (`DECISIONS.md` §7). The
arithmetic assumes `ticks_per_day` 14,400 and 30 Hz.

| Thing | Target | The arithmetic |
|---|---|---|
| `hungry_ticks` | 4,800 | A third of a day, so three meals a day a person. Short enough that the canteen is somewhere people actually go, long enough that eating is not most of what a crew member does. |
| `starving_ticks` | 7,200 | Half a day — a full meal cycle *past* being hungry. A working kitchen never reaches it, so the slow-down is a failure signal rather than a routine tax. |
| canteen recipe | 2 bamboo → 3 meals, 300 ticks | 6 bamboo a day a crew member; 24 for four crew, a fifth of a fed mill's 120. A ten-second craft is visible as cooking. Flat out the room makes eight times what a full crew eats, so it is buffer-limited and mostly idle — correct for a kitchen, and it means the canteen's *rate* is not what sets bamboo demand; the crew are. |
| canteen | Production, 2 slots, 5 poles, no `power_draw` | A thornwright's price: dearer than a mill (4), well short of a salvage rig (8). An early, obvious build rather than a commitment. Unpowered on purpose; see above. |
| bunk | Quarters, 2 slots, 3 poles, `sleepers` 2 | Joint-cheapest in the pack with a storeroom, because the alternative to a bed is a crew member who degrades a little more every day, and a bed should never be a gate. The real price of quarters is the slots, not the poles. |
| `rested_max_ticks` | 8,640 | 0.6 of a day, a shade over the 8,352-tick day shift, so a day worker ends their shift nearly empty and a night worker never does. |
| `tired_ticks` | 1,440 | A tenth of a day of work left. A bunked day worker crosses it about four-fifths through their shift, so the tower flags in the late afternoon. |
| `rest_gain_per_tick` | 2 | Two ticks of rest a tick asleep, so 6,048 ticks off shift refills 12,096 — comfortably over the cap, which is what makes a bunk feel like a solved problem rather than a managed one. |
| `no_bunk_rest_gain` | 1 | Bunkless is a net loss of 2,304 a day and permanent tiredness inside four. Visibly degrading, never fatal, and fixable for three poles at any moment. |
| `hungry_work_pct` | 60 | Past `starving_ticks`, everything takes about two-thirds longer. Plainly slower on screen without reading as broken. |
| `tired_work_pct` | 60 | Deliberately the same number as hunger's: one visible failure mode with two causes, so a player learns the *look* of a crew member working badly once and then asks why, rather than learning two separate symptoms. |
| `dark_work_pct` | 75 | The mildest of the three — you can work by feel, just not well. Composed with the other two the worst case is 27%: crawling, never stopped. A need that halts the tower is a death spiral rather than a pressure. |
| `crew_cap` | 6 → **8** | `BALANCE.md`'s current row defers the plan's eventual eight to "whatever M4's shift rota earns". The rota earns it: eight crew split across two shifts is about five awake at once, which is where six unrota'd crew already sat. Raising the cap without the rota would have been a straight throughput gift; with it, it buys coverage. Still aspirational either way — M3's enclave offers exactly one recruit (§3.5), so a run cannot approach either number until M5's enclave economy. |
| `item.meals` | glyph 🍲, order 25 | Between poles (20) and darts (30) in the display order, because meals sit beside poles as the other thing bamboo becomes. Nothing mechanical reads `order`; the indices come from the sorted string IDs (`DECISIONS.md` §6). |

New command:

| Command | Effect | Rejects on |
|---|---|---|
| `SetShift { crew, shift }` | put one crew member on the day or the night shift | no such crew |

One crew member per command rather than a bulk setter: commands batch cheaply
(`DECISIONS.md` §3), a rejection then names the crew member it is about, and the replay reads
as a list of decisions about people.

### 4.5 The art pass

Flat-vector solarpunk-tropical: overgrowth on the tower, warm interiors, verdigris and worked
brass against deep jungle green. `v2-plan.md` §0 planned "readable placeholder
(rectangles-with-personality) through M3" with the real pass here.

**Most of the foundation landed early, and this pass builds on it rather than starting from
boxes.** `web/src/engine/palette.ts` is already the solarpunk-tropical palette, with a
day/night blend (`atNight`), per-band terrain colours including the drowned city's own, and
warm-metal salvage against cold stone. `scene.ts` already draws per-category room silhouettes
with a body height and a crown — a sail, a cell rack, a vent stack, a cutter boom, a battery
barrel — off the room widths §0.5 settled, so a floor of mixed rooms already reads as a skyline
rather than a row of crates. Planters already sit on every deck's outboard edge and across the
roof garden, damage already splits timber and spills lamplight through a breach, and shell
plating already reads as bolted-on metal. **The remaining work is not "add art", it is "make one
frame say home".** Concretely:

1. **Crew have to become people.** This is the largest single item, and it is the one the
   screenshot test turns on: a frame full of rounded lozenges is a diagram whoever looks at it
   will read as a factory. Crew need a walk cycle driven off the fractional `slot` they already
   carry, a laden posture distinct from an empty one, a sitting pose for `Eating`, and a lying
   pose for `Sleeping`. Nothing about this needs new snapshot data — the states and the
   fractional positions are already there.
2. **Two new silhouettes, and a `Quarters` arm in the two functions that switch on category.**
   `roomColor` and `roomProfile` both fall through to a generic box for an unknown category, so
   a bunk would work and look like a crate. A bunk draws its `sleepers` as hammocks — a
   solarpunk answer to a bed, and one that makes occupancy diegetic: you can see who is asleep
   and whether a bed is spare, without a number. A canteen draws a hearth: warm interior light
   and rising steam while it is crafting, cold and dim when starved. That is the single warmest
   image available in the tower, and it doubles as the kitchen chain's own stall signal — a cold
   hearth is *why* people are going hungry, in the same place you notice that they are.
3. **Overgrowth beyond the planters.** Vines trailing between floors, moss at the shell lip,
   growth thickening on the leeward side — all with per-floor phase derived from the existing
   `hash01` helper so it is stable frame to frame rather than crawling.
4. **Warm interiors, per room.** A working room shows a lit window; a stalled one does not. It
   reinforces the signal the dimmed body already carries rather than adding a second, different
   one, which is the §8-compliant way to add emphasis.
5. **Verdigris as its own colour.** The palette's cool blue-green is currently `roomStorage`
   doing double duty on shell plating. Naming `verdigris` and `brass` and using them on shaft
   rails, plating and joinery is what makes the metal read as aged copper rather than as paint.
6. **Light through the canopy.** Dappling on the tower's face keyed to the band underfoot, and
   motes or fireflies after dark. Both are pure JS animation with no state behind them, which
   is where cosmetic motion belongs.

**What must not happen in this pass.** No numeric badge, no warning icon, no hunger bar over
anybody's head. The diegetic signals for the two new needs are behaviour: a hungry crew member
walks to the canteen, a tired one moves visibly slower, a sleeping one is lying in a hammock.
Precision is a hover layer, per `DECISIONS.md` §8, and the moment a crew member acquires a
floating status bar the screenshot test is unwinnable — because a frame full of floating bars
is a spreadsheet with legs, which is the exact failure the sprint question names.

The one new piece of chrome M4 does add is a **crew roster**: portrait, name, what they are
doing, and a day/night toggle that issues `SetShift`. That is a panel, and it is defensible
under §8 because a rota is a schedule the player writes rather than a readout of state — the
same category as the elevator's per-daypart programs. What is not defensible is the roster
becoming the primary place hunger and tiredness are read. If the tower can only be understood
through the roster, the pass failed.

### 4.6 Audio

**No audio exists.** `SoundEvent` has nineteen arms, emitted throughout the tick and carried
across the bridge by `frame()`, and `Game.ts` throws the list away with a comment pointing at
this milestone. So the plumbing is already done and the whole subsystem is JS-side work.

**The boundary is fixed: Rust decides *that* something happened, JS decides whether and how it
sounds.** Events are fire-and-forget — emitted during a tick, consumed or dropped by the
`AudioManager`, never read back into the simulation. Nothing about the mix, the volume, the
voice count, or whether audio is even enabled may reach `GameState`. If a sound needs to know
something, it reads the snapshot; it does not ask the simulation to remember anything for it.

**The audio layer reads two inputs, and telling them apart is the whole design.** The
eyes-closed test asks whether you can hear how the tower is *doing*, which is continuous state
— and `SoundEvent` is punctuation. A starved mill going quiet is not an event at all; it is the
*absence* of a loop, and the fact driving it is `RoomView.stalled` in the per-frame view. So:

* **`frame()`'s event list** drives one-shots: things that happened.
* **`view()`'s `ViewSnapshot`** drives loops: things that are ongoing.

Both already cross the bridge every frame. Getting this split wrong is precisely how a project
ends up with a warning beep where a silence belonged.

**Loops, from the snapshot.** Each is a bed whose gain is a function of state, and each goes
silent when its state stops — never replaced by a different sound saying it stopped:

| Loop | Read from | Silent when |
|---|---|---|
| a room working | `RoomView.stalled` false and `progress` advancing | starved, backed up, unpowered, wrecked — all of which sound identical, because from outside they are |
| the legs | `journey.halt`, not `power.walking` | stopped, halted at a fork, arrived, or browned out — see below |
| a car running | `ShaftView` car state | idle |
| footsteps on the stairs | crew in `climb` | nobody on them |
| the sails | `ClockView.exposure_pct` | shaded, or after dark |
| the tower's electrical hum | `PowerView.fill_permille` | thins as the bank drains, and drops out entirely on `brownout` |
| the day bed | `ClockView.permille` and `daypart` | crossfades with the night bed |
| the night bed | as above | — |
| the jungle | the band underfoot, `WorldView` | never; it is the floor under everything |

**The legs are the one loop with a trap in it, and the snapshot already contains the answer.**
`PowerView.walking` is the player's *intent*; `GameState.strode` is whether the legs actually
ran, and it is deliberately not in the snapshot as a raw flag. What is there is
`journey.halt` — `Walking`, `Stopped`, `Fork`, `Arrived`, or `Brownout` — precisely because §3.3
required the halted states to be distinguishable and the renderer needed telling which one it
was drawing. Audio inherits that for free, and should use all five: a tower that stopped and a
tower that cannot afford to move must not sound the same, and the brown-out case has a
treatment to match already — `drawLegs` gives it a stuttering lift that never becomes a step,
and the sound of that is a motor asking and not being answered.

**Two soundscapes, day and night**, as `v2-plan.md` §9 asks: a day of canopy-hum, insects, and
the tower's own working noise; a night of a different insect register, wind, distant movement,
and the tower's lamps and machines standing out against it because there is less around them.
The crossfade follows the sun curve rather than the daypart index, for the same reason the sun
curve is a curve — a step change at a boundary reads as a bug (§1.1).

**One-shots, from the event list.** Every existing arm of `SoundEvent` is punctuation and
already correctly shaped; M4 adds three:

| Event | New | Why |
|---|---|---|
| `MealServed` | yes | The warmest moment in the tower deserves a cue, and it is the audible confirmation that the kitchen chain is alive. |
| `ShiftChange` | yes | The handover is the tower's one daily ritual, and it is the only reliable way to *hear* what time it is. |
| `EnemyLeaves` | yes | Closes M2's deferral (§2.9): `Leaving` and `Dying` are distinct in state and in the snapshot but have sounded identical, so walking a wave off and shooting it down were indistinguishable. §2.2 refuses to count the first as repelled; the audio has to refuse too. |

**The diegetic rule, stated once because everything else follows from it: a starved production
loop goes silent, it does not gain a warning sound.** No alarm on a stalled room, no beep on a
queue, no sting on a full buffer. `wait_ticks` and the red tint are the bottleneck instrument
(`DECISIONS.md` §8) and audio's contribution to them is the mill you can no longer hear. This
is the rule that makes the eyes-closed test winnable at all: a tower whose problems announce
themselves with tones is one where you hear the *alarms*, not the tower.

**Three practical constraints that fall out of the existing frame loop.**

* **Coalesce per frame.** `GameEngine::frame` runs up to `MAX_TICKS_PER_FRAME` ticks in one
  call, and at 4× a frame routinely contains several ticks' worth of events. Three mills
  finishing on one tick must not be three times as loud, and a busy frame must not machine-gun.
  The `AudioManager` deduplicates by kind within a frame and rate-limits each kind, which is a
  JS concern entirely and needs no change in Rust.
* **`SoundEvent` stays payload-free, and M4 does not do positional audio.** Adding a floor or a
  room id to the events that could use one would change the bridge's public contract from a
  string union to tagged objects, and the eyes-closed test is about the tower's *state*, not
  about where in it something happened. Deliberate cut, recorded here rather than discovered
  later by someone wondering why `Craft` does not say which mill.
* **Audio cannot start without a gesture.** Browser autoplay policy means the `AudioContext` is
  suspended until the player clicks, so the opening frames are silent and the Playwright smoke
  test never hears anything. Neither is a bug; both need to be true on purpose rather than
  discovered as a mystery.

### 4.7 Tick order, current

> **Supersedes §3.8.** M4 inserts one system, `needs`, between `defence` and `haul`, so haul
> becomes 9 and everything after it shifts by one.

1. **clock** — advance the day.
2. **power income** — recompute capacity from the banks; collect from sails and burners.
3. **transport** — cars move.
4. **intake** — harvest ground covered last tick; extract from a ruin while berthed.
5. **production** — recipes advance, consume, emit. Powered rooms pay here. The canteen cooks.
6. **siege** — creatures approach and attack; provocation decays.
7. **defence** — emplacements fire at what siege just moved.
8. **needs** — hunger rises, rest drains or refills, and the shift band decides who is awake.
9. **haul** — crew advance their legs, then idle crew claim work, eat, sleep, or mend.
10. **repair** — crew already at damage put hit points back.
11. **lighting** — lamps, after dark.
12. **stride** — region crossings, the halts, the distance advance, and terrain streaming.

**Why needs lands where it does.** It must run *before* haul, because haul both reads the work
multiplier and executes every leg of going to eat and going to sleep — a crew member's speed
this tick and their decision this tick should be about the same tick's hunger. It must run
*after* production, so a meal cooked this tick is available to eat this tick rather than next.
And it must not be inside haul: haul's job is moving people, and hanging counter accrual off the
top of it would bury two needs inside the most intricate system in the crate.

**Needs cannot make tick order matter more than it already does.** Tick order is load-bearing
for exactly two things: determinism, and charge priority — consumers draw from a shared pool as
they run, so who runs first is who gets served when the pool is thin (§1.6). **`needs` draws no
charge, and neither does eating or sleeping.** The canteen is unpowered by decision (§4.3), so
nothing in this milestone joins the priority order, and the insertion moves nothing except the
golden fixture. That fixture goes stale anyway the moment `Crew` changes shape, which is why an
insertion in the middle is acceptable here where it would not be in a milestone that changed
nothing else about state.

What must not happen is somebody moving `needs` after `haul` to avoid the insertion. It would
work — a one-tick lag, deterministic, imperceptible, exactly the shape of `paces_last` (§3.8) —
and it would put the decision to go and eat a tick behind the hunger that motivated it for no
gain, since the fixture is being regenerated either way.

### 4.8 What M4 changes in code that already exists

Not a task list — a list of the places where existing code assumes something M4 stops being
true, collected so they are found before they are debugged. §3.9's record is worth reading
first: two of that milestone's surprises were found by something running rather than by anyone
reading, and both were in this category.

- **`Crew` gains three fields** (`hunger`, `rested`, `shift`) and widens a fourth (below), so
  the state hash changes, so **every replay and the golden fixture go stale.** Regenerate, and
  extend the recording to cover `SetShift`, a meal, and a sleep.
- **`Crew.repair: Option<RepairJob>` generalises to `Crew.errand: Option<Errand>`** with
  `Repair`, `Meal` and `Bunk` arms, each carrying the floor and slot to stand at. Three
  parallel `Option`s alongside `task` is the alternative and it duplicates `repair_leg`'s
  routing three times; the enum keeps one router. `haul::resume`, `assign_idle`,
  `repair::pick_repair` and every test naming `crew.repair` move with it.
- **`CrewState` gains `Eating { ticks_left }` and `Sleeping`**, and the matches on it are
  exhaustive in more places than the enum's definition suggests: `haul::advance`,
  `snapshot.rs`'s `CrewStateTag`, `web/src/bridge/types.ts`'s `CrewStateTag` union, and
  `scene.ts`'s `drawCrew`, which switches on the tag string. A tag the renderer does not know
  draws as a standing figure, which is a graceful failure and also an invisible one.
- **`walk_ticks_per_slot` and `climb_ticks_per_floor` stop being fixed**, and the multiplier
  must scale the *duration* rather than the `Fx` step — the trap spelled out in §4.4. Applying
  a percentage to `Fx::ratio(1, ticks)` reproduces the truncation that made four terrain yields
  behave as two before M3 (`intake::terrain_effort`).
- **Sleeping crew must be excluded from everything that assigns work**, in `haul::assign_idle`
  and in `repair::pick_repair`. They hold no task, so `committed_pickup` and
  `committed_delivery` are already safe, but a sleeper who gets handed a repair is a crew member
  who works in their sleep.
- **`wait_ticks` must stay zero in the new arms.** `Sleeping` and `Eating` are not blocked
  states, and `CrewView.stressed` is driven purely by `wait_ticks` — a red-tinted sleeper would
  break the one instrument §8 rests on.
- **`state.rs`'s `add_crew` placeholder name list** becomes real content, still selected by
  index, never by a roll (§3.5). The comment there already says "placeholders until M4".
- **`power::lighting`** is the code the §4.4 decision is about: it is left alone, and the
  reasoning for leaving it alone belongs in a comment next to it, because "make lamps
  occupancy-gated" is the obvious next idea somebody will have.
- **`RoomCategory` gains `Quarters`**, so `content::validate`'s category-wiring check, the
  catalog's `RoomInfo`, the build menu, `roomColor` and `roomProfile` all need an arm. The last
  two fall through to a generic box, so the failure is a bunk that looks like a crate rather
  than a crash.
- **`SoundEvent` gains three arms**, mirrored in `web/src/bridge/types.ts`'s union, and
  `Game.ts`'s "sounds are produced and dropped until the audio pass in M4" comment becomes
  false — it is the one line in the frontend that names this milestone directly.
- **`CrewView` gains `hunger`, `rested`, `shift` and `asleep`** — a change to the bridge's
  public contract, and therefore a frontend change made deliberately rather than discovered
  (`AGENTS.md` §IV). `tests/snapshot.rs` guards the shape.
- **All four harnesses build towers with no canteen and no bunks.**
  `examples/siege_run.rs`, `examples/throughput.rs`, `examples/journey.rs` and
  `examples/record_golden.rs` each work a shopping list and then measure; every one of them will
  now measure a starving, exhausted tower and report it as an economy. This is the same shape as
  M3's fork omission — a harness that silently measures the wrong tower with total confidence —
  and it needs fixing in the same commit as the needs, not after the numbers come out wrong.
  `throughput.rs` is the most sensitive: it measures shaft contention over 600 s with crew whose
  walking speed M4 has just made variable.
- **`examples/journey.rs`'s shade-versus-sun comparison is the milestone's own instrument**, per
  §4.3, and its policies need the canteen in their shopping lists before the biomass axis can be
  said to have come alive.
- **`tests/balance_doc.rs` is bidirectional** — every new `balance.ron` field needs a graded
  `BALANCE.md` row in the same commit, and the content-constants group row needs the canteen and
  the bunk added.
- **`BALANCE.md`'s `crew_cap` row and its `storeroom` row both defer explicitly to M4**: the
  first for whether the rota earns the plan's eight, the second for what to do about shelf
  typing. Neither is optional to answer; the second is answered in §4.3 and §4.10.
- **Per-daypart elevator programs still have no UI.** They exist in the data model, the command
  layer and the replay format; §1.7 deferred the UI and said it "should land alongside M4's
  shift rota if not before", and §2.9 carried that forward unchanged. The rota's roster is the
  natural home for it — both are schedules written against the daypart clock — so M4 inherits
  it. **Done**, and one thing had to change in the bridge to make it possible: `ShaftView` did
  not publish the programs at all, so there was no way to *read back* what was set. A schedule
  you cannot see is one you cannot edit, and that is most of why this went three milestones
  without a UI despite the command existing the whole time. The editor shows the current
  daypart and edits that one rather than offering a grid of every daypart against every floor —
  a player setting a night program at midday cannot see what they are doing, and the version of
  this that is a spreadsheet is the version that gets built and never opened.
- **`web/e2e/capture.spec.ts`** gains the stills §4.9 needs. It is also the only tool the
  project has for answering a visual question, so anything in §4.5 that cannot be seen in a
  capture is not finished.

**What the list above missed, found by building it.** Four things, kept because three of them
are the kind of thing that is obvious afterwards and invisible before.

- **A run opened with its whole crew asleep.** The clock started at permille 0, which is
  predawn, which is the night band; every crew member defaults to the day shift; so the first
  2,592 ticks — 86 seconds at 1× — had nobody moving. Nothing in §4.4 is wrong, and the
  interaction is fatal anyway: the only reading available to somebody who has not yet been
  taught what a rota is, is that the game is broken. `Clock::new` now starts a run at the
  handover onto the day shift, found from the pack rather than hardcoded, and the tower sets
  out in the morning.
- **Stranded carriers had to be allowed to do everything except put the load down.** The
  ladder in §4.4 gates bed and meal on empty hands, which is right, and `pick_repair` had
  already carved out an exception in M3 for a carrier the tower has nowhere to put — no free
  shelf, no hungry room. Sleep and meals needed the same exception for a stronger reason:
  without it, a crew member stranded by the shelf-typing deadlock never slept and never ate
  *again*, and spent the rest of the run permanently tired, permanently starving, and working
  at 36% with the deadlock as the invisible cause. Measured, it was the difference between 14%
  and 41% of crew-hours spent asleep — the second figure being what the night band is actually
  worth.
- **A throughput window is a whole number of days, or it is a measurement of what time it
  started.** `throughput.rs` and the elevator test both used windows that were not, so once
  crew slept, two thirds of the measurement fell across the night when the tower does not
  queue. The elevator reported 19 crafts without against 18 with; the same two towers over a
  whole day read 18 against 36. The effect had not moved — the instrument had stopped pointing
  at it. This is the same failure as M3's fork omission, and it will recur every time the
  simulation grows a new rhythm.
- **The canteen's authored recipe contradicted its own design paragraph**, and the arithmetic
  in §4.3 was the thing that caught it. See §4.9.

- **`RoomDef.short` codes have to be unique and nothing checks it.** The bunk shipped `"BNK"`,
  which the cell bank already owned; invisible until the two stand on the same floor, at which
  point the cross-section shows two of the same room. Found by looking at `home-evening.png`.
  A content-validation rule would catch the next one, and is not written.

### 4.9 Exit criteria

Both of M4's criteria are answered by looking and listening. Neither can be asserted, and a
passing test suite is evidence about the code rather than about the game — which is why each
one below says what would actually demonstrate it.

- [~] **The eyes-closed test: can you hear how the tower is doing?** Demonstrated by a
      listener with the screen off, given three unlabelled sixty-second recordings from a real
      run, answering four questions about each: is it day or night; is the chain running or
      stalled; is something attacking; is the tower walking or stopped. Four binaries, twelve
      answers, and the criterion is getting them from sound alone. A listener who can tell day
      from night but cannot tell a working mill from a starved one has found that the loops are
      decorating the mix rather than reporting it.

      **There is no harness for this, and the honest version of one is buildable.** Playwright
      cannot assert audio. What it would have to be: replay the golden fixture headlessly,
      capture each frame's `SoundEvent[]` plus a once-a-second `ViewSnapshot` digest into a
      log, then drive the same `AudioManager` from that log under an `OfflineAudioContext` and
      render a WAV. That makes the soundscape deterministic, diffable, and reviewable without a
      browser — the audio counterpart of `capture.spec.ts` — and it would catch the regression
      nobody notices, which is a loop that stopped being wired to the state it claims to
      report. Until it exists, the criterion is answered by a person listening, and that should
      be said rather than implied.

      **Built, and unjudged.** `web/src/engine/AudioManager.ts` is the whole subsystem: nine
      continuous beds read off `view()` and nineteen one-shots fired from `frame()`'s event
      list, coalesced by kind within a frame and rate-limited per kind, all synthesised from
      oscillators and filtered noise so there is nothing to fetch and nothing to fail to load.
      The split the spec insisted on is the shape of the file — loops from the snapshot,
      one-shots from the events — and the diegetic rule holds throughout: no arm on a stalled
      room, no beep on a queue, and the legs read `journey.halt`'s five states rather than
      `power.walking`, so a tower that stopped and a tower that cannot afford to move do not
      sound the same. The three new events are wired: `MealServed`, `ShiftChange`, and
      `EnemyLeaves`, the last of which closes M2's deferral by making a creature that walked
      away audibly different from one that was shot down.

      **The harness got built, and it found the mix was failing.** `web/e2e/audio.spec.ts`
      does what the paragraph above sketches: steps three sixty-second excerpts at 1×,
      drives the *shipping* `AudioManager` from them under an `OfflineAudioContext`, and
      writes `capture/audio/{a,b,c}.wav` plus per-second energy in five bands. The four
      binaries are then answered from those numbers alone, using discriminators fixed in
      advance from the mixer's own documented band layout, and only then marked.

      First run: **three of the four were unrecoverable, and two were backwards.** A fully
      working tower and a completely switched-off one differed by 6% in the band the
      production loops own, and a *stopped* tower read louder in the leg band than a walking
      one. The wiring was right the whole time; the mix was wrong, in three ways that are
      worth writing down because none of them is audible as a bug — they are audible as
      "atmospheric".

      1. **A `lowpass` on white noise is not a bed, it is a wall.** The jungle sat on a
         lowpass at 620 Hz, which keeps *everything* underneath it, so one decorative loop
         held eight times the energy of every state-carrying loop combined. Beds are
         bandpassed now, and the bottom of the spectrum is left to the two things that mean
         something down there.
      2. **A pure tone beats broadband noise for presence at a fraction of the amplitude.**
         The electrical hum is a 50 Hz sine and the legs are filtered noise; at similar gains
         the hum owned the whole bottom octave and "is the tower walking" had no answer. The
         hum is a fifth of what it was.
      3. **Combat was quieter than the chain.** Impacts were written at roughly a mill's
         level, which is wrong twice: the chain fires constantly and a bite does not, and
         something biting your home should be the loudest thing in the frame. A minute with
         a creature on the tower for a fifth of it was *indistinguishable* from a quiet
         minute.

      After those three: **12 of 12.** Night separates 2.3× on the ratio between the two
      insect bands, a working chain 13× on the band the room loop owns, walking 12× on the
      leg band, and a wave shows as a burst well above the clip's own floor.

      One nuance the harness surfaced rather than papered over. The excerpt scored as "quiet"
      turned out to have had enemies out for a third of it — a wave had announced itself on
      the horizon — and the audio said so. Scored against *contact*, that reads as a false
      positive; scored against the question a listener is actually answering, it is correct.
      Both numbers are reported.

      **Still `[~]`, because none of that is listening.** A signal can carry a fact and still
      not be legible to an ear, and nothing here says whether the tower sounds *good*. What
      changed is that "is it in there" is now answered, checkable, and regression-tested.

- [~] **The screenshot test: does one frame say "solarpunk home, not war machine"?**
      Demonstrated by two stills from `web/e2e/capture.spec.ts` shown to somebody who has never
      seen the game, asked only "what is this place?". `home-evening.png` — dusk, lamps on,
      the canteen's hearth lit and steaming, two crew sitting to a meal, one asleep in a
      hammock, planters full, the jungle going blue behind it — has to come back as somebody's
      home, greenhouse, or ark. If it comes back as a factory, a rig, or a gun platform, the
      pass failed. `home-siege.png` — the same tower mid-wave — is the control: it should read
      as a home under threat, not as a fortress that has finally found its purpose.

      A frame with no people in it cannot pass, which is why crew-as-people (§4.5 item 1) is
      the load-bearing item in the art pass rather than the overgrowth.

      **Both stills exist and both have people in them**, which took more of the harness than
      expected and is worth recording, because the same trap is waiting for the next visual
      criterion. A capture that walks to roughly the right hour and photographs whatever is on
      screen gets a still of three figures standing in a corridor: sleep is easy to catch (the
      night band is a third of the day) but a meal is 300 ticks out of 4,800, so a loop that
      stops at the first interesting state it sees *always* stops on a sleeper. `capture the
      home` now holds out for both and bounds the search to one night band, and prints what the
      crew were actually doing so a still that failed to find a meal says so instead of being
      filed as though it had. It currently reports `climb/sleep/eat`: somebody on the stairs,
      somebody in a hammock, somebody sitting to a bowl.

      Three things the stills caught that no test would have. The bunk and the cell bank both
      shipped `short: "BNK"`, which is invisible until they stand on the same floor and then
      reads as two of the same room — the bunk is `"BED"` now. The harness's build helper
      compared `send`'s result against `null`, which is never what it answers, so every attempt
      read as a failure and the loop bought *four canteens*. And **the crew were dressed in
      almost exactly the colour of the tower's unlit interior** — a muted forest green, which
      is what people in a jungle would sensibly wear and which made them invisible in the one
      frame whose whole job is having people in it. They wear sun-bleached linen now and stand
      against a soft dark halo. All three were found by looking at the picture, which is the
      argument for having the picture.

      **Still `[~]`: nobody who has not seen the game has been shown them.** That is the
      criterion, and it cannot be self-assessed — the whole point of asking a stranger "what is
      this place?" is that the person who drew it already knows the answer.

- [x] **The roster carries both schedules**, and `the roster writes both of the player's
      schedules` in `web/e2e/smoke.spec.ts` drives them through the DOM the way a player does
      rather than through the bridge, because a panel can look right and be wired to nothing.
      It caught one thing immediately: the diagnostics readout sits in the same corner and was
      silently eating clicks on the bottom of the crew list — the button highlighted, nothing
      happened, and there was no way to tell that from a rejected command. It is
      `pointer-events: none` now.

- [x] **The kitchen chain has visibly given bamboo somewhere to go.** `examples/journey.rs`'s
      shade-versus-sun comparison reported *exactly* 100 stalks on both routes at M3 and
      **576 against 539** now, on two routes 8% apart in weighted yield. M3's biggest open
      finding is closed.

      It took three fixes rather than one, and only the first was the one §4.3 predicted.
      **The canteen was authored three times too cheap** — 2 bamboo for *three* meals, against
      a design stated twice as "2 bamboo a meal … six a day a crew member … about a fifth of
      what a fed mill draws" — so a crew member ate two stalks a day and the canteen drew 5% of
      a mill. **The harness had no standing shopping list**, so its towers built two rooms,
      stopped wanting anything, filled every buffer and reported the size of their own shelves
      as a harvest; `siege_run.rs` had diagnosed exactly that for provocation and fixed it
      there, and this is the same fix arriving late. And **stranded carriers never ate**: the
      priority ladder gated eating on empty hands, so a tower deadlocked on shelf typing had
      crew holding a crate forever, never eating, never sleeping, and working at 36% with the
      deadlock as the invisible cause.

      The order matters for anyone re-reading this: the canteen's price was the fix that moved
      the number (353→576), and the other two were what stopped it moving before that.

- [x] **The siege balance is re-earned**, at a cost: `provocation_per_100_harvested` moves
      240 → 300 and the three-way shape returns — subsistence never rises above 6 and ends
      whole at 999‰, greedy peaks at 56 and is eaten down to 62‰, answered holds 809‰ and sees
      off 15.

      The interesting part is why break-even arithmetic gives the wrong answer here. **Sleep
      made provocation spiky.** Harvest now arrives entirely inside the day band while decay
      runs around the clock, so what draws a wave is the mid-afternoon *peak* rather than the
      daily mean, and the two stopped moving together. Break-even says 370; at 370 a
      subsistence tower's peak cleared the wave threshold every afternoon and it took 210 hit
      points on day one, which is not what living within your means is supposed to buy.

      `BALANCE.md`'s Siege section no longer opens with the stale-economy warning, and carries
      two things it explicitly does *not* act on instead: the pressure table has dropped a band
      (mending is crew-hours, and sleep took two-fifths of them), which stays coherent only
      because the provocation a real tower reaches fell by the same order; and the plating
      comparison came back "worse on 7 of 8 seeds" for the third time, which is still an
      artefact of comparing hit-points-missing between towers whose maxima differ by the
      plating under test.

- [x] Golden replay regenerated — it now covers a canteen, two bunks, a `SetShift`, 26 meals
      and 72,854 crew-ticks asleep — and hash parity is green natively and in wasm.

- [x] `make check` and the smoke suite green: 274 Rust tests, clippy clean, 8 Playwright
      tests including native/wasm parity.

**Deferred out of M4:**

- **Role priorities.** `v2-plan.md` §6.7 describes "haul / operate / gun / repair, as role
  priorities per crew member (RimWorld-lite, one screen)". §9's M4 brief does not list them,
  and they are cut deliberately rather than overlooked. There are two kinds of work in the
  game — hauling and mending — so a priority list would have one meaningful row in it, and a
  per-crew priority screen is exactly the kind of menu `DECISIONS.md` §8 argues against when the
  diegetic version already exists: `repair_hp_per_shift` and the carrying rule in
  `assign_idle` already encode a triage policy that the player shapes by what they build. When
  there are four kinds of work — M5's tier-two chains and second emplacement — the row count
  might justify the screen.
- **Positional audio.** Reasoned about and cut in §4.6: `SoundEvent` stays payload-free.
- **~~An audio regression harness.~~** Built after all — `web/e2e/audio.spec.ts`, and it
  earned its keep immediately by finding that three of the four eyes-closed binaries were
  unrecoverable from the mix (§4.9). What is still deferred is the *regression* half: the WAVs
  are rendered and measured but nothing compares them against a stored baseline, so a mix that
  drifts will be noticed by whoever next reads the numbers rather than by CI.
- **Occupancy-gated lighting.** Reasoned about and rejected in §4.4. Recorded because it is the
  obvious idea and the reason not to do it is not obvious.
- **Crew that wake themselves.** Rejected in §4.4: an automatic wake on attack would make the
  rota decorative.
- **Anything that makes a well-fed crew better than baseline.** §4.4: needs are a penalty for
  neglect, never a buff to chase.

### 4.10 Open questions

Things that genuinely cannot be settled without building them.

1. **Does the rota read as a decision, or as an administrative chore?** The argument in §4.4 is
   that day throughput against night cover is a real trade with two named costs. The risk is
   that a player finds one answer, sets it once, and never thinks about it again — at which
   point the rota is a settings screen that cost a milestone. The tell to watch for is whether
   anybody ever uses the surge lever, since that is the only part of the rota that is a
   decision made *during* a run rather than at the start of one. If nobody does, the fix is
   probably to make the night genuinely more dangerous, not to make the rota more complicated.
2. **How much does sleep actually cost, and is the tower still playable at three crew?**
   Removing two-fifths of crew-hours is the largest single economic change in M4 and it lands
   on the economy `starting_crew` was measured against — where going from two crew to three
   moved throughput ninety percent (§1.7). Three crew all on the day shift is not three crew
   any more. The honest possibilities are that `starting_crew` has to rise, that the day band
   has to widen, or that the whole thing is fine because the chain was never crew-limited at
   night anyway. Only the harnesses in §4.8 can say which, and they cannot say it until they
   have a canteen in them.
3. **What is the answer to shelf typing?** `BALANCE.md`'s `storeroom` row hands M4 the deadlock
   where bamboo claims every shelf and the chain stops, and meals make it a five-item
   competition on four shelves. §4.3 makes it non-fatal — nobody starves, because crew eat at
   the canteen — but non-fatal is not solved. The candidates are a player-set item filter per
   shelf (a real infrastructural verb, and a new command), more shelves per storeroom (which
   postpones rather than fixes), or a chute that dumps surplus (M5 scope). The filter is the
   most likely right answer and the most likely to be scope creep; deciding is an owner call,
   and leaving it undiagnosed is the one option that row already ruled out.
4. **Can the eyes-closed test be passed without any sound the diegetic rule would forbid?** The
   rule says a starved mill goes quiet. A tower with many problems therefore sounds like a
   tower with nothing happening, and *quiet* is a hard signal to distinguish from *fine* with
   your eyes shut. The intended answer is that the beds underneath — the jungle, the day and
   night soundscapes, the electrical hum thinning as the bank drains — keep the mix from ever
   being silent, so absence reads against a floor rather than against nothing. Whether that is
   enough is the question the recordings answer, and if it is not, the temptation will be a
   warning tone. That is the wrong fix, and it is worth writing down now, while nobody is
   frustrated.
5. **Do barks survive contact with repetition?** A run is two to four hours and there are up to
   eight crew. Lines that charm on the first hearing are the ones that grate on the fortieth,
   and rate-limiting them into rarity is the standard answer and also the answer that makes
   them stop doing their job. This cannot be settled from the desk, only by hearing the same
   line for the twentieth time and noticing how it feels.

---

## M5 — The Refugia *(depth within rules)*

**Sprint question:** does a second tier add depth, or does it just add rooms?

**Scope:** the tier-two chains `v2-plan.md` §6.1 has carried since the plan was locked, and the
raw materials they turn out to need; chutes; the rest of the creature taxonomy; region 3 and
the arrival that ends a run well; an enclave economy that makes `crew_cap` reachable;
toolkit-widening unlocks and the delivery mechanism §11 left open; balance telemetry and the
difficulty pass that turns `DESIGNED` into `PLAYTESTED`; and the itch.io release cut.

**The content gate is absolute, and it is the whole of what keeps this milestone honest:**
*nothing ships unless an existing system consumes it at runtime.* v1 authored sixteen modifier
effects and applied zero, and that is the failure this rule exists to prevent (`v2-plan.md` §10
rule 1). Every item below names its consumer in the same sentence that introduces it, and
anything that cannot name one is cut here rather than discovered dead later.

### 5.1 The shape of the thing

**M5 is where the route stops being one axis.** Since M1 the world has offered a single trade —
shade is biomass-rich and sun-poor, open ground the reverse — and M4 finally made the biomass
half feelable by giving bamboo a consumer that scales with the crew (§4.3). But the sun half
still only buys *charge*, which is a means rather than an end, so a sun-seeking route reads as
a sacrifice with a rebate rather than as a different way to play.

The tier-two chains fix that by hanging a second material on the other end of the same axis.
Shade grows bamboo; open sun grows **produce**; the ruin belt yields **scrap**, which has been
harvestable since M3 and, until now, has had *no chain consumer at all* — it dead-ends at the
enclave's trade board, which is a fine thing for a material to also do and a poor thing for it
to only do. Three materials, three parts of the map, three tiers built on top of them. That is
the depth M5 is for.

**What this milestone must not become is a wider menu.** Rule 2 of `v2-plan.md` §10 —
"core-mechanic depth beats content breadth" — is the one most at risk here, because a tier list
is the easiest thing in the world to keep extending. The test each new room has to pass is not
"is it interesting" but "does it change a decision the player was already making". A room that
only adds a step to a chain is breadth wearing depth's clothes.

### 5.2 The materials, and what eats them

Chain depth stays at three and no recipe takes more than two inputs (`v2-plan.md` §6.1). The
full pack after M5, with the new entries in bold:

| Tier | Item | Made by | Consumed by |
|---|---|---|---|
| raw | `bamboo` | cutter arm, shade-weighted | mill, canteen, burner |
| raw | `scrap` | salvage rig, at ruins | **sun-forge** (new) |
| raw | **`fiber`** | **fiber comb** (new), mid-band-weighted | **ropery** (new) |
| raw | **`produce`** | **garden** (new), sun-weighted | **bombary** (new) |
| T1 | `poles` | mill | construction, repair, thornwright, **fitter** |
| T1 | `meals` | canteen | crew, three times a day |
| T1 | `darts` | thornwright | dart battery |
| T1 | **`rope`** | **ropery** (new) | every shaft and every emplacement's build cost |
| T1 | **`alloy`** | **sun-forge** (new), charge-hungry | **fitter**, **cellwright** |
| T2 | **`mechanisms`** | **fitter** (new) | elevator build cost, **seed thrower** (new) |
| T2 | **`seed bombs`** | **bombary** (new) | seed thrower, as ammo |
| T2 | **`charge cells`** | **cellwright** (new) | cell bank build cost |

**Meals stay on bamboo, and that is a deliberate departure from `v2-plan.md` §6.1**, which
lists them as `produce`→kitchen. M4 built them from bamboo for a specific reason — bamboo had
no consumer and the biomass axis was therefore unfeelable (§4.3) — and the measurement that
justified it is on the record: two routes eight percent apart in weighted yield harvested 353
stalks against 351 before the canteen was priced properly, and 576 against 539 after. Switching
meals to produce now would hand that back. Produce gets seed bombs instead, which is a better
job for it anyway: a material that grows in the sun feeding a weapon that denies ground is a
cleaner opposite to bamboo feeding poles than two food chains would be.

**Three of these rooms are the point and three are plumbing.** Worth being honest about which:

- The **sun-forge** is the point. It is the first room that turns a raw material the tower
  cannot grow into something the chain needs, it is charge-hungry enough to compete with
  striding, and it gives every ruin the tower walks past a second reason to stop. It is also
  what makes the drowned city a destination rather than a corridor.
- The **garden** is the point. It produces on sunlight rather than on ground covered, which
  makes it the first intake a *stopped* tower runs at full rate — the exact inverse of the
  cutter arm (§3.6), and therefore the first real argument for standing still.
- The **fitter** is the point, because mechanisms gate the elevator (§5.3).
- The **ropery**, the **bombary** and the **cellwright** are plumbing: one input, one output,
  no decision of their own. They earn their slots by what they feed, and if any of them reads
  as a step rather than as a choice in play, the right answer is to fold its output into an
  existing room rather than to make it more interesting.

**Opening values**, as design targets with an argument behind them rather than measurements.
None is a `BALANCE.md` row until it is authored, at which point it gets a graded row in the
same commit (`DECISIONS.md` §7).

| Thing | Target | The arithmetic |
|---|---|---|
| fiber comb | Intake, 2 slots, 4 poles, `paces_per_item` 90 | A shade under the cutter arm's 78, because fiber is the *second* thing a band gives up. Weighted to clearing and drowned street, so the bands poorest in bamboo are rich in something. |
| garden | Intake, 2 slots, 5 poles, `ticks_per_item` 260 scaled by exposure | **Per tick, scaled by sun, not per pace.** A stopped tower harvests nothing today; a garden keeps working while the legs are off, which is what makes berthing cost less than it does now. At full sun that is one produce every 8.7 s; under dense canopy it is nearly nothing. |
| ropery | Production, 2 slots, 4 poles; 2 fiber → 1 rope, 120 ticks | The mill's price and rhythm exactly, because it is the mill's opposite number and the two should feel like siblings. |
| sun-forge | Production, 2 slots, 8 poles; 3 scrap → 1 alloy, 240 ticks, `power_draw` 4 | Four times the thornwright's draw and the largest single sink in the tower: a forge running flat out costs more charge across a day than continuous striding, so *running the forge* and *walking far* become the same decision. Eight seconds a bar makes it visibly the slowest thing in the chain. |
| fitter | Production, 2 slots, 6 poles + 4 rope; 1 alloy + 2 poles → 1 mechanism, 300 ticks | The first build cost that is not poles alone, and the first recipe drawing two inputs from different chains. |
| bombary | Production, 2 slots, 6 poles + 4 rope; 2 produce + 1 fiber → 2 seed bombs, 200 ticks | Cheaper per shot than darts and slower to make, so a thrower is the answer to *many* things rather than to one hard thing. |
| cellwright | Production, 2 slots, 6 poles; 2 alloy → 1 charge cell, 300 ticks | Storage is built, so the tower's charge ceiling becomes something the chain earns rather than something poles buy. |
| seed thrower | Defence, 2 slots, 6 poles + 3 rope + 2 mechanisms; 1 seed bomb a shot, 8 damage across a 30-pace band, `reload_ticks` 90 | **Area, not aim.** The dart battery answers one creature well; the thrower answers a wave badly and cheaply. Two emplacements with different failure modes is what makes "what do I build" a question at all. |
| `rope` in shaft costs | stairs 0, dumbwaiter +3, elevator +6 | Shafts stop being a pure pole cost, so the material easiest to get in the *middle* bands is what buys vertical transport. A tower that never leaves the canopy can afford poles and not rope. |
| `mechanisms` in the elevator's cost | 2 | **The elevator becomes a tier-two building**, which is the sharpest single expression of "depth within rules": M1's centrepiece stops being something a starting tower can rush, and the route that reaches it runs through the ruins. |
| `charge cells` in the cell bank's cost | 2, replacing 8 poles | See above: batteries are built. |

### 5.3 What each tier-two item is *for*, stated as a consumer

The content gate applied one item at a time, because "it feeds the next room" is not a
consumer, it is a postponement.

**`mechanisms` gate the elevator and the seed thrower.** This is the load-bearing one. Until M5
the elevator is 18 poles and a decision about *when*; after M5 it is 18 poles, 6 rope and 2
mechanisms and a decision about *whether the route you took can build one at all*. A tower that
walked the shaded branches every time has bamboo and no scrap, so no alloy, so no mechanisms,
so it climbs its stairs. That is route choice reaching all the way into the transport layer,
which is where this game's thesis lives.

**`charge cells` are the tower's charge ceiling.** A cell bank costing cells rather than poles
means the answer to "I keep browning out at night" stops being "spend poles" and becomes "run
the forge, which costs charge" — a loop to be climbed rather than bought out of. It is the first
place in the game where fixing a problem costs the resource the problem is about.

**`seed bombs` are the second emplacement's ammo**, and the second emplacement exists to give
defence a *shape* rather than a level. A dart battery kills one thing at a time and is the right
answer to a borer at a shaft column; a thrower scatters and is the right answer to four skitters
on a panel. Neither is an upgrade of the other, which is the only way a second anything is
allowed to exist here.

**`rope` is what every shaft and every emplacement is partly made of.** Not a tier so much as a
second construction currency, and its job is to stop poles being the answer to everything. Fiber
comes from the middle bands, so the tower that can build transport is the one that walked
through ordinary ground rather than optimising for either extreme.

**`alloy` is the only thing that consumes scrap**, and therefore the reason to berth. §3.4 built
berthing, wardens and the salvage rig, and §3.11 asked whether stopping at a ruin is ever
genuinely "it depends" rather than always yes or always no. It could not be: scrap bought poles
at the enclave and nothing else, so the answer was "yes, if you happen to be passing". With a
forge aboard, scrap is the input to the two things that gate the elevator and the charge
ceiling, and the question becomes a real one.

### 5.4 Chutes, and the answer to shelf typing

`BALANCE.md`'s `storeroom` row has described the sharpest emergent failure in the game since M2
and handed the fix forward twice: a shelf takes whichever item lands on it first and holds only
that until it empties, so a material arriving faster than it is consumed claims shelf after
shelf until nothing else can be put down and the chain deadlocks. §4.3 made it non-fatal — crew
eat at the canteen, so nobody starves — and said plainly that non-fatal is not solved. §4.10's
third open question named three candidates and called the decision an owner call.

**The call: a chute, which is the candidate that is a piece of infrastructure rather than a
setting.** A chute is a shaft kind (§1.3) with no cars, no capacity and no charge draw — things
fall down it — ending at a spill gate on the ground floor. Anything a crew member drops in
leaves the tower. Haul gains one destination of last resort, below a shelf in priority: an item
with no hungry inbox and no free shelf goes down a reachable chute rather than stranding whoever
is holding it.

Why this rather than a per-shelf item filter:

- **It is a thing you build, not a menu you set.** The player answers the deadlock by spending
  slots and materials on a piece of the tower, which is how every other problem in this game is
  answered. A filter would be the first place the answer was a dropdown.
- **It costs something ongoing.** A chute is a slot column on every floor it spans, like every
  other shaft (`DESIGN.md` pillar 2), and what goes down it is gone. Surplus bamboo dumped is
  bamboo not milled later.
- **It is visible.** You can watch the overflow leave, which puts the deadlock's *cause* on
  screen rather than leaving it to be inferred from a chain that stopped.
- **It composes with the stranded-carrier rule.** M4 already lets a stranded carrier sleep, eat
  and mend while holding a load they cannot put down (§4.8); a chute turns that from a
  survivable dead end into something the player can fix.

The filter is not rejected as a bad idea, only as a worse first one. If chutes ship and towers
still jam — because the surplus is something you *wanted* and a chute is too blunt — a per-shelf
filter is the next thing to try, and it will land on a game that has already made overflow
visible, which is a better game to add it to.

| Thing | Target | The arithmetic |
|---|---|---|
| chute | Shaft, 1 slot column, **6 poles, no rope**, no charge, no capacity | Cheaper than a dumbwaiter (8 poles) because it does less: one direction, no machinery, nothing comes back up. **Specced at 6 poles + 2 rope and shipped without the rope**: rope needs a ropery, a ropery needs fiber to reach it, and fiber having nowhere to go *is the jam* — so the rope made the escape hatch affordable only before you needed it. Everything else in the pack may sit behind a chain; this one may not. |
| spill priority | below `PRIORITY_SHELF` | Never preferred to somewhere useful. A chute is where things go when there is nowhere else, and a tower with spare shelf space should never spill. |
| what may be spilled | nothing a live inbox wants, nothing anything is **built** with, nothing a **settlement takes** | The third clause was missing and a chute was eating salvage. `wanted` asked two questions — does a live room's inbox take it, and is it a build cost — and **scrap answers no to both**: its only room consumer is the sun forge, so a tower without one has no inbox wanting it, and nothing is built from it. Yet scrap is the entire point of berthing at a ruin (§3.4). A player with a chute stopped, woke the wardens, took the damage, collected the scrap, and watched their crew carry it out of the tower, with nothing on screen saying so. The gap is structural rather than an oversight: an enclave's `Trade`, `Recruit` and `Reinforce` are **commands**, so what they consume appears in no room's inputs and is invisible to a check that only reads rooms. `Content::settlements_take` closes it. Deliberately not conditional on a settlement being *in reach* — "there is no buyer within forty minutes" is not a reason to throw something away, and a chute that reasoned that way could not be planned around. |
| what is left spillable | fiber, darts, meals, seed bombs, charge cells | Narrow on purpose, and narrower than it looks: rope is a build cost, so it was already safe; fiber is spillable only while no ropery is running, which is exactly the case §5.4 was written about. **Caught by the screenshot harness rather than by a test** — the capture run berthed, salvaged, and then photographed an enclave board it could not afford to buy from. |

### 5.5 The rest of the taxonomy, region 3, and the Refugia

**Two creatures complete the set**, and both exist to attack something no current creature
does. The four shipped kinds all converge on the tower and bite it; what is missing is a threat
to the *chain* rather than to the structure.

| Creature | Shape | Why it is not a fifth biter |
|---|---|---|
| **glean-crow** | Fast, fragile, `min_provocation` ~250. Lands on an outbox, takes what is in it, leaves. | Attacks throughput rather than hit points. The tower is undamaged and the day's harvest is gone — a loss repair cannot answer and defence can. |
| **mire-hulk** | Very slow, very tough, region 3 only. Grapples a *leg* and slows the stride while attached. | Attacks the journey. The one creature that cannot be walked away from, because it is what is stopping you walking — the counterpart to the warden, which is what stopping wakes. |

**Region 3 is the coast approach**: the canopy thins, the ground opens, sun is abundant and
biomass is poor. Mechanically it is the mirror of region 1, which is what makes a run's shape an
arc rather than a ramp — a tower tuned for shade arrives somewhere its habits do not work, and
the garden that was marginal in the jungle is what keeps it fed. The mire-hulk lives here, and
so does the last stretch of walking.

**The Refugia is an arrival, not a victory.** §3.7 already ends a run two ways and presents the
far edge as somewhere the tower got to rather than something it won (`DECISIONS.md` §8); M5
changes the destination from an edge to a place. What arriving shows: the tower, stopped, with
whatever it still has aboard; the crew by name and what became of them; the route it walked as a
line through three regions; and the seed, so the run can be handed to somebody else. **No score,
no rank, no stars.** A run's ending is a description.

**The name stays "the Refugia."** `v2-plan.md` §11 open #1 marked it provisional and asked for a
decision during M3's world-writing, which never happened because M3 never reached region 3.
Deciding it now, by keeping it: it has been the word in every document for the whole project, it
means what it should mean, and a rename at this point would be churn for its own sake.

### 5.6 The enclave economy

`BALANCE.md`'s `crew_cap` row raised the ceiling to eight at M4 on the strength of the rota and
noted that a run cannot approach it, because M3's single enclave offers exactly one recruit. M5
is where that stops being aspirational.

- **An enclave in every region**, three in a run rather than one, each with its own board.
  Ropewalk in the jungle at 30,000 paces, High Water in the drowned city, Tidewatch on the coast.
- **Boards sell what their region has and buy what it has not.** Ropewalk lays rope and cuts
  thorn darts and has no metal at all, so it sells both for timber and buys scrap. High Water and Tidewatch
  are the reverse: they sit on metal and want timber, alloy and produce. A player carrying a
  surplus finds a buyer for it somewhere, which turns "I have too much of this" from a jam into a
  plan.

  *Corrected against the pack, which said the opposite.* An earlier draft here had the drowned
  city "wanting poles and selling scrap". It does not and should not: the city's `scrap 4 →
  poles 3` is the payoff for the salvage rig and the reason scrap exists at all (§3.4), and
  reversing it would take the point out of berthing at a ruin. **Scrap has one price at every
  board on purpose** — 4 for 3, everywhere — so there is nothing to buy in one region and sell in
  the next, which is §5.11 open question 3's whole worry.
- **Ropewalk exists so region 1 can reach the elevator.** An elevator costs six rope; rope costs
  a ropery, a ropery costs fiber, and fiber costs a comb. That put vertical transport — the bet
  the whole game rests on (`DESIGN.md` pillar 2) — three rooms deep on a tower twenty minutes
  old. Six poles a pair, four pairs, is one elevator and two spare — and poles are what the shaft's own
  frame costs, so the rope competes with the thing it unlocks. The chain stays far cheaper for a
  tower that means to keep using rope.
- **Recruits cost more each time**, so crew growth is a curve rather than a switch and the
  eighth crew member is a decision about a whole run's savings. Ropewalk 18 poles, High Water 30,
  Tidewatch 24 poles *and* 4 alloy, three times over.
- **And the arithmetic has to close, which is why there are three.** `crew_cap` is 8 and a run
  starts with 3. Two settlements offer four recruits between them, so the cap was unreachable by
  one — a ceiling nothing can touch is a number pretending to be a decision. Five offered makes
  eight possible for a run that spends everything on people, and nothing else.
- **Shell work stays region-two only.** It was withdrawn once for making the tower worse and put
  back with a measurement (`BALANCE.md`'s `reinforce` row); spreading it across three enclaves
  would multiply a thing that is barely worth its price.

#### 5.6.1 The wayhouse — proposed, and **not yet decided**

> **Read §5.11 open question 0 first.** This section was drafted to answer that question and it
> does not answer it. The measurement that closed the question also showed that **a buyer would
> raise a tower's harvest by about a quarter and change the route comparison by nothing at all**,
> because yield and sun were already cancelling on purpose. So a wayhouse has to be justified as
> a mechanic somebody wants to play, not as a fix for a defect — there is no defect. Recorded in
> full because the design is sound and the reasoning behind its shape is still worth having.

**Three enclaves with finite boards are three events.** The §5.6 bullet above promises that a
player carrying a surplus "finds a buyer for it somewhere, which turns 'I have too much of this'
from a jam into a plan," and a buyer that does that has to be there when the jam happens — on the
road, continuously, for the whole run. That much stands regardless of open question 0.

The constraints on its shape are real and were expensive to find:

- **A buyer that pays in items moves the jam one step along.** Poles leave, whatever came back
  fills a shelf instead, and the mill stalls a week later rather than a day later.
- **A sink bounded by crew count is bounded.** That is why the canteen fixed meals and fixed
  nothing else: crew eat three a day for ever, but four people only eat twelve.
- **A sink that scales with tower size × time is unbounded, and is the overgrowth prototype**
  (`4a4683a`), which works and is not shipped because a tower that is never quite whole cannot
  tell you the difference between "something is attacking" and "it is Tuesday."

That leaves one axis: **distance walked.** A buyer that recurs down the road has a total appetite
that grows with the run, without any single one of them being a bottomless pit.

**The proposal: a wayhouse, and a deck to trade from.**

- **A wayhouse is a world feature**, generated per band the way ruins are, in every region. Each
  carries a finite pool the way a ruin carries salvage, and depletes as it is traded with. They
  recur, so what scales is the run rather than the building.
- **Berthing stays implicit** (`room.salvage_rig`'s comment is the precedent): a tower stopped
  within range of a wayhouse, with a deck that works, is trading. There is no Berth command and
  no Trade command here — stopping next to a wayhouse with no deck does nothing, and the empty
  space in the build menu is the affordance.
- **It buys, it never sells.** One direction only. `Trade` at an enclave stays what it is — an
  event, with unique goods and finite stock — and the wayhouse is the routine, boring counterpart
  that is always there. Nothing has two prices anywhere, so there is nothing to arbitrage, which
  is open question 3's whole worry.
- **What it pays in is charge.** Charge is the only resource in the game consumed continuously
  and for ever, by the legs, every tick, so it is the one payment that cannot back up into a
  shelf and re-create the jam. It also points the mechanic at the trade the game is already
  making: shade gives ground and takes power, and a wayhouse lets a shade route convert what the
  ground gave it back into power. **This does not widen the route gap** — the ceiling measurement
  in §5.11 rules that out — but it does let a player *choose* which side of the opposition to
  lean on, which is a different and better claim than the one this section originally made.
- **It costs the same hands as everything else.** Poles reach the deck by crew haul, on the same
  stairs, competing with every other errand (`DESIGN.md` insight 1). A wayhouse is not a drain
  bolted to the side of the tower; it is another mouth on the same shift.

**The trade this creates, stated plainly:** stop, and spend walking time to convert timber into
power. Walk on, and keep the time but leave the surplus jamming your shelves. That is the same
stop-or-walk decision the salvage rig introduced at M3, pointed at the other end of the chain,
and it is why the deck is a built room rather than a free action.

### 5.7 Unlocks, and how they arrive

`v2-plan.md` §11 open #2 asks for a decision between enclave gifts, Heartseed cultivars and a
journal. **The call: a journal, kept by the crew.**

- A run ends. The journal gains an entry naming something the tower did that it had not done
  before — reached the drowned city, ran a forge, walked away from a warden, fed eight people.
- Entries unlock **rooms and route options, never numbers.** `v2-plan.md` §3's structural call
  is absolute here: a first-run tower and a fiftieth-run tower start identical, and the veteran
  has more tools rather than better ones.
- Unlocked content is added to the build menu, so the surface a new player sees is small and the
  surface a veteran sees is wide, and neither is stronger.

Why the journal over the other two: an enclave gift makes the unlock a thing that happened
*during* a run, so a run's outcome depends on whether the gift turned up, which is the seed
deciding the meta. A Heartseed cultivar makes the unlock a property of the tower, which reads as
a stat even when it is not. A journal is a record of what you did, it is legible without a
tutorial, and it is the only one of the three that cannot be mistaken for power.

**Determinism.** The journal is player-level state, not run state. It never enters `GameState`,
never enters the replay, and a shared seed reproduces a run regardless of who plays it — which
is `v2-plan.md` §6.6's promise and would be silently broken by any unlock that changed what a
seed generates. What an unlock changes is which commands the *player* may send, and a replay
carries the commands.

### 5.8 Telemetry, and the difficulty pass

The exit criteria ask for every balance constant graded `PLAYTESTED`, and today most are
`DESIGNED` because the instruments are scripted harnesses rather than sessions. M5 adds the
missing half: **a run log**, written at the end of every run, recording the seed, the route,
what was built and when, what was harvested and spent, every wave and what it cost, and how the
run ended. It is a file the player can read and hand over, and it is the input to the difficulty
pass.

**Built, and for a long time only half built.** Every run has been recorded to the journal
since M5, and there was no way to get one out — the arrival said how many runs were written
down and stopped there, which made the log a thing the game kept rather than a thing the
player has. `Chrome.tsx`'s `RunLogs` now offers to copy the lot as JSON, and
`e2e/capture.spec.ts` asserts at the arrival that there is something to hand over and that it
carries a seed and an ending. That gap was the whole distance between somebody playing and
these constants being graded from run logs rather than from harnesses, which is what the
exit criterion asks for in as many words.

A copy action rather than a table, deliberately: the arrival is not a place for a dashboard
(`DECISIONS.md` §8), and the reader this is for is somebody doing a difficulty pass with a
text file open rather than a player admiring their statistics.

Two rules it inherits. **It is not a score** — no derived rating, no grade, nothing that reads
as a mark out of ten (`DECISIONS.md` §8). And **it is written from the snapshot, never from
inside the simulation**: a counter that exists only to be logged is a counter that will drift
from the thing it claims to count, and `RunStats` already carries what a log needs.

### 5.9 What M5 changes in code that already exists

Not a task list — the places where existing code assumes something M5 stops being true.

- **`IntakeSource` gains a third arm.** It is `Terrain { paces_per_item }` and
  `Ruin { range_paces }` today; a garden is neither, because it accrues per *tick* scaled by
  exposure. That is a new shape rather than a new parameter, and `intake.rs` grows a third
  branch. The trap is `terrain_effort`'s: scale the *interval*, never the per-tick `Fx` step
  (§4.4).
- **`ShaftKind` gains `Chute`**, the first shaft with no capacity, no charge draw and no riders.
  `transport.rs`, `haul::best_shaft` (crew must never route *through* one) and `scene.ts`'s
  shaft drawing all switch on kind.
- **`HaulDestination` gains `Spill`**, below `Shelf` in priority, and `find_destination` learns a
  third case. The stranded-carrier rule (§4.8) stays exactly as it is — a chute is a
  destination, so a carrier who can reach one is no longer stranded, and one who cannot still
  sleeps and eats while holding.
- **Build costs stop being poles-only in practice.** `check_stock`/`spend` have taken a list
  since M0, so nothing structural changes — but every harness, every capture script and
  `place_when_affordable`'s wait-for-the-money loop currently reason about one material. A
  shopping list that waits for poles and needs rope waits forever.
- **`RegionDef` count goes from two to three**, and the contiguous-`order` check, the journey
  roll, and `state.rs`'s `enclave_stock` — sized for one enclave with a comment saying it becomes
  a list per enclave when a second lands — all move.
- **Two more `SoundEvent` arms** (`Spill`, `Steal`) and their mirrors in `types.ts`.
- **`CatalogSnapshot` grows**, and the build menu's grouping starts to matter at ~18 rooms in a
  way it does not at 12.
- **Every balance harness needs the new rooms in its shopping list**, which is the third time
  this has been true (§3.9, §4.8) and the reason it is written down again: a harness that
  measures a tower without the milestone's rooms in it measures the previous milestone with
  total confidence.

### 5.10 Exit criteria

- [~] **A full run to the Refugia in 2–4 hours**, played rather than scripted, ending as an
      arrival rather than a score.

      **The length is measured and it is right; the *played* half still needs a person.**
      `examples/journey.rs`'s "how long is a whole run" walks every seed start to finish:
      **12 of 12 reach the Refugia, in 129–147 minutes at 1× — 2.2 to 2.4 hours**, 16 to 18
      in-game days. One seed had ever been asked before, which for a length rolled per region
      per run was no answer at all; the spread turns out to be 14%, so the window is not a
      lucky seed.

      A scripted walker is the **floor**, not the estimate. It never stops, never berths, never
      reads a board, and answers every fork the instant it appears — a person does all four, and
      every one of them adds time. So the asymmetry matters: a walker under two hours would not
      prove the run too short, but a walker over four would prove it too long, because nothing a
      player does makes a run shorter. At 2.2–2.4 hours the floor sits just inside the window
      with the whole of the top half free for a player to spend.

      **"Ending as an arrival rather than a score" is now asserted rather than assumed.**
      `e2e/capture.spec.ts` reaches the arrival and checks the card for score vocabulary and for
      the *shape* of a score — a mark over a maximum, a percentage — and checks that the crew
      line names people rather than counting them, because "Aboard: 3" would be a score with a
      friendly face on it (`DESIGN.md` §2 structural call 4). It reports *"arrival reads as a
      description, not a score"*.

      Worth having as a guard rather than a comment: `DECISIONS.md` §8 forbids a rank, and the
      arrival is exactly the screen where somebody being helpful would add one. Two words had to
      come out of the check first — "out of" and "final" both matched ordinary prose, and a deed
      that reads *"Walked a tower out of the deep jungle"* is not a mark out of ten. A guard that
      cries wolf on the writing is a guard the next person deletes.

      What is unverified is whether two hours of it is *enjoyable*, which is not a thing a
      harness can be pointed at.
- [x] **A shared seed reproduces it.** Unlocks are player-level, never touch `GameState` and
      never enter the replay, so a veteran's recording replays exactly for somebody who has
      unlocked nothing — they watch a tower build a room they could not build themselves. The seed
      crosses the bridge as a *string*, because it is 64 bits and JavaScript numbers are not, and
      a seed that does not round-trip is a seed that cannot be shared.
- [~] **The second tier changed a decision** — but not the decision this said it would, and the
      gate moved because building it proved the specced one wrong.

      Gating the elevator behind *mechanisms* put it behind alloy, scrap and the ruin belt, which
      means a rig, a forge and a fitter: four rooms and two floors more than a five-floor tower
      holds, and a tower that grows to fit them shades its own sails and browns out for good.
      Measured on the golden recorder — every buffer empty, the cutter arm at 0/8, a fixture that
      could not be recorded. The gate is **rope** now, which comes from the middle bands, so it
      still says "your route decides whether you can build this" and now says "you have to have
      walked ordinary ground" rather than "you have to have gone to the ruins". Better sentence,
      reachable tower.

      What is *not* demonstrated is the tier changing a decision in play, which needs somebody
      playing. It is no longer blocked on open question 0, though: that question is answered
      (yield and sun are a balanced pair, tuned to cancel), so the reason a richer route does not
      end up with more of a material is that it is *supposed* to pay for the extra ground in
      charge. The tier's decision is about what the tower can *reach* — rope for an elevator,
      alloy for a charge ceiling — rather than about how much a route hands it, and that is a
      better claim than the one this criterion was written against.
- [x] **Stopping at a ruin is sometimes the wrong call and sometimes the right one.**
      **Met — and what decides it is not the ground, which is a better answer than the one this
      was written expecting.**

      Three towers, same seeds, same rooms, differing only in when they stop. Poles-equivalent
      per 1,000 ticks, scrap valued at the enclave's own published 4-for-3:

      | policy | against never stopping |
      | --- | --- |
      | **reckless** — berths the moment it owns a rig | **−29% to −94%**, loses on 12 of 12 |
      | **careful** — berths only once a battery is up *and loaded* | **+0.5% to +66%**, wins on 12 of 12 |

      `ruin_richness_pct` predicts none of it: the 138% seeds and the 62% seeds behave alike. The
      thing that decides a berth is **whether the tower can answer what the berth wakes**, and
      the reason that is a real decision rather than a formality is the order the prices come in.
      A rig costs 8 of a tower's 10 starting poles; a dart battery costs 6 poles *and 2 rope*,
      and rope needs a ropery, which needs fiber, which needs a comb. **So a tower can afford to
      open a ruin long before it can afford to survive one.**

      What that costs, watched tick by tick on seed 4: berth at tick 0, two wardens up by tick
      1,000, the cutter arm at 92 of 260 by 3,000 and gone by 4,000, and `repelled` still zero
      because the tower never fired a dart. After that it cannot recover — mending costs poles,
      poles come from the mill, the mill eats bamboo, and bamboo needs the arm. The run does not
      end; it continues for an hour as a tower that cannot feed itself.

      So the sentence the criterion makes true is: **stopping is right if you have paid the whole
      entry price, and ruinous if you have paid only the part with a room attached to it.**

      **The one thing left open is telegraphing, and it is a real one.** Nothing tells a player
      that the rig is half a purchase. The empty space in the build menu is the affordance for
      *having* a rig (`salvage_rig.ron`), and there is no equivalent for needing a loaded battery
      before using one — a first-time player buys the obvious thing, stops at the obvious place,
      and is quietly ruined four thousand ticks later. Whether that wants a diegetic signal, a
      cost change, or nothing at all is a judgement about how punishing this game means to be,
      and it wants somebody playing before it is answered.

      *(Superseded: this was recorded as "always the wrong call" on the strength of a policy
      that berthed as soon as it owned a rig — the reckless row above. The measurement was
      right and the conclusion was drawn from one of the two towers.)*

      `examples/journey.rs`'s "is stopping at a ruin ever the right call" runs two towers that
      differ in exactly one behaviour — same seed, same rig, same battery, same thornwright, same
      shopping list, one berths at every ruin it can reach and one never stops — and scores both
      in poles-equivalent per 1,000 ticks, with scrap valued at the enclave's own published
      4-for-3. **Berthing loses on 12 seeds out of 12, by 29% to 94%**, and `ruin_richness_pct`
      does not predict it: the 138% seeds lose as badly as the 62% ones.

      **The mechanism is a death spiral, and it is the actual finding.** A berth wakes wardens,
      wardens go for the rooms, and the room they take is the cutter arm. Measured on seed 4: the
      arm is at **0 of 260 hit points by tick 20,000 and still there 100,000 ticks later**, with
      the tower walking the whole region and harvesting nothing. Mending costs poles; poles come
      from the mill; the mill eats bamboo; bamboo needs the arm. **There is no way out of that
      inside the tower**, and the run does not end — it continues for another hour as a tower
      that cannot feed itself.

      The way out is outside the tower: every board buys scrap 4-for-3, and 51 scrap is 38 poles.
      The harness now sells at the first settlement and it is not enough, because by 30,000 paces
      the arm has been dead for a day. So a player *can* recover, and only if they notice early.

      Three things this could be, and it is an owner's call which:

      1. **Working as intended** — `DECISIONS.md` §11 makes walking a free answer to any wave, so
         a tower that stayed and lost its arm made a choice. The objection is that the punishment
         is unbounded and silent: nothing on screen says "you can no longer recover".
      2. **A defence problem** — one dart battery does not hold a warden off a rig, so the real
         entry price of salvaging is higher than the rig's 8 poles suggests.
      3. **A repair problem** — the tower cannot mend the one room that pays for mending. A
         reserve, a cheaper first repair, or a warden that prefers panels to rooms would each
         break the loop.

      What is *not* in doubt is that the criterion as written is unmet. Whether the answer should
      be "it depends" or "it is a real risk you can be ruined by" is a design decision, and it
      wants a person playing it before anything is changed.
- [ ] **Every constant in `BALANCE.md` graded `PLAYTESTED`**, from run logs rather than from
      scripted harnesses.

      **All 133 rows are now `MEASURED`, and 0 are `PLAYTESTED`, and that gap is deliberate.**

      Every row in the file has been checked against an instrument — eleven of them, run by
      `make instruments`. Seven were written or repaired for this: `charge`, `needs`,
      `haulcycle`, `chain`, `worldrate`, `bestiary`, `prices`. The audit found, in order: a
      lighting figure **70% out**; a meals figure **a third out**; a sails row built on a stride
      cost **six times** the current constant; **four build costs naming the wrong currency**,
      including a cell bank that had moved to a tier-two material and an elevator whose rope was
      missing from its price; **three instruments dead or lying**, one of which hid the fact that
      M2's defence comparison had never once run; and two Makefile targets pointing at examples
      deleted with v1.

      That is real work and it deserved a grade. **It is not playtesting.** This file's own
      legend defines `PLAYTESTED` as *"someone played with it, and with neighbouring values, and
      this one won"* — and nobody has sat with `hungry_ticks` at 4,800 and at 6,000 and preferred
      one. Marking these rows `PLAYTESTED` would have closed this criterion by redefining the
      word, which is exactly what §5.11's fourth open question predicted somebody would be
      tempted to do. `MEASURED` was added instead: *an instrument confirms the effect this
      constant exists to produce, and the limit of that measurement is written into the row.*

      So what remains here is not instrument work. Every number now has a measurement behind it
      and a row that says what the measurement could not see. **What is left is somebody
      playing**, and the questions they would answer are the ones no harness can: is a third of a
      crew member's life at 60% speed too harsh; is 3.4% queueing felt as contention or as
      nothing; is two hours of this enjoyable.

- [x] Golden replay regenerated and verified natively and in wasm; hash-parity runs in CI.
- [x] `make check` and the smoke suite green.

**Deferred out of M1:** per-daypart elevator programs exist in the data model, the command
layer, and the replay format, but have no UI — a player cannot yet change them without
issuing a command by hand. The night-shift program is the reason they exist, so this should
land alongside M4's shift rota if not before.

---

## M2 — The Siege *(the load test)*

**Sprint question:** does combat-as-logistics-stress produce drama without any aimed weapon?

**Scope:** enemies that damage infrastructure, emplacements that are fed by the chain, a
repair loop that competes with everything else for the same crew and the same poles, and a
provocation knob that ties all of it back to how you have been playing.

**Non-goals (M3+):** no feral wardens or ruin salvage, no enclaves, no regions, no meta.

### 2.1 The shape of the thing

Combat is not a mode. There is no phase change, no pause, no separate screen — that split
is precisely what made v1's most interesting moment structurally impossible. A wave is a
**demand spike on the circulation you already have**: darts to the batteries, repair crews
to the breach, on the same stairs the mill is using.

The player's verbs stay infrastructural: where a battery goes decides what it can reach,
whether the chain keeps it fed decides whether it fires at all, and triage — which repair,
if any, is worth committing a crew member to right now — falls out of what has been built
and how full the storeroom is, not a menu. Nothing in M2 adds an aimed weapon, a targeting
priority list, or a tower-level ability to fiddle with, and nothing should.

### 2.2 Enemies

They live on the terrain layer, approach the tower, and attack **infrastructure** rather
than a hit-point bar. Each type teaches one lesson, and a type without a lesson is clutter:

| Type | Lesson |
|---|---|
| skitters | ammo drain economics — cheap, numerous, and they make you count darts |
| canopy leapers | drop onto *upper* decks from overhanging trees, so height is exposure |
| root-borers | gnaw legs and shaft columns, so transport needs redundancy |
| night predators | nocturnal pressure — the reason you banked charge in M1 |

Content, not enum arms, following `ShaftDef`'s precedent: `assets/data/enemies/*.ron`,
interned like everything else.

Contact does not last forever. Once a creature reaches the tower it holds on for
`cling_ticks` — content on `EnemyDef`, tuned per type in `BALANCE.md` — and when that runs
out it lets go and is left behind, rather than being destroyed. The clock only runs while
the tower is actually striding (`GameState.strode`, not the player's `walking` intent — a
tower that cannot afford the charge to move shakes nothing off either), and it runs on
anything that has reached the tower, not only on a creature mid-bite: one that has run out
of things to chew and dropped back to circling is still counting down, so a tower stripped
to its Heartseed doesn't keep a wave orbiting it indefinitely. That makes `SetStriding`
(`command.rs`) a real answer to a wave that costs nothing in poles or darts — keeping the
legs moving is a legitimate way to survive one, and stopping to work mid-assault is a
genuine risk rather than a free action. Cling timers are tuned per creature rather than
uniform: a root-borer's is well short of the time it needs to sever a shaft alone, so one
borer on a walking tower gets a column partway down and loses its grip before finishing,
and it takes two overlapping borers, or a tower that stopped moving, to actually sever one.

A creature leaves the fight one of two ways, and the distinction is a fact the player can
see, not an implementation detail: `Dying` is shot down by an emplacement, `Leaving` is a
creature whose grip ran out. Both fade over `enemy_fade_ticks` rather than disappearing on
the tick they end — long enough at 1x, and still visible at 4x, that an outcome reads as
something that happened rather than something a counter reports after the fact. Only
`Dying` counts toward the `repelled` readout: walking away from something is not the same
as seeing it off, and per the tone guardrail in `DECISIONS.md` §8 the game should never
claim otherwise.

### 2.3 Damage as a state of the tower

Damage attaches to the things the player built, because that is what makes it legible:

* **Panels** — per floor. Breached panels let things inside.
* **Rooms** — a damaged room works slower; a destroyed one is gone, with its contents.
* **Shafts** — a severed shaft column splits the tower's circulation in two. This is the
  signature emergency, and `best_shaft` already routed around what did not span a trip, so
  the reroute falls out of the existing model rather than needing a special case:
  `tests::siege::a_severed_shaft_forces_a_live_reroute`,
  `a_severed_shaft_is_no_longer_a_route`,
  `a_severed_shaft_puts_everyone_on_it_back_on_their_feet`, and
  `a_tower_with_one_shaft_stalls_when_it_is_cut` (the case where there is nowhere to
  reroute to) all hold.

### 2.4 Emplacements

Rooms with a `defence` block: a dart battery on a balcony, a seed-bomb mortar on a deck.
They auto-fire at the nearest live target in range — a battery has no judgement of its own;
the player's judgement went into where they put it — and consume ammo from a **local
rack**, which is just an input stack, so feeding them is the haul system's existing job,
and a battery that runs dry does so for exactly the same reason a mill does.

That equivalence is the whole design. If emplacements get their own special supply
mechanism, combat stops being a load test and becomes a parallel game.

The chain behind that rack has its own arithmetic, and it is not free of tradeoffs. A
thornwright turns poles into darts three to the one. Its own craft timer runs a touch
slower than a mill's, which reads as "a thornwright can't outpace one mill" on paper — but
a mill's *realised* pole output is well short of its nominal rate once haul latency is
counted, so in practice a single thornwright reliably out-consumes a single mill's actual
output. A tower that wants a steady dart supply **and** poles left over for repair needs a
second mill, not a faster thornwright (`docs/BALANCE.md`, thornwright recipe). Shooting
stays the cheap side of that trade regardless: a skitter costs two darts to put down and
does about 48 hit points of damage over a full, unanswered cling — the better part of five
poles to mend (`repair_poles_per_10_hp`) — so putting one down is consistently cheaper than
letting it bite, provided the rack has darts in it at all.

### 2.5 Repair

Repair consumes poles and crew time. It is a chain sink like any other, and it
competes for the same three crew. Triage — letting a floor stay breached because the mill
matters more right now — is the interesting decision, so repair must never be automatic and
never free.

Crew do not chase scratches. A repair job is only started against damage worth at least one
shift (`repair_hp_per_shift`) of hit points, because a shift costs its poles whether it
mends twenty hit points or one — starting one on a mark that has lost four would throw poles
away for almost nothing. Below a shift's worth of damage, the mark simply stays on the
tower: the cross-section is still doing its job as the health readout, it just isn't a job
worth a crew member's time yet.

### 2.6 Provocation

One knob, raised by aggressive harvesting, burner smoke, and (from M3) salvaging ruins.
It feeds the threat table. Tone-safe by construction: the creatures defend their territory
and the tower is the thing passing through — see `DECISIONS.md` §8. Nothing in the UI
should frame this as a kill count.

### 2.7 Loss

The Heartseed is already placed, already unique, already undemolishable. M2 gives it hit
points and makes its destruction the end of the run.

### 2.8 Tick order, current

> **Supersedes §1.6.** M2 inserted `siege` and `defence` between production and haul, and
> `repair` after haul.

1. **clock** — advance the day.
2. **power income** — recompute capacity from the banks; collect from sails and burners.
3. **transport** — cars move. First claim on charge, because a car freezing mid-shaft
   should be the last thing that happens, not the first.
4. **intake** — harvest the band underfoot.
5. **production** — recipes advance, consume, emit. Powered rooms pay here.
6. **siege** — creatures approach and attack; provocation decays. Runs before defence so an
   emplacement fires at where a creature actually is this tick, not where it stood a tick
   ago.
7. **defence** — emplacements fire at what siege just moved.
8. **haul** — crew advance their legs, then idle crew claim work.
9. **repair** — crew already at damage put hit points back. Runs after haul because repair
   competes with hauling for the same crew, and hauling's claims on that crew are settled
   first.
10. **lighting** — lamps, after dark.
11. **stride** — the tower walks if it can still afford to, and terrain streams in.

### 2.9 Exit criteria

- [x] A severed shaft mid-assault forces a live reroute. **Confirmed**:
      `tests::siege::a_severed_shaft_forces_a_live_reroute`,
      `a_severed_shaft_is_no_longer_a_route`,
      `a_severed_shaft_puts_everyone_on_it_back_on_their_feet`, and
      `a_tower_with_one_shaft_stalls_when_it_is_cut` (the case where there is nowhere left
      to reroute to). The second half of this criterion — that the reroute is *legible*,
      that you can see why the crew changed route without opening a debug view — is a
      question you answer by looking, and a passing test cannot answer it. The renderer
      draws a severed column as two pieces sheared past each other with dust still falling
      out of it, but no captured still has yet caught one mid-run. Listed as deferred
      below rather than claimed here.
- [x] A brown-out night assault is survivable with banked charge and lethal without.
      **Confirmed**: `tests::power::a_banked_night_is_survivable_and_an_empty_one_is_not`
      (carried over from M1, still holds).
- [x] Does combat-as-logistics-stress produce drama without an aimed weapon? **Yes**,
      measured by the five-day, three-tower comparison in
      `cargo run -p understory-core --example siege_run`: `subsistence` (one cutter arm,
      never expands) is left alone, ending at full (1000‰) integrity with 220 poles banked
      and nothing left to spend them on; `greedy` (a second cutter arm and nothing else)
      degrades to 743‰, runs out of poles by day 2, and has stopped repairing entirely by
      day 4; `answered` (the second arm, plus a mill, a thornwright, and a dart battery)
      holds full (1000‰) integrity through two days of rising provocation, sees off 51
      creatures, and is still mending on day 5. Defence is a choice that pays for itself;
      expanding without it has a visible, mounting price.
- [x] Golden replay regenerated; hash parity green natively and in wasm. **Confirmed**: the
      fixture now runs 30,000 ticks and exercises the elevator, a thornwright, a
      demolition, a dart battery, and a wave with damage and repair
      (`crates/core/examples/record_golden.rs`).
- [x] `make check` and the smoke suite green — 166 Rust tests, 7 Playwright tests.

**Deferred out of M2:**

- **Most of the creature taxonomy is unexercised in play.** The pack defines four
  creatures, but `min_provocation` gates the canopy leaper at 330, the night prowler at
  200, and the root-borer at 500 (`docs/BALANCE.md`), and the measured five-day run above
  peaks around provocation 250. So the skitter, and barely the night prowler, are the only
  creatures a normal run has actually met; the leaper's "height is exposure" lesson and the
  borer's shaft-severing emergency are reachable only by a much louder or much longer run
  than the one measured here. They are tested in isolation (`tests/siege.rs`) but not
  balanced in situ. M5's full taxonomy pass is where this gets settled.
- **A severed shaft has not been looked at.** The reroute is tested and the renderer draws
  the break, but the screenshot harness (`web/e2e/capture.spec.ts`) has never caught one:
  it takes a root-borer, which is gated at provocation 500, and a captured run peaks around
  250. Until somebody has seen it, the legibility half of the first exit criterion above is
  a claim about code rather than about the game.
- **One emplacement, and no priority targeting.** M2's brief in `v2-plan.md` §9 asked for
  a dart battery *and* a seed-bomb mortar "with priority targeting." Only the battery is
  built, so "which defence to build" is not yet a decision — only "whether" — and
  `defence.rs` picks the nearest creature in range with no way for the player to say
  otherwise. The targeting half is a deliberate cut rather than an oversight: a priority
  list is the kind of menu `DECISIONS.md` §8 argues against, and placement already decides
  what an emplacement can reach. The second emplacement is not a cut, just undone, and
  belongs with M5's taxonomy pass where there is something for it to be good against.
- **Repair costs poles and crew time, not rope.** The brief said "poles+rope+crew". There
  is no rope item in the pack and nothing that would make one anything but a second tax on
  the same haul, so it was not authored — `v2-plan.md` §10's first process rule says no
  content type until a system consumes it.
- **Building under fire is neither slow nor exposed.** The brief asked for construction to
  cost something extra during an assault. It does not: `PlaceRoom` is as instant mid-wave
  as it is in the quiet. The pressure M2 does apply — that poles spent building are poles
  not spent mending — turned out to be sharp enough on its own in the measured runs, so
  this was left rather than stacked on top of an economy that was already too tight. If it
  comes back it should be revisited against the numbers, not added on principle.
- **A creature giving up sounds like a creature dying.** `EnemyState::Leaving` and
  `EnemyState::Dying` are distinct in the state and in the snapshot (§2.2, above), but the
  audio layer has no separate cue yet, so walking a wave off and shooting it down sound the
  same.
- **Per-daypart elevator programs still have no UI**, carried forward unchanged from M1's
  deferred note (§1.7, above).

---

## M3 — The Journey *(the run)*

**Sprint question:** does run pacing work, and does the route-is-your-power-mix tension
actually bite?

**Scope:** the run becomes a journey. Two regions with opposed terrain palettes, route
forks that split the way ahead, drowned ruins worth stopping to strip, the feral wardens
that guard them, one enclave a short way into the second region, and the walk/stop decision
made economically real by moving intake off the clock and onto the ground covered.

**Non-goals (M4+):** no region 3 and no Refugia arrival, no unlocks or meta-progression,
no art pass, no crew needs or shift rota, no per-region creature tables beyond a single
threat multiplier, no enclave repair.

### 3.1 The shape of the thing

A run is a walk through an ordered sequence of **regions**. Nothing about the world model
changes to accommodate that: terrain is still a stream, `World::generate_ahead` and
`prune_behind` still keep a constant-size window around the tower (§0.4), and the tower
still walks down one distance axis from zero. What M3 changes is *which* bands the
generator draws from — it asks the region the tower is in rather than a single pack-wide
weight table — and it hangs three things on that axis worth stopping for: a **fork**, a
**ruin**, and an **enclave**.

Distance is the run's clock, and the tower only advances while it is walking. That makes
stop-or-go the verb that actually spends the run, which is why M3's other half is making
that verb cost something in both directions (§3.6). Everything below is a reason to stop or
a reason not to.

There is no map screen, no node graph, and no travel mode. A region is a stretch of the
same axis with a different palette; a fork is a place on that axis where the world runs out
until you say which way it continues; an enclave is a place you can park next to. The
`v2-plan.md` §3 structural call — "continuous world, no node map" — survives intact, and
none of the three additions needs a second kind of space to live in.

### 3.2 Regions

A **region** is content: `assets/data/regions/*.ron`, interned like everything else
(`DECISIONS.md` §6).

```
RegionDef {
  id: String,                  // "region.deep_jungle"
  name: String,
  order: u8,                   // position in the journey, 0-based
  length_min_paces: i64,       // rolled once per run, from the world stream
  length_max_paces: i64,
  ruin_richness_min_pct: i64,  // likewise; scales what this region's ruins hold
  ruin_richness_max_pct: i64,
  palette: Vec<TerrainWeight>, // { terrain: String, weight: i64 }
  threat_pct: i64,             // multiplier on a wave's threat budget
  fork_interval_paces: i64,    // 0 for a region with no forks
  branches: Vec<BranchDef>,
  enclave: Option<EnclaveDef>, // where in the region, if anywhere, people live
}
```

Regions are traversed in order and sorted by `order` rather than by `id`, so `RegionIdx`
is both the interned index and the position in the journey. That is the second deliberate
exception to the sort-by-string-ID rule in `DECISIONS.md` §6, for the same reason dayparts
are the first: a list whose meaning is a sequence must be stored in that sequence, or the
index lies. Validation requires the `order` values to be a contiguous run from zero, with
no duplicates.

**Two things about a region are rolled from the seed, not authored.** They are the answer to
a real risk in this design: fork spacing, the enclave, and the palettes are all content, and
region length was going to be too — leaving two seeds to differ only in band order and where
the ferns are, which is unlikely to clear the exit criterion that two seeds feel meaningfully
different (§3.10). Rather than give up the predictable fork rhythm to buy variety, the
variety is bought structurally and cheaply:

* **Length**, drawn once per region from `[length_min_paces, length_max_paces]`. Fork
  positions stay on the fixed interval (§3.3), so what the roll changes is **how many
  decisions a region contains** — a long draw fits another fork, a short one does not —
  without forks becoming unpredictable *within* a run. The min/max pair mirrors
  `band_min_paces`/`band_max_paces`, which is how the pack already expresses "a length with
  a range."
* **Ruin richness**, a percentage drawn once per region and applied to what each of its
  ruins holds (§3.4). This is the roll the player will actually feel, because it changes
  whether stopping is worth it: one run's drowned city is picked over and grudging, the
  next one's is worth berthing at three times. It scales the amount rather than the count,
  so a poor city still has ruins in it — visibly near-empty ones, which reads as
  disappointment rather than as absence.

Both draw from `world`, never `cosmetic`: how long a region is and how much is left in it
are facts about the run, and two people sharing a seed must get the same journey
(`DECISIONS.md` §2).

**Every roll happens once, at run start.** `World` carries the result:

```
World {
  ...
  journey: Vec<RegionRoll>,    // { end: Paces, ruin_richness_pct: i64 }, one per region
  region: RegionIdx,
  region_start: Paces,
}
```

Rolling the whole journey up front rather than region by region on entry is what keeps the
generator simple. Because a branch is a palette override rather than a detour (§3.3), region
boundaries are still fixed absolute distances for the life of a run — they are just fixed by
the seed rather than by the content — so "which region is the tower in" and "which region is
the generator producing into" both stay pure functions of a distance. Rolling on entry would
mean the generator, which streams up to `stream_ahead_paces` past a boundary, had to produce
terrain for a region whose length had not been decided yet.

`region` and `region_start` are derivable from `distance` and `journey`, and are stored
anyway for one specific reason: crossing a boundary is an **event**, not just a fact. The
stride system compares the stored region against the region the tower's new distance falls
in, and on a mismatch it does the region's one-time setup — placing the enclave, resetting
the fork schedule. `region_start` keeps the fork arithmetic local and readable rather than a
running sum recomputed at each use.

**The two regions.**

| | Region 1 — Deep Jungle | Region 2 — The Drowned City |
|---|---|---|
| Character | canopy-heavy: biomass-rich, sun-poor | ruin-heavy: sun-rich, biomass-poor |
| Palette | canopy heavy, clearing moderate, ruin-field light | drowned street and ruin-field heavy, clearing light, canopy light |
| Ruins | scarce | common, and how much they hold is a seeded roll |
| `threat_pct` | 100 | higher |
| Enclave | none | one, a short way in |

The opposed sun and biomass rates `v2-plan.md` §9 asks M3 for **already exist**, per
terrain, from M0: `TerrainDef` carries `yield_pct` against `sun_pct`, and
`assets/data/terrain/canopy.ron` is 140 against 35 where `ruin_field.ron` is 50 against
130 — no band is allowed to be good at both, and
the field comment says so. What M3 adds is that opposition at *region* scale. In M0 and M1
the opposition was a texture: a band lasts 300–900 paces, roughly sixteen to fifty seconds
at 1×, so the tower crosses the whole spectrum several times an in-game day and the mix
averages out. A region lasts long enough that it does not average out. Walking into the
drowned city means the mill goes hungry and the banks fill for hours, not for a minute —
the journey itself changes the tower's power mix, and the player has to rebuild around it
rather than wait it out. That is the difference between "your route is your power mix" as a
sentence and as a decision.

Two consequences the palette has to respect:

* **A palette needs at least three kinds with positive weight.** `pick_band_kind` never
  repeats the previous band's kind. With a two-entry palette that rule degenerates into
  strict alternation — a perfectly regular ABABAB horizon, which reads as a bug. Validation
  rejects a palette with fewer than three positive weights, in the same spirit as the
  anti-frustration constraints already living invisibly inside the generator.
* **The drowned city needs a band of its own.** Re-weighting the three M0 kinds would make
  region 2 read as "region 1 with more ruins" rather than as somewhere else. M3 authors one
  new terrain, `terrain.drowned_street` — high sun, low yield, ruin-bearing — so the city
  has a face. Reusing `clearing` and `canopy` at low weight is what keeps the green
  reclaiming the concrete visible.

`TerrainDef.weight` is deleted. Its job — how often a kind comes up — now belongs to the
region palette, and leaving a pack-wide weight in place as a second knob doing the same job
would be one of the two numbers going stale. Validation gains the matching check: every
terrain kind must appear with positive weight in at least one region palette, because a
terrain no region can produce is content with no consumer (`DECISIONS.md` §9.1).

`threat_pct` multiplies the provocation-scaled threat budget in `siege::maybe_spawn_wave`,
after the affordability gate and before `base_threat`'s floor. One knob, not a table: M2
already deferred most of the creature taxonomy as unexercised in play (§2.9), and adding
per-region creature tables on top of creatures a run has never met would be authoring
content for a system that is not yet consuming what it has. Per-region threat tables belong
with M5's taxonomy pass.

### 3.3 Route forks

At `fork_interval_paces` intervals inside a region, the route splits in two.

Fork **spacing** is content, not seed: the first fork sits at `region_start +
fork_interval_paces`, the next one an interval further on, and so forth. Predictable
punctuation is a feature — the player should learn the rhythm of "another choice is coming"
without having to watch for it, and a fork that could arrive at any moment would be an
interruption rather than a beat.

A fork is skipped if it falls within `fork_edge_margin_paces` of either end of the region or
of the region's enclave, so a decision never lands on top of a boundary or a berth and
competes with it for the same stretch of horizon.

What the seed decides is therefore **how many** forks a region has and **what each one
offers**. Fork count falls out of the region's rolled length against the fixed interval
(§3.2): a long draw fits one more decision in than a short one. The two branch archetypes at
each fork are drawn from the `world` stream. Spacing stays regular; the number of beats and
the content of each one do not.

Each region authors a set of **branch archetypes**:

```
BranchDef {
  id: String,                  // "branch.canopy_passage"
  name: String,
  length_paces: i64,
  palette: Vec<TerrainWeight>,
  threat_pct: i64,
}
```

The two a fork draws are always distinct, and the choice between them is the choice. Taking
one overrides the region palette for
`length_paces` past the fork and multiplies the region's `threat_pct` by the branch's, then
the route rejoins the region. **A branch is a palette override, not a detour.** There is no
second distance axis, no route tree in state, and no rejoin arithmetic: the tower keeps
walking down the one axis it has always walked down, and for a stretch the terrain it walks
through is drawn from somewhere else. Everything downstream — streaming, pruning, region
boundaries, replay — is unchanged by construction.

**What the player is told before committing.** The fork card names each branch and lists
its two heaviest terrain kinds and a word for its threat (*quieter* / *as usual* /
*louder*), all derived from the branch's own palette and `threat_pct`. There is deliberately
no authored blurb: a hand-written line describing a branch can drift out of step with its
palette during tuning, and a game that misdescribes the only informed choice it asks the
player to make is worse than one that describes it drily. The description is generated from
the data it describes, so it cannot lie.

**The tower halts at a fork it has not been given an answer for.** New command:

| Command | Effect | Rejects on |
|---|---|---|
| `TakeFork { branch }` | commit to branch 0 or 1 of the pending fork | no fork pending, no such branch |

Three things make the halt right rather than arbitrary.

*It is what the world does, not a rule imposed on top of it.* The generator produces terrain
`stream_ahead_paces` in front of the tower, and past an unanswered fork there is nothing to
produce — the palette beyond depends on an answer that does not exist. So generation stops
at the fork line. The tower halts because the ground it would walk onto has not been decided
yet, and the renderer draws exactly that: the terrain strip ends, and the fork is the
horizon. The property test `the_tower_is_always_standing_somewhere` continues to hold,
because the tower never crosses into the ungenerated stretch.

*It costs the player the thing M3 has just made expensive.* The halt reuses the stop
machinery wholesale (§3.6): `strode` stays false, so nothing clinging to the tower loses its
grip (`DECISIONS.md` §11), the cutter arms harvest nothing, and the charge the legs would
have burned is banked. A fork reached in the middle of a wave is a genuine emergency, and a
fork reached with the banks nearly empty is a small mercy. That is the same trade every
other stop in the game presents, which is exactly why it needs no new machinery and no new
UI mode.

*It needs no modal dialogue and no pause.* The fork is visible from up to
`stream_ahead_paces` out — roughly fifty seconds at 1× — and `TakeFork` is legal from the
moment it appears. A player who answers early never stops at all. The halt is not the
decision; it is what happens when the decision is late. That distinction is what keeps the
fork from being a speed bump, and it is the reason to reject the obvious alternatives: a
modal pause would make the choice a mode (against `v2-plan.md` §3), and auto-picking a
branch would make it not a choice.

An answer may be replaced while the fork is still pending — the last `TakeFork` before the
tower reaches the fork line is the one that counts. Once the tower crosses, the branch is
committed and the fork is cleared. A player who never answers stands there indefinitely,
banking charge while the waves keep arriving on their own schedule; that is a legitimate,
bad outcome and needs no special handling.

**The halted states have to be visually distinct, and this is a renderer requirement rather
than a simulation one.** A tower the player stopped, a tower waiting at a fork, and a tower
that has reached the end of the world are the same silhouette with the same legs still and
the same `strode` false, and they are three completely different situations. The fork halt
in particular must read as *waiting for you* — the terrain strip ending at a fork in the
path, with two ways named — or it looks like the game has frozen, which is the one reading
that would make the whole argument above worthless. The arrival halt needs its own reading
too, since standing still at the far edge is the run being over rather than a decision
pending.

**Anything that drives the engine without a player has to answer forks.** `examples/siege_run.rs`
and `examples/throughput.rs` both step tens of thousands of ticks with no commands beyond a
shopping list, and a harness that walks into a fork and stops measures a parked tower with
total confidence — the exact failure mode `siege_run.rs`'s own comments record it having had
once already, when a room it thought it had built had in fact been refused. Both harnesses,
the golden-replay recorder, and the Playwright smoke test need a standing fork answer, and
that is a change to make in the same commit as the halt, not after the numbers come out
wrong.

```
World {
  ...
  fork: Option<PendingFork>,   // { at, branches: [BranchIdx; 2] }
  branch: Option<ActiveBranch>,// { def, from, to }
}
```

### 3.4 Berthing: ruins, the salvage rig, feral wardens

Some `Feature`s are **berthing sites** — drowned ruins with something left in them. The
world module has been ready for this since M0, and says so:

> Drawn from the **world** stream rather than the cosmetic one, because M3 turns ruins into
> berthing sites — where a ruin stands has to be a fact about the run, not about the frame.
> — `crates/core/src/state/world.rs`

That comment is the whole justification for the field's existence on the economically live
stream, and M3 is what cashes it. A ruin's position, and how much it holds, are facts about
the seed; two people sharing a seed pass the same ruins.

```
Feature {
  at, kind, scale, layer,
  salvage: i64,   // whole units of scrap left; 0 for ordinary scenery
  roused: bool,   // this ruin has already woken its wardens
}
```

`TerrainDef` names which of its `feature_kinds` are ruins and the range each holds, so the
canopy's ferns and the ruin-field's broken frames scatter through the same generator and
only the latter come out salvageable. Both the choice of which features are ruins and the
amount each holds are drawn from the `world` stream at scatter time, in
`World::scatter_features`, alongside position and scale — and the amount is then scaled by
the **ruin richness** rolled for the region the band belongs to (§3.2). A picked-over
drowned city and a generous one are the same ruins in the same places holding different
amounts, which is why the renderer draws `salvage` rather than just drawing a ruin: the
player can see from the strip which stops are worth making, and a lean region reads as
disappointment rather than as an empty map.

**The salvage rig** is a new intake room, `room.salvage_rig`, ground floors only. It works
exactly like a cutter arm except for where it draws from, and that difference is expressed
in the content rather than in a special case:

```
enum IntakeSource {
  Terrain { paces_per_item: i64 },
  Ruin    { ticks_per_item: u32, range_paces: i64 },
}
```

A `Terrain` source accrues per pace walked (§3.6). A `Ruin` source accrues per tick, and
only while the tower is stopped with a ruin inside `range_paces`. **Walking harvests
bamboo; stopping harvests scrap.** The two intakes are exact opposites, and the stop/go
decision is therefore also an intake-mix decision — which is the cleanest possible statement
of what M3 is for. A rig with a full outbox stalls in place like every other intake room
(§0.7), and the ruin keeps whatever it has not given up.

**Berthing is implicit.** There is no `Berth` command. A tower that is stopped with a
working rig in range of a ruin is berthing; a tower that walks on is not. Range is a
property of the rig (`range_paces`), the way a dart battery's reach is a property of the
battery — a longer-reaching rig is a thing M5 can author, and putting the number on the room
means the player reads it where they choose to build it. Stopping next to a ruin with no
salvage rig does nothing at all: nothing is extracted, nothing is roused, and there is no
error, because there is no command to reject. The empty space in the build menu is the
affordance.

**Feral wardens** are what the ruin has instead of a lock. The first tick a rig extracts
from a ruin rouses it: one wave, sized against how much the ruin held at that moment
(`warden_threat_per_100_salvage`, floored at a single warden), spawning out of the ruin
itself rather than at the usual `spawn_paces_ahead`, offset by `warden_wake_paces` so there
is a few seconds of warning between the ground moving and the first bite. `Feature.roused`
means a ruin wakes once and only once, so leaving and coming back is not an exploit.

Wardens are content, `assets/data/enemies/feral_warden.ron`, with one new field on
`EnemyDef`:

| Field | Value | Why |
|---|---|---|
| `wave_eligible` | `false` | Ordinary waves draw from every creature whose `min_provocation` the tower has passed. A warden is not summoned by attention; it is summoned by berthing, so it has to be excluded from that pool explicitly rather than fenced off with an out-of-range `min_provocation`. |

**This is the counterweight to the cling rule, and it is the best thing in the milestone.**
`DECISIONS.md` §11 makes a creature's grip count down only while the tower is actually
striding: walking shakes things off, so "keep the legs moving" is a real, free answer to a
wave, and stopping to work mid-assault is a real risk. Berthing is the one time the tower
*cannot* walk away — not because a new rule forbids it, but because walking away is what
ends the salvage. So the one place the game puts something worth stopping for is the one
place the existing escape hatch is closed, and it closes itself. No new mechanic produces
this; §11 already did, and M3 simply builds the room that makes it matter. A warden's grip
never runs out while you keep working, and the moment you decide the scrap is not worth it
you start walking and it does.

Salvaging also raises provocation, closing the forward reference M2 already recorded in
§2.6 ("aggressive harvesting, burner smoke, and (from M3) salvaging ruins"). The rate should
sit close to `provocation_per_100_harvested` rather than far above it: the wardens are the
price of a ruin, and charging a second, much louder price in provocation on top would make
salvage a thing nobody does twice.

The arithmetic the balance pass has to settle, stated here so whoever runs it knows what
the equation is: a ruin's scrap is worth some number of poles at the enclave (§3.5), the
wardens it rouses cost some number of darts to see off and some number of poles to mend
what they chew through, and salvage is only a decision if those two numbers are close
enough that the answer depends on the tower. Both sides — ruin size and the enclave's
exchange rate — move together and must be tuned together.

### 3.5 The enclave

One, standing a short way **into** region 2 rather than on the boundary. It is a settlement
the tower walks past: berthing works exactly as it does at a ruin — stop within range — and
the tower that keeps walking loses it, because there is no going back down the axis.

```
EnclaveDef {
  id: String,
  name: String,
  at_paces: i64,           // offset from the start of the owning region
  offers: Vec<OfferDef>,   // { give: (item, amount), take: (item, amount), stock: i64 }
  recruits: u8,
  recruit_cost: Vec<CostEntryDef>,
}
```

**Where it stands is a considered departure from a locked plan, so it is recorded here
rather than buried.** `v2-plan.md` §6.6 says "enclaves between regions," and the obvious
reading of that is the boundary. Spec'd that way, the enclave puts scrap's only consumer
*upstream* of the only region that produces much scrap — the drowned city — so everything
salvaged past it is dead weight, and the milestone's headline mechanic pays out only for the
handful of ruins region 1 scatters. Worse, it compounds: region 2's palette already halves
bamboo yield, per-pace intake (§3.6) means every minute spent berthing costs bamboo the
tower did not walk past, and `threat_pct` is higher — a tower that engages with salvage
arrives in the hard region poorer, louder, and with no restock ahead of it. That is a design
that punishes the player for playing the thing M3 is about.

A short way in fixes all of it and buys a pacing beat the boundary version did not have: you
cross into the hard region, work its edge, find out what its ruins are worth this run, and
*then* find people to trade with. `at_paces` is small relative to the region — far enough in
that arriving with something to trade is the normal case, near enough that the enclave still
provisions the bulk of the region rather than arriving after it matters. The spirit of
"between regions" survives; the letter does not.

Two things happen here.

**Trade** exchanges items at posted rates. Each offer has finite `stock`, so the enclave is
a windfall rather than an exchange to farm, and the rates are deliberately worse than the
chain's own: the enclave is where a tower that lacks a room buys its way around the gap
once, not a substitute for building the room. The offers that earn their place are scrap for
poles (the payoff for everything in §3.4, and the reason scrap exists at all in M3), surplus
bamboo for poles at a rate a mill beats comfortably, and poles for darts at a rate a
thornwright beats comfortably — so a tower heading deeper into the city without a
thornwright can still arm itself, and pays for the privilege.

**Recruit** adds one crew member for poles, up to a new `crew_cap` (a `CrewBalance` field;
M3's target is a little above the starting three, with the plan's cap of around eight left
for M4's shift rota to earn). Priced steeply, because `starting_crew` was the first constant
in the game to be graded `PLAYTESTED` and what it measured was that going from two crew to
three moved throughput by ninety percent (§1.7). A fourth pair of hands is the largest single
change a player can buy, and it should cost like it. The enclave offers exactly one. The new crew member takes the next name from the
placeholder list in `state.rs` by index — not by a roll — so recruiting perturbs no stream;
their `fidget` is drawn from `cosmetic`, as every crew member's is (`DECISIONS.md` §2).

| Command | Effect | Rejects on |
|---|---|---|
| `Trade { offer }` | take one offer, once | not berthed at an enclave, no such offer, offer exhausted, insufficient stock |
| `Recruit` | one crew member for poles | not berthed at an enclave, no recruits left, crew at cap, insufficient stock |

Both spend from and deliver to storeroom shelves through the existing `check_stock` /
`spend` path in `engine/commands.rs` — the chain pays for the enclave the same way it pays
for the tower. `v2-plan.md` §6.2 leaves room for trade tokens at enclaves; M3 declines to
introduce one. A currency that exists in exactly one place is a second economy with a single
customer, and item-for-item exchange keeps the shelves the only thing worth filling. If M5's
enclave economy wants a token, it can add one against several enclaves that use it.

The berth range for an enclave is a `JourneyBalance` field rather than a property of a room,
since docking at a settlement is not the salvage rig's job. That new balance section is where
`fork_edge_margin_paces` (§3.3) lives too — the handful of journey-wide constants that belong
to no single room or creature.

**What is unavoidably UI, honestly.** The berth is diegetic: the enclave is drawn on the
terrain strip, the tower parks beside it, and walking on ends it — no screen is entered and
nothing is paused. The transaction is not. Taking an offer is a button on a posted board,
and the goods appear on the shelves without a crew member carrying them. The genuinely
diegetic version — the enclave as a dock the crew haul to and from, with each accepted offer
becoming a haul job — was considered and cut for M3: it needs a `HaulDestination` outside the
tower and a leg of the crew state machine that walks off the edge of the cross-section, which
is a change to the system M1 was a bet on, spent on a single waystation. If it is worth
doing, M5's enclave economy is where there is enough enclave to justify it.

The tone guardrail applies to the board's copy as much as to anything else (`DECISIONS.md`
§8): the enclave is people who live here and the tower is passing through, so the offers
read as an exchange between neighbours, not as a merchant's inventory.

**Shell work, and what scrap is for.** The settlement will plate the tower's hull for
scrap: `ReinforceDef` on the enclave, a cost and a `panel_hp` figure and a number of times
they will do it. It is the only permanent upgrade in the game.

It exists because salvage had nowhere to go. Scrap's only other consumer is the trade board,
whose offers are finite and behind you the moment you walk on, so a run that berthed at the
drowned city's ruins banked metal it could not spend — recorded as a deferred hole when the
enclave moved into region 2, and closed here. Plating turns a city's worth of old metal into
hull, which is what it ought to become.

Three properties that matter more than the numbers:

* **It applies to floors that do not exist yet.** The bonus lives on `Tower.shell_bonus`, not
  on each floor, so a storey built afterwards arrives already plated. Otherwise growing
  taller would mean growing a soft spot, and the player would have to remember which floors
  had been done.
* **New material arrives as material.** `max` and `hp` both go up, so plating does not leave
  an undamaged tower reading as freshly damaged.
* **It is not a repair.** A panel already breached comes back plated and still breached.
  Shell work cannot be used to skip the repair loop (§2.5) it is meant to make survivable —
  which is also why enclave *repair*, listed in `v2-plan.md` §6.6, stays cut.

### 3.6 Walk and stop

`v2-plan.md` §9 calls this "walk/stop throttle economics." The interpretation taken,
recorded here because it is a deliberate narrowing of what §6.2 of that plan and
`BALANCE.md`'s `stride_paces_per_100_ticks` row both anticipated:

**`SetStriding` stays binary.** Walk or stop. No speed notches, no continuous dial. The plan
describes striding as "a player-set throttle," and a throttle implies a range — but a range
would be a new decision layered on top of an old one that does not yet cost anything, and
M3's job is to make the existing decision economically real, not to add a knob. Five speeds
over a choice that is nearly free is a worse game than two speeds over a choice where each
option gives up something the other has. If, after M3, walk-or-stop turns out to be *too*
coarse in play, notches are a small addition to a system that already prices motion; adding
them first would have priced nothing.

Four opposed forces bear on that one binary choice:

| Walking | Stopping |
|---|---|
| spends charge (`stride_charge_per_100_ticks`) | banks it |
| covers ground — distance is the run's clock | holds a berth: salvage, trade, recruit |
| harvests, because the cutter arms strip what they pass | harvests nothing at all |
| shakes off anything clinging (`DECISIONS.md` §11) | lets everything attached keep its grip |

Three of those four are already true. The fourth is the change that makes the decision bite.

**Intake becomes per-pace rather than per-tick.** `IntakeDef` for a `Terrain` source is
authored as `paces_per_item` instead of `ticks_per_item`, and `intake::run` accrues against
the ground actually covered rather than against the clock. A stopped tower harvests nothing.

This is not a new claim about the fiction; it is the fiction finally being true. The cutter
arm's own description already says so: *"Strips bamboo from the terrain as the tower walks
past it"* (`assets/data/rooms/cutter_arm.ron`). Per-tick intake meant a parked tower stripped
bamboo out of ground it had already stripped, indefinitely, which is the sort of thing that
is invisible until stopping becomes a thing players do on purpose — and M3 gives them three
reasons to.

**The shipped value is 78 paces, not the 54 a one-for-one conversion gives.** At
`stride_paces_per_100_ticks` of 60, the cutter arm's authored `ticks_per_item` of 90 is
`paces_per_item` of 54 — and the arm had never run at 90 ticks. Intake accrued a truncated
per-tick *fraction of an item*: `Fx::ratio(1, 90)` is `Fx(2)` in Q8.8, so the arm ran at 128
ticks, and every M2 measurement was taken against that figure rather than against the
content. Worse, scaling a fraction that small was very nearly a no-op — `Fx(2) * 1.40` is
also `Fx(2)` — so dense canopy and open clearing harvested at *identical* rates, and ruin
field and drowned street collapsed together too. Four authored terrain kinds behaved as two,
and the flagship contrast of the route being the power mix (`DESIGN.md` pillar 1) was not in
the simulation at all. The region palettes in §3.2 are built on exactly those differences,
so per-pace intake could not be shipped on top of it.

Rooms now accumulate *effort* against a threshold rather than a fraction of an item against
one, which keeps the precision where it is needed; `intake::terrain_effort`'s doc comment is
the authoritative account. 78 paces is 130 ticks at the shipped stride — a shade slower than
the mill's 120 ticks a pole, which is what the M2 economy was actually measured against.
Shipping 54 alongside the fix would have made the arm 33% faster than one mill can consume,
and because a shelf holds one kind, bamboo would claim every shelf, poles would have nowhere
to go, and the chain would deadlock with the storeroom three-quarters full — recoverable
only out of poles stuck inside the mill, which construction cannot draw from. See
`docs/BALANCE.md`'s `storeroom` row for that failure written up in full; it is reachable
today with a second arm and no second mill, and it will read as a bug to a player who does
not know why.

**This is the riskiest change in M3.** It revalues every constant the M2 balance run settled,
and it does so indirectly, through a chain that is long enough to be hard to reason about
from the desk: stopping cuts bamboo, which cuts poles, which cuts both repair and darts,
which moves the whole siege curve — and because provocation is driven by
`provocation_per_100_harvested`, a tower that stops is also quieter, so the difficulty dial
moves in the opposite direction at the same time. There is one genuinely new failure mode to
watch for: a tower too poor in charge to walk (`power::pay_for_stride` already stops it) now
also cannot harvest, so it cannot make poles, so it cannot build its way out — a brown-out
becomes a spiral rather than a bad night.

That spiral has a floor, and it is worth knowing where it is before treating this as a
reason not to make the change. `collect_solar` does not read `walking` or `strode`: the
sails fill whether or not the legs are running, so a tower stranded overnight is walking
again by mid-morning under its own power, having lost a night's harvest rather than the
run. The genuinely unrecoverable case is narrower — a night entered with no banked charge,
no bamboo to burn, and the tower already stopped — and it is reachable only after the
player has already spent everything twice over. Design that as a bad night with a hard
morning, not as a trap; if the re-measurement shows it landing more often than that, the
lever is `starting_charge` or the burner's fuel cost, not reverting per-pace intake.

**What was promised here, and what is true instead.** This passage originally said twice
that the always-walking case would be *arithmetically identical* to M2's, because 54 paces
is 90 ticks and 90 ticks was what the arm was authored at. That rested on a false premise:
the author did not know the rate was being truncated, and an identical *authored* number
would have been a 42% faster *actual* arm. What holds instead is the honest version of the
same guarantee — the always-walking case reproduces the rate M2 was **measured** at, because
the arm has been re-authored to state that rate out loud (78 paces, 130 ticks, against the
128 it was really running). The conversion is a measurement, not an arithmetic identity, and
that is a weaker claim, so the second bound matters more than it did: the terrain palettes in
§3.2 move the numbers on their own, and — now that `yield_pct` is doing anything at all —
they move them further than anyone had reason to expect.

Both bounds were re-measured together against
`cargo run --release -p understory-core --example siege_run`, and that harness is the reason
`provocation_per_100_harvested` came down from 300 to 240: with the yield multiplier finally
live and region 1's palette at 55% canopy, a subsistence tower gained about 13% a day and
started provoking on its own. `docs/BALANCE.md`'s five Siege rows —
`wave_interval_ticks`, `base_threat`, `provocation_per_100_harvested`,
`provocation_decay_per_100_ticks`, and `repair_poles_per_10_hp` — carry the re-measured
figures and stay `PLAYTESTED`; they were graded against this harness, and this harness has
been re-run. The shape it now reports is sharper than M2's rather than merely preserved:
`subsistence` ends whole with 180 poles banked, `greedy` is pole-broke from day 3 and slides
to 860‰, and `answered` holds 992‰, sees off 63 creatures **and** still banks 83 poles — so
defence visibly pays for itself instead of just slowing a decline.

### 3.7 The run

A run is a seed. It starts in region 1 at distance zero, with the tower, crew, and stock
`GameState::new` already builds, and it ends exactly two ways:

* **The Heartseed is lost.** M2's loss condition, unchanged (§2.7).
* **The far edge of region 2 is reached.** The generator has nothing past the last region,
  so the tower halts there the same way it halts at an unanswered fork — the world has run
  out — and `arrived` is set.

Arrival is presented as an arrival, not a victory: the same register as the elegy the
frontend already shows when the Heartseed goes, reporting where the tower got to rather than
grading it (`DECISIONS.md` §8).

**M3's finish line is a placeholder.** `v2-plan.md` §6.6 puts three regions between the
start and the Refugia; M5 adds the third and the actual arrival. Region 2 in M3 is
deliberately shorter than region 1 — it exists to prove the region machinery works with more
than one region in it and to give the drowned city's opposed palette somewhere to bite, not
to be a second full act.

The snapshot grows one group, `journey`, carrying the current region and how far through it
the tower is, any pending fork and its two branches, any active branch, whatever the tower
is berthed at, the enclave's remaining offers, and `arrived`. `FeatureView` grows `salvage`
so the renderer can draw a ruin that still has something in it differently from one that has
been stripped — a change to the bridge's public contract, and therefore a frontend change
made deliberately rather than discovered (`AGENTS.md` §IV).

**Opening values.** Collected here so whoever authors the content has one list rather than
nine paragraphs to mine. Every figure below is a **design target** — a first value with an
argument behind it, not a measurement. None of them is a `BALANCE.md` row until it has been
authored, and each gets a graded row in the same commit that authors it (`DECISIONS.md` §7).
All the arithmetic assumes the tower's current 0.6 paces per tick and 30 Hz.

| Thing | Target | The arithmetic |
|---|---|---|
| region 1 length | 52,000–68,000 | About 55 minutes of unbroken 1× walking at the midpoint (60,000 paces is 100,000 ticks, roughly seven in-game days), sized so a session that stops for a berth and a fork or two lands in the 45–60 minute window the exit criterion asks for. The ±8,000 spread is a little over half a fork interval, so most seeds differ by one fork and none by more than one. |
| region 2 length | 34,000–46,000 | Shorter on purpose: a placeholder act, not a second full one. |
| region ruin richness | 60–140% | Wide enough that a lean drowned city and a generous one are different propositions rather than different rounding. |
| region 1 palette | canopy 55 / clearing 30 / ruin-field 15 | Canopy-heavy, so the region reads as biomass-rich and sun-poor, with ruins scarce enough that a berth is an event. |
| region 2 palette | drowned street 40 / ruin-field 35 / clearing 15 / canopy 10 | Sun-rich and barren, with the jungle still visibly taking it back. Four kinds clears the three-kind minimum comfortably. |
| `terrain.drowned_street` | `yield_pct` 60, `sun_pct` 120 | Between clearing and ruin-field on both axes, so the city is poor rather than dead. |
| `threat_pct` | 100 / 150 | Region 2 is half again as dangerous for the same provocation. |
| `fork_interval_paces` | 15,000 | Three forks in region 1, about fourteen minutes apart at 1×. Punctuation, not a metronome. |
| `fork_edge_margin_paces` | 5,000 | Keeps the last fork clear of the enclave, so the two never compete for the same stretch of horizon. |
| branch `length_paces` | 6,000 | Roughly five and a half minutes at 1× — long enough to change what the chain is doing, short enough that a bad pick is not a lost region. |
| branch `threat_pct` | 80–130 | The quieter branch is meaningfully quieter; the louder one is not punishing on its own. |
| ruin `salvage` | 25–60 units, before richness | At the rig's rate, 50 seconds to two minutes of berthing at 100% richness. A commitment, not a top-up. |
| enclave `at_paces` | 8,000 into region 2 | About seven minutes of 1× walking past the boundary — one or two of the city's ruins, so the normal case is arriving with something to trade, while the remaining four-fifths of the region is still ahead of you to be provisioned for. |
| salvage rig | `Ruin { ticks_per_item: 60, range_paces: 60 }`, `buffer_max` 8, 8 poles, ground floors only | Two seconds a unit. `range_paces` 60 matches the dart battery's reach, which is the number the player already has a feel for. Priced above a mill and below a dumbwaiter. |
| cutter arm | `Terrain { paces_per_item: 78 }` | **Authored at 78, not the 54 this table first targeted.** 54 is the authored `ticks_per_item` of 90 converted at 0.6 paces/tick, but truncated intake had the arm running at 128 ticks, so 78 paces (130 ticks) is the rate the M2 economy was actually measured against. See §3.6. |
| feral warden | hp 300, damage 12, `attack_ticks` 60, speed 20, `cling_ticks` 2,400, `threat` 30, Ground, `wave_eligible: false` | A dart battery needs 800 ticks and 20 darts to put one down (300 hp against 15 damage every 40 ticks), and takes about 160 damage doing it — roughly 16 poles to mend. Toughest thing in the pack, slowest approach, and the longest grip: 80 seconds of walking to shake one that you have decided to leave. |
| `warden_threat_per_100_salvage` | 120 | A 25-unit ruin rouses one warden, a 50-unit ruin rouses two. The payout and the price are the same number, which is the whole point. |
| `warden_wake_paces` | 120 | About twenty seconds between the ground moving and the first bite, at a warden's own pace — the same order of warning `spawn_paces_ahead` gives an ordinary wave. |
| `provocation_per_100_salvaged` | 300 | The same three points per unit that cutting costs. The wardens are the price of a ruin; charging a much louder second price would make salvage a thing nobody does twice. |
| `enclave_berth_paces` | 80 | About a five-second window at 1×, under two at 4×. If that reads as fiddly in play, the answer is a larger number, not an approach-and-dock mechanic. |
| `crew_cap` | 6 | Twice the starting three, and short of the plan's eventual eight, which M4's shift rota should have to earn. |
| enclave offers | `4 scrap → 3 poles` ×20 · `10 bamboo → 4 poles` ×10 · `4 poles → 6 darts` ×12 | Every rate is worse than the chain's own: the mill turns bamboo into poles one for one, and the thornwright turns a pole into three darts. The enclave is where a tower without the room buys around the gap once. |
| enclave recruit | 30 poles, one only | A 40-unit ruin is 30 poles at the scrap rate — so one good berth on the city's edge is one pair of hands, which is the trade the walk into region 2 is arranged around. |

### 3.8 Tick order, current

> **Unchanged from §2.8.** M3 adds no system. Two existing systems do more, and one new
> value crosses between them.

1. **clock** — advance the day.
2. **power income** — recompute capacity from the banks; collect from sails and burners.
3. **transport** — cars move.
4. **intake** — harvest. Terrain-sourced rooms accrue against the ground covered by *last*
   tick's stride; ruin-sourced rooms accrue per tick while berthed, and the first extraction
   from a ruin rouses its wardens.
5. **production** — recipes advance, consume, emit. Powered rooms pay here.
6. **siege** — creatures approach and attack; provocation decays; wardens roused in step 4
   spawn here, through the ordinary spawn path.
7. **defence** — emplacements fire at what siege just moved.
8. **haul** — crew advance their legs, then idle crew claim work.
9. **repair** — crew already at damage put hit points back.
10. **lighting** — lamps, after dark.
11. **stride** — region crossings, the fork halt, the arrival halt, the distance advance,
    and terrain streaming.

**Why the new work lands where it does.** Region tracking, fork resolution, and arrival all
gate or follow the distance advance, and `stride` already owns distance, `generate_ahead`,
and `prune_behind` — so they extend a system in place rather than needing one of their own.
The three halt conditions (the player stopped, a fork is unanswered, the run is over)
collapse into a single predicate that `power::pay_for_stride` reads, so a halted tower pays
no charge for standing still whichever reason it is standing still for, and `strode` stays
false in every case. Salvage is an intake room, so it belongs in `intake`, which already
runs before `production` (nothing can consume scrap before it lands) and already reaches
across into siege to charge provocation for harvesting — the warden rousing follows exactly
that existing call shape, and because `intake` runs before `siege`, a roused warden spawns
on the same tick.

**The one ordering hazard.** Intake now depends on how far the tower moved, and stride runs
*last* — its position is not negotiable, because charge priority is tick order and walking
is the first thing a tower short of power gives up (§1.6). So `intake` at tick *T* reads the
motion of tick *T−1*. `GameState` gains `paces_last`, written by stride, read by intake. The
one-tick lag is deterministic and invisible at 30 Hz, and it is not a new pattern: `strode`
is already exactly this — "did the legs run last tick" — written by stride and read by
siege's cling logic. `paces_last` is its quantitative sibling. What must not happen is
someone moving stride earlier to make intake read the current tick; that invalidates every
golden replay and reorders the charge priority, to fix a lag nobody can perceive.

### 3.9 What M3 changes in code that already exists

Not a task list — a list of the places where existing code assumes something M3 stops being
true, collected so they are found before they are debugged.

**Written before any of it was built, and kept as written.** Almost all of it has now
landed, and the value of the list is no longer as a plan but as a record of which of these
were spotted in advance and which were not. Two were not, and both were found by something
running rather than by anyone reading: the golden recorder walked to the fork at 15,000
paces and stood there for the rest of its script, and a storeroom turned out to be a
one-way sink for anything without a `take_stock` consumer, so bamboo that reached a shelf
could never come off it again. The first is on this list; the second is not, and is exactly
the kind of thing a list like this is bad at catching.

- **`examples/siege_run.rs`, `examples/throughput.rs`, `examples/record_golden.rs`,
  `web/e2e/smoke.spec.ts`** — all drive the engine with no player, so all will walk into a
  fork and silently measure a parked tower (§3.3). Each needs a standing fork answer.
- **`power::pay_for_stride`** early-returns on `!state.walking`; the fork and arrival halts
  must go through the same predicate, or a halted tower pays charge for standing still.
- **`world::pick_band_kind`** reads `content.terrain_runtime[i].weight` pack-wide; the
  region or branch palette replaces it, and its no-repeat rule is what forces the
  three-kind palette minimum (§3.2).
- **`World::generate_ahead`** streams unconditionally; it now stops at an unanswered fork
  and at the end of the last region, and it needs the palette for the region covering
  `generated_to` rather than the one under the tower.
- **`Feature.kind`'s "Presentation only" comment** becomes false — features are
  economically live once ruins hold salvage. Legal only because they were already drawn
  from the `world` stream, which is the whole point of the M0 note quoted in §3.4.
- **`FeatureView` gains `salvage` and `ViewSnapshot` gains `journey`** — a change to the
  bridge's public contract, so `Chrome.tsx` needs the fork card and the enclave board, and
  the renderer needs the fork horizon, the ruin fill level, and three distinguishable
  halted states (§3.3).
- **`intake::run`** computes one `yield_mul` from the band underfoot and applies it to every
  intake room; a `Ruin` source must not be scaled by terrain it is not drawing from.
- **`IntakeDef.ticks_per_item` becomes the `IntakeSource` enum**, touching `RoomRuntime`,
  `content::validate`'s category wiring check, and `RoomInfo` in the catalog.
- **`TerrainDef.weight` is deleted**; `TerrainRuntime` loses the field and
  `content::validate`'s "no terrain band has a positive weight" check moves onto palettes.
- **`siege::maybe_spawn_wave`** filters eligibility on `min_provocation` alone; without
  `wave_eligible` wardens leak into ordinary waves.
- **`tests/balance_doc.rs` is bidirectional** — every new `balance.ron` field needs a graded
  `BALANCE.md` row in the same commit, and the content-constants group row needs rewording
  for `paces_per_item`.
- **`assets/replays/golden.json`** goes stale twice over, from per-pace intake and from the
  new palettes; regenerate, and extend the recording to cover the new commands.
- **`ids.rs`, `Content`, `command.rs`** gain `RegionIdx`/`BranchIdx`, a `regions` list, and
  three commands with their rejections.
- **`state.rs`'s `place_starting_crew`** picks names by index, not by a roll. Recruiting
  must keep doing that, or adding a crew member perturbs a stream (`DECISIONS.md` §2).

### 3.10 Exit criteria

- [x] **Two runs on the same seed are identical.** The golden fixture runs 40,571 ticks
      through a fork answer and a berth with a rousing, and verifies natively and in wasm
      against the same embedded bytes (`DECISIONS.md` §5). The enclave is deliberately *not*
      in it: reaching one means walking into region 2, which took the recording to 115,000
      ticks and 261 KB — a quarter of a megabyte embedded in the WASM bundle to cover two
      handlers that are pure state arithmetic. `Trade` and `Recruit` are covered by tests
      instead, and their survival through the replay format by
      `every_command_survives_the_replay_format`, whose match is exhaustive so a new command
      cannot be added without a decision about it.

      Backed by thirteen property tests over 300 seeds: rolled lengths inside their authored
      range, journeys identical for a seed across runs, no band repeating its predecessor,
      every band drawn from a palette that covers it, nothing generated past an unanswered
      fork, no fork crowding a region edge or the enclave, salvage only in ruins, and seeds
      differing in how many decisions a region asks.

      Two bugs those tests found that no example test would have: a band could **straddle
      the fork line**, meaning the generator had already drawn the far side from the near
      side's palette — running past a decision that had not been made, the one thing the
      halt depends on it never doing; and changing a fork answer kept that straddling band,
      so the rejected branch's ground survived the rejection.

- [x] **Two seeds feel meaningfully different.** `cargo run --release -p understory-core
      --example journey` plays region 1 across twelve seeds under one policy, then one seed
      under three, and puts the two kinds of variation side by side — because the criterion
      is a comparison, and a comparison belongs on screen rather than in somebody's head.

      **Measured 3 of 3.** Forks offered: 1 across seeds against 0 within one. Widest
      terrain band: 7 points against 1. Salvage in the region's path: 47,280 against 12,274
      — the richness roll is doing exactly what it was added for, so whether a city is worth
      stopping at is a fact about the seed rather than about the player.

      The instrument also settled a design question while being built. A tower that berths
      at a ruin and stays until it is empty **dies**, every time, even having bought a
      battery and a thornwright first — the Heartseed inside a day. That is not a bug: a
      berthed tower has `strode` false, so the wardens it woke never lose their grip, and
      the answer the game gives you is the one M2 built. The policy that *leaves* when it
      has been hurt enough, keeping whatever it already pulled out, reaches the boundary in
      64 minutes having salvaged 60 scrap off 28 ruins. §3.4's trade is real and it is
      played by walking away.

- [x] **A 45–60 minute session reaches the drowned city.** Measured across twelve seeds:
      **48 to 61 minutes** at 1×, median about 54. That is the half a harness can answer —
      it rules out the region being wrong by a factor. The other half, how long a person
      actually takes at the speeds they actually use with the stops they actually choose, is
      answered by playing it and writing the number down (`v2-plan.md` §10 rule 3), and has
      not been.

- [x] Golden replay regenerated; hash parity green natively and in wasm.

- [x] `make check` and the smoke suite green — 237 Rust tests, 7 Playwright.

**The sprint question — run pacing, and does route-is-your-power-mix actually bite?**

Pacing: yes, 48 to 61 minutes to the drowned city, median 54.

The power mix: **partly, and the half that does not show is worth naming.** Measured on one
seed with two policies that differ only in which branch they take at every fork — one always
the shadiest on offer, one always the most open (`Policy::Forager` and `Policy::Sunseeker`
in `examples/journey.rs`, both picking on the branch palette's own sunlight rather than on
its name):

| | shadiest route | most open route |
|---|---|---|
| mean exposure | 34% | 39% |
| canopy walked | 42% | 37% |
| ruin field walked | 15% | 27% |
| salvage in the path | 21,142 | 38,063 |

Five points of sunlight and eighty per cent more salvage, from nothing but the choice at
four forks. The sun half of the argument is live, and it comes with a second axis nobody
designed deliberately: the open route is also the ruin-rich one, so choosing sunlight is
also choosing more to stop for.

The biomass half does not show, and the reason is not the route. Both towers harvest
exactly 100 bamboo, because a tower that builds nothing has no consumer for the poles its
mill makes, so the shelves fill, the mill's outbox backs up, and the arm stalls — long
before the difference between a 140% band and a 50% one could accumulate into anything. The
yields *are* distinct in the simulation now (§3.6, and `every_authored_yield_is_a_different_harvest_rate`
guards it), but a starting tower cannot spend fast enough to feel them. That is a real
answer rather than a null result: the biomass axis needs somewhere for biomass to go, which
is what M4's meals and M5's tier-two chains are. Until then, the route is a power mix in
the literal sense — sunlight — and a salvage mix, but not yet a food one.

**Deferred out of M3:**

- **Region 3, the coast approach, and the Refugia arrival.** Named non-goals in
  `v2-plan.md` §9; region 2's far edge is a placeholder finish line and is labelled as one
  in §3.7.
- **Per-region creature tables.** `RegionDef` scales threat with a single multiplier and
  nothing else. M2 already recorded that most of the four-creature taxonomy is unexercised in
  play (§2.9); giving each region its own table would author selection rules for creatures a
  run has still never met. It belongs with M5's taxonomy pass, alongside the second
  emplacement.
- **Enclave repair.** `v2-plan.md` §6.6 lists enclaves as "trade, recruit, repair"; §9's M3
  scope lists "trade, recruit." Repair is left out deliberately rather than overlooked —
  M2's repair loop is a crew-time-and-poles decision (§2.5), and letting a waystation
  shortcut it would remove the triage pressure the loop exists to create.
- **The enclave as a place the crew haul to.** Reasoned about and cut in §3.5. Trade is a
  posted board and a button; the diegetic version needs a haul destination outside the tower.
- **A continuous stride throttle.** Reasoned about and cut in §3.6. `SetStriding` stays
  binary.
- **~~Scrap has exactly one consumer, and you pass it once.~~ Closed.** Shell work (§3.5)
  is the second consumer, and the one that scales with how hard a run salvaged. The original
  note is kept below because the reasoning that led to it is what produced the fix.
 Moving the enclave into region 2
  (§3.5) means most salvage now has somewhere to go, but the offers have finite stock and
  the enclave is behind you for the rest of the run, so scrap taken late banks against M5's
  sun-forge and its alloy. This is the same shape of deliberate exception M1 recorded for
  darts (§1.4) — the content rule is satisfied, since the enclave consumes scrap at runtime
  — reduced from a structural hole to a tail. If M5's forge slips, the honest fix is a
  second enclave later in the journey rather than leaving late ruins as decorative dead
  weight.

- **The siege balance is ungraded again, and this is the biggest thing M3 leaves behind.**
  Three correctness fixes in one milestone — per-pace intake, a yield multiplier that had
  been rounding to a no-op, and a storeroom that had been a one-way sink — each made the
  economy more forgiving, and together they dissolved the three-way shape M2's
  `PLAYTESTED` rows were tuned to produce. Re-measured, every plan finishes untouched with
  poles banked. `BALANCE.md`'s Siege section opens by saying so. Recovering it needs an
  instrument that keeps a tower genuinely constrained, which is a balance pass and not
  something to bolt onto the end of a systems milestone. First thing at M4.
- **The biomass half of "route is your power mix" is real but unfeelable.** The four terrain
  yields are genuinely distinct now and a test guards it, but a starting tower fills its
  shelves and stalls long before a 140% band and a 50% one add up to anything, so both a
  shade-seeking and a sun-seeking route harvest identically. The sun half measures clearly
  (§3.10). What the biomass half needs is somewhere for biomass to go — M4's meals, M5's
  tier-two chains — not a change to the routes.
- **Region 2 is barely exercised.** Everything measured routinely stops at the region-1
  boundary; only `a_run_can_be_played_from_the_first_pace_to_the_last` and the whole-way
  section of `examples/journey.rs` go further. The drowned city's palette, its higher
  `threat_pct` and the enclave in it have been walked through, and that is all — nobody has
  looked at how the second region *plays*.

### 3.11 Open questions

Things that genuinely cannot be settled without building them.

1. **Does the fork halt read as a decision or as a lurch?** The argument in §3.3 is that a
   player who sees the fork coming answers it early and never stops. Whether they actually
   see it — at 2×, with a chain to watch and a wave inbound — is a question for the
   cross-section, not for the spec. If they consistently do not, the fix is a louder approach
   in the terrain strip, not a modal pause.
2. **Is one ruin worth one warden fight?** §3.4 states the equation and both of its sides
   move together. Whether there is a setting where the answer is genuinely "it depends on the
   tower" rather than always yes or always no is the thing the balance pass has to find, and
   it may turn out that salvage needs a second payoff — something the tower does with scrap
   directly — to be a decision at all. Ruin richness (§3.2) helps here by making the answer
   differ between runs, but a mechanic that is worth it at 140% and never worth it at 60%
   is a mechanic that is off half the time, which is not the same as a decision.
3. **How bad a night is a brown-out, once harvesting stops with the legs?** §3.6 identifies
   the loop and its floor — the sails fill regardless of whether the tower is walking, so
   morning ends it. What the instrument has to find is the cost of that night in a run that
   was otherwise going well, and whether a competently-played tower ever pays it twice.
4. **How long is a region, in minutes?** The paces figure is arithmetic; the minutes figure
   depends on how much of a session is spent at 1× versus 4×, and how often people stop. The
   instrument narrows it, playing settles it.
5. **Should a branch be able to change *what* comes, not just how much?** Threat is one
   multiplier in M3. A branch that trades "quieter" for "different" — the ruin road that
   wakes machines, the canopy passage that drops leapers — is a better choice than a branch
   that trades quiet for loud, but it depends on M5's taxonomy landing first.

---

## M4 — The Home *(the tone)*

**Sprint question:** does it feel like a home reclaiming the world, or a spreadsheet with legs?

**Scope:** the tower stops being a machine and becomes somewhere people live. Crew get names,
faces and things to say; they get two needs — meals from a canteen chain, sleep in a bunk on a
shift rota the player sets — and both needs are demands on the circulation that already
exists. Then the two presentation passes: the flat-vector solarpunk-tropical art pass
`v2-plan.md` §0 scheduled here from the start, and the entire audio subsystem — which does not
exist at all, and which M1's brief already asked for and did not get ("starvation/stall/
brown-out all readable *and audible* — silence = broken").

**Non-goals (M5):** no region 3 and no Refugia arrival, no tier-two chains, no unlocks or
meta-progression, no second emplacement or full creature taxonomy, no enclave economy beyond
M3's single waystation, and no role-priority system — crew remain generalists who haul, mend,
eat and sleep, and the "haul / operate / gun / repair" priorities `v2-plan.md` §6.7 sketches
are deferred with a reason (§4.9).

### 4.1 The shape of the thing

**Nothing in M4 adds an economy.** It adds *needs*, and a need is not a new resource loop — it
is a new customer for the loops already running. Meals are bamboo the mill did not get; a
sleeping crew member is a pair of hands the stairs did not carry. Every number M4 introduces
is a claim on circulation, which is the one thing this game has always been about
(`DESIGN.md` §2 insight 1). If a system in this milestone needs its own supply mechanism, its
own screen, or its own currency, it has been designed wrong — the same test §2.4 applied to
emplacements applies here.

The second half is presentation, and it is not decoration. Every milestone so far has been
answered by a test or an instrument. M4's two exit criteria are a **screenshot** and a
**recording**, answered by looking and listening (§4.9), and that is a genuine change in how
this milestone gets judged: `make check` passing tells you nothing about whether M4 worked.

**The ordering inside the sprint matters, and it is not the order of the sections below.**
`SYSTEMS.md` §3.10 leaves the siege balance ungraded — three M3 correctness fixes dissolved
the three-way shape M2's `PLAYTESTED` rows were tuned to produce — and says the re-measurement
is the first thing at M4. It cannot be. Sleep takes something on the order of two-fifths of
the tower's crew-hours out of the economy (§4.4), and meals take a fifth of a mill's bamboo
draw (§4.3); re-measuring before those land means measuring an economy that is about to move
again. So: **build the needs, then re-measure once.** The balance pass is the end of M4, not
the beginning of it, and the harnesses it runs on need the changes in §4.8 before their
numbers mean anything.

### 4.2 Crew as named individuals

Crew are named individuals with jobs, not stat blocks (`DESIGN.md` §2 structural call 4).
Three things carry that: a name, a face, and something to say. None of the three is allowed
anywhere near the simulation.

**Names are content, chosen by index.** `assets/data/crew/names.ron` holds an ordered list,
longer than `crew_cap`, and `GameState::add_crew` takes the next one by index exactly as it
does today — the rule §3.5 established for the enclave's recruit, kept because a name drawn
from a stream would perturb the stream (`DECISIONS.md` §2). `Crew.name` stays a `String` in
state: it is already there, it is already in the hash, and interning it would buy nothing
mechanical. What that does mean is that the *order* of the name list is load-bearing for
replays, the same way the sort order of every other content list is (`DECISIONS.md` §6);
reordering the file is a simulation change, and editing the text of an existing entry changes
the content hash. Both are ordinary, and both are worth knowing before someone alphabetises
the file.

**Portraits and barks live entirely in the frontend.** This is stronger than the firewall rule
requires and the strongest available reading of it: rather than draw a bark line from
`cosmetic`, M4 puts no part of a bark in `GameState` at all. Rust already publishes everything
the frontend needs to know that something bark-worthy happened — the event list from `frame()`
and the per-frame `ViewSnapshot` (§4.6) — so the JS layer decides *whether*, *which line*, and
*when*, and the simulation cannot be perturbed by a bark because there is nothing to perturb.
`DECISIONS.md` §2 names a bark drawing from `sim` as the exact catastrophe to avoid; a bark
that does not draw at all cannot be that catastrophe on any future day either.

Where a bark needs a *stable* per-person choice — this crew member's face, and which of
several equivalent lines is hers — it derives from `CrewView.fidget`, the per-crew `u16`
already drawn from `cosmetic` at spawn and already in the snapshot for the renderer's idle
phase. One more consumer of an existing cosmetic draw costs nothing, adds no state, and is
stable for the life of a crew member and reproducible from a seed. `portrait = fidget % faces`
is the whole mechanism.

**What each bark is triggered by, and from which of the two inputs.** Barks are punctuation on
top of state, so they follow the same split as sound (§4.6):

| Bark occasion | Read from | Note |
|---|---|---|
| picked something up, set something down | `Pickup` / `Deliver` event | the most common, so the most heavily rate-limited |
| stuck in a queue | `CrewView.stressed` going true | the diegetic red tint already says it; the line is colour, not information |
| sat down to a meal | `MealServed` event | the warmest moment in the tower, and worth a line |
| going off shift | `ShiftChange` event | the handover is the tower's one daily ritual |
| woken early | `CrewView.asleep` going false without a `ShiftChange` | the surge lever (§4.4) should sound like an imposition |
| a wave on the horizon | `WaveArrives` event | defenders, not soldiers — see below |
| something gave up and left | `EnemyLeaves` event | never triumphant; §2.2's distinction is a tone rule as much as a mechanical one |
| a breach, a wreck, a severed column | `Breach` / `Wrecked` / `Severed` | concern for the tower, not for the enemy |

Tone follows `DECISIONS.md` §8 without exception. The crew are gardeners and porters who
defend a home they live in; they are not troops, they do not report kills, they do not banter
about ordnance, and nothing they say frames a creature as a target. A creature that lost its
grip and walked away has not been *beaten* — §2.2 already refuses to count it as repelled, and
a bark that celebrates it would undo that refusal in the one register the player actually
attends to. A line that reads as militaristic is a tone bug even though it is mechanically
inert, and the check is `AGENTS.md` §VII's: read it aloud and ask whether it belongs in
Nausicaä or in a shooter.

Barks are fire-and-forget in the same sense sound is: emitted by a situation, drawn on screen
or dropped, and never read back into anything. There is no bark log, no morale that barks feed,
and no relationship state — `v2-plan.md` §3's fourth structural call rules the last one out
explicitly.

### 4.3 Meals, and the kitchen chain

A **canteen** takes bamboo and makes meals. Meals are an ordinary item on ordinary shelves,
hauled by the same crew through the same shafts, subject to the same buffer stalls. There is no
food logistics layer; there is one more production room whose output happens to be eaten by
people instead of by a battery.

```
room.canteen — Production, 2 slots, 5 poles
  recipe: 2 bamboo -> 3 meals, craft_ticks 300
```

**The canteen is deliberately not powered.** Every other production room could take a
`power_draw` and the thornwright does, but a kitchen that goes dark in a brown-out and stops
feeding people turns one bad night into a spiral inside a spiral — and the "powered production
is a charge sink" lesson is already taught by a room the player chose to build for a reason
other than survival. The canteen stays free to run.

**`Crew.hunger` counts ticks since the last meal.** Not a fraction, not an `Fx`, not a
per-mille with an accumulator: a `u32` tick count, which is how this codebase already expresses
every duration (`DECISIONS.md` §1) and which makes the accrual exactly `hunger += 1` with no
rounding anywhere. It rises every tick, awake or asleep. A meal resets it to zero — a meal is a
meal, and a crew member who ate late does not carry the deficit forward.

Two thresholds, and the gap between them is the design:

| Threshold | Ticks | What happens |
|---|---:|---|
| `hungry_ticks` | 4,800 | they go and eat: an errand outranking a new haul, but never a load already in hand |
| `starving_ticks` | 7,200 | they work *slower* — the multiplier in §4.4 — and keep working |

**A fed tower never sees the penalty.** That is the point of two thresholds rather than one. If
going-to-eat and slowing-down were the same number, crew would trudge to the canteen every
time, and a working kitchen would read as a permanent tax; with a third of a day between them,
the slow-down appears only when the kitchen has actually failed — no bamboo, no canteen, or a
canteen nobody can reach. **Hunger is a supply problem.** It is invisible while the chain
works and it is the chain's failure that you feel, which is the same shape as every other
signal in this game.

At 14,400 ticks a day, `hungry_ticks` of 4,800 is three meals a day a person. At 2 bamboo a
meal that is **6 bamboo a day a crew member** — four crew is 24 a day, about a fifth of what a
fed mill draws (one bamboo per 120 ticks is 120 a day) and a fifth of what one cutter arm
brings in on region 1's palette. That ratio is the whole judgement: enough that a route rich
in biomass visibly feeds people better, not so much that meals displace poles as what bamboo
is *for*.

**Crew eat where the meals are, and the arrival machinery already exists.** Eating is not a
haul and it is not a new pathfinder. It is shaped exactly like a repair: go to a place, stand
there, spend ticks (§2.5, `haul::repair_leg`). A hungry crew member picks the nearest room
holding at least one meal — the canteen's own outbox, or any shelf a meal has been hauled to —
walks and climbs to it on the ordinary legs, and enters `CrewState::Eating`, which consumes one
meal on completion. If the meal is gone when they arrive, the errand clears and they look
again: the same failure path `CrewState::Loading` already has when somebody else got there
first, and for the same reason no reservation bookkeeping is added for it.

**Eating at the source is load-bearing, not a convenience.** `BALANCE.md`'s `storeroom` row
documents the sharpest emergent failure in the game — a shelf takes whichever item lands on it
first, so bamboo can claim every shelf and deadlock the chain — and explicitly hands the
question of what to do about it to M4. Meals make it worse: they are a fifth claimant on four
shelves per storeroom, alongside bamboo, poles, darts and scrap, and a shelf-starved tower
would now starve its people as well as its mill. Because crew can eat straight out of the
canteen's outbox, the meal chain survives a fully-claimed storeroom: distribution to shelves is
an *optimisation*, not a requirement. That optimisation is real and worth discovering — a
storeroom high in the tower becomes a pantry, and where the pantry is decides how far people
walk to eat — but nobody starves for want of a free shelf. The deadlock itself is still
unanswered; see §4.10.

**This is load-bearing well beyond M4, and it is why the canteen's rate is the number to watch.**
`SYSTEMS.md` §3.10 records that the biomass half of "your route is your power mix" is real in
the simulation and unfeelable in play: the four terrain yields are genuinely distinct and a
test guards it, but a starting tower has nothing to *do* with bamboo, so the shelves fill, the
mill's outbox backs up, the arm stalls, and a shade-seeking route and a sun-seeking one harvest
identically — measured at exactly 100 bamboo each, over a whole region. **Meals are the
consumer that fixes it.** They are a demand that scales with the crew rather than with shelf
space, they cannot be satisfied by stockpiling, and they run all day.

So whoever authors the canteen's rate owes one more thing: **re-run `examples/journey.rs`'s
shade-versus-sun comparison afterwards** — the `Policy::Forager` against `Policy::Sunseeker`
table in §3.10 — with a canteen and bunks in the shopping list, and check that harvest is no
longer identical across the two routes. If it still is, the canteen is too cheap to feed
people with, and the lever is the meal's bamboo cost, not the terrain yields. That measurement
is the single clearest test of whether M4 closed M3's biggest open finding.

### 4.4 Sleep, and the shift rota

**`Crew.shift` is `Day` or `Night`, and the player sets it.** Crew are awake when the current
daypart belongs to their shift and asleep when it does not. Sleeping crew do no work at all:
they take no tasks, advance no legs, mend nothing, and accumulate no `wait_ticks` — a sleeper
tinting red would make the only bottleneck instrument in the game lie (`DECISIONS.md` §8).

**Which dayparts belong to which shift is content.** `DaypartDef` gains a `shift` field, so the
handover is a designer's decision rather than a constant buried in a system, and the seven
dayparts already in the pack divide as:

| Shift | Dayparts | Per-mille | Ticks |
|---|---|---:|---:|
| Day | morning, midday, afternoon | 180–760 | 8,352 |
| Night | dusk, night, predawn, dawn | 760–180 (wrapping) | 6,048 |

Validation requires both bands to be non-empty and each to be one contiguous run modulo the
day, in the same spirit as the contiguous-`order` check on regions (§3.2): a rota with two
separate night stretches is not a rota, it is a bug in the content pack, and a broken pack is a
load error rather than a runtime condition (`AGENTS.md` §IV).

The bands are deliberately unequal, and the night band is deliberately the one that covers the
lamplit hours. Against the shipped sun curve, exposure drops below `night_light_threshold` at
about 835‰ and climbs back through it at about 150‰ — entirely inside the night shift. **The
night shift is the dark shift, exactly.** It is also the shorter one, and that asymmetry is its
compensation: a night worker is awake for 6,048 ticks against a day worker's 8,352, so staffing
the night costs more hands than it returns.

**`Crew.rested` is ticks of work left in them.** A `u32`, like hunger, counting down one a tick
while awake and up while asleep — `rest_gain_per_tick` in a bunk, and less on the floor.
Below `tired_ticks` they work slower, on the same multiplier hunger uses. The arithmetic is
arranged so that a bunked crew member on either shift wakes full:

| | ticks awake | ticks asleep | rest banked |
|---|---:|---:|---:|
| Day shift | 8,352 | 6,048 | 12,096 (capped at `rested_max_ticks` 8,640) |
| Night shift | 6,048 | 8,352 | capped |

**A day-shift crew member with a bunk flags at the end of every shift, and that is intended.**
`rested_max_ticks` of 8,640 against 8,352 ticks awake leaves almost no slack, so they cross
`tired_ticks` about eighty percent of the way through the day and work the last stretch of it
slowly. The tower visibly tires in the late afternoon and picks up at dawn. That is not a
balance oversight to tune out — it is the day having a shape, which is most of what "somewhere
people live" means on screen, and it costs nothing but the shape of two constants.

**Tiredness is a scheduling problem, the way hunger is a supply problem.** There is no mid-shift
nap: a crew member cannot fix being tired the way a hungry one can walk to the canteen, because
the only thing that refills `rested` is being off shift. So the two needs fail differently and
are read differently. Hunger says *your chain broke*; tiredness says *your rota is wrong, or
you have no beds*.

**No bunk means sleeping where they stand, and worse.** A crew member with nowhere to lie down
enters `CrewState::Sleeping` on the deck where they stopped — visible, and drawn as such — and
gains rest at `no_bunk_rest_gain` of 1 a tick instead of 2. The arithmetic is unforgiving and
exact: a day-shift sleeper on the floor banks 6,048 against the 8,352 they spend, a net loss of
2,304 a day, so they slide into permanent tiredness inside four days and never climb out.
Bunkless is survivable and visibly degrading, which is the right shape for a cost the player
can fix at any time for three poles.

```
room.bunk — Quarters, 2 slots, 3 poles, sleepers: 2
```

**A bunk is a new room category, because the pack's validation demands one.**
`content::validate` rejects any room whose category has no matching behaviour block
(`DECISIONS.md` §9.1), and a bunk has no recipe, no storage, no intake and no defence. So
`RoomCategory::Quarters` requires `quarters: Option<QuartersDef { sleepers: u8 }>`. Two
sleepers to a two-slot bunk is one slot a person — the tightest legible packing — which makes
quarters a floor tax that grows with the crew that pays for it. That cost is the point and
should not be quietly relieved; if it reads as too dear in play, the lever is `sleepers`, not
free beds.

**And here the rota pays for itself in floor space, which nobody designed and which falls
straight out of the model: beds are shared between shifts.** A bed is only occupied while its
sleeper is off shift, so a tower with everyone on the day shift needs one bed a head — three
crew, two bunks — while a tower at cap split four and four needs only four beds, also two
bunks, because the night watch is up while the day crew are in them. Eight crew unrota'd would
want four bunks, eight slots, more than a floor has left once the stairs have taken theirs.
Staffing the night halves the dormitory, and a player who works that out has found a real
reason to do it that has nothing to do with the prowler.

**Bunk occupancy is derived, never stored.** How many sleepers a bunk holds is counted by
scanning the crew whose errand names it — the same trick `haul::shaft_queues` already uses to
give one crew member a picture of the whole queue while the borrow checker only lets them see
themselves. `Room` gains no field, and there is no occupancy counter to get out of step with
reality. Beds are claimed in crew order, which is creation order, which is `CrewId` order, so
who gets the last bed is a pure function of state.

**The rota is a real decision, and both of its costs are already in the game.**

*Night cover.* The night prowler is `night_only` (`assets/data/enemies/`), so the hours a tower
is least able to answer a wave are precisely the hours something is out. A tower with everyone
on the day shift has nobody to run darts up to a battery or mend a breach between dusk and
dawn; a tower that staffs the night pays for that cover in daylight throughput, on the shift
where the chain actually flows. Neither answer is free and neither is wrong.

*Lamps, which are charge.* This connection needed a decision, because as shipped it is not
true: `power::lighting` buys light for the whole tower whenever exposure is below
`night_light_threshold`, regardless of whether anybody is awake in it. So a night shift adds no
charge cost, and "night operations need light" was a sentence rather than a mechanic. **The
call: lighting stays unconditional, and working in the dark joins hunger and tiredness on the
same slow-down multiplier.** A crew member working while `Power.lit` is false — a brown-out,
not merely a dark night — works at `dark_work_pct`. That makes a night shift's dependence on
charge sharp and immediate (a brown-out does not just dim the tower, it wastes the shift you
staffed) without adding a single new charge sink, and it strengthens M1's and M2's signature
emergency instead of relaxing it.

The alternative — gating lighting on somebody being awake — was considered and rejected. It
reads well and it is three lines, but the default rota is all-Day, so it would make every
night's lamps free for most towers and quietly relax the brown-out pressure `BALANCE.md`'s
power rows were measured against. Making the dark expensive by slowing the people in it costs
no constants and revalues nothing.

**The rota's one emergency verb is a surge, and it costs what it should.** Because awake means
"the current daypart belongs to my shift", setting a sleeping day-worker to `Night` in the
middle of the night wakes them immediately — unrested, on the slow multiplier — and come
morning they are off shift and will sleep through the day you needed them for. That is a real
all-hands lever with a real price, built out of nothing but the definition above. It is also
why **crew are never woken automatically.** An attack does not rouse a sleeper: if the
simulation woke people when things got bad, the rota would be decorative, and the interesting
decision — do I burn tomorrow morning to answer tonight — would be made by the game instead of
the player.

**Two invariants the state machine has to hold.** A crew member never falls asleep holding
something: going off shift stops them taking *new* work, and they head for a bunk once their
hands are empty, which is the same reasoning `assign_idle` already applies to a carrier who
would otherwise be pulled onto a repair. And an errand whose room is gone — a bunk demolished
under a sleeper, a canteen removed mid-meal — clears to `Idle` rather than spinning, exactly as
`CrewState::Boarding` already handles a shaft demolished out from under a queue.

**Priority order for an idle crew member**, extending the ladder in `haul::assign_idle`:

1. a task already under way — pick the trip back up rather than re-deciding it
2. a load in hand with somewhere to put it — finish the delivery; nothing carried is ever
   dropped
3. off shift — go to a bunk, or lie down where they are
4. past `hungry_ticks` — go and eat
5. damage worth a shift — mend it (§2.5)
6. a haul

Eating above mending is deliberate: a crew member past `starving_ticks` mends slowly too, and a
meal is 300 ticks against a repair shift's 80 plus the walk. Feeding them first is the cheaper
order.

**The multiplier, and the one trap in implementing it.** Three causes — starving, tired, working
unlit — compose multiplicatively into a `work_pct`, and that percentage scales **the duration of
an action, never the fixed-point step that advances it.**

```
work_pct      = 100, times each active penalty / 100, floored at 1
effective(t)  = t * 100 / work_pct        // integer, computed once per leg
```

This is not a stylistic preference. `haul::advance` currently steps position by
`Fx::ratio(1, walk_ticks_per_slot)`, and `Fx::ratio(1, 12)` is already `Fx(21)` — 1.6% off the
authored rate. Scaling *that* by a percentage is precisely the arithmetic that broke intake
before M3: `Fx::ratio(60, 1200)` truncates to `Fx(12)`, a 21-tick slot rather than the 20 the
constants describe, and the errors compound per penalty. `intake::terrain_effort`'s doc comment
is the authoritative account of what that class of mistake cost the game the first time — four
authored terrain yields behaving as two, and the flagship contrast of `DESIGN.md` pillar 1
absent from the simulation entirely. Scaling the tick count instead keeps a single integer
division, leaves the Fx precision exactly where it already is, and makes the penalties
inspectable as tick counts. `Loading`, `Unloading`, `Eating` and `Repairing` are trivially
exact, since they are already `ticks_left` counters.

**`work_pct` never exceeds 100.** A fed, rested crew member in a lit tower is the baseline, not
a buff — being cared for is normal and neglect is what costs you. A food that made people
*faster* would turn the crew into a throughput stat to optimise, which is the one thing
structural call 4 exists to prevent.

**Opening values.** Every figure in §4.3 and §4.4, collected so whoever authors the content has
one list rather than nine paragraphs to mine. **Each is a design target** — a first value with
an argument behind it, not a measurement — and none is a `BALANCE.md` row until it has been
authored, at which point it gets a graded row in the same commit (`DECISIONS.md` §7). The
arithmetic assumes `ticks_per_day` 14,400 and 30 Hz.

| Thing | Target | The arithmetic |
|---|---|---|
| `hungry_ticks` | 4,800 | A third of a day, so three meals a day a person. Short enough that the canteen is somewhere people actually go, long enough that eating is not most of what a crew member does. |
| `starving_ticks` | 7,200 | Half a day — a full meal cycle *past* being hungry. A working kitchen never reaches it, so the slow-down is a failure signal rather than a routine tax. |
| canteen recipe | 2 bamboo → 3 meals, 300 ticks | 6 bamboo a day a crew member; 24 for four crew, a fifth of a fed mill's 120. A ten-second craft is visible as cooking. Flat out the room makes eight times what a full crew eats, so it is buffer-limited and mostly idle — correct for a kitchen, and it means the canteen's *rate* is not what sets bamboo demand; the crew are. |
| canteen | Production, 2 slots, 5 poles, no `power_draw` | A thornwright's price: dearer than a mill (4), well short of a salvage rig (8). An early, obvious build rather than a commitment. Unpowered on purpose; see above. |
| bunk | Quarters, 2 slots, 3 poles, `sleepers` 2 | Joint-cheapest in the pack with a storeroom, because the alternative to a bed is a crew member who degrades a little more every day, and a bed should never be a gate. The real price of quarters is the slots, not the poles. |
| `rested_max_ticks` | 8,640 | 0.6 of a day, a shade over the 8,352-tick day shift, so a day worker ends their shift nearly empty and a night worker never does. |
| `tired_ticks` | 1,440 | A tenth of a day of work left. A bunked day worker crosses it about four-fifths through their shift, so the tower flags in the late afternoon. |
| `rest_gain_per_tick` | 2 | Two ticks of rest a tick asleep, so 6,048 ticks off shift refills 12,096 — comfortably over the cap, which is what makes a bunk feel like a solved problem rather than a managed one. |
| `no_bunk_rest_gain` | 1 | Bunkless is a net loss of 2,304 a day and permanent tiredness inside four. Visibly degrading, never fatal, and fixable for three poles at any moment. |
| `hungry_work_pct` | 60 | Past `starving_ticks`, everything takes about two-thirds longer. Plainly slower on screen without reading as broken. |
| `tired_work_pct` | 60 | Deliberately the same number as hunger's: one visible failure mode with two causes, so a player learns the *look* of a crew member working badly once and then asks why, rather than learning two separate symptoms. |
| `dark_work_pct` | 75 | The mildest of the three — you can work by feel, just not well. Composed with the other two the worst case is 27%: crawling, never stopped. A need that halts the tower is a death spiral rather than a pressure. |
| `crew_cap` | 6 → **8** | `BALANCE.md`'s current row defers the plan's eventual eight to "whatever M4's shift rota earns". The rota earns it: eight crew split across two shifts is about five awake at once, which is where six unrota'd crew already sat. Raising the cap without the rota would have been a straight throughput gift; with it, it buys coverage. Still aspirational either way — M3's enclave offers exactly one recruit (§3.5), so a run cannot approach either number until M5's enclave economy. |
| `item.meals` | glyph 🍲, order 25 | Between poles (20) and darts (30) in the display order, because meals sit beside poles as the other thing bamboo becomes. Nothing mechanical reads `order`; the indices come from the sorted string IDs (`DECISIONS.md` §6). |

New command:

| Command | Effect | Rejects on |
|---|---|---|
| `SetShift { crew, shift }` | put one crew member on the day or the night shift | no such crew |

One crew member per command rather than a bulk setter: commands batch cheaply
(`DECISIONS.md` §3), a rejection then names the crew member it is about, and the replay reads
as a list of decisions about people.

### 4.5 The art pass

Flat-vector solarpunk-tropical: overgrowth on the tower, warm interiors, verdigris and worked
brass against deep jungle green. `v2-plan.md` §0 planned "readable placeholder
(rectangles-with-personality) through M3" with the real pass here.

**Most of the foundation landed early, and this pass builds on it rather than starting from
boxes.** `web/src/engine/palette.ts` is already the solarpunk-tropical palette, with a
day/night blend (`atNight`), per-band terrain colours including the drowned city's own, and
warm-metal salvage against cold stone. `scene.ts` already draws per-category room silhouettes
with a body height and a crown — a sail, a cell rack, a vent stack, a cutter boom, a battery
barrel — off the room widths §0.5 settled, so a floor of mixed rooms already reads as a skyline
rather than a row of crates. Planters already sit on every deck's outboard edge and across the
roof garden, damage already splits timber and spills lamplight through a breach, and shell
plating already reads as bolted-on metal. **The remaining work is not "add art", it is "make one
frame say home".** Concretely:

1. **Crew have to become people.** This is the largest single item, and it is the one the
   screenshot test turns on: a frame full of rounded lozenges is a diagram whoever looks at it
   will read as a factory. Crew need a walk cycle driven off the fractional `slot` they already
   carry, a laden posture distinct from an empty one, a sitting pose for `Eating`, and a lying
   pose for `Sleeping`. Nothing about this needs new snapshot data — the states and the
   fractional positions are already there.
2. **Two new silhouettes, and a `Quarters` arm in the two functions that switch on category.**
   `roomColor` and `roomProfile` both fall through to a generic box for an unknown category, so
   a bunk would work and look like a crate. A bunk draws its `sleepers` as hammocks — a
   solarpunk answer to a bed, and one that makes occupancy diegetic: you can see who is asleep
   and whether a bed is spare, without a number. A canteen draws a hearth: warm interior light
   and rising steam while it is crafting, cold and dim when starved. That is the single warmest
   image available in the tower, and it doubles as the kitchen chain's own stall signal — a cold
   hearth is *why* people are going hungry, in the same place you notice that they are.
3. **Overgrowth beyond the planters.** Vines trailing between floors, moss at the shell lip,
   growth thickening on the leeward side — all with per-floor phase derived from the existing
   `hash01` helper so it is stable frame to frame rather than crawling.
4. **Warm interiors, per room.** A working room shows a lit window; a stalled one does not. It
   reinforces the signal the dimmed body already carries rather than adding a second, different
   one, which is the §8-compliant way to add emphasis.
5. **Verdigris as its own colour.** The palette's cool blue-green is currently `roomStorage`
   doing double duty on shell plating. Naming `verdigris` and `brass` and using them on shaft
   rails, plating and joinery is what makes the metal read as aged copper rather than as paint.
6. **Light through the canopy.** Dappling on the tower's face keyed to the band underfoot, and
   motes or fireflies after dark. Both are pure JS animation with no state behind them, which
   is where cosmetic motion belongs.

**What must not happen in this pass.** No numeric badge, no warning icon, no hunger bar over
anybody's head. The diegetic signals for the two new needs are behaviour: a hungry crew member
walks to the canteen, a tired one moves visibly slower, a sleeping one is lying in a hammock.
Precision is a hover layer, per `DECISIONS.md` §8, and the moment a crew member acquires a
floating status bar the screenshot test is unwinnable — because a frame full of floating bars
is a spreadsheet with legs, which is the exact failure the sprint question names.

The one new piece of chrome M4 does add is a **crew roster**: portrait, name, what they are
doing, and a day/night toggle that issues `SetShift`. That is a panel, and it is defensible
under §8 because a rota is a schedule the player writes rather than a readout of state — the
same category as the elevator's per-daypart programs. What is not defensible is the roster
becoming the primary place hunger and tiredness are read. If the tower can only be understood
through the roster, the pass failed.

### 4.6 Audio

**No audio exists.** `SoundEvent` has nineteen arms, emitted throughout the tick and carried
across the bridge by `frame()`, and `Game.ts` throws the list away with a comment pointing at
this milestone. So the plumbing is already done and the whole subsystem is JS-side work.

**The boundary is fixed: Rust decides *that* something happened, JS decides whether and how it
sounds.** Events are fire-and-forget — emitted during a tick, consumed or dropped by the
`AudioManager`, never read back into the simulation. Nothing about the mix, the volume, the
voice count, or whether audio is even enabled may reach `GameState`. If a sound needs to know
something, it reads the snapshot; it does not ask the simulation to remember anything for it.

**The audio layer reads two inputs, and telling them apart is the whole design.** The
eyes-closed test asks whether you can hear how the tower is *doing*, which is continuous state
— and `SoundEvent` is punctuation. A starved mill going quiet is not an event at all; it is the
*absence* of a loop, and the fact driving it is `RoomView.stalled` in the per-frame view. So:

* **`frame()`'s event list** drives one-shots: things that happened.
* **`view()`'s `ViewSnapshot`** drives loops: things that are ongoing.

Both already cross the bridge every frame. Getting this split wrong is precisely how a project
ends up with a warning beep where a silence belonged.

**Loops, from the snapshot.** Each is a bed whose gain is a function of state, and each goes
silent when its state stops — never replaced by a different sound saying it stopped:

| Loop | Read from | Silent when |
|---|---|---|
| a room working | `RoomView.stalled` false and `progress` advancing | starved, backed up, unpowered, wrecked — all of which sound identical, because from outside they are |
| the legs | `journey.halt`, not `power.walking` | stopped, halted at a fork, arrived, or browned out — see below |
| a car running | `ShaftView` car state | idle |
| footsteps on the stairs | crew in `climb` | nobody on them |
| the sails | `ClockView.exposure_pct` | shaded, or after dark |
| the tower's electrical hum | `PowerView.fill_permille` | thins as the bank drains, and drops out entirely on `brownout` |
| the day bed | `ClockView.permille` and `daypart` | crossfades with the night bed |
| the night bed | as above | — |
| the jungle | the band underfoot, `WorldView` | never; it is the floor under everything |

**The legs are the one loop with a trap in it, and the snapshot already contains the answer.**
`PowerView.walking` is the player's *intent*; `GameState.strode` is whether the legs actually
ran, and it is deliberately not in the snapshot as a raw flag. What is there is
`journey.halt` — `Walking`, `Stopped`, `Fork`, `Arrived`, or `Brownout` — precisely because §3.3
required the halted states to be distinguishable and the renderer needed telling which one it
was drawing. Audio inherits that for free, and should use all five: a tower that stopped and a
tower that cannot afford to move must not sound the same, and the brown-out case has a
treatment to match already — `drawLegs` gives it a stuttering lift that never becomes a step,
and the sound of that is a motor asking and not being answered.

**Two soundscapes, day and night**, as `v2-plan.md` §9 asks: a day of canopy-hum, insects, and
the tower's own working noise; a night of a different insect register, wind, distant movement,
and the tower's lamps and machines standing out against it because there is less around them.
The crossfade follows the sun curve rather than the daypart index, for the same reason the sun
curve is a curve — a step change at a boundary reads as a bug (§1.1).

**One-shots, from the event list.** Every existing arm of `SoundEvent` is punctuation and
already correctly shaped; M4 adds three:

| Event | New | Why |
|---|---|---|
| `MealServed` | yes | The warmest moment in the tower deserves a cue, and it is the audible confirmation that the kitchen chain is alive. |
| `ShiftChange` | yes | The handover is the tower's one daily ritual, and it is the only reliable way to *hear* what time it is. |
| `EnemyLeaves` | yes | Closes M2's deferral (§2.9): `Leaving` and `Dying` are distinct in state and in the snapshot but have sounded identical, so walking a wave off and shooting it down were indistinguishable. §2.2 refuses to count the first as repelled; the audio has to refuse too. |

**The diegetic rule, stated once because everything else follows from it: a starved production
loop goes silent, it does not gain a warning sound.** No alarm on a stalled room, no beep on a
queue, no sting on a full buffer. `wait_ticks` and the red tint are the bottleneck instrument
(`DECISIONS.md` §8) and audio's contribution to them is the mill you can no longer hear. This
is the rule that makes the eyes-closed test winnable at all: a tower whose problems announce
themselves with tones is one where you hear the *alarms*, not the tower.

**Three practical constraints that fall out of the existing frame loop.**

* **Coalesce per frame.** `GameEngine::frame` runs up to `MAX_TICKS_PER_FRAME` ticks in one
  call, and at 4× a frame routinely contains several ticks' worth of events. Three mills
  finishing on one tick must not be three times as loud, and a busy frame must not machine-gun.
  The `AudioManager` deduplicates by kind within a frame and rate-limits each kind, which is a
  JS concern entirely and needs no change in Rust.
* **`SoundEvent` stays payload-free, and M4 does not do positional audio.** Adding a floor or a
  room id to the events that could use one would change the bridge's public contract from a
  string union to tagged objects, and the eyes-closed test is about the tower's *state*, not
  about where in it something happened. Deliberate cut, recorded here rather than discovered
  later by someone wondering why `Craft` does not say which mill.
* **Audio cannot start without a gesture.** Browser autoplay policy means the `AudioContext` is
  suspended until the player clicks, so the opening frames are silent and the Playwright smoke
  test never hears anything. Neither is a bug; both need to be true on purpose rather than
  discovered as a mystery.

### 4.7 Tick order, current

> **Supersedes §3.8.** M4 inserts one system, `needs`, between `defence` and `haul`, so haul
> becomes 9 and everything after it shifts by one.

1. **clock** — advance the day.
2. **power income** — recompute capacity from the banks; collect from sails and burners.
3. **transport** — cars move.
4. **intake** — harvest ground covered last tick; extract from a ruin while berthed.
5. **production** — recipes advance, consume, emit. Powered rooms pay here. The canteen cooks.
6. **siege** — creatures approach and attack; provocation decays.
7. **defence** — emplacements fire at what siege just moved.
8. **needs** — hunger rises, rest drains or refills, and the shift band decides who is awake.
9. **haul** — crew advance their legs, then idle crew claim work, eat, sleep, or mend.
10. **repair** — crew already at damage put hit points back.
11. **lighting** — lamps, after dark.
12. **stride** — region crossings, the halts, the distance advance, and terrain streaming.

**Why needs lands where it does.** It must run *before* haul, because haul both reads the work
multiplier and executes every leg of going to eat and going to sleep — a crew member's speed
this tick and their decision this tick should be about the same tick's hunger. It must run
*after* production, so a meal cooked this tick is available to eat this tick rather than next.
And it must not be inside haul: haul's job is moving people, and hanging counter accrual off the
top of it would bury two needs inside the most intricate system in the crate.

**Needs cannot make tick order matter more than it already does.** Tick order is load-bearing
for exactly two things: determinism, and charge priority — consumers draw from a shared pool as
they run, so who runs first is who gets served when the pool is thin (§1.6). **`needs` draws no
charge, and neither does eating or sleeping.** The canteen is unpowered by decision (§4.3), so
nothing in this milestone joins the priority order, and the insertion moves nothing except the
golden fixture. That fixture goes stale anyway the moment `Crew` changes shape, which is why an
insertion in the middle is acceptable here where it would not be in a milestone that changed
nothing else about state.

What must not happen is somebody moving `needs` after `haul` to avoid the insertion. It would
work — a one-tick lag, deterministic, imperceptible, exactly the shape of `paces_last` (§3.8) —
and it would put the decision to go and eat a tick behind the hunger that motivated it for no
gain, since the fixture is being regenerated either way.

### 4.8 What M4 changes in code that already exists

Not a task list — a list of the places where existing code assumes something M4 stops being
true, collected so they are found before they are debugged. §3.9's record is worth reading
first: two of that milestone's surprises were found by something running rather than by anyone
reading, and both were in this category.

- **`Crew` gains three fields** (`hunger`, `rested`, `shift`) and widens a fourth (below), so
  the state hash changes, so **every replay and the golden fixture go stale.** Regenerate, and
  extend the recording to cover `SetShift`, a meal, and a sleep.
- **`Crew.repair: Option<RepairJob>` generalises to `Crew.errand: Option<Errand>`** with
  `Repair`, `Meal` and `Bunk` arms, each carrying the floor and slot to stand at. Three
  parallel `Option`s alongside `task` is the alternative and it duplicates `repair_leg`'s
  routing three times; the enum keeps one router. `haul::resume`, `assign_idle`,
  `repair::pick_repair` and every test naming `crew.repair` move with it.
- **`CrewState` gains `Eating { ticks_left }` and `Sleeping`**, and the matches on it are
  exhaustive in more places than the enum's definition suggests: `haul::advance`,
  `snapshot.rs`'s `CrewStateTag`, `web/src/bridge/types.ts`'s `CrewStateTag` union, and
  `scene.ts`'s `drawCrew`, which switches on the tag string. A tag the renderer does not know
  draws as a standing figure, which is a graceful failure and also an invisible one.
- **`walk_ticks_per_slot` and `climb_ticks_per_floor` stop being fixed**, and the multiplier
  must scale the *duration* rather than the `Fx` step — the trap spelled out in §4.4. Applying
  a percentage to `Fx::ratio(1, ticks)` reproduces the truncation that made four terrain yields
  behave as two before M3 (`intake::terrain_effort`).
- **Sleeping crew must be excluded from everything that assigns work**, in `haul::assign_idle`
  and in `repair::pick_repair`. They hold no task, so `committed_pickup` and
  `committed_delivery` are already safe, but a sleeper who gets handed a repair is a crew member
  who works in their sleep.
- **`wait_ticks` must stay zero in the new arms.** `Sleeping` and `Eating` are not blocked
  states, and `CrewView.stressed` is driven purely by `wait_ticks` — a red-tinted sleeper would
  break the one instrument §8 rests on.
- **`state.rs`'s `add_crew` placeholder name list** becomes real content, still selected by
  index, never by a roll (§3.5). The comment there already says "placeholders until M4".
- **`power::lighting`** is the code the §4.4 decision is about: it is left alone, and the
  reasoning for leaving it alone belongs in a comment next to it, because "make lamps
  occupancy-gated" is the obvious next idea somebody will have.
- **`RoomCategory` gains `Quarters`**, so `content::validate`'s category-wiring check, the
  catalog's `RoomInfo`, the build menu, `roomColor` and `roomProfile` all need an arm. The last
  two fall through to a generic box, so the failure is a bunk that looks like a crate rather
  than a crash.
- **`SoundEvent` gains three arms**, mirrored in `web/src/bridge/types.ts`'s union, and
  `Game.ts`'s "sounds are produced and dropped until the audio pass in M4" comment becomes
  false — it is the one line in the frontend that names this milestone directly.
- **`CrewView` gains `hunger`, `rested`, `shift` and `asleep`** — a change to the bridge's
  public contract, and therefore a frontend change made deliberately rather than discovered
  (`AGENTS.md` §IV). `tests/snapshot.rs` guards the shape.
- **All four harnesses build towers with no canteen and no bunks.**
  `examples/siege_run.rs`, `examples/throughput.rs`, `examples/journey.rs` and
  `examples/record_golden.rs` each work a shopping list and then measure; every one of them will
  now measure a starving, exhausted tower and report it as an economy. This is the same shape as
  M3's fork omission — a harness that silently measures the wrong tower with total confidence —
  and it needs fixing in the same commit as the needs, not after the numbers come out wrong.
  `throughput.rs` is the most sensitive: it measures shaft contention over 600 s with crew whose
  walking speed M4 has just made variable.
- **`examples/journey.rs`'s shade-versus-sun comparison is the milestone's own instrument**, per
  §4.3, and its policies need the canteen in their shopping lists before the biomass axis can be
  said to have come alive.
- **`tests/balance_doc.rs` is bidirectional** — every new `balance.ron` field needs a graded
  `BALANCE.md` row in the same commit, and the content-constants group row needs the canteen and
  the bunk added.
- **`BALANCE.md`'s `crew_cap` row and its `storeroom` row both defer explicitly to M4**: the
  first for whether the rota earns the plan's eight, the second for what to do about shelf
  typing. Neither is optional to answer; the second is answered in §4.3 and §4.10.
- **Per-daypart elevator programs still have no UI.** They exist in the data model, the command
  layer and the replay format; §1.7 deferred the UI and said it "should land alongside M4's
  shift rota if not before", and §2.9 carried that forward unchanged. The rota's roster is the
  natural home for it — both are schedules written against the daypart clock — so M4 inherits
  it. **Done**, and one thing had to change in the bridge to make it possible: `ShaftView` did
  not publish the programs at all, so there was no way to *read back* what was set. A schedule
  you cannot see is one you cannot edit, and that is most of why this went three milestones
  without a UI despite the command existing the whole time. The editor shows the current
  daypart and edits that one rather than offering a grid of every daypart against every floor —
  a player setting a night program at midday cannot see what they are doing, and the version of
  this that is a spreadsheet is the version that gets built and never opened.
- **`web/e2e/capture.spec.ts`** gains the stills §4.9 needs. It is also the only tool the
  project has for answering a visual question, so anything in §4.5 that cannot be seen in a
  capture is not finished.

**What the list above missed, found by building it.** Four things, kept because three of them
are the kind of thing that is obvious afterwards and invisible before.

- **A run opened with its whole crew asleep.** The clock started at permille 0, which is
  predawn, which is the night band; every crew member defaults to the day shift; so the first
  2,592 ticks — 86 seconds at 1× — had nobody moving. Nothing in §4.4 is wrong, and the
  interaction is fatal anyway: the only reading available to somebody who has not yet been
  taught what a rota is, is that the game is broken. `Clock::new` now starts a run at the
  handover onto the day shift, found from the pack rather than hardcoded, and the tower sets
  out in the morning.
- **Stranded carriers had to be allowed to do everything except put the load down.** The
  ladder in §4.4 gates bed and meal on empty hands, which is right, and `pick_repair` had
  already carved out an exception in M3 for a carrier the tower has nowhere to put — no free
  shelf, no hungry room. Sleep and meals needed the same exception for a stronger reason:
  without it, a crew member stranded by the shelf-typing deadlock never slept and never ate
  *again*, and spent the rest of the run permanently tired, permanently starving, and working
  at 36% with the deadlock as the invisible cause. Measured, it was the difference between 14%
  and 41% of crew-hours spent asleep — the second figure being what the night band is actually
  worth.
- **A throughput window is a whole number of days, or it is a measurement of what time it
  started.** `throughput.rs` and the elevator test both used windows that were not, so once
  crew slept, two thirds of the measurement fell across the night when the tower does not
  queue. The elevator reported 19 crafts without against 18 with; the same two towers over a
  whole day read 18 against 36. The effect had not moved — the instrument had stopped pointing
  at it. This is the same failure as M3's fork omission, and it will recur every time the
  simulation grows a new rhythm.
- **The canteen's authored recipe contradicted its own design paragraph**, and the arithmetic
  in §4.3 was the thing that caught it. See §4.9.

- **`RoomDef.short` codes have to be unique and nothing checks it.** The bunk shipped `"BNK"`,
  which the cell bank already owned; invisible until the two stand on the same floor, at which
  point the cross-section shows two of the same room. Found by looking at `home-evening.png`.
  A content-validation rule would catch the next one, and is not written.

### 4.9 Exit criteria

Both of M4's criteria are answered by looking and listening. Neither can be asserted, and a
passing test suite is evidence about the code rather than about the game — which is why each
one below says what would actually demonstrate it.

- [~] **The eyes-closed test: can you hear how the tower is doing?** Demonstrated by a
      listener with the screen off, given three unlabelled sixty-second recordings from a real
      run, answering four questions about each: is it day or night; is the chain running or
      stalled; is something attacking; is the tower walking or stopped. Four binaries, twelve
      answers, and the criterion is getting them from sound alone. A listener who can tell day
      from night but cannot tell a working mill from a starved one has found that the loops are
      decorating the mix rather than reporting it.

      **There is no harness for this, and the honest version of one is buildable.** Playwright
      cannot assert audio. What it would have to be: replay the golden fixture headlessly,
      capture each frame's `SoundEvent[]` plus a once-a-second `ViewSnapshot` digest into a
      log, then drive the same `AudioManager` from that log under an `OfflineAudioContext` and
      render a WAV. That makes the soundscape deterministic, diffable, and reviewable without a
      browser — the audio counterpart of `capture.spec.ts` — and it would catch the regression
      nobody notices, which is a loop that stopped being wired to the state it claims to
      report. Until it exists, the criterion is answered by a person listening, and that should
      be said rather than implied.

      **Built, and unjudged.** `web/src/engine/AudioManager.ts` is the whole subsystem: nine
      continuous beds read off `view()` and nineteen one-shots fired from `frame()`'s event
      list, coalesced by kind within a frame and rate-limited per kind, all synthesised from
      oscillators and filtered noise so there is nothing to fetch and nothing to fail to load.
      The split the spec insisted on is the shape of the file — loops from the snapshot,
      one-shots from the events — and the diegetic rule holds throughout: no arm on a stalled
      room, no beep on a queue, and the legs read `journey.halt`'s five states rather than
      `power.walking`, so a tower that stopped and a tower that cannot afford to move do not
      sound the same. The three new events are wired: `MealServed`, `ShiftChange`, and
      `EnemyLeaves`, the last of which closes M2's deferral by making a creature that walked
      away audibly different from one that was shot down.

      **The harness got built, and it found the mix was failing.** `web/e2e/audio.spec.ts`
      does what the paragraph above sketches: steps three sixty-second excerpts at 1×,
      drives the *shipping* `AudioManager` from them under an `OfflineAudioContext`, and
      writes `capture/audio/{a,b,c}.wav` plus per-second energy in five bands. The four
      binaries are then answered from those numbers alone, using discriminators fixed in
      advance from the mixer's own documented band layout, and only then marked.

      First run: **three of the four were unrecoverable, and two were backwards.** A fully
      working tower and a completely switched-off one differed by 6% in the band the
      production loops own, and a *stopped* tower read louder in the leg band than a walking
      one. The wiring was right the whole time; the mix was wrong, in three ways that are
      worth writing down because none of them is audible as a bug — they are audible as
      "atmospheric".

      1. **A `lowpass` on white noise is not a bed, it is a wall.** The jungle sat on a
         lowpass at 620 Hz, which keeps *everything* underneath it, so one decorative loop
         held eight times the energy of every state-carrying loop combined. Beds are
         bandpassed now, and the bottom of the spectrum is left to the two things that mean
         something down there.
      2. **A pure tone beats broadband noise for presence at a fraction of the amplitude.**
         The electrical hum is a 50 Hz sine and the legs are filtered noise; at similar gains
         the hum owned the whole bottom octave and "is the tower walking" had no answer. The
         hum is a fifth of what it was.
      3. **Combat was quieter than the chain.** Impacts were written at roughly a mill's
         level, which is wrong twice: the chain fires constantly and a bite does not, and
         something biting your home should be the loudest thing in the frame. A minute with
         a creature on the tower for a fifth of it was *indistinguishable* from a quiet
         minute.

      After those three: **12 of 12.** Night separates 2.3× on the ratio between the two
      insect bands, a working chain 13× on the band the room loop owns, walking 12× on the
      leg band, and a wave shows as a burst well above the clip's own floor.

      One nuance the harness surfaced rather than papered over. The excerpt scored as "quiet"
      turned out to have had enemies out for a third of it — a wave had announced itself on
      the horizon — and the audio said so. Scored against *contact*, that reads as a false
      positive; scored against the question a listener is actually answering, it is correct.
      Both numbers are reported.

      **Still `[~]`, because none of that is listening.** A signal can carry a fact and still
      not be legible to an ear, and nothing here says whether the tower sounds *good*. What
      changed is that "is it in there" is now answered, checkable, and regression-tested.

- [~] **The screenshot test: does one frame say "solarpunk home, not war machine"?**
      Demonstrated by two stills from `web/e2e/capture.spec.ts` shown to somebody who has never
      seen the game, asked only "what is this place?". `home-evening.png` — dusk, lamps on,
      the canteen's hearth lit and steaming, two crew sitting to a meal, one asleep in a
      hammock, planters full, the jungle going blue behind it — has to come back as somebody's
      home, greenhouse, or ark. If it comes back as a factory, a rig, or a gun platform, the
      pass failed. `home-siege.png` — the same tower mid-wave — is the control: it should read
      as a home under threat, not as a fortress that has finally found its purpose.

      A frame with no people in it cannot pass, which is why crew-as-people (§4.5 item 1) is
      the load-bearing item in the art pass rather than the overgrowth.

      **Both stills exist and both have people in them**, which took more of the harness than
      expected and is worth recording, because the same trap is waiting for the next visual
      criterion. A capture that walks to roughly the right hour and photographs whatever is on
      screen gets a still of three figures standing in a corridor: sleep is easy to catch (the
      night band is a third of the day) but a meal is 300 ticks out of 4,800, so a loop that
      stops at the first interesting state it sees *always* stops on a sleeper. `capture the
      home` now holds out for both and bounds the search to one night band, and prints what the
      crew were actually doing so a still that failed to find a meal says so instead of being
      filed as though it had. It currently reports `climb/sleep/eat`: somebody on the stairs,
      somebody in a hammock, somebody sitting to a bowl.

      Three things the stills caught that no test would have. The bunk and the cell bank both
      shipped `short: "BNK"`, which is invisible until they stand on the same floor and then
      reads as two of the same room — the bunk is `"BED"` now. The harness's build helper
      compared `send`'s result against `null`, which is never what it answers, so every attempt
      read as a failure and the loop bought *four canteens*. And **the crew were dressed in
      almost exactly the colour of the tower's unlit interior** — a muted forest green, which
      is what people in a jungle would sensibly wear and which made them invisible in the one
      frame whose whole job is having people in it. They wear sun-bleached linen now and stand
      against a soft dark halo. All three were found by looking at the picture, which is the
      argument for having the picture.

      **Still `[~]`: nobody who has not seen the game has been shown them.** That is the
      criterion, and it cannot be self-assessed — the whole point of asking a stranger "what is
      this place?" is that the person who drew it already knows the answer.

- [x] **The roster carries both schedules**, and `the roster writes both of the player's
      schedules` in `web/e2e/smoke.spec.ts` drives them through the DOM the way a player does
      rather than through the bridge, because a panel can look right and be wired to nothing.
      It caught one thing immediately: the diagnostics readout sits in the same corner and was
      silently eating clicks on the bottom of the crew list — the button highlighted, nothing
      happened, and there was no way to tell that from a rejected command. It is
      `pointer-events: none` now.

- [x] **The kitchen chain has visibly given bamboo somewhere to go.** `examples/journey.rs`'s
      shade-versus-sun comparison reported *exactly* 100 stalks on both routes at M3 and
      **576 against 539** now, on two routes 8% apart in weighted yield. M3's biggest open
      finding is closed.

      It took three fixes rather than one, and only the first was the one §4.3 predicted.
      **The canteen was authored three times too cheap** — 2 bamboo for *three* meals, against
      a design stated twice as "2 bamboo a meal … six a day a crew member … about a fifth of
      what a fed mill draws" — so a crew member ate two stalks a day and the canteen drew 5% of
      a mill. **The harness had no standing shopping list**, so its towers built two rooms,
      stopped wanting anything, filled every buffer and reported the size of their own shelves
      as a harvest; `siege_run.rs` had diagnosed exactly that for provocation and fixed it
      there, and this is the same fix arriving late. And **stranded carriers never ate**: the
      priority ladder gated eating on empty hands, so a tower deadlocked on shelf typing had
      crew holding a crate forever, never eating, never sleeping, and working at 36% with the
      deadlock as the invisible cause.

      The order matters for anyone re-reading this: the canteen's price was the fix that moved
      the number (353→576), and the other two were what stopped it moving before that.

- [x] **The siege balance is re-earned**, at a cost: `provocation_per_100_harvested` moves
      240 → 300 and the three-way shape returns — subsistence never rises above 6 and ends
      whole at 999‰, greedy peaks at 56 and is eaten down to 62‰, answered holds 809‰ and sees
      off 15.

      The interesting part is why break-even arithmetic gives the wrong answer here. **Sleep
      made provocation spiky.** Harvest now arrives entirely inside the day band while decay
      runs around the clock, so what draws a wave is the mid-afternoon *peak* rather than the
      daily mean, and the two stopped moving together. Break-even says 370; at 370 a
      subsistence tower's peak cleared the wave threshold every afternoon and it took 210 hit
      points on day one, which is not what living within your means is supposed to buy.

      `BALANCE.md`'s Siege section no longer opens with the stale-economy warning, and carries
      two things it explicitly does *not* act on instead: the pressure table has dropped a band
      (mending is crew-hours, and sleep took two-fifths of them), which stays coherent only
      because the provocation a real tower reaches fell by the same order; and the plating
      comparison came back "worse on 7 of 8 seeds" for the third time, which is still an
      artefact of comparing hit-points-missing between towers whose maxima differ by the
      plating under test.

- [x] Golden replay regenerated — it now covers a canteen, two bunks, a `SetShift`, 26 meals
      and 72,854 crew-ticks asleep — and hash parity is green natively and in wasm.

- [x] `make check` and the smoke suite green: 274 Rust tests, clippy clean, 8 Playwright
      tests including native/wasm parity.

**Deferred out of M4:**

- **Role priorities.** `v2-plan.md` §6.7 describes "haul / operate / gun / repair, as role
  priorities per crew member (RimWorld-lite, one screen)". §9's M4 brief does not list them,
  and they are cut deliberately rather than overlooked. There are two kinds of work in the
  game — hauling and mending — so a priority list would have one meaningful row in it, and a
  per-crew priority screen is exactly the kind of menu `DECISIONS.md` §8 argues against when the
  diegetic version already exists: `repair_hp_per_shift` and the carrying rule in
  `assign_idle` already encode a triage policy that the player shapes by what they build. When
  there are four kinds of work — M5's tier-two chains and second emplacement — the row count
  might justify the screen.
- **Positional audio.** Reasoned about and cut in §4.6: `SoundEvent` stays payload-free.
- **~~An audio regression harness.~~** Built after all — `web/e2e/audio.spec.ts`, and it
  earned its keep immediately by finding that three of the four eyes-closed binaries were
  unrecoverable from the mix (§4.9). What is still deferred is the *regression* half: the WAVs
  are rendered and measured but nothing compares them against a stored baseline, so a mix that
  drifts will be noticed by whoever next reads the numbers rather than by CI.
- **Occupancy-gated lighting.** Reasoned about and rejected in §4.4. Recorded because it is the
  obvious idea and the reason not to do it is not obvious.
- **Crew that wake themselves.** Rejected in §4.4: an automatic wake on attack would make the
  rota decorative.
- **Anything that makes a well-fed crew better than baseline.** §4.4: needs are a penalty for
  neglect, never a buff to chase.

### 4.10 Open questions

Things that genuinely cannot be settled without building them.

1. **Does the rota read as a decision, or as an administrative chore?** The argument in §4.4 is
   that day throughput against night cover is a real trade with two named costs. The risk is
   that a player finds one answer, sets it once, and never thinks about it again — at which
   point the rota is a settings screen that cost a milestone. The tell to watch for is whether
   anybody ever uses the surge lever, since that is the only part of the rota that is a
   decision made *during* a run rather than at the start of one. If nobody does, the fix is
   probably to make the night genuinely more dangerous, not to make the rota more complicated.
2. **How much does sleep actually cost, and is the tower still playable at three crew?**
   Removing two-fifths of crew-hours is the largest single economic change in M4 and it lands
   on the economy `starting_crew` was measured against — where going from two crew to three
   moved throughput ninety percent (§1.7). Three crew all on the day shift is not three crew
   any more. The honest possibilities are that `starting_crew` has to rise, that the day band
   has to widen, or that the whole thing is fine because the chain was never crew-limited at
   night anyway. Only the harnesses in §4.8 can say which, and they cannot say it until they
   have a canteen in them.
3. **What is the answer to shelf typing?** `BALANCE.md`'s `storeroom` row hands M4 the deadlock
   where bamboo claims every shelf and the chain stops, and meals make it a five-item
   competition on four shelves. §4.3 makes it non-fatal — nobody starves, because crew eat at
   the canteen — but non-fatal is not solved. The candidates are a player-set item filter per
   shelf (a real infrastructural verb, and a new command), more shelves per storeroom (which
   postpones rather than fixes), or a chute that dumps surplus (M5 scope). The filter is the
   most likely right answer and the most likely to be scope creep; deciding is an owner call,
   and leaving it undiagnosed is the one option that row already ruled out.
4. **Can the eyes-closed test be passed without any sound the diegetic rule would forbid?** The
   rule says a starved mill goes quiet. A tower with many problems therefore sounds like a
   tower with nothing happening, and *quiet* is a hard signal to distinguish from *fine* with
   your eyes shut. The intended answer is that the beds underneath — the jungle, the day and
   night soundscapes, the electrical hum thinning as the bank drains — keep the mix from ever
   being silent, so absence reads against a floor rather than against nothing. Whether that is
   enough is the question the recordings answer, and if it is not, the temptation will be a
   warning tone. That is the wrong fix, and it is worth writing down now, while nobody is
   frustrated.
5. **Do barks survive contact with repetition?** A run is two to four hours and there are up to
   eight crew. Lines that charm on the first hearing are the ones that grate on the fortieth,
   and rate-limiting them into rarity is the standard answer and also the answer that makes
   them stop doing their job. This cannot be settled from the desk, only by hearing the same
   line for the twentieth time and noticing how it feels.

---

## M5 — The Refugia *(depth within rules)*

**Sprint question:** does a second tier add depth, or does it just add rooms?

**Scope:** the tier-two chains `v2-plan.md` §6.1 has carried since the plan was locked, and the
raw materials they turn out to need; chutes; the rest of the creature taxonomy; region 3 and
the arrival that ends a run well; an enclave economy that makes `crew_cap` reachable;
toolkit-widening unlocks and the delivery mechanism §11 left open; balance telemetry and the
difficulty pass that turns `DESIGNED` into `PLAYTESTED`; and the itch.io release cut.

**The content gate is absolute, and it is the whole of what keeps this milestone honest:**
*nothing ships unless an existing system consumes it at runtime.* v1 authored sixteen modifier
effects and applied zero, and that is the failure this rule exists to prevent (`v2-plan.md` §10
rule 1). Every item below names its consumer in the same sentence that introduces it, and
anything that cannot name one is cut here rather than discovered dead later.

### 5.1 The shape of the thing

**M5 is where the route stops being one axis.** Since M1 the world has offered a single trade —
shade is biomass-rich and sun-poor, open ground the reverse — and M4 finally made the biomass
half feelable by giving bamboo a consumer that scales with the crew (§4.3). But the sun half
still only buys *charge*, which is a means rather than an end, so a sun-seeking route reads as
a sacrifice with a rebate rather than as a different way to play.

The tier-two chains fix that by hanging a second material on the other end of the same axis.
Shade grows bamboo; open sun grows **produce**; the ruin belt yields **scrap**, which has been
harvestable since M3 and, until now, has had *no chain consumer at all* — it dead-ends at the
enclave's trade board, which is a fine thing for a material to also do and a poor thing for it
to only do. Three materials, three parts of the map, three tiers built on top of them. That is
the depth M5 is for.

**What this milestone must not become is a wider menu.** Rule 2 of `v2-plan.md` §10 —
"core-mechanic depth beats content breadth" — is the one most at risk here, because a tier list
is the easiest thing in the world to keep extending. The test each new room has to pass is not
"is it interesting" but "does it change a decision the player was already making". A room that
only adds a step to a chain is breadth wearing depth's clothes.

### 5.2 The materials, and what eats them

Chain depth stays at three and no recipe takes more than two inputs (`v2-plan.md` §6.1). The
full pack after M5, with the new entries in bold:

| Tier | Item | Made by | Consumed by |
|---|---|---|---|
| raw | `bamboo` | cutter arm, shade-weighted | mill, canteen, burner |
| raw | `scrap` | salvage rig, at ruins | **sun-forge** (new) |
| raw | **`fiber`** | **fiber comb** (new), mid-band-weighted | **ropery** (new) |
| raw | **`produce`** | **garden** (new), sun-weighted | **bombary** (new) |
| T1 | `poles` | mill | construction, repair, thornwright, **fitter** |
| T1 | `meals` | canteen | crew, three times a day |
| T1 | `darts` | thornwright | dart battery |
| T1 | **`rope`** | **ropery** (new) | every shaft and every emplacement's build cost |
| T1 | **`alloy`** | **sun-forge** (new), charge-hungry | **fitter**, **cellwright** |
| T2 | **`mechanisms`** | **fitter** (new) | elevator build cost, **seed thrower** (new) |
| T2 | **`seed bombs`** | **bombary** (new) | seed thrower, as ammo |
| T2 | **`charge cells`** | **cellwright** (new) | cell bank build cost |

**Meals stay on bamboo, and that is a deliberate departure from `v2-plan.md` §6.1**, which
lists them as `produce`→kitchen. M4 built them from bamboo for a specific reason — bamboo had
no consumer and the biomass axis was therefore unfeelable (§4.3) — and the measurement that
justified it is on the record: two routes eight percent apart in weighted yield harvested 353
stalks against 351 before the canteen was priced properly, and 576 against 539 after. Switching
meals to produce now would hand that back. Produce gets seed bombs instead, which is a better
job for it anyway: a material that grows in the sun feeding a weapon that denies ground is a
cleaner opposite to bamboo feeding poles than two food chains would be.

**Three of these rooms are the point and three are plumbing.** Worth being honest about which:

- The **sun-forge** is the point. It is the first room that turns a raw material the tower
  cannot grow into something the chain needs, it is charge-hungry enough to compete with
  striding, and it gives every ruin the tower walks past a second reason to stop. It is also
  what makes the drowned city a destination rather than a corridor.
- The **garden** is the point. It produces on sunlight rather than on ground covered, which
  makes it the first intake a *stopped* tower runs at full rate — the exact inverse of the
  cutter arm (§3.6), and therefore the first real argument for standing still.
- The **fitter** is the point, because mechanisms gate the elevator (§5.3).
- The **ropery**, the **bombary** and the **cellwright** are plumbing: one input, one output,
  no decision of their own. They earn their slots by what they feed, and if any of them reads
  as a step rather than as a choice in play, the right answer is to fold its output into an
  existing room rather than to make it more interesting.

**Opening values**, as design targets with an argument behind them rather than measurements.
None is a `BALANCE.md` row until it is authored, at which point it gets a graded row in the
same commit (`DECISIONS.md` §7).

| Thing | Target | The arithmetic |
|---|---|---|
| fiber comb | Intake, 2 slots, 4 poles, `paces_per_item` 90 | A shade under the cutter arm's 78, because fiber is the *second* thing a band gives up. Weighted to clearing and drowned street, so the bands poorest in bamboo are rich in something. |
| garden | Intake, 2 slots, 5 poles, `ticks_per_item` 260 scaled by exposure | **Per tick, scaled by sun, not per pace.** A stopped tower harvests nothing today; a garden keeps working while the legs are off, which is what makes berthing cost less than it does now. At full sun that is one produce every 8.7 s; under dense canopy it is nearly nothing. |
| ropery | Production, 2 slots, 4 poles; 2 fiber → 1 rope, 120 ticks | The mill's price and rhythm exactly, because it is the mill's opposite number and the two should feel like siblings. |
| sun-forge | Production, 2 slots, 8 poles; 3 scrap → 1 alloy, 240 ticks, `power_draw` 4 | Four times the thornwright's draw and the largest single sink in the tower: a forge running flat out costs more charge across a day than continuous striding, so *running the forge* and *walking far* become the same decision. Eight seconds a bar makes it visibly the slowest thing in the chain. |
| fitter | Production, 2 slots, 6 poles + 4 rope; 1 alloy + 2 poles → 1 mechanism, 300 ticks | The first build cost that is not poles alone, and the first recipe drawing two inputs from different chains. |
| bombary | Production, 2 slots, 6 poles + 4 rope; 2 produce + 1 fiber → 2 seed bombs, 200 ticks | Cheaper per shot than darts and slower to make, so a thrower is the answer to *many* things rather than to one hard thing. |
| cellwright | Production, 2 slots, 6 poles; 2 alloy → 1 charge cell, 300 ticks | Storage is built, so the tower's charge ceiling becomes something the chain earns rather than something poles buy. |
| seed thrower | Defence, 2 slots, 6 poles + 3 rope + 2 mechanisms; 1 seed bomb a shot, 8 damage across a 30-pace band, `reload_ticks` 90 | **Area, not aim.** The dart battery answers one creature well; the thrower answers a wave badly and cheaply. Two emplacements with different failure modes is what makes "what do I build" a question at all. |
| `rope` in shaft costs | stairs 0, dumbwaiter +3, elevator +6 | Shafts stop being a pure pole cost, so the material easiest to get in the *middle* bands is what buys vertical transport. A tower that never leaves the canopy can afford poles and not rope. |
| `mechanisms` in the elevator's cost | 2 | **The elevator becomes a tier-two building**, which is the sharpest single expression of "depth within rules": M1's centrepiece stops being something a starting tower can rush, and the route that reaches it runs through the ruins. |
| `charge cells` in the cell bank's cost | 2, replacing 8 poles | See above: batteries are built. |

### 5.3 What each tier-two item is *for*, stated as a consumer

The content gate applied one item at a time, because "it feeds the next room" is not a
consumer, it is a postponement.

**`mechanisms` gate the elevator and the seed thrower.** This is the load-bearing one. Until M5
the elevator is 18 poles and a decision about *when*; after M5 it is 18 poles, 6 rope and 2
mechanisms and a decision about *whether the route you took can build one at all*. A tower that
walked the shaded branches every time has bamboo and no scrap, so no alloy, so no mechanisms,
so it climbs its stairs. That is route choice reaching all the way into the transport layer,
which is where this game's thesis lives.

**`charge cells` are the tower's charge ceiling.** A cell bank costing cells rather than poles
means the answer to "I keep browning out at night" stops being "spend poles" and becomes "run
the forge, which costs charge" — a loop to be climbed rather than bought out of. It is the first
place in the game where fixing a problem costs the resource the problem is about.

**`seed bombs` are the second emplacement's ammo**, and the second emplacement exists to give
defence a *shape* rather than a level. A dart battery kills one thing at a time and is the right
answer to a borer at a shaft column; a thrower scatters and is the right answer to four skitters
on a panel. Neither is an upgrade of the other, which is the only way a second anything is
allowed to exist here.

**`rope` is what every shaft and every emplacement is partly made of.** Not a tier so much as a
second construction currency, and its job is to stop poles being the answer to everything. Fiber
comes from the middle bands, so the tower that can build transport is the one that walked
through ordinary ground rather than optimising for either extreme.

**`alloy` is the only thing that consumes scrap**, and therefore the reason to berth. §3.4 built
berthing, wardens and the salvage rig, and §3.11 asked whether stopping at a ruin is ever
genuinely "it depends" rather than always yes or always no. It could not be: scrap bought poles
at the enclave and nothing else, so the answer was "yes, if you happen to be passing". With a
forge aboard, scrap is the input to the two things that gate the elevator and the charge
ceiling, and the question becomes a real one.

### 5.4 Chutes, and the answer to shelf typing

`BALANCE.md`'s `storeroom` row has described the sharpest emergent failure in the game since M2
and handed the fix forward twice: a shelf takes whichever item lands on it first and holds only
that until it empties, so a material arriving faster than it is consumed claims shelf after
shelf until nothing else can be put down and the chain deadlocks. §4.3 made it non-fatal — crew
eat at the canteen, so nobody starves — and said plainly that non-fatal is not solved. §4.10's
third open question named three candidates and called the decision an owner call.

**The call: a chute, which is the candidate that is a piece of infrastructure rather than a
setting.** A chute is a shaft kind (§1.3) with no cars, no capacity and no charge draw — things
fall down it — ending at a spill gate on the ground floor. Anything a crew member drops in
leaves the tower. Haul gains one destination of last resort, below a shelf in priority: an item
with no hungry inbox and no free shelf goes down a reachable chute rather than stranding whoever
is holding it.

Why this rather than a per-shelf item filter:

- **It is a thing you build, not a menu you set.** The player answers the deadlock by spending
  slots and materials on a piece of the tower, which is how every other problem in this game is
  answered. A filter would be the first place the answer was a dropdown.
- **It costs something ongoing.** A chute is a slot column on every floor it spans, like every
  other shaft (`DESIGN.md` pillar 2), and what goes down it is gone. Surplus bamboo dumped is
  bamboo not milled later.
- **It is visible.** You can watch the overflow leave, which puts the deadlock's *cause* on
  screen rather than leaving it to be inferred from a chain that stopped.
- **It composes with the stranded-carrier rule.** M4 already lets a stranded carrier sleep, eat
  and mend while holding a load they cannot put down (§4.8); a chute turns that from a
  survivable dead end into something the player can fix.

The filter is not rejected as a bad idea, only as a worse first one. If chutes ship and towers
still jam — because the surplus is something you *wanted* and a chute is too blunt — a per-shelf
filter is the next thing to try, and it will land on a game that has already made overflow
visible, which is a better game to add it to.

| Thing | Target | The arithmetic |
|---|---|---|
| chute | Shaft, 1 slot column, **6 poles, no rope**, no charge, no capacity | Cheaper than a dumbwaiter (8 poles) because it does less: one direction, no machinery, nothing comes back up. **Specced at 6 poles + 2 rope and shipped without the rope**: rope needs a ropery, a ropery needs fiber to reach it, and fiber having nowhere to go *is the jam* — so the rope made the escape hatch affordable only before you needed it. Everything else in the pack may sit behind a chain; this one may not. |
| spill priority | below `PRIORITY_SHELF` | Never preferred to somewhere useful. A chute is where things go when there is nowhere else, and a tower with spare shelf space should never spill. |
| what may be spilled | nothing a live inbox wants, nothing anything is **built** with, nothing a **settlement takes** | The third clause was missing and a chute was eating salvage. `wanted` asked two questions — does a live room's inbox take it, and is it a build cost — and **scrap answers no to both**: its only room consumer is the sun forge, so a tower without one has no inbox wanting it, and nothing is built from it. Yet scrap is the entire point of berthing at a ruin (§3.4). A player with a chute stopped, woke the wardens, took the damage, collected the scrap, and watched their crew carry it out of the tower, with nothing on screen saying so. The gap is structural rather than an oversight: an enclave's `Trade`, `Recruit` and `Reinforce` are **commands**, so what they consume appears in no room's inputs and is invisible to a check that only reads rooms. `Content::settlements_take` closes it. Deliberately not conditional on a settlement being *in reach* — "there is no buyer within forty minutes" is not a reason to throw something away, and a chute that reasoned that way could not be planned around. |
| what is left spillable | fiber, darts, meals, seed bombs, charge cells | Narrow on purpose, and narrower than it looks: rope is a build cost, so it was already safe; fiber is spillable only while no ropery is running, which is exactly the case §5.4 was written about. **Caught by the screenshot harness rather than by a test** — the capture run berthed, salvaged, and then photographed an enclave board it could not afford to buy from. |

### 5.5 The rest of the taxonomy, region 3, and the Refugia

**Two creatures complete the set**, and both exist to attack something no current creature
does. The four shipped kinds all converge on the tower and bite it; what is missing is a threat
to the *chain* rather than to the structure.

| Creature | Shape | Why it is not a fifth biter |
|---|---|---|
| **glean-crow** | Fast, fragile, `min_provocation` ~250. Lands on an outbox, takes what is in it, leaves. | Attacks throughput rather than hit points. The tower is undamaged and the day's harvest is gone — a loss repair cannot answer and defence can. |
| **mire-hulk** | Very slow, very tough, region 3 only. Grapples a *leg* and slows the stride while attached. | Attacks the journey. The one creature that cannot be walked away from, because it is what is stopping you walking — the counterpart to the warden, which is what stopping wakes. |

**Region 3 is the coast approach**: the canopy thins, the ground opens, sun is abundant and
biomass is poor. Mechanically it is the mirror of region 1, which is what makes a run's shape an
arc rather than a ramp — a tower tuned for shade arrives somewhere its habits do not work, and
the garden that was marginal in the jungle is what keeps it fed. The mire-hulk lives here, and
so does the last stretch of walking.

**The Refugia is an arrival, not a victory.** §3.7 already ends a run two ways and presents the
far edge as somewhere the tower got to rather than something it won (`DECISIONS.md` §8); M5
changes the destination from an edge to a place. What arriving shows: the tower, stopped, with
whatever it still has aboard; the crew by name and what became of them; the route it walked as a
line through three regions; and the seed, so the run can be handed to somebody else. **No score,
no rank, no stars.** A run's ending is a description.

**The name stays "the Refugia."** `v2-plan.md` §11 open #1 marked it provisional and asked for a
decision during M3's world-writing, which never happened because M3 never reached region 3.
Deciding it now, by keeping it: it has been the word in every document for the whole project, it
means what it should mean, and a rename at this point would be churn for its own sake.

### 5.6 The enclave economy

`BALANCE.md`'s `crew_cap` row raised the ceiling to eight at M4 on the strength of the rota and
noted that a run cannot approach it, because M3's single enclave offers exactly one recruit. M5
is where that stops being aspirational.

- **An enclave in every region**, three in a run rather than one, each with its own board.
  Ropewalk in the jungle at 30,000 paces, High Water in the drowned city, Tidewatch on the coast.
- **Boards sell what their region has and buy what it has not.** Ropewalk lays rope and cuts
  thorn darts and has no metal at all, so it sells both for timber and buys scrap. High Water and Tidewatch
  are the reverse: they sit on metal and want timber, alloy and produce. A player carrying a
  surplus finds a buyer for it somewhere, which turns "I have too much of this" from a jam into a
  plan.

  *Corrected against the pack, which said the opposite.* An earlier draft here had the drowned
  city "wanting poles and selling scrap". It does not and should not: the city's `scrap 4 →
  poles 3` is the payoff for the salvage rig and the reason scrap exists at all (§3.4), and
  reversing it would take the point out of berthing at a ruin. **Scrap has one price at every
  board on purpose** — 4 for 3, everywhere — so there is nothing to buy in one region and sell in
  the next, which is §5.11 open question 3's whole worry.
- **Ropewalk exists so region 1 can reach the elevator.** An elevator costs six rope; rope costs
  a ropery, a ropery costs fiber, and fiber costs a comb. That put vertical transport — the bet
  the whole game rests on (`DESIGN.md` pillar 2) — three rooms deep on a tower twenty minutes
  old. Six poles a pair, four pairs, is one elevator and two spare — and poles are what the shaft's own
  frame costs, so the rope competes with the thing it unlocks. The chain stays far cheaper for a
  tower that means to keep using rope.
- **Recruits cost more each time**, so crew growth is a curve rather than a switch and the
  eighth crew member is a decision about a whole run's savings. Ropewalk 18 poles, High Water 30,
  Tidewatch 24 poles *and* 4 alloy, three times over.
- **And the arithmetic has to close, which is why there are three.** `crew_cap` is 8 and a run
  starts with 3. Two settlements offer four recruits between them, so the cap was unreachable by
  one — a ceiling nothing can touch is a number pretending to be a decision. Five offered makes
  eight possible for a run that spends everything on people, and nothing else.
- **Shell work stays region-two only.** It was withdrawn once for making the tower worse and put
  back with a measurement (`BALANCE.md`'s `reinforce` row); spreading it across three enclaves
  would multiply a thing that is barely worth its price.

#### 5.6.1 The wayhouse — proposed, and **not yet decided**

> **Read §5.11 open question 0 first.** This section was drafted to answer that question and it
> does not answer it. The measurement that closed the question also showed that **a buyer would
> raise a tower's harvest by about a quarter and change the route comparison by nothing at all**,
> because yield and sun were already cancelling on purpose. So a wayhouse has to be justified as
> a mechanic somebody wants to play, not as a fix for a defect — there is no defect. Recorded in
> full because the design is sound and the reasoning behind its shape is still worth having.

**Three enclaves with finite boards are three events.** The §5.6 bullet above promises that a
player carrying a surplus "finds a buyer for it somewhere, which turns 'I have too much of this'
from a jam into a plan," and a buyer that does that has to be there when the jam happens — on the
road, continuously, for the whole run. That much stands regardless of open question 0.

The constraints on its shape are real and were expensive to find:

- **A buyer that pays in items moves the jam one step along.** Poles leave, whatever came back
  fills a shelf instead, and the mill stalls a week later rather than a day later.
- **A sink bounded by crew count is bounded.** That is why the canteen fixed meals and fixed
  nothing else: crew eat three a day for ever, but four people only eat twelve.
- **A sink that scales with tower size × time is unbounded, and is the overgrowth prototype**
  (`4a4683a`), which works and is not shipped because a tower that is never quite whole cannot
  tell you the difference between "something is attacking" and "it is Tuesday."

That leaves one axis: **distance walked.** A buyer that recurs down the road has a total appetite
that grows with the run, without any single one of them being a bottomless pit.

**The proposal: a wayhouse, and a deck to trade from.**

- **A wayhouse is a world feature**, generated per band the way ruins are, in every region. Each
  carries a finite pool the way a ruin carries salvage, and depletes as it is traded with. They
  recur, so what scales is the run rather than the building.
- **Berthing stays implicit** (`room.salvage_rig`'s comment is the precedent): a tower stopped
  within range of a wayhouse, with a deck that works, is trading. There is no Berth command and
  no Trade command here — stopping next to a wayhouse with no deck does nothing, and the empty
  space in the build menu is the affordance.
- **It buys, it never sells.** One direction only. `Trade` at an enclave stays what it is — an
  event, with unique goods and finite stock — and the wayhouse is the routine, boring counterpart
  that is always there. Nothing has two prices anywhere, so there is nothing to arbitrage, which
  is open question 3's whole worry.
- **What it pays in is charge.** Charge is the only resource in the game consumed continuously
  and for ever, by the legs, every tick, so it is the one payment that cannot back up into a
  shelf and re-create the jam. It also points the mechanic at the trade the game is already
  making: shade gives ground and takes power, and a wayhouse lets a shade route convert what the
  ground gave it back into power. **This does not widen the route gap** — the ceiling measurement
  in §5.11 rules that out — but it does let a player *choose* which side of the opposition to
  lean on, which is a different and better claim than the one this section originally made.
- **It costs the same hands as everything else.** Poles reach the deck by crew haul, on the same
  stairs, competing with every other errand (`DESIGN.md` insight 1). A wayhouse is not a drain
  bolted to the side of the tower; it is another mouth on the same shift.

**The trade this creates, stated plainly:** stop, and spend walking time to convert timber into
power. Walk on, and keep the time but leave the surplus jamming your shelves. That is the same
stop-or-walk decision the salvage rig introduced at M3, pointed at the other end of the chain,
and it is why the deck is a built room rather than a free action.

### 5.7 Unlocks, and how they arrive

`v2-plan.md` §11 open #2 asks for a decision between enclave gifts, Heartseed cultivars and a
journal. **The call: a journal, kept by the crew.**

- A run ends. The journal gains an entry naming something the tower did that it had not done
  before — reached the drowned city, ran a forge, walked away from a warden, fed eight people.
- Entries unlock **rooms and route options, never numbers.** `v2-plan.md` §3's structural call
  is absolute here: a first-run tower and a fiftieth-run tower start identical, and the veteran
  has more tools rather than better ones.
- Unlocked content is added to the build menu, so the surface a new player sees is small and the
  surface a veteran sees is wide, and neither is stronger.

Why the journal over the other two: an enclave gift makes the unlock a thing that happened
*during* a run, so a run's outcome depends on whether the gift turned up, which is the seed
deciding the meta. A Heartseed cultivar makes the unlock a property of the tower, which reads as
a stat even when it is not. A journal is a record of what you did, it is legible without a
tutorial, and it is the only one of the three that cannot be mistaken for power.

**Determinism.** The journal is player-level state, not run state. It never enters `GameState`,
never enters the replay, and a shared seed reproduces a run regardless of who plays it — which
is `v2-plan.md` §6.6's promise and would be silently broken by any unlock that changed what a
seed generates. What an unlock changes is which commands the *player* may send, and a replay
carries the commands.

### 5.9 What M5 changes in code that already exists

Not a task list — the places where existing code assumes something M5 stops being true.

- **`IntakeSource` gains a third arm.** It is `Terrain { paces_per_item }` and
  `Ruin { range_paces }` today; a garden is neither, because it accrues per *tick* scaled by
  exposure. That is a new shape rather than a new parameter, and `intake.rs` grows a third
  branch. The trap is `terrain_effort`'s: scale the *interval*, never the per-tick `Fx` step
  (§4.4).
- **`ShaftKind` gains `Chute`**, the first shaft with no capacity, no charge draw and no riders.
  `transport.rs`, `haul::best_shaft` (crew must never route *through* one) and `scene.ts`'s
  shaft drawing all switch on kind.
- **`HaulDestination` gains `Spill`**, below `Shelf` in priority, and `find_destination` learns a
  third case. The stranded-carrier rule (§4.8) stays exactly as it is — a chute is a
  destination, so a carrier who can reach one is no longer stranded, and one who cannot still
  sleeps and eats while holding.
- **Build costs stop being poles-only in practice.** `check_stock`/`spend` have taken a list
  since M0, so nothing structural changes — but every harness, every capture script and
  `place_when_affordable`'s wait-for-the-money loop currently reason about one material. A
  shopping list that waits for poles and needs rope waits forever.
- **`RegionDef` count goes from two to three**, and the contiguous-`order` check, the journey
  roll, and `state.rs`'s `enclave_stock` — sized for one enclave with a comment saying it becomes
  a list per enclave when a second lands — all move.
- **Two more `SoundEvent` arms** (`Spill`, `Steal`) and their mirrors in `types.ts`.
- **`CatalogSnapshot` grows**, and the build menu's grouping starts to matter at ~18 rooms in a
  way it does not at 12.
- **Every balance harness needs the new rooms in its shopping list**, which is the third time
  this has been true (§3.9, §4.8) and the reason it is written down again: a harness that
  measures a tower without the milestone's rooms in it measures the previous milestone with
  total confidence.

### 5.10 Exit criteria

- [~] **A full run to the Refugia in 2–4 hours**, played rather than scripted, ending as an
      arrival rather than a score.

      **The length is measured and it is right; the *played* half still needs a person.**
      `examples/journey.rs`'s "how long is a whole run" walks every seed start to finish:
      **12 of 12 reach the Refugia, in 129–147 minutes at 1× — 2.2 to 2.4 hours**, 16 to 18
      in-game days. One seed had ever been asked before, which for a length rolled per region
      per run was no answer at all; the spread turns out to be 14%, so the window is not a
      lucky seed.

      A scripted walker is the **floor**, not the estimate. It never stops, never berths, never
      reads a board, and answers every fork the instant it appears — a person does all four, and
      every one of them adds time. So the asymmetry matters: a walker under two hours would not
      prove the run too short, but a walker over four would prove it too long, because nothing a
      player does makes a run shorter. At 2.2–2.4 hours the floor sits just inside the window
      with the whole of the top half free for a player to spend.

      The ending is an arrival rather than a score by construction (§3.7, and `Chrome.tsx`'s
      arrival card has no rank in it). What is unverified is whether two hours of it is
      *enjoyable*, which is not a thing a harness can be pointed at.
- [x] **A shared seed reproduces it.** Unlocks are player-level, never touch `GameState` and
      never enter the replay, so a veteran's recording replays exactly for somebody who has
      unlocked nothing — they watch a tower build a room they could not build themselves. The seed
      crosses the bridge as a *string*, because it is 64 bits and JavaScript numbers are not, and
      a seed that does not round-trip is a seed that cannot be shared.
- [~] **The second tier changed a decision** — but not the decision this said it would, and the
      gate moved because building it proved the specced one wrong.

      Gating the elevator behind *mechanisms* put it behind alloy, scrap and the ruin belt, which
      means a rig, a forge and a fitter: four rooms and two floors more than a five-floor tower
      holds, and a tower that grows to fit them shades its own sails and browns out for good.
      Measured on the golden recorder — every buffer empty, the cutter arm at 0/8, a fixture that
      could not be recorded. The gate is **rope** now, which comes from the middle bands, so it
      still says "your route decides whether you can build this" and now says "you have to have
      walked ordinary ground" rather than "you have to have gone to the ruins". Better sentence,
      reachable tower.

      What is *not* demonstrated is the tier changing a decision in play, which needs somebody
      playing. It is no longer blocked on open question 0, though: that question is answered
      (yield and sun are a balanced pair, tuned to cancel), so the reason a richer route does not
      end up with more of a material is that it is *supposed* to pay for the extra ground in
      charge. The tier's decision is about what the tower can *reach* — rope for an elevator,
      alloy for a charge ceiling — rather than about how much a route hands it, and that is a
      better claim than the one this criterion was written against.
- [x] **Stopping at a ruin is sometimes the wrong call and sometimes the right one.**
      **Met — and what decides it is not the ground, which is a better answer than the one this
      was written expecting.**

      Three towers, same seeds, same rooms, differing only in when they stop. Poles-equivalent
      per 1,000 ticks, scrap valued at the enclave's own published 4-for-3:

      | policy | against never stopping |
      | --- | --- |
      | **reckless** — berths the moment it owns a rig | **−29% to −94%**, loses on 12 of 12 |
      | **careful** — berths only once a battery is up *and loaded* | **+0.5% to +66%**, wins on 12 of 12 |

      `ruin_richness_pct` predicts none of it: the 138% seeds and the 62% seeds behave alike. The
      thing that decides a berth is **whether the tower can answer what the berth wakes**, and
      the reason that is a real decision rather than a formality is the order the prices come in.
      A rig costs 8 of a tower's 10 starting poles; a dart battery costs 6 poles *and 2 rope*,
      and rope needs a ropery, which needs fiber, which needs a comb. **So a tower can afford to
      open a ruin long before it can afford to survive one.**

      What that costs, watched tick by tick on seed 4: berth at tick 0, two wardens up by tick
      1,000, the cutter arm at 92 of 260 by 3,000 and gone by 4,000, and `repelled` still zero
      because the tower never fired a dart. After that it cannot recover — mending costs poles,
      poles come from the mill, the mill eats bamboo, and bamboo needs the arm. The run does not
      end; it continues for an hour as a tower that cannot feed itself.

      So the sentence the criterion makes true is: **stopping is right if you have paid the whole
      entry price, and ruinous if you have paid only the part with a room attached to it.**

      **The one thing left open is telegraphing, and it is a real one.** Nothing tells a player
      that the rig is half a purchase. The empty space in the build menu is the affordance for
      *having* a rig (`salvage_rig.ron`), and there is no equivalent for needing a loaded battery
      before using one — a first-time player buys the obvious thing, stops at the obvious place,
      and is quietly ruined four thousand ticks later. Whether that wants a diegetic signal, a
      cost change, or nothing at all is a judgement about how punishing this game means to be,
      and it wants somebody playing before it is answered.

      *(Superseded: this was recorded as "always the wrong call" on the strength of a policy
      that berthed as soon as it owned a rig — the reckless row above. The measurement was
      right and the conclusion was drawn from one of the two towers.)*

      `examples/journey.rs`'s "is stopping at a ruin ever the right call" runs two towers that
      differ in exactly one behaviour — same seed, same rig, same battery, same thornwright, same
      shopping list, one berths at every ruin it can reach and one never stops — and scores both
      in poles-equivalent per 1,000 ticks, with scrap valued at the enclave's own published
      4-for-3. **Berthing loses on 12 seeds out of 12, by 29% to 94%**, and `ruin_richness_pct`
      does not predict it: the 138% seeds lose as badly as the 62% ones.

      **The mechanism is a death spiral, and it is the actual finding.** A berth wakes wardens,
      wardens go for the rooms, and the room they take is the cutter arm. Measured on seed 4: the
      arm is at **0 of 260 hit points by tick 20,000 and still there 100,000 ticks later**, with
      the tower walking the whole region and harvesting nothing. Mending costs poles; poles come
      from the mill; the mill eats bamboo; bamboo needs the arm. **There is no way out of that
      inside the tower**, and the run does not end — it continues for another hour as a tower
      that cannot feed itself.

      The way out is outside the tower: every board buys scrap 4-for-3, and 51 scrap is 38 poles.
      The harness now sells at the first settlement and it is not enough, because by 30,000 paces
      the arm has been dead for a day. So a player *can* recover, and only if they notice early.

      Three things this could be, and it is an owner's call which:

      1. **Working as intended** — `DECISIONS.md` §11 makes walking a free answer to any wave, so
         a tower that stayed and lost its arm made a choice. The objection is that the punishment
         is unbounded and silent: nothing on screen says "you can no longer recover".
      2. **A defence problem** — one dart battery does not hold a warden off a rig, so the real
         entry price of salvaging is higher than the rig's 8 poles suggests.
      3. **A repair problem** — the tower cannot mend the one room that pays for mending. A
         reserve, a cheaper first repair, or a warden that prefers panels to rooms would each
         break the loop.

      What is *not* in doubt is that the criterion as written is unmet. Whether the answer should
      be "it depends" or "it is a real risk you can be ruined by" is a design decision, and it
      wants a person playing it before anything is changed.
- [ ] **Every constant in `BALANCE.md` graded `PLAYTESTED`**, from run logs rather than from
      scripted harnesses. The run log is built (§5.8); the sessions are not. This is the criterion
      §5.11's fourth question expected to be missed honestly, and it is being missed honestly.

      **Standing at 89 of 137.**

      **Two rows are graded as decisions rather than findings, and that distinction is worth
      keeping.** `meals glyph / order` and `scrap order` are presentation choices — a glyph
      cannot be wrong the way a lighting figure can, because it makes no claim. They carry a
      grade only because `tests/balance_doc.rs` requires every field to have one, and what
      "verified" means for them is that the renderer draws them and the capture harness
      photographs them. Anything else would be manufacturing a measurement to satisfy a
      checkbox, which is the exact failure this grading system exists to prevent. Moved this pass: `reinforce`, once the plating comparison
      stopped counting the wrong thing; the three region-length rows, on the whole-run
      measurement that is the number they exist to produce (12 of 12 seeds arriving in 2.2–2.4
      hours); and the three Power rows, on a new instrument.

      **`examples/charge.rs` is that instrument, and it found the Power section was arithmetic
      nobody had ever checked.** Every row in it was a multiplication done by hand from the
      constant beside it. Two survive contact and one does not: striding measures 2,712 charge a
      day against a derived 2,880 (the gap is ticks spent standing at forks), and lamps measure
      **648 against a derived ~380 — 70% more**. The constant is right and the day it was
      multiplied by was wrong: exposure is sun *after terrain*, so a canopy region is dark for
      41–56% of a day rather than the ~33% the bare sun curve implies, and every hand-derived
      lighting figure in the file inherited that mistake.

      It also puts a number on §6.3's height cost that is worth having: a tower four floors
      taller earns **nothing at all** and browns out for the entire day, because the new roof
      shades the sail deck. Not a tax on growing — a wall.

      **`examples/needs.rs` does the same for M4's crew rows and finds the same shape of
      mistake.** `hungry_ticks` 4,800 is a third of a day and the row reads it as three meals a
      person a day. Measured over a week on a tower with a kitchen, beds and nothing attacking
      it: **2.00 meals a person a day**. Hunger rises around the clock but *eating* does not, and
      a crew member sleeps about 9.6 of 24 hours — during which hunger climbs past a whole cycle,
      so they cannot help waking a meal in debt. The clock says three; the rota can deliver two.

      The shares that falls out of are the number the difficulty pass actually wants: a crew
      member on a well-run tower spends **33.3% of their life hungry, 13.4% starving, 22.0%
      tired, and 35.5% slowed** — and `hungry_work_pct`/`tired_work_pct` are both 60, so that
      last figure is a third of a person's life spent working two-thirds as fast. Whether that is
      too harsh is a judgement for somebody playing. That it is not what the rows claimed is not.

      **`examples/haulcycle.rs` does the movement constants, and turns up something about a
      pillar.** A crew member on a fed, housed starting tower spends **34.2% of the day walking,
      12.1% climbing, 25.8% carrying, 3.4% queued, 38.9% asleep and 0.2% idle**, completing 20.1
      hauls a day each. Three readings:

      - **Walking costs nearly three times what climbing does**, which the per-unit constants
        hide: `climb_ticks_per_floor` is two and a half times `walk_ticks_per_slot`, so the rows
        read as though vertical movement dominates. On four floors there is far more horizontal
        distance than vertical. `DESIGN.md` pillar 2's "vertical transport is the belt" is a
        claim about a *tall* tower, and the crossover is worth finding before the elevator's
        price is judged.
      - **Queueing is 3.4%** — insight 1's contention thesis is present rather than felt on a
        starting tower, and has to arrive with height and room count or not at all.
      - **Crew are idle 0.2% of the time.** The tower is saturated, so any throughput measurement
        taken here measures the crew rather than the thing being added — which is a standing
        warning for every other instrument in the file, and probably explains a few of the flat
        readings this project has spent milestones arguing with.

      **The terrain pair is graded too**, on the ceiling measurement in open question 0 rather
      than on new work: `yield_pct` and `sun_pct` cancel to within two percent on every tower
      shape, which is exactly what they were written to do.

      **`examples/worldrate.rs` does the World section**, and it is the one that comes out
      clean: striding measures 59.8 paces per 100 ticks against an asserted 60 (the gap is the
      tick a fork answer costs), bands run 586 paces mean against a 300–900 roll, and both
      streaming windows are bounded and flat with distance — 1,795 ahead against a 1,800 ceiling,
      1,195 behind against 1,200. Those two *should* exceed their constants: generation and
      pruning each finish the band they are in, so the retained window is the constant plus up to
      one `band_max_paces`. Recorded because the next person to measure it will otherwise file a
      bug.

      **`examples/chain.rs` measures the starting chain, and the answer reframes the Content
      rows.** The mill is stalled **88% of the time** — 43.1% with a full outbox, 45.2% with an
      empty inbox — the cutter arm sits on a full outbox 40.4% of the time, and the tower
      harvests 92 and crafts 34 a day. Crew are idle 0.2%. So the rows that size a craft time
      against a harvest time ("deliberately close to the cutter arm's rate and on the near side
      of it") are reasoning about a tower that does not exist: **neither room runs anywhere near
      its rate, because both are waiting on hands.** The constant that would change this tower's
      throughput is `starting_crew` or `carry_capacity`, not `mill craft_ticks`.

      The two mill figures being nearly equal is the buffers doing their job — big enough to ride
      out a queue, small enough that backpressure reaches the arm — and the arm's 40.4% is that
      backpressure arriving, which `AGENTS.md` requires to be visible rather than silently
      absorbed. Enlarging either buffer would hide it.

      **`examples/prices.rs` asks the only question a `build_cost` row can be tested on** —
      is it payable when the thing is wanted? — by walking one tower for eight days, buying
      nothing, and recording when each buildable first becomes affordable. The answer splits
      cleanly in two and **no row described the split**: everything priced in poles arrives
      inside the first fifth of day one, and everything priced in **rope** is never affordable
      at all, because rope needs a ropery, a ropery needs fiber, and fiber needs a comb.

      That caught four stale rows at once. The cell bank's said "8 poles"; the pack charges **2
      charge cells**, so it moved from an opening build to a tier-two one and its row still
      described the old game. The dumbwaiter, elevator and dart battery each said poles only and
      each charge rope as well. A reader pricing the elevator off its row would conclude vertical
      transport is eighteen poles away when it is three rooms and a chain away — and the dart
      battery's hidden rope is exactly what makes berthing a trap, since a salvage rig is 8 poles
      and affordable at once.

      `a_documented_build_cost_names_everything_the_pack_charges` now guards it: the sibling test
      covers `balance.ron` and nothing covered the content pack, which is where all four had
      drifted.

      **`examples/bestiary.rs` puts one of each creature against one tower**, which nothing had
      done — the pressure table measures whole waves, so it can say a tower died at provocation
      500 without saying what killed it. The dart costs confirm the rows almost exactly: a
      glean-crow dies to **1 dart** (the row derives 1 from its 14 hp), a warden to **19** against
      a derived 20, a skitter to 2, a mire-hulk to 19.

      The headline is the column nobody had looked at: **no single creature is a threat to a
      walking tower.** All seven end between 989‰ and 1000‰ undefended, because `cling_ticks`
      runs out and the stride carries them off. That is `DECISIONS.md` §11 working as written, and
      it means every dangerous number in the Siege section is about *volume*, not about any one
      stat block. It also corrects the warden row, whose "about 160 damage" is a **berthed**
      tower's figure — a walking one takes 2‰.

      **The Siege section turned up the worst of the four, and it is M2's exit criterion.**
      `siege_run.rs`'s pressure table compares a bare tower, a plated one and one with "battery +
      darts". The battery costs 2 rope; the pressure tower starts with ten poles and no rope;
      every build was silently refused because nothing checked the return value. **`Shape::
      Answered` was byte-for-byte the bare tower** — identical 629 hit points lost at provocation
      300, and zero darts fired at every level of the table. M2 shipped on "answered is
      meaningfully better off than greedy for the poles it spent" and that comparison had never
      once run.

      The file already carries a comment about this exact lie happening before, when hard-coded
      coordinates meant a thornwright was refused for a slot clash. That was fixed by placing
      rooms anywhere; the *cost* now blocks them instead, and the lesson — **assert the build
      succeeded** — had not been drawn. It is now, and the harness pays for the rooms first.

      With a battery that exists, the answer is a good one: **972‰ against a bare tower's 951‰ at
      provocation 100, 198 hit points lost against 316, and 46 creatures seen off for 96 darts**;
      926‰ against 902‰ at 300. Five Siege constants graded on the curve behind it — a starting
      tower holds at 100, is worn at 300 and is lost on 4 of 6 seeds at 500, which is the right
      knee.

      **The Transport section is graded off an extension to `throughput.rs`**, and it explains
      that instrument's own result. A car spends **67% of its busy time dwelling and 32% moving**
      on a four-floor tower: `dwell_base_ticks` plus `dwell_per_unit_ticks` is a fixed cost paid
      per stop, and four floors do not give enough travel between stops to amortise it. That is
      why the elevator cuts contention without raising throughput, and it is the right shape — an
      elevator should earn its slot by height — but it means these constants are sized against a
      tower nobody has measured them on. **The crossover height is the number actually worth
      finding**, and it is the same question `haulcycle.rs` raises from the other side.

      A crew member's wait at a shaft is **1 tick median, 14–15 mean, ~110 worst**. Most calls
      are served instantly and the mean is a tail, so those constants shape an exception rather
      than an experience — worth knowing before anybody tunes one to fix a queue they felt.

      **And one row was stale by a factor of six.** `canopy sails charge_per_100_ticks` compared
      a good-sun day's ~20,000 income against "~17,700 of continuous striding" — but
      `stride_charge_per_100_ticks` is 20, which is 2,880 a day and 2,712 measured. 17,700
      implies about 123 per 100 ticks, which is what that constant *used to be*, two rows further
      up the same table. The income half stands; what changed is the other side of the sum.
      Striding is now a seventh of a good-sun day rather than most of it, so the real pressure on
      the bank is night lighting and terrain rather than the legs — which is a different game
      from the one that row describes, and nobody had noticed because nothing recomputes prose.

      **And the criterion contradicts what this project has actually been doing, which is worth
      settling before somebody grades the other 122.** It says "from run logs rather than from
      scripted harnesses" — but every `PLAYTESTED` grade in the file was awarded on a harness:
      `elevator_capacity` on `throughput.rs`, `wave_interval_ticks` and `base_threat` on
      `siege_run.rs`. Either those grades are wrong, or the criterion means something narrower
      than it says. The defensible reading, and the one the rows themselves use, is: **a constant
      is `PLAYTESTED` when the effect it exists to produce has been measured and confirmed, with
      the limit of that measurement written into the row.** What no harness can supply is whether
      the result is *enjoyable*, and no row should ever claim it.

      On that reading the remaining 122 are not blocked on sessions so much as on somebody
      building an instrument per constant, which is a milestone's work rather than an afternoon's.
      On the literal reading they are blocked entirely. Saying which is meant is the decision this
      criterion actually needs.
- [x] Golden replay regenerated; hash parity green natively and in wasm. The fixture lost its
      salvage rig on the way: three rooms reach the ground and carry `max_floor: 1`, and two
      ground floors do not hold all three once the Heartseed, a cell bank and a storeroom have
      taken theirs. It trades M3 coverage `tests/journey.rs` duplicates for M5 coverage that
      exists nowhere else.
- [x] **Every instrument in the repo runs.** Worth stating because one did not: M1's
      `examples/throughput.rs` panicked on startup — *"could not build the elevator: need 18
      item.poles, have 0"* — and had done since M5 gated the elevator on rope and raised its
      poles from 12 to 18. Nothing runs the examples on the way past, so a broken instrument is
      invisible until somebody asks it a question. `make check` does not cover them and still
      does not; this is a note for the next person rather than a new gate.
- [x] `make check` and the smoke suite green. 283 Rust tests, clippy clean at `-D warnings`,
      `tsgo`/`oxlint` clean, and 11 Playwright specs — smoke, capture, audio, and the built-bundle
      check. One known flake, documented in `e2e/capture.spec.ts`: the two approach stills come
      out empty on some runs, because that harness interleaves stepping with real-time waits while
      the frame loop runs and so lands the same seed in different places. It is a property of the
      harness, not of the game, and the three explanations that were *not* the cause are recorded
      there so nobody spends the afternoon again.
- [~] **An itch.io build a stranger can open.** `make release` cuts it and it works: the bundle
      builds, `index.html` sits at the zip root where itch expects it, and `e2e/release.spec.ts`
      boots the built bundle — not the dev server — and plays it, which is the failure this whole
      target exists to catch (a bundle that 404s its own WASM and shows a blank canvas passes
      every other check in the project). Note for anyone cutting one on Windows: the target
      shells out to `zip`, which Git Bash does not ship; `Compress-Archive -Path web/dist/*`
      produces the same layout.

      **"and play without being told anything" is the half that needs a stranger.** Nobody has
      been shown it.

**Deferred out of M5, which is to say out of the plan:**

- **Role priorities.** §4.9 cut them for having only two kinds of work to prioritise and said
  four might justify a screen. M5 brings the fourth (haul, mend, operate a forge, feed a
  thrower) — and the answer is still no: `assign_idle`'s ladder and what the player chooses to
  build already encode a triage policy, and a per-crew priority grid is the RimWorld tax rather
  than the RimWorld idea. Recorded as decided rather than as forgotten.
- **A second Heartseed, difficulty modes, and any run modifier.** All three are ways of making a
  run harder that are not the player playing loudly, and provocation is the only difficulty dial
  this game has (§2.6).
- **The repo rename** (`logdef` → `understory`), `v2-plan.md` §11 open #4. Cosmetic, and a
  release cut is the worst possible moment for it.

### 5.11 Open questions

0. **Does terrain yield change a decision, or is it decoration?**
   *(Answered twice: first "neither — it is half of a balanced pair",
   then, at M6, by deleting the other half. See the end, then the
   postscript after it.)*

   > **Superseded by §6.10.** The analysis below is correct and is kept
   > in full, but the system it analyses is gone: M6 cut the canopy
   > sails, so `sun_pct` no longer pays a tower anything and the
   > opposition it describes has one side left. Read this for the
   > method — it is the best short lesson in this repo about measuring
   > your own instruments — and §6.10 for what the ground means now.

   > **Answered — and the answer is that nothing is broken. Read the end
   > before acting on anything in the middle.** Terrain yield is not
   > decorative and it is not weak; it is one half of a deliberate
   > opposition with `sun_pct`, and the two cancel to within two percent
   > on purpose. Every diagnosis recorded below — chains terminating in
   > buffers, crew scarcity, jammed shelves — was a description of the
   > *harness*, and each one was withdrawn by the next measurement. The
   > reasoning is kept in full because the wrong turns are the useful
   > part, and because this question generated four confident wrong
   > answers before the right one.

   `examples/journey.rs` measures a shade-seeking route against a
   sun-seeking one, and after M5 it reports **exactly 219 bamboo and 68
   produce for both** — on routes 7 percentage points apart in canopy
   cover. That is M3's original finding (§3.10) returning in a new place,
   and M4's fix explains why: the canteen worked because meals have a
   *true* sink — crew eat three a day, for ever, and no buffer can hold
   the demand. Nothing else in the pack does.

   Poles have a near-sink (construction, repair) that dries up when a
   tower is finished. Darts have one only while something is attacking.
   Rope, alloy, mechanisms, charge cells and seed bombs all terminate in
   a buffer that fills and stops, and so, one step upstream, do fiber,
   scrap and produce. **A tower's harvest is therefore capped by its
   consumption, and its consumption is fixed** — so the ground underfoot
   changes only how fast it *could* have harvested, never how much it
   did. Every terrain yield in `BALANCE.md` is, in the steady state,
   decorative.

   This is not a tuning problem and no number fixes it. The shapes that
   would are structural, and each is a real design decision rather than a
   patch:

   - **A sink that scales with the run**, the way meals scale with the
     crew. The tower's own upkeep is the obvious candidate: a structure
     that needs maintaining in materials rather than only in poles-after-
     damage would make every material's demand continuous.
   - **Somewhere to put a surplus that is worth something.** The enclave
     boards are exactly this and are one-shot; a standing buyer would
     turn "I have too much fiber" into a reason to walk a route.
   - **Or accept it**, and say plainly that route choice is about *sun,
     scrap and danger* rather than about yield — in which case
     `yield_pct` should stop being four distinct numbers pretending to
     matter.

   Recorded at length because it is the single most important thing this
   milestone found, it was invisible until three tiers existed to make it
   visible, and the temptation will be to fix the instrument.

   **Prototyped, measured, and deliberately not shipped.** The first of
   the three shapes above was built and taken out again: continuous
   overgrowth, the jungle taking the tower back a little at a time and
   more of a bigger tower, scaled by the band's own `yield_pct` — so
   poles gain a sink that scales with the run the way meals scale with
   the crew, and the ground that grows bamboo fastest grows over you
   fastest. On a simple tower it does exactly what it was supposed to:
   measured across a full region, a shade-seeking route harvested **364
   bamboo against a sun-seeking route's 346**, in the right direction,
   where both had previously reported the same number to the unit.

   It is out because it is an owner's decision rather than a balance
   one. A tower under continuous overgrowth is never quite whole, so
   "something is attacking" and "it is Tuesday" stop being
   distinguishable on the standing readout — and the first sizing
   attempt, at 161 poles a day against a mill's 120, had upkeep outrun
   income permanently, crew mending instead of hauling, and harvest
   falling from 219 to 67. The right sizing is roughly a quarter of a
   mill (4 hit points a floor per 1,000 ticks), and even there it breaks
   six tests that assume a tower stays whole when nothing is biting it.
   None of that is an argument against doing it; all of it is an
   argument for it being chosen rather than slipped in.

   **Answered, and every paragraph above this one is wrong. The ground
   is not decorative; it is exactly half of a balanced pair, and it was
   built that way on purpose.**

   The measurement that settles it asks the question in a form no harness
   detail can distort: **take the cap off and see what is left.**
   `examples/journey.rs`'s `Uncapped` tower has bottomless buffers, so
   nothing in it can ever be full, the cutter arm never stalls, and its
   harvest is whatever the ground and the legs allowed. It is not
   playable — it is a ceiling. Whatever a standing buyer, a second chute,
   a bigger storeroom or any other sink could ever be worth, it is worth
   no more than that row. Bamboo over twelve seeds, both route policies,
   a fixed 109,520 ticks:

   | tower | shade | sun | gap |
   | --- | --- | --- | --- |
   | bare, with a kitchen | 6501 | 6412 | +1.4% |
   | + garden, comb, ropery | 5596 | 5702 | −1.9% |
   | + the same, with a chute | 3568 | 3632 | −1.8% |
   | + the same, with a burner | 3672 | 3601 | +2.0% |
   | **bare, bottomless buffers** | **7988** | **8099** | **−1.4%** |

   **Uncapping a tower is worth about a quarter of its harvest, and
   nothing at all on the route.** 6,501 to 7,988 says the cap is real and
   a sink would be worth building if more harvest is what you want. The
   −1.4% at the ceiling — *the sun route ahead* — says no sink will ever
   make terrain yield legible, because yield was never the thing binding.

   **Why: yield and sun cancel, and `TerrainDef.sun_pct` says so in as
   many words** — *"deliberately opposed to `yield_pct`: shade is
   biomass-rich and sun-poor… that opposition is the whole of 'your route
   is your power mix', and it only works if no band is good at both."*
   Terrain intake is paid in **ground covered**, not in ticks, and a
   browned-out tower stops walking. So shade buys richer ground and less
   power to cross it, sun buys the reverse, and measured across every
   tower shape here the two cancel to within two percent. That is a dead
   heat, and a dead heat is precisely what "no band is good at both" asks
   for. The pair was tuned correctly and this question was reading one
   half of it in isolation.

   **So the answer is none of the three shapes above.** Route choice is
   legible in what it *costs* — charge, threat, what there is to salvage
   — and not in total bamboo, and `BALANCE.md`'s four yield values are
   load-bearing rather than decorative: they are what makes the shade
   side of the trade worth taking at all. Collapsing them would break the
   opposition, not tidy it.

   **What this question actually produced, which is worth more than the
   answer: four confident wrong findings in a row, every one of them a
   property of the harness.**

   - *"Every chain terminates in a buffer, so terrain cannot matter."*
     A true observation about the pack, and not the cause.
   - *"The tower is crew-limited."* Wrong, and the tell was in the
     measurement all along: adding crew made the flat reading **worse**,
     219 down to 150. A crew-limited tower improves when you add crew.
   - *"A jammed shelf costs 55% of the harvest and a chute recovers a
     third of it."* An artifact of a shopping list that stopped at the
     first thing it could not afford, so the jammed tower never built its
     storerooms either. With a list that skips blockers, the jam is worth
     14%, and the chute — whose slot column displaces the storerooms that
     were doing the real work — comes out *behind*.
   - *"The route is worth +9.7%."* Two outlier seeds out of eight. Per
     seed the gap swings **−3.9% to +71.9%**; twelve seeds give −1.4%.

   The standing lesson, which cost a milestone: **this harness has
   produced more findings about itself than about the game.** Before any
   number here becomes a design decision, change something that should
   not matter — the seed set, the build order, the storeroom count — and
   check the number survives it.

   **Postscript, M6: the question was closed and then the system was
   deleted.** See §6.10. The answer above was that `yield_pct` and
   `sun_pct` are one constant in two columns, cancelling to within two
   percent on every tower shape measured — which is the design working.
   It was *also*, read a different way, a description of a choice that
   costs the player nothing to get wrong, expressed as two numbers on a
   screen that have to be mentally netted off against each other. The
   sails were cut for that second reading rather than the first. Nothing
   in the analysis above was found to be wrong; the thing it was
   analysing was found not to be worth having.

1. **Does gating the elevator behind the ruin belt make the shaded route a trap?** The argument
   in §5.3 is that it makes route choice reach into the transport layer. The risk is that it
   makes one branch strictly correct — take the ruins, get the elevator — which would be worse
   than no gate at all. The tell is whether a shade-heavy run ever *wins* on the strength of what
   shade gave it.
2. **Is the garden the first thing that makes standing still correct, or the thing that makes
   walking optional?** It is deliberately the inverse of the cutter arm, and a tower that can
   feed itself parked is a tower with less reason to walk — the one behaviour this whole game is
   built around. If berthing stops being a decision, the garden's rate is the lever, not the
   stride cost.
3. **Do three enclaves make the economy legible, or make it a market?** Regional boards that
   differ in direction are a good idea on paper and are also how a game accidentally becomes
   about arbitrage.
4. **Can the difficulty pass be done from run logs at all**, or does grading every constant
   `PLAYTESTED` need more sessions than a solo project will ever have? This is the criterion most
   likely to be honestly missed, and saying so now is better than quietly redefining
   `PLAYTESTED` later.

---

## M6 — The Watch *(the player's hands during a wave)*

**Sprint question:** does a wave become a thing you *do* something about, without the tower
becoming a war machine?

**Scope:** the verbs a player has while a wave is landing. Charge priority handed over; a
creature the emplacements can be told to prefer; a person posted to a room; kit that belongs to
somebody named; the berth given its own halt; and a thief answered by somebody standing in the
room. Alongside them, the pacing pass that brought a run to forty minutes and the balance
changes the shaft economy turned out to need once it was measured rather than assumed.

**The tone gate is absolute.** `DECISIONS.md` §8 is defenders rather than soldiers, and
creatures defending their territory rather than a gallery to clear. Every verb below was checked
against it before it was built, and the two that could not pass — weapon loadouts, and crew
fighting boarders — are cut here rather than softened. What survived is the shape of *attention*
rather than the shape of a fight.

### 6.1 The shape of the thing

**Understory already had most of FTL and had not noticed.** Real time with a pause; a shared
power bank; crew whose repair work competes with their day job; systems that break mid-fight and
have to be mended by the same people who were carrying things; and an escape valve. What it did
not have was any way for the player to *act* during the twenty seconds a creature spends chewing
on a panel. The tower was a machine you configured in advance and then watched.

Four verbs close that, and each one already had its state sitting in the simulation:

| Verb | What it was before | What it is now |
|---|---|---|
| **Charge priority** | the tick order, a constant | a ranking the player owns |
| **Focus** | nearest-in-range, always | nearest, unless you name one |
| **Stationing** | crew hauled or mended, nothing else | a person can be put in a room |
| **Kit** | nothing at all | a tool that belongs to somebody named |

**The escape valve stays.** §11 makes striding the free answer to any wave, and it is the game's
identity — *keep walking* — so none of this removes it. FTL-ness lives inside the one place a
tower is already committed, which is a berth (§6.4), rather than replacing the ability to leave.

### 6.2 Charge priority, which was a constant pretending to be a decision

`systems/power.rs` says it in its own header: **"charge priority *is* the tick order."** Lifts
drew first because transport runs first; legs last because striding runs last. The most
consequential scarcity in the game was settled by the order somebody wrote the systems in.

`PowerUse` names the four draws — **Lifts, Works, Lamps, Legs** — and `SetPowerPriority` ranks
them. The command takes the whole order and rejects anything that is not all four exactly once,
because a partial order would leave the rest ranked by an accident of list position.

**The tick order does not move.** Reordering `systems::tick` would invalidate every replay (§1),
so the ranking is honoured by a *reserve*: a use may not draw the pool below what higher-ranked
uses that have not yet spent this tick are owed. Only later-in-the-tick uses can be starved by an
earlier one, so only those are reserved for — a higher-ranked use that has already spent needs
nothing held back. Without that the ranking would be decoration and the tick order would still
decide who gets the last of the bank.

That needs each use's demand known before anything spends, so `estimate_demand` runs at the end
of income. **They are estimates and the code says so.** A use that draws early cannot know what a
late one will ask for without running it first; slightly high makes the tower cautious for a
tick, slightly low costs the high-ranked use nothing because it still draws against whatever is
actually left. Neither can create charge or lose it — this only decides who is refused first.

**The default ranking is the old tick order**, and a test asserts every reserve is zero under it.
An untouched tower behaves exactly as it did before, which every balance row measured against the
old behaviour depends on.

### 6.3 Focus, stationing, and what "equip" means here

**Focus.** `defence.rs` shoots the nearest creature in range, and its comment says why: *a
battery has no judgement — the player's judgement went into where they put it.* A focus does not
give the battery judgement. It adds a second moment for the player to supply theirs, live, at the
cost of their attention during a wave. Nothing focused is the normal case and the old behaviour
exactly, and a focus out of range falls back to nearest rather than holding fire — a battery
sitting idle while something chewed on the tower would be a trap rather than a decision. It is
drawn as a soft ring of the tower's own lamplight, never a reticle.

**Stationing.** A person posted to a room runs it at `manned_work_pct` and stops hauling. **The
price is the person, not a resource** — three crew and one staircase means posting somebody is a
standing decision to take a porter off the stairs, which is `DESIGN.md` insight 1 made explicit
rather than a cost bolted on. Needs outrank it: a posted person goes to eat when hungry and to
bed when their shift ends, and comes back. A station is not a cage.

**Equip is a kit, and it belongs to a person.** The obvious reading — weapons bolted to the
tower — is a different game, and the tower's half of it already exists: what you feed a dart
battery *is* the choice. So a kit is the other half. `DESIGN.md` §2 structural call 4 says crew
are named individuals and not stat blocks, and a kit is the smallest mechanic that makes that
true in the simulation rather than only in the fiction — Wren carries the lamp, and you know
which of them it is.

| Kit | Answers | Where it comes from |
|---|---|---|
| **Hand lamp** | `dark_work_pct` (75), for one person | built at a kitbench, behind mechanisms |
| **Porter's harness** | one more item per trip | the coast enclave, one ever |
| **Mender's kit** | `repair_hp_per_shift` at 150% | the coast enclave, one ever |

**Lent, not consumed.** The item leaves the shelves while it is carried and goes back when handed
in, so equipping is a decision the player can take back — and a kit in somebody's hands is not on
the shelves for anybody else, which is the whole of the scarcity. A swap returns the old kit
*first* and refuses if there is nowhere to put it, because nothing this game hands the player
ever vanishes.

### 6.4 The berth, named

`HaltView::Berthed`: stopped at a ruin, with a working rig in reach. **The one place a wave
cannot be walked away from**, and until M6 it reported as an ordinary halt — indistinguishable
from a tower somebody had parked for a rest.

That matters because §11 makes striding the free answer to any wave, so *being unable to stride*
is the only real commitment the game has, and a commitment the player cannot see is one they
cannot decide about. It outranks `Stopped` and nothing else: a tower at a fork, arrived, or
browned out is not choosing to be there, and each of those answers "why aren't we moving" better
than the ruin does.

It requires a rig that can reach the ruin, not merely a ruin nearby. A tower stopped beside one
it has no way to open is parked, and saying otherwise would promise something it cannot do.

### 6.5 Thieves, and the only defensive verb a person gets

**The glean crow was already a boarder**, and the tone-safe kind: `steals: true` sends it to the
highest floor with something in an outbox, it takes what is lying out, and it fights nobody. What
was missing was the crew's half.

A person walks to the room and stands in it for `shoo_ticks`, and the creature leaves. **Nobody
fights.** What somebody does about a crow in the outbox is *be there* — a crow that finds a
person in the room goes, the way it would if you walked into your own kitchen. Three lines keep
it from becoming combat:

- **Thieves only.** The `steals` flag is the line: a crow can be shooed, a mire hulk cannot.
  Pretending otherwise would turn standing in a doorway into a fight.
- **It does not count as `repelled`.** The creature goes to `Leaving`, sharing the fade with a
  cling timer running out. That number means the darts worked, and conflating it with standing in
  a doorway would stop it measuring what it exists to measure. Being asked to leave is not being
  seen off.
- **The roster says "seeing something out"**, not "chasing it off" or "defending".

A thief outranks damage in the assignment order, and the order is the argument: a wrecked panel
has already happened and will still be there in a minute, while a crow is taking something now.
Both sit below hunger and the rota, because neither is worth skipping dinner over.

### 6.6 Pacing, and the shaft economy

Two bodies of balance work landed alongside the verbs, both driven by measurement rather than by
intent.

**A run is now 37–44 minutes at 1×**, down from 129–147. The journey layer scaled by 3.5 — region
lengths, `fork_interval_paces`, `fork_edge_margin_paces` and every enclave's `at_paces` together,
because a position measured in absolute paces means nothing on its own and anything left behind
falls outside the region it belongs to. **Nothing else moved**, and three things that were tried
are why: an economy 1.5× faster made the golden fixture fail *earlier* (crew hauling is the
binding constraint, so speeding production only fills shelves the crew cannot clear), storage
scaled to match made it fail earlier still, and `starting_stock` bought nothing at all. What
closed the gap was the fixture's own housekeeping — it had been banking 24 rope and 24 meals on
an eight-shelf tower.

**The staircase now charges for freight.** `climb_ticks_per_item` (15) means an empty climber
takes 30 ticks a floor and a fully laden one 75. Before it, a staircase was a perfectly good
freight line and **neither built shaft had a job**: `examples/lift.rs` measured an elevator worth
+1% hauls at five floors and *not chosen at all* at eight or eleven, where crew queued sixty
thousand crew-ticks on the stairs beside a powered, empty car. After it the lift is +54% at five
floors and +650% at fourteen, and the dumbwaiter becomes worth building — the ladder the pack has
always described starts existing. The elevator's price fell from 18 poles and 6 rope to 12 and 4
to match: a thing the tower needs from minute twelve should not be the last thing it can afford.

**Two smaller shape changes.** `min_floor` gives the chain a direction — three intake rooms were
pinned low because they reach the ground, nothing was pinned high, and so a whole chain could sit
beside its own intake and never haul anything upward. A burner (a chimney) and a bunk (you sleep
above the works) are pinned to floor 2, and nothing else is. And `canopy_climb_pct_per_floor` is
the counterweight to `top_floor_only`: growing taller shades your own sail deck, so `charge.rs`
records a four-floors-taller tower earning *nothing at all* and browning out for most of the day.
Re-roof and the wall is gone.

### 6.7 What M6 changes in code that already exists

- `state/power.rs` — `PowerUse`, a `priority` list, a `demand` estimate, and `draw` taking the
  class that is spending. Every draw site is tagged with its class.
- `state/crew.rs` — `stationed` and `kit` on `Crew`; `Manning` and `Shooing` states; `Station`
  and `Shoo` errands, both routed by the existing `errand_leg`.
- `state/siege.rs` — `focus`, cleared in `siege::run` when its creature dies or leaves, so the
  mark can never point at nothing.
- `content.rs` — `KitDef` on `ItemDef`; `min_floor` on `RoomDef`; `manned_work_pct` and
  `climb_ticks_per_item` on the crew balance; `shoo_ticks` and `canopy_climb_pct_per_floor`.
- `systems/power.rs` — `roof_exposure_pct`, split from `exposure_pct` so harvest keeps reading
  the ground figure. A cutter arm sweeping the forest floor does not care how many storeys are
  stacked above it.
- `snapshot.rs` — `HaltView::Berthed`, `CrewStateTag::{Man, Shoo}`, `StoreView` (`StockView` plus
  `space`), and the power ranking.
- Four new commands, every one caught by the replay exhaustiveness match on the way in.

### 6.8 Exit criteria

1. **A wave is a thing you do something about.** Four verbs exist, each with a test. *Met in
   code; whether it reads as agency needs a person.*
2. **None of it reads as a war machine.** Focus is a mark rather than a reticle, a thief is seen
   out rather than fought, and nothing new is counted or celebrated. *Met by construction and by
   review rather than by measurement, which is the honest limit of this criterion.*
3. **The default behaves as it did.** The charge ranking defaults to the old tick order and a
   test asserts every reserve is zero under it. *Met.*
4. **A shaft is worth building.** `lift.rs` measures the lift positive at every height and the
   dumbwaiter worth having at five. *Met.*
5. **A run is under an hour.** 12 of 12 seeds reach the Refugia in 37–44 minutes at 1×. *Met for
   pace; a walker is the floor and a player adds to it.*

### 6.10 The sails, cut

**The canopy sails are gone, and with them the whole solar economy.** `room.canopy_sails`,
`SolarDef`, `collect_solar`, `roof_exposure_pct` and `canopy_climb_pct_per_floor` are deleted.
Charge now has exactly two sources: burners, which a tower builds and feeds, and the Heartseed,
which trickles.

**Why.** §5.11 open question 0 spent a milestone establishing that `yield_pct` and `sun_pct` are
one constant in two columns, deliberately opposed, cancelling to within two percent on every
tower shape measured. That is the design working. It is also a description of a decision that
costs nothing to get wrong, presented as two numbers a player has to net off against each other
in their head. Sun-versus-shade asked the player to read a figure off the sky; the fork now asks
about danger, salvage and ground richness instead, which are things you can see.

Nothing in open question 0's analysis was found to be wrong. The thing it was analysing was
found not to be worth having.

#### What replaced it

| Was | Is |
|---|---|
| Sails: free charge from the roof, scaled by terrain | Burners: 800 charge a stalk of bamboo |
| Growing taller shades your income to zero | Growing taller costs lamps, poles and haul distance |
| A parked tower still earns | A parked tower spends its bank |
| Nothing | The Heartseed's 6 per 100 ticks, so running dry is recoverable |

**The burner idles unless the bank can take the whole burn.** This is the load-bearing change,
and it is what makes a single source survivable: fuel consumption becomes what the tower
*spends* rather than what the clock says, so striding hard, lighting fourteen floors and running
a forge all cost bamboo, and a parked tower with a full bank costs none. The old "the burner is
the dirty option" pressure is replaced by "working the tower hard is the dirty option", which is
a better sentence and a better mechanic.

#### The three things that broke, which are the interesting part

**1. A deadlock with no way out.** Every remaining source of charge required already having
charge. A burner eats bamboo; bamboo is harvested from *ground covered*; a tower with no charge
cannot walk. Measured on `journey.rs`'s seed 1: charge **4 of 2,300**, fuel 0, and the tower
parked at one pace for **160,000 ticks** — unrecoverable at any skill. The Heartseed's trickle is
the floor that fixes it, sized as a limp rather than an income (6 per 100 ticks against
striding's 20), and `tests/power.rs::a_tower_that_runs_completely_dry_can_still_crawl_out` is
the property stated directly. **Any future change that makes an income depend on an output of
that income needs this test to still pass.**

**2. `top_floor_only` was enforced in exactly one place, and it was inside the sails.**
`collect_solar` filtered to the roof; nothing else did. Deleting it deleted the rule, and left
the garden — the only room still carrying the flag — dimmed by the snapshot while it grew at
full rate, which is a diegetic lie (`DECISIONS.md` §8). `intake.rs` enforces it now. **The
general shape: a rule implemented inside one consumer disappears with that consumer**, and
nothing in the type system says so.

**3. The opening tower's economy is ~31% smaller, and every number downstream moved.** With the
mill, the canteen and now the burner all eating bamboo, the mill lost 31% of its crafts (65
against 94 over 24,000 ticks at identical harvest). Three constants absorbed it, all recorded in
`BALANCE.md` with their measurements: the burner's rate (50 → 800 charge a stalk, over three
too-small corrections), `provocation_per_burn` (18 → 12, and this one decided whether the tower
*lived* — at 18 its only cutter arm was wrecked at tick 18,000 and never mended), and
`starting_stock` (10 → 16 poles, which is one mend of slack).

#### The failure mode worth naming

**A tower with one cutter arm that loses it is dead in a way nothing else in the game is.**
Repair wants poles, poles want the mill, the mill wants bamboo, and bamboo wants the arm that
just died. The sails hid this by funding enough slack to always mend; without them it is one bad
wave away on the opening tower. `starting_stock` buys one mend of margin and that is all it
buys. **This is not solved and it is not the difficulty pass's problem either** — it is a
structural single point of failure, and the honest fix is either a second intake room the
opening tower can afford or a floor under repair. Carried into §6.9 as an open question.

#### Two instrument bugs the change surfaced

- **A burner that idled only at a *completely* full bank** spent a whole stalk to add 50 charge,
  because `power.add` clips. That made the 200 → 400 efficiency change do nothing at all: 22
  stalks burned either way. Headroom, not fullness.
- **A shared headroom figure let every burner fire at once**, clipping each other, so a
  fourteen-floor tower with three burners earned *less* than the same tower with one (543
  against 1,839 a day). **More of a thing cannot make less of what it makes** — that shape is
  the tell, and it is the fourth entry in this project's list of instruments that measured
  themselves.

### 6.11 The opening five minutes

**A first turn used to open on twenty build cards and a factory somebody else had built.**
The tower arrived with a cutter arm, a mill, a cell bank, a storeroom and a sail deck already
running, four floors of mostly empty deck, and every room in the pack on the menu. Nothing on
that screen said which of the twenty mattered, and the chain was already working, so the first
thing a new player did was watch.

The opening is now: **two floors, three crew, a Heartseed and a bed.**

#### The ladder

`RoomDef.unlocked_by` names the room that has to be standing before this one may be built.
The chain is short and it is the whole tutorial:

| Rung | Opens | Why that order |
|---|---|---|
| — | **farm** | The only card on turn one. Food before anything. |
| farm | **cutter arm** | The tower fed itself; now it can cut. |
| cutter arm | **storeroom** | Shelves mean nothing until there is a second material. |
| cutter arm | **burner** | There is no fuel until something cuts it. |
| burner | **everything else** | The opening is over. |

**Validated at the command boundary, not filtered in a menu.** `engine::commands` returns
`CommandError::Locked`, and `ViewSnapshot::unlocked` tells the UI which gates are currently
open so the two cannot drift — a card that offers something the simulation refuses is worse
than no card. It reads off `GameState`, so it is deterministic and a replay carries it.

**This is not the journal.** §5.7's unlocks are player-level, never enter `GameState`, and can
only ever filter a menu; a replay carries commands, so a veteran's saved run has to replay
identically for a first-time player. The ladder is a fact about *this tower on this run*, which
is why it is allowed near the command layer at all. Keep the two apart.

#### The farm needs two people

`RoomDef.crew_required` is a **requirement**, not M6's `manned_work_pct` bonus: a room with it
does not work at all until that many crew are posted. The farm is the only room in the pack
that carries it, and it is deliberately the first thing the game teaches — three crew, and two
of them are now farmers. The other rooms merely go faster when somebody is standing in them.

An unstaffed farm stalls in place rather than resetting, like a starved mill, so somebody being
called away to eat does not throw away the crop.

#### Two things the shape forced

**The Heartseed carries three shelves.** A build cost is paid off a shelf, so something has to
have shelves or the first buildable thing is unbuildable. A pre-placed storeroom would do it
and would also eat the only three contiguous slots on the ground floor — which is exactly what
a salvage rig needs (`max_floor` 1, three wide). Three rather than four, and 60 capacity against
a storeroom's 80: enough that the opening cannot jam itself, nowhere near enough to save with.
Two deadlocked, measured — bamboo and produce held both while the mill's poles had nowhere to
land, and the storeroom that would have fixed it cost three of them.

**`starting_stock` is 24 poles, and the number is arithmetic.** Until a mill exists the tower
cannot make a single pole, so the founding stores have to cover the whole ladder and the first
mill or the opening is a dead end that looks like a difficulty spike: farm 5, cutter arm 4,
burner 5, the floor the mill stands on 6, mill 4 — **24 exactly**. At 16 the golden recorder
finished the ladder with two poles and twenty produce and never afforded a floor in 63,000
ticks.

#### What it cost to build

The starting tower was the fixture nearly every test and instrument rested on, so cutting it
broke 90 of 299 tests and every harness in `examples/`. `tests::engine` now walks the ladder
once and hands back the tower the old one used to return; `tests::opening` is the shipped
one, and four new tests in `tests/commands.rs` are about the ladder itself.
`harness::chain_tower` does the same job for the instruments, and `debug_grant` does it for the
browser specs — which had been earning everything, because there was no other way, and were
therefore all economy tests wearing UI tests' clothes.

**Three real bugs fell out of it**, all of them latent and none of them findable from the old
fixture:

- **A car dwelling at the floor its callers are standing on never opened its doors.**
  `depart` answers "someone is calling from this very floor" with
  `Dwelling { ticks_left: 0 }` — the doors staying open — and the guard on `service_stop` only
  fired on a transition *into* Dwelling, so it suppressed the one call it exists to make.
  Measured: three crew on floor 0, waiting **18,857 ticks**, boarding a car parked on floor 0
  with no riders. It needed a car to be dwelling *before* the callers appeared, which is what
  cutting the stairs out from under them produces and very little else does.
- **A dumbwaiter unloaded onto the first shelf it passed.** It chose a floor because a hungry
  recipe was on it — an inbox scores 3 against a shelf's 2 — then walked the rooms once,
  offering each its inbox *and* its shelves, so a storeroom at a lower slot swallowed the load
  before the mill three slots along was asked. Invisible until a fixture put a storeroom and a
  consumer on the same floor.
- **`top_floor_only` was enforced only inside the sails** (§6.10), so cutting them cut the
  rule and left the garden dimmed by the snapshot while it grew at full rate.

### 6.12 Picking people out, and pushing them

**Stationing answered "this is your job"; it did not answer "everybody on the mill, now".**
M6's posting is a standing order — somebody works a room until the player says otherwise —
and during a wave, or a jam, what a player wants is a shove that they do not then have to
remember to undo.

- **Click a person** to pick them out; click again to drop them. A person beats the room
  behind them, which is unavoidably where they are standing.
- **Drag a box** over the tower to pick out everybody in it. Crew cluster, so clicking each
  in turn does not work — two of three are often on the same pixel.
- **Right-click a room** to push everybody picked at it.
- **Right-click nothing** to let them all go.

**The push expires by itself, and that is the whole reason it is safe.** `until_tired` on
`StationCrew` sets `Crew::post_until_tired`, and `needs.rs` clears the posting when `rested`
falls to `tired_ticks`. A push lasts the rest of somebody's shift and no longer. A permanent
version of the same verb would leave half the crew standing in a room nobody remembers sending
them to, which is a worse tower than the one the player started with.

Calling somebody back always clears the flag, so a push followed by a real posting does not
inherit the expiry.

**None of the selection is in `GameState`.** Who is highlighted is a fact about somebody's
attention, not about the tower: it never enters a replay, and two people watching the same
seed are free to have different people picked. What *is* in the simulation is the posting and
its expiry, because those change what the tower does.

**Right-click means three things and they cannot collide**, because only one of them is ever
in progress: push the people you have picked, put the placement cursor down, or let the
selection go — in that order.

Drawn on the people rather than in a panel (`DECISIONS.md` §8): a soft ring under the feet for
picked, a brighter one for pushed. Two marks, because a job and a shove read identically on a
cross-section and are not the same thing.

### 6.13 Weapons, and where they go

**Three rules, and the first one is the one everything else hangs off.**

**A weapon goes on the leading edge.** `RoomDef::front_only` is validated in
`engine::commands` as `CommandError::NotAtTheFront`, and the front is `floor_slots - width`, so
a wide weapon sits flush with the edge rather than being banned from it. A thing that reaches
*out* of the tower has to be on the outside, and on a cross-section that is the edge the tower
is walking into and the edge everything arrives from. It is also what makes the weapon bar
honest: a list is only a list if the things on it are somewhere specific.

The floor widened from eight slots to ten to pay for it. The frontmost column a two-wide room
could occupy was 6-7, which is the column the first elevator goes in — **54 of 306 tests failed
on that collision**, every one a shaft that could no longer find a clear full-height column.
Columns 8 and 9 are the weapons'; 7 is still the shaft's.

**The cutter arm is dual use, and the second use is free.** `RoomDef::melee_damage` is dealt to
anything clinging to the arm's own floor, every tick it works. No ammo, no reload, no range: an
emplacement is a supply question (`systems/defence.rs` opens on exactly that), and this is not
that. It is a blade on a boom already doing its job, and a creature that climbs into the arc has
climbed into a blade on a boom. The player who built an arm to harvest has already built the
thing that answers a skitter, and finding that out is a better moment than being sold a weapon.

It reaches only its own floor and only what is *attached*, which is what keeps it from being a
free emplacement: a wave on the roof of a tall tower is not answered by a boom on the ground.
`a_cutter_arm_cuts_what_climbs_into_it` is the test, and it needed a ground-approach creature to
write — a leaper lands on the roof, which is exactly the limit.

**The tower sets out with one gun.** `room.thorn_gun` fires sharpened bamboo: 2 stalks a shot, 8
damage against a dart's 15, a 90-tick reload, and 5 paces of reach. Every other emplacement eats
something the chain has to *make*, which is the design working — but a tower whose only answer
to the first creature is to walk away has one verb, and §11 means walking to be the *free*
answer rather than the only one. So this one burns the raw material: weak, wasteful (a stalk
burnt is a pole the mill did not make), and always affordable. The player upgrades away from it
by building something that eats a crafted round.

**The weapon bar is a readout, not a second place to build.** `UiState.weapons` lists what the
tower can point at something and how many rounds are on each rack, dimmed when dry — the same
language a starved mill speaks (`DECISIONS.md` §8). Ammo counts rather than a green light,
because "3 darts" is a number a player can plan with and "ready" is not. The placement is still
the decision; this says what that decision bought.

#### The tone gate

Weapon *loadouts* were cut at M6 (§6) and are still cut. What is here is not that: a harvest
room that also cuts, and one gun that burns what the tower harvests. The tower defends itself
with the tools it works with rather than growing a separate set for fighting, which is the whole
of `DECISIONS.md` §8 — defenders rather than soldiers.

### 6.14 Beats on the route

**"Right now it's like a screensaver."** Between one fork and the next the tower walked
through scenery and decided nothing. Forks are rare by design (`fork_interval_paces` is 4,300)
and an enclave is a whole settlement; there was no small thing in between.

A **waypoint** is that small thing. It comes into range, asks one question, and goes past.

| | fork | enclave | waypoint |
|---|---|---|---|
| Stops the tower | yes, until answered | no, you berth | **no** |
| Decision shape | which way | a whole board | **one button** |
| Ignoring it | impossible | free | **free, and the default** |
| Comes back | — | no | **no** |

Three rules make it a beat rather than a chore:

- **Ignoring it is free.** There is no penalty branch. §11's rule that walking is always
  available applies here too — the tower walking on is never wrong, only sometimes less good.
- **It resolves in one click.** `TakeWaypoint` carries no id, because the only one you can
  take is the one alongside. A thing that needs a decision *tree* is an enclave.
- **It is gone once passed.** The axis runs one way (§3.5), so a waypoint behind you is a
  thing that happened rather than a thing you are still owed.

Generated off the **`world` stream, not `cosmetic`** (`DECISIONS.md` §2): where a beat falls
and which one it is are facts about the run, and a shared seed has to reproduce them.
Per-region intervals, so a region has its own rhythm — the deep jungle is thick with them
(1,100 paces), the coast is nearly empty (2,000), and zero means none at all, which is a real
authoring choice rather than an oversight.

#### The four in the pack, and what the fourth one taught

| | asks | gives | attention | ground |
|---|---|---|---|---|
| Seep Pool | stop and wash | — | **−45** | −90 |
| Fallen Carrier | strip it | 5 poles | +15 | −140 |
| Wire Tangle | 4 poles | 2 mechanisms | +10 | −110 |
| Snare Thicket | push through | — | +60 | **+300** |

**The Seep Pool is the only thing in the game that lowers provocation.** Everything else a
tower does raises it — cutting, burning, pushing through — and a dial that only goes one way is
a countdown rather than a decision.

**Two of these gave the wrong things at first, and the golden recorder found it.** The Fallen
Carrier handed over scrap and the Seep Pool handed over produce: both thin supplies, both
plausible, and both with almost no consumer in a young tower. A shelf holds one kind and the
opening tower has three shelves. Measured: **thirty scrap and forty-three produce squatting the
shelves**, the fiber the ropery needed with nowhere to land, and a tower that could not afford
a dart battery while holding a fortune in things it could not use.

**A gift the tower cannot spend is a jam wearing a reward's clothes.** The beats give poles,
mechanisms, ground and quiet — things every tower wants.

#### What it did to the fixture

`record_golden.rs` takes beats as it walks, which is both coverage and solvency: without them
it walked its whole journey and died on the elevator at zero poles. It takes only *free* ones,
and only once a storeroom exists — two rules a player would keep, and the second is the one the
scrap-and-produce failure taught.

### 6.15 Sized creatures, and a reason to stand

**"Combat needs to be a bigger part of the game."** Two changes, and the second is the one
that matters.

**Waves land more often.** `wave_interval_ticks` 4,800 → **3,400** — from one every two and a
half minutes at 1× to one every minute and three quarters, which is often enough that a tower
has to be *arranged* for a wave rather than repaired after one. It did not go further because
the reasoning that set 4,800 still binds: at 2,400 a recorded run went into a spiral it could
not climb out of, 120 poles of repair against nothing in the bank, because a damaged mill mills
more slowly and a tower that cannot mill cannot mend.

**And something worth standing still for.** `enemy.thicket_mother` is 500 hp — half again a
mire-hulk, the largest number in the pack — at threat 60, gated behind provocation 420. A dart
battery needs 34 hits to fell one; a lone thorn gun would need 63 and run the tower out of
bamboo first. It is felled by a *tower*, not by an emplacement.

It drops 2 alloy and 8 scrap. **That is the whole point of it.**

§11 makes walking away the free answer to every wave, which is right — and *a free answer with
no alternative is not a decision*. A mother is slow enough to walk away from (9 paces per 100
ticks) and carries enough that you might not. Alloy otherwise needs a salvage rig, a forge and
a ruin to stop at, so felling one is a shortcut through a whole chain rather than a pile of the
material you already have.

#### It is a resident, not a boss

`DECISIONS.md` §8 is defenders rather than soldiers, and creatures defending territory rather
than a gallery to clear. A mother is not a health bar with a name on it and it is not hunted:
it is what has been living here the whole time, and it comes out when a tower has made enough
noise in its home that ignoring the tower stops being an option. You are in somebody's house.

That framing is also why **only the residents carry anything**. A jungle where every skitter
pays out is a jungle you farm, and `ordinary_creatures_leave_nothing` is the test that keeps it
one creature.

`felled()` is called from every path that can kill — a battery's shot and a cutter arm's blade
today, whatever comes next tomorrow — because a drop that depends on *how* you fought is a
distinction this game makes nowhere else. Anything that does not fit on a shelf is lost, which
is the same rule the waypoints keep: a tower with nowhere to put two alloy has told you
something about itself.

### 6.16 Growing sideways

**"More buildings, more people, more jobs."** The tower could only ever grow *up*. It can now
grow out as well: `WidenTower` adds `widen_slots` (2) to every floor, up to `max_slots` (16),
for 10 poles against a floor's 6.

**Dearer than growing, and unlike a floor it buys nothing until you put something in it.** A
floor goes on top of what is already there; a wider hull is new frame along the whole height.
Per slot it is cheaper (5 against 6-for-eight), which is what makes it the considered option
rather than the default one.

#### The new frame goes on the back

Everything aboard slides *forward* by two slots — rooms and shafts alike — and the hull grows
at the tail.

That is the one arrangement that works. Weapons are `front_only` (§6.13), so a tower that grew
at the nose would leave every gun it owns standing two slots *inside* itself, at the place the
front used to be. Growing at the back costs a loop over every stored slot index instead, which
is a chore rather than a design problem. Shafts move with the rooms for the same reason: a
shaft occupies a slot *column*, and a column that did not shift would come out running through
whatever the rooms slid into.

The visible consequence is that **the new deck appears behind the stairs**, which move forward
with everything else. A widened tower has open frame at its tail, which is exactly where a
player would expect to be able to build.

#### What it is worth, measured

> **And read §6.30 too: this section asked the wrong question.** "Width is not relief for the
> climb" is true and beside the point — width is relief for *space*, and for the rooms that
> carry `max_floor: 1` it is the only relief there is, since they have two floors however tall
> the tower grows. It is what opens the rope chain, and therefore what decides whether a run
> has a shaft in it.
>
> **Read §6.24 before trusting the numbers below.** This sweep held the harness's room plan
> fixed while widening the hull, so the extra slots stayed *empty* — and the cutter arm was
> capped at one per tower, so nothing could have filled them anyway. It measured an unfurnished
> extension. The conclusion may still hold; it is no longer supported, and the sweep wants
> re-running with a plan that builds into what it buys.

**Width is not relief for the climb, and the price is per floor because of it.**

`examples/lift.rs` sweeps hull width at eight floors, three seeds a row. A stairs-only tower
hauls **56, 56, 56, 58** at ten, twelve, fourteen and sixteen slots, and crew-ticks spent
climbing do not move at all — 18,550, 18,506, 18,401, 18,620. Six extra slots buy four
percent.

That is the answer to what was §6.9's second open question, and it is the reassuring one:
**widening does not undo the shaft.** The chain still spans the tower whatever the floors are,
so a wider floor shortens no climb and M6's argument has no side door in it. What width buys
is *somewhere to put a room*, which makes it a competitor to `BuildFloor` and never to a
shaft.

So it is priced against a floor and it is dearer: three poles per floor of hull, for two
slots. On an eight-floor tower that is 24 poles for 16 slot-floors — 1.5 poles a slot-floor
against a floor's 0.6. Two and a half times dearer than growing up, which is what a purchase
that adds no throughput should cost. **Per floor rather than flat**, because a widening is new
frame along the whole height and a flat fee charged a fourteen-floor tower the same as a
two-floor one for seven times the frame.

#### The thing width actually costs: walking

The same sweep found the more interesting half. **The elevator's value halves as the hull
widens** — +91% hauls at ten slots, +77% at twelve, +57% at fourteen, +48% at sixteen — and
not because climbing got cheaper. Crew-ticks climbing on the lift rows are flat at ~4,000
throughout. What moves is **walking: 21,309 to 28,361**, a third more, because the shaft
stands in one column and a wider floor is further to cross to reach it.

That is the first measurement of the placement hypothesis `lift.rs` has carried in its header
since M6 — *where you put a shaft is the decision, and the game teaches nothing about it*. It
is no longer a hypothesis. A wide tower with one lift at one column is a tower whose crew walk
to the lift.

**The dumbwaiter goes the other way, and the reason is the design.** +93% at ten slots, +284%
at twelve, and it holds there — while its walking *falls*, 9,469 to 4,864. A dumbwaiter is
item-only: nothing rides it, so nobody walks to it. Width hurts the shaft you have to reach
and helps the one that comes to you.

#### The preview was lying, in both directions

`scene.ts`'s `placementFits` carries the comment *"Mirrors the command validation, so the
highlight never lies"*, and it had stopped doing so. It knew nothing about `front_only`
(§6.13) or `front_slots` (§6.21), so the ghost offered a weapon **every** free slot on a floor
and offered an ordinary room the **weapons deck** — and the only way to find out either was to
click and be refused.

Both rules are in it now, checked against a room's whole footprint rather than its left edge,
and `the placement preview tells the truth about the leading edge` holds it.

#### Why it is the whole hull and not one floor

The obvious refinement — widen the floor that needs it and leave the rest — is not available,
and the reason is structural rather than a decision to revisit. Widening slides everything
forward to keep the leading edge where it is, because weapons are `front_only`. A shaft
occupies one slot *column* across every floor it spans. Widen floor 3 alone and floor 3's
rooms shift two slots while floor 4's do not, and the shaft column now runs through whatever
floor 3 slid into.

Either the front is a fixed edge and per-floor width is free, or the front is the high end and
width is a whole-hull property. The tower faces right and its weapons stand on the leading
edge, so it is the second.

### 6.17 Practice, and the order the tower works in

Two things, and they are one thing: **what somebody is good at, and what the tower
reaches for first.**

#### The work order

`assign_idle`'s ladder was hardcoded and had been since M0. It is still a ladder, and the
top of it is still fixed — a trip already under way, the rota, and dinner, in that order —
but the four rungs below are now `GameState.work`, which the player sets.

| Job | What it is |
| --- | --- |
| answering | Go and stand between a thief and what it is taking |
| mending | Put damage back |
| working a post | Go to the room you were stationed to |
| hauling | Carry something somewhere |

The enum order is the default order and it is an argument rather than a habit: something
happening *now* beats something that already happened, which beats a standing order, which
beats the background work that is always there. A player who disagrees says so, and the
one that matters most is **whether a stationed gunner leaves the post to mend a wall**.
By default they do.

**Needs are not jobs and are not offered as settings.** There is no rung for eating or
sleeping, because a player who could rank hauling above dinner would only be building the
starvation trap — offering it as a setting would be the game pretending a mistake is a
strategy. `tests/needs.rs::no_work_order_lets_anybody_skip_dinner` is that property.

`SetWorkOrder` takes the whole order and rejects anything that is not a permutation. A
list with a job left out is a list that has quietly made that job unreachable: nobody
would ever mend again and nothing would say so.

**One order for the tower, not a rota per person.** A per-person matrix is the shape that
turns crew into a spreadsheet, and it answers a question the player is rarely asking —
what they want to say is *stop mending and get the harvest in*, which is one sentence
about the whole tower. Somebody who should be doing one specific thing has `stationed`
already, and that is per-person precisely because it is the exception.

#### Practice

**This is the one place `DESIGN.md`'s fourth structural call has been amended rather than
followed.** That call read "named individuals with jobs, not stat blocks", and the second
half was doing work the first half did not need: three people who are identical on day one
and identical on day nine are three units, and naming them does not fix it.

A tick of doing a job is a tick of practice at it. 3,600 ticks — a bit under half a day
shift — is a rank; three ranks is the ceiling; each is worth ten percent. What that buys,
per job:

- **hauling** — every leg is quicker: the walk, the climb, the loading, the unloading
- **mending** — a repair shift is shorter
- **answering** — a shooing is shorter
- **working a post** — the room's craft is shorter, on top of `manned_work_pct`

Eating, sleeping and the *length* of a posting are untouched, and the omissions are
deliberate: eating faster is not a skill anybody wants modelled, sleep is the one thing in
the game left deliberately un-optimisable, and a posting has no duration to shorten.
`arrive` decides all five in one place and says so.

**Walking and climbing count as hauling wherever they are going.** What a porter learns is
the building — which stair is quicker with a crate on, where the landings are — and that
does not evaporate because this particular trip is toward a broken panel.

Four terms keep this from being a character sheet:

1. **Earned by doing, never assigned.** There is no screen where a player spends anything.
2. **Small.** +30% at the ceiling. Enough that the veteran on your stairs is *your*
   veteran; never enough that there is a correct assignment.
3. **Shown as pips, with the numbers on hover** (`DECISIONS.md` §8), and only the best job
   per person — a four-column grid of everybody's skill at everything is the spreadsheet
   this design keeps refusing.
4. **Cannot be lost or spent.** Somebody who spent a week on the stairs and is now stood at
   a gun still knows the stairs.

It is also **the first multiplier in the game allowed above 100%.** `work_pct` still clamps
there, because being fed and rested is the baseline and neglect is what costs you; practice
is a separate figure that stacks on top, so a starving expert is still starving. Keeping
them apart is what stops a rank quietly cancelling an empty pantry.

**A posting takes the best rank in the room, not the sum.** Two people at a mill is already
worth something — `crew_required` counts heads — and adding ranks on top would make
stacking bodies the answer to everything. What a rank says is *somebody here knows this
machine*, and a second person does not make that truer.

#### What is not settled

Nobody has played a run at 20% a rank or at 5%. The measurement behind these numbers is a
maximum against a zero — `tests/needs.rs::a_practised_crew_gets_more_done`, two towers on
one seed where the second tower's crew start at the ceiling — which honestly answers *does
any of it reach the tower* and does not answer *is ten percent right*. Carried into §6.9.

### 6.18 One shaft, two jobs

**The dumbwaiter is gone, and the lift does its work.** There were two built shafts that went
up and down; now there is one, and `shaft.dumbwaiter` no longer exists in the pack, the enum
or the frontend.

#### Why it was the dumbwaiter that had to go, and not the lift

The ladder the design described was *dumbwaiter first, elevator when the tower is tall*. The
measurement said something else. `examples/lift.rs`'s width sweep, at eight floors:

| hull | stairs | dumbwaiter | elevator |
|---|---|---|---|
| 10 slots | 56 hauls | +93% | +91% |
| 12 | 56 | **+284%** | +77% |
| 14 | 56 | +286% | +57% |
| 16 | 58 | +272% | **+48%** |

The cheap rung beat the dear one at every width, and the gap *widened* as the hull grew — the
elevator losing nearly half its value while the dumbwaiter held. The cause is one line of
design: **nothing rides a dumbwaiter, so nobody walks to one.** An elevator's crew have to
cross the floor to reach it, and a wider floor is further to cross; a dumbwaiter's cargo does
not walk anywhere.

A ladder whose bottom rung wins on both counts is not a ladder. So the thing that made the
dumbwaiter good became something the lift does.

#### What the merged shaft is

A lift with nobody calling it goes and fetches stock, in batches of four, using the same
scoring the crew use — a hungry recipe outranks a shelf. Concretely:

- `seek_freight` runs on a car `dispatch` left idle with no riders, no stops and no load. It
  is the old dumbwaiter finder, **unchanged in its scoring**, because that scoring is the
  thing worth keeping.
- Where a dumbwaiter drove itself to a `target`, the lift pushes the destination onto its
  `stops`. Its own routing carries the crate, so **a rider calling mid-trip is served on the
  way** rather than waiting for the freight to finish.
- `service_stop` unloads freight alongside the riders. A lift already stopping to open its
  doors puts the crate down while it is there — which is why the merge costs one call rather
  than a second state machine.
- **Riders always outrank freight.** Fetching only ever starts on an idle car.

**The daypart program binds freight too**, and forgetting that was the one bug this
introduced: a lift told not to serve a floor went and fetched from it anyway, because the
finder scanned the shaft's whole span and knew nothing about the schedule.
`an_unserved_floor_is_not_stopped_at` caught it. A program is the player saying *this shaft is
not for that floor right now*, and a statement binding half a shaft's traffic is worse than
none.

#### What it measured

The same sweep, after:

| hull | stairs | the lift |
|---|---|---|
| 10 slots | 57 hauls | +256% |
| 12 | 55 | +271% |
| 14 | 56 | +264% |
| 16 | 57 | +251% |

**Flat across width** — the elevator's collapse from +91% to +48% is gone, because the half of
the shaft's work that used to require a walk no longer does. Across height it is +119% at five
floors, +256% at eight, +493% at eleven, +913% at fourteen. The last figure is against a
stairs-only tower that is nearly dead (15 hauls), so read it as *height without a shaft is
ruinous* rather than as a number about the lift.

#### The prices

10 poles + 3 rope, between the dumbwaiter's 8+3 and the lift's old 12+4 and nearer the low
end: this is the cheap rung as well as the dear one now, and a tower that must save for the
lift or have nothing has no answer at all for the first twelve minutes of queueing `lift.rs`
measures. `charge_per_floor` drops 5 → 4, because one shaft doing both jobs runs in every gap
between riders rather than only when called, and the per-floor draw is paid far more often.

#### What is not settled

The merged shaft has not been swept — 10 poles and 4 charge are set against the two rows they
replace, not measured against neighbours. And there is now exactly **one** built shaft that
goes up, so the whole "which shaft" decision the ladder was supposed to offer is gone. What is
left is *whether*, *where* and *how tall*, which §6.9's shaft-placement question already says
the game teaches nothing about.

### 6.19 A shorter run, and what it cost

**The journey is scaled to 0.85 and the walker's floor is 31–36 minutes**, down from 37–44.
Every pace-denominated distance in the region pack moved together — region lengths, waypoint
and fork intervals, branch lengths, enclave positions — so the *shape* of a run is untouched:
the same number of beats, the same number of forks, the enclave at the same fraction of the
way through. Only the clock moved.

Twelve seeds, twelve arrivals.

#### 30 was the target and 30 does not work yet

The ask was thirty minutes. A scale of 0.73 delivers it — **28–32 minutes, 12/12 arriving** —
and it is not shipped, because at that length **a tower cannot afford a lift before the run
ends.**

Measured on `examples/record_golden.rs`, which walks a scripted tower the whole way and buys
in the best order found: at 0.73 the tower covers all 33,753 paces, builds its ladder, its
chain, a storeroom, a canteen, two burners, a comb and a ropery — and reaches the Refugia
holding **four poles** of the ten a shaft costs. It never gets them, because a tower that has
arrived earns nothing. A lift is worth +256% hauls at eight floors (§6.18), so a 30-minute run
is a run without the game's largest single improvement in it.

Four compensations were tried and all four measured worse:

1. **Raise harvest.** `paces_per_item` 78 → 57 on the cutter arm made the tower *poorer*: the
   arm outran the mill, bamboo claimed the shelves, and poles had nowhere to land. The cutter
   arm's own balance row has warned about this since M2, and this is the second time it has
   been paid for.
2. **Buy the shaft first**, on the grounds that it is the throughput multiplier. Worse — ten
   purchases against twelve. The lift's +256% is measured at eight floors and this tower has
   five.
3. **Cut the price to six poles.** No effect: four is less than six too.
4. **Grant the tail.** Diverges the replay at the first granted pole, because a grant is not a
   `GameCommand` and the fixture replays commands. The fixture was right to refuse it.

0.85 is the shortest scale at which the fixture completes. That makes **31–36 minutes the
current floor for a run that contains a shaft**, and the gap between that and thirty is a real
design question rather than a tuning oversight: either the run is a little longer than thirty,
or a tower's mid-run income has to rise without the shelves jamming — and nothing measured so
far does the second.

#### Two things dropped out of the fixture on the way

Both because a shorter run pays for less, and both worth knowing:

- **The weapons.** A dart battery and a thornwright used to be scripted. They are not
  affordable at this length. Weapon *behaviour* is still covered — the opening tower ships
  with a thorn gun (§6.11) — but placing one is now only covered by `tests/`.
- **The second bunk.** The opening tower ships with one bed and three crew, so a tower is
  already short of hammocks and the fixture still records both halves of sleep — somebody in a
  bed and somebody on the deck — without buying anything.

**And the berth had to move earlier.** The script is longer than the run: 30 minutes is about
54,000 ticks and the recording is 119,000, so by the time the tower has saved for a shaft it
has *arrived* — parked, with no ground streaming past and therefore no ruin that could ever
come into reach. It failed exactly that way, forty thousand ticks of a stationary tower
looking for one. Anything in the fixture that needs *terrain* now happens while the legs are
still moving.

#### What is not settled

The thirty-minute target, above. And every figure in `BALANCE.md`'s Journey section is now
doubly stale — read off a broken harness, and then measured against a journey 15% shorter.

### 6.20 Another car, not another column

**`AddCar` puts a second and a third car in a shaft that already exists.** The simulation has
been ready for this since M1 — `run_elevator` loops over cars, and `transport.rs`'s header
notes that fixed queues belonging to *floors* rather than to cars is "what lets a second car
share a shaft later". What was missing was the command, a ceiling, and a price.

**The argument is scarcity.** A shaft is a *column*, and a column is the scarcest thing a tower
owns: it costs a slot on every floor it passes through, for ever (`DESIGN.md` pillar 2). So the
late-game answer to a queue should not have to be another column. A car costs no width at all.

Capacity is applied per car, so **a second car is a second carload rather than a faster one**.
What it buys is a *turn*, not pace — which is the right shape for a queue, because a queue is
people waiting for a turn.

Three cars maximum, at 6 poles and 2 rope each — two thirds of the shaft, in the same two
materials. A new car starts at the foot of the shaft rather than beside its sibling: spawned
next to the existing one it would arrive with the same sweep in front of it and spend its first
minutes shadowing it.

#### What it measured, and what that says about the tower

`examples/lift.rs` gained a sweep: eight floors, three seeds, crew across and cars down.

| crew | 1 car | 2 cars | 3 cars |
| --- | --- | --- | --- |
| 3 | 202 hauls (4,174 queued) | 225 **+11%** (1,718) | 229 +13% (1,694) |
| 5 | 203 (6,625) | 224 +10% (2,212) | 225 +11% (2,200) |
| 8 | 210 (10,514) | 225 **+7%** (2,780) | 227 +8% (2,346) |

**A car does exactly what a car is for**: crew-ticks spent standing at the shaft fall 60–75% at
every crew count. That is the number it is bought to move and it moves a long way.

**And it pays less at eight crew than at three, which is the opposite of the intended shape.**
The reason is in the first column: going from three crew to eight buys **+4% hauls**. This
tower is not crew-bound. Hauls plateau near 225 whatever is thrown at the transport, so
something downstream of the shaft binds, and no number of bodies can make a car more necessary
while that is true.

That is the honest state of "make multiple cars necessary late game through economy and more
crew": the command exists, the car works, and **the pressure that would require one does
not**. The sweep holds the room plan fixed — growing the tower alongside the crew is the next
piece of work, and until it is done this is a purchase that helps a little everywhere rather
than a lot late. Carried into §6.9.

### 6.21 The weapons' deck

**The outermost two columns of every floor take nothing but weapons.**

`front_only` has said since §6.13 that a weapon must stand on the leading edge. It never said
the leading edge was *for* weapons — so an ordinary room could take that column, and the floor
became unarmable with nothing on the card explaining why. `front_slots` is the other half of
the rule, and `OnTheWeaponsDeck` is what a mill aimed at the edge now gets.

Checked against the room's whole footprint rather than its left edge, for the same reason
`slot_range_blocked` is: a three-wide room at slot 7 of ten covers 7, 8 and 9, and testing the 7
alone reserves nothing.

Two slots, because two is the widest `front_only` room in the pack — a deck that could not hold
the widest weapon would be a deck with a footnote, and there is a test asserting it against the
pack rather than against the number. Out of ten that reserves a fifth of every floor, paid on
every floor, so **a taller tower reserves more**. That is the cost of being a tower that can
shoot back, and it is deliberately the same shape as a shaft's column tax.

Every fixture in the repo already avoided these columns, so turning the rule on moved no
measured number anywhere. That says the rule agrees with how towers were already being built;
it does not say what it costs a player who wanted that slot.

### 6.22 Weapons that answer one kind of trouble

**Every emplacement used to answer every approach**, and that made the set a ladder rather than
a loadout: a dart battery beat a thorn gun at everything, so there was a best weapon and the
rest were worse ones. Variety was a damage number.

`DefenceDef.targets` is the axis it now lives on. Creatures already arrive three ways — along
the ground, out of the canopy, and up through the legs — and each is a genuinely different
problem. A weapon that answers one of them well and the others not at all is a *choice*.

| weapon | eats | answers | damage | reload | range | the fantasy |
| --- | --- | --- | ---: | ---: | ---: | --- |
| thorn gun | bamboo | all | 8 | 90 | 5 | the thing you can always feed |
| dart battery | darts | all | 15 | 40 | 60 | the standoff workhorse |
| seed thrower | seed bombs | all | 8 | 90 | 30 | the mid-range answer |
| **lantern mast** | charge cells | canopy | 12 | 120 | 20 | *lighting your own canopy* |
| **tanglenet** | rope | ground | 4 | 20 | 12 | *making the ground sticky* |
| **root ward** | alloy | burrow | 25 | 75 | 10 | *guarding your own legs* |

The three that answer everything stay generalists — an empty `targets` means all — so the
opening tower is not suddenly a puzzle. The new three are specialists, and each one asks the
tower for something different.

#### Each is a different demand on the chain, not a different number

- **The lantern mast burns charge cells**, which need alloy, a cellwright and a chain. `Lamps`
  and `Lifts` already compete for the bank (`state/power.rs`); this puts a third claim on it.
  It is two slots wide — exactly `front_slots` — so a floor with a mast on it is a floor that
  is watching the sky and doing nothing else.
- **The tanglenet burns rope**, and that closes a problem this project has carried since M5.
  Rope's only consumer was a build cost, which is a one-off, so a ropery left running filled the
  tower with something nothing ate and the answer was *remember to switch it off* — recorded as
  a poor one by §5.11 and by `BALANCE.md`'s ropery row. A tanglenet eats rope for as long as
  there is anything to throw it at.
- **The root ward burns alloy**, the dearest material in the game: a salvage rig, a ruin worth
  stopping at, and a forge. So answering burrowers is something a tower that *berthed* can do
  and one that walked past every ruin cannot. §5.10 wanted route choice to reach into another
  layer, and this is it reaching into defence.

#### The tone gate

M6 cut crew fighting boarders and weapon loadouts rather than softening them (§6), so a new
weapon has to earn its place against `DECISIONS.md` §8 — defenders rather than soldiers,
creatures defending territory rather than a gallery to clear.

**The lantern mast is the clearest pass**: it is not a gun. It is a mast of lamps that makes the
branches above the tower somewhere a leaper does not want to launch from. It says *we are awake
up here*, and the creature goes somewhere else.

**The tanglenet is the second**: it does almost no damage, and it is not supposed to. It holds
something at arm's length while the legs carry the tower out from under it — §11's walking-away
made into a room.

The root ward is the one that reads most like a weapon, and it is deliberately the one pointed
at the ground the tower is about to walk over rather than at a creature across a field.

#### What is not settled

The filter is measured; the *set* is not. Nobody has played a tower choosing between a mast and
a ward with two front slots and a wave inbound, which is the decision all of this exists to
create. Carried into §6.9.

### 6.23 Is the lift mandatory? Measured, mostly yes

The ask was to balance the tower so a lift is more or less mandatory. **It already is, and the
figure is the argument**, so this section is mostly a measurement and one small change.

`examples/lift.rs`, three seeds a row, two whole days after a warm-up:

| floors | stairs only | with a lift |
| ---: | ---: | ---: |
| 5 | 107 hauls | 235 **+119%** |
| 8 | 57 | 203 **+256%** |
| 11 | 30 | 178 **+493%** |
| 14 | 15 | 152 **+913%** |

**A stairs-only tower at eight floors does 28% of the work a lifted one does**, and at fourteen
it does 10%. That is not a tower with a disadvantage; it is a tower that has stopped. The
design intent — a shaft is the belt, and a tower that grows without one is a factory with no
belts (`DESIGN.md` pillar 2) — is doing exactly what it says.

Five floors is the only height where a tower survives without one, at 45%. That is the right
shape: the opening should be playable on stairs, and everything above it should not.

#### What is actually wrong is *when*, not *whether*

The golden recorder walks a scripted tower and prints the tick each purchase lands on. The
ropery — the last room before rope exists — lands at **tick 46,129 of a run that ends near
56,000**. A lift bought in the last fifth of a run is a trophy, not infrastructure.

The gate is not the cause. A fiber comb and a ropery are four poles each and both open the
moment the burner is lit, at tick 2,400 — so a player who *wanted* a lift could have the chain
by minute ten and skip the canteen and the second burner to do it. The recorder buys comfort
first, which is a reasonable thing for a script to do and hides the option.

So the one change here is the last coil: **the lift costs 2 rope rather than 3.** A ropery
makes one coil from two fiber every four seconds, so each coil is a separate wait behind a
separate pair of fiber, and the third one buys nothing the second did not. The gate is *which*
material — fiber comes from the middle bands, so it still says "you have to have walked
ordinary ground" (§5.10) — and three said that no better than two, and said it later.

#### What is not settled

Whether a player *feels* the 28% before they can act on it. The number says a stairs-only tower
is failing; nothing in the game says so out loud except the crew tinting red at a queue, and
§6.19 already carries the affordability half of this. Carried into §6.9 with them.

### 6.24 A tower could own exactly one cutter arm

**This is the largest balance finding in the project, and it is one line of content.**

`cutter_arm.ron` read `max_floor: 1`. The arm is also `front_only`, and a floor's leading edge
holds one two-wide room. So: one arm per floor, two floors allowed, and the opening tower's
thorn gun already standing on floor 0's edge — **a tower could own exactly one cutter arm, for
the whole run, and nothing said so.**

Every economy measurement in this repo was taken against that ceiling.

#### How it was found

§6.20's car sweep went looking for the pressure that makes a second car necessary and found the
opposite: three crew to eight bought **+4% hauls**, and hauls plateaued near 225 whatever was
thrown at the transport. That went into §6.9 as *what actually binds a tower, if not crew?*

Doubling the cutter arm's rate through `UNDERSTORY_PACK` — one edit, forty seconds — moved the
plateau from 225 to 408. So it was intake.

Then three more arms were added to `lift.rs`'s plan and **none of them landed**, twice, with
the numbers coming out unchanged and nothing to read. Printing the first rejection rather than
the last said it in one line: *a weapon goes on the front of the tower: slot 10, not 0*.

**The last error was useless and the first was the whole answer.** `place` reported the final
slot it tried, which is always "slots run past the floor width" — the search reaching the end
rather than the reason it got there. It reports the first now.

#### The change, and what it costs

The cap is gone. The fiction it rested on — *the arm reaches the ground* — was never quite
right: bamboo grows tall, and an arm on floor five cuts at floor five's height as the tower
walks through. **Intake now scales with height**, which is what the factory pillar wanted all
along: build up, harvest more, haul more, need a shaft.

`front_only` stays, and it stops being an afterthought: an arm is two slots and a floor's
weapons deck is two slots (§6.21), so **a floor that harvests cannot shoot.** Every storey is a
choice between feeding the tower and defending it, which is the loadout tension the deck was
reserved for.

#### What it measured

`examples/lift.rs`, eight floors, three seeds, with the plan grown to four arms and three
mills — because a chain that does not scale with its intake simply jams, which the first
uncapped run showed by *halving* hauls at one car.

| crew | 1 car | 2 cars | 3 cars |
| ---: | ---: | ---: | ---: |
| 3 | 92 hauls | 134 +46% | 275 **+199%** |
| 5 | 94 | 245 +161% | 372 **+296%** |
| 8 | 81 | 230 +184% | **447 +452%** |

Against the same sweep before the uncap — 202/225/229 at three crew, 210/225/227 at eight —
three things changed and all three are the design working:

1. **Crew matter.** Three to eight crew was +4%; at three cars it is now **+63%**.
2. **Transport is the binding constraint**, which every design argument in the project assumes
   and none of them could previously demonstrate. One car holds a tower to 81–94 hauls
   *whatever* its crew; three cars take the same tower to 447.
3. **A tower that grows without transport chokes.** One car is now *worse* than before the
   uncap. That is `DESIGN.md` pillar 2 exactly — a factory with no belts — and it is the
   pressure a shaft is relief from.

#### What this invalidates

**§6.16's width sweep.** It concluded "width is not relief — six extra slots buy four percent",
and the harness plan was fixed, so the extra slots stayed *empty*. It measured an unfurnished
extension. The conclusion may still be right and it is no longer supported; the sweep wants
re-running with a plan that fills what it builds.

Everything in `BALANCE.md` measured through `lift.rs` or `chain.rs` predates the uncap.

#### What is not settled

The chain now has to be scaled by hand to match the intake, and nothing in the game teaches
that. A player who builds four arms and one mill gets the jam this section found by accident.
Carried into §6.9.

### 6.25 Traits: everybody aboard is somebody in particular

**One trait per person, drawn when they come aboard.**

Three crew who are identical on day one and identical on day nine are three units, and naming
them does not fix it — that was §6.17's argument for practice, and this is its other half.
Practice is what somebody *became*; a trait is what they arrived as.

**Forty of them, in three rarity tiers**, so a run shows you a handful of many.

| tier | weight | how many | what they are |
| --- | ---: | ---: | --- |
| common | 100 | 20 | one axis, modest — nocturnal, big appetite, long-legged, anxious |
| uncommon | 40 | 13 | two axes or a real trade — glutton, pathfinder, field medic, nightborn |
| rare | 10 | 7 | striking — sleepless, ox, featherfoot, lamplighter, born aboard |

**Rarity is the reason there are forty rather than five.** A pack where every trait is equally
likely has no rare ones by definition, and somebody merely *unusual* is worth more than
somebody strong: the common traits are quirks you plan around, and the rare ones are why you
remember a particular run's roster. Seven rare traits at a tenth of a common one's weight come
up about three percent of the time, and `a_rare_trait_is_actually_rare` measures that through
the draw rather than asserting it about the weights — a weighted table with a bug in it still
has the right weights in it.

The rare tier is deliberately **not priced**. An *Ox* carries double the base load for no cost,
and *Sleepless* is a crew member the rota barely applies to. They are not balanced against the
common ones and are not meant to be; they are balanced against how seldom they arrive.

#### Shaped like a need, not like a bonus

A trait that read "+10% to everything" would turn the crew back into a build order, which is
what `DESIGN.md`'s fourth structural call spends its length refusing. So the axes are all
things the tower already has a system for, and most land on the **rota** or the **canteen** —
systems the game has and barely uses.

Thirteen axes, every one a percentage of the pack's own constant: hunger, rest in a bed, rest
on the deck, the tired threshold, how much a night buys, walking, climbing, when stress shows,
mending, what they carry, whether they see in the dark, which shift they start on, and what
they already know how to do. A trait that is every default fails to load.

Two are worth their own note. **Sure-footedness scales the per-floor climb and not the
per-item one** — being good on stairs is about the stairs, and the tax on carrying freight up
them is what a shaft exists to answer (§6.24); a trait that undercut it would be a person who
does not need the game's central building. And **stress is presentation with real teeth**:
`wait_ticks` is the whole bottleneck instrument (`DECISIONS.md` §8), so somebody who shows it
late is somebody whose queue you notice last, which is a mixed blessing rather than a gift.

`SetShift` has existed since M4 and most towers never touch it, because putting somebody on
nights is a straight loss. Somebody who rests *better* off-shift is the first reason to open
the roster and move a name. And the two sleep traits point in opposite directions on purpose:
one hands the tower a bunk back, the other makes the second bunk a decision.

**The mender's negative carry is the most interesting field in the file.** Somebody who is a
poor porter is somebody you *post* to a room (§6.12) rather than leave on the stairs — which is
a reason the stationing system has wanted since it shipped.

And the big appetite is deliberately **not** paired with a compensating bonus. Somebody can
just be hungry. A crew where every quirk nets to zero is a crew of interchangeable people
wearing labels.

#### Which stream it rolls on, and why that is the whole determinism question

**The `sim` stream, not `cosmetic`.** A name and a `fidget` are cosmetic precisely so that
adding a bark can never perturb an economic roll (`DECISIONS.md` §2). A trait changes how fast
somebody gets hungry and how much they carry — it *is* economic — so it belongs on the stream
that already carries the economy. Recruiting somebody perturbing that stream is correct:
recruiting is an economic act.

`a_trait_rolls_on_the_sim_stream_not_the_cosmetic_one` holds it from both sides: the same seed
gives the same crew, and ten seeds do not all give the same one.

Every field is a percentage of the pack's own constant, so a trait always reads as *this
person, against everybody else* rather than as an absolute nobody can check. They compose
multiplicatively, so a pack that adds a sixth trait cannot drive a stat negative by accident.

#### The three you set out with have none

**And that is a decision about the opening, not an oversight.** §6.11 rebuilt the first ten
minutes around a build ladder precisely so a new player is not handed a roll they cannot read,
and a run is now half an hour (§6.19) with a tower already one purchase short of a shaft. Three
crew drawn from a table containing *Heavy-footed*, *Quick to tire* and *Big appetite* is a
large swing on the most fragile part of the game, decided before the first pace.

Measured while building this: one unlucky opening draw moved the golden recorder's ropery from
tick 46,129 to **65,886**, and its lift stopped being affordable at all. That is a 43% slower
run from a roll nobody made.

So variety arrives with the people you *choose* to bring aboard. The roll still happens for the
starting three and is discarded, so the `sim` stream advances identically whether or not this
rule changes again.

#### What it does not do

**There is no offer, and no accept or reject.** Recruiting is still: berth at an enclave, pay
the price, and the next name off `names.ron` comes aboard — now with a trait attached. Turning
that into *this named person, with these traits, yes or no* is the obvious next thing and it is
not built.

Nor is there race. It is nearly free in the simulation and expensive everywhere else: `ART.md`
has fourteen prompts unwritten and the game ships zero images, with portraits done as
`fidget % faces`. It would commit the setting in a way the fiction has not yet.

### 6.26 Which silence it is

**§6.9's first open question was wrong, and measuring it produced a better answer.**

It read: *a tower that harvests from every floor with one mill jams — bamboo claims every shelf
and poles have nowhere to land.* That mechanism was asserted, not measured. Four arms, one
mill, thirty thousand ticks:

```
shelves: bamboo=0  poles=100  free=0 | mill in=6 out=6 | idle crew=3/3
arm buffers: [8/8] [8/8] [8/8] [8/8]
```

**It is the other way round.** The mill converts everything it is given, so every shelf ends up
holding *poles* and bamboo reads zero. Nothing is broken: backpressure propagates exactly as
designed — shelves fill, so the mill's outbox fills, so the mill stalls, so bamboo is not
consumed, so every arm's buffer fills, so the arms stall. The tower stops cleanly and holds a
hundred poles, which is a rich tower rather than a jammed one, and the escape is to spend.

Two other things the measurement settled:

- **A pole sink does not help on its own.** Adding a thornwright changes nothing, because its
  darts have nowhere to go either — once every shelf holds one item, the tower can never store
  a second kind again.
- **Nor does a chute**, and that is correct rather than a gap. `find_destination` will only
  spill what *nothing wants*, and poles are wanted by every build cost in the game. A chute
  that threw poles into the jungle would be the worse failure the `may_spill` rule was written
  to prevent.

#### What was actually missing

`RoomView.stalled` was a single boolean, so **a room quiet because nobody has brought it
anything and a room quiet because nobody wants what it makes looked identical** — and they are
answered by opposite actions. Feed the first. Spend from the second.

Four cutter arms standing quiet because the tower is full of poles read exactly like four arms
on bare ground.

`StallTag` is the fix: wrecked, off, shaded, unarmed, unfuelled, starved, backed up — first
match wins, most-answerable first. It reaches the player as **hover text on the room label**,
never as a banner and never as a number: `DECISIONS.md` §8 still holds, the silence is still
the signal, and this only says which silence it is.

| tag | what the tower says |
| --- | --- |
| starved | *waiting on something nobody has brought* |
| backed up | *nowhere to put what it makes* |
| unfuelled | *nothing to burn* |
| unarmed | *nothing on the rack* |
| shaded | *in the tower's own shadow* |

#### And the cross-section draws them differently

The hover is the precision layer, and the precision layer is the one players do not use. So the
two silences are drawn apart as well:

- **Starved** stays dim. The lights are off, nobody has brought anything, and the room going
  quiet is the same signal it has always been.
- **Backed up** is *not* dim. It is **packed** — the stock it cannot get rid of drawn as
  stacked bands filling the room to the ceiling.

**The first version of that was a single translucent wash of `outputFill`, and it was wrong**:
it came out brighter than the working rooms around it, which inverts the whole point. A room
that has stopped must never be the loudest thing on the screen. Bands read as stock piled up
rather than as a highlight, and they use the vocabulary the rest of the cross-section already
speaks — shelves are pips, buffers are wells, quantity is repeated marks.

`e2e/capture.spec.ts::capture the two silences` builds the tower that produces both at once and
photographs it. It asserts nothing. It is there to be looked at, which is how visual questions
get answered here (`RENDERER.md`).

### 6.27 A weapon is unlocked by the room that feeds it

**An economy review, and it found the problem in what M6 had just shipped.**

Mapping every material to its consumers turned up one anomaly and one outright error.

#### Ammo chains, not build costs

The build tree is *shallow* — of twenty-four rooms, exactly one was tier two. What is deep is
what a weapon **eats**:

| weapon | costs | eats | chain to feed it |
| --- | --- | --- | --- |
| thorn gun | 3 poles | bamboo | intake — always |
| dart battery | 6p + 2r | darts | thornwright |
| tanglenet | 4p + 2r | rope | ropery |
| seed thrower | 6p + 3r + 2 mech | seed bombs | bombary |
| **lantern mast** | 5p + 2r | charge cells | rig → *berth* → forge → cellwright |
| **root ward** | 5p + 1 alloy | alloy | rig → *berth* → forge |

The mast and the ward — both added in §6.22 — are **cheap to build and need the deepest chain in
the game to feed**. A player spends five poles and gets a weapon that never fires. That is the
exact failure the pack already records against M1's thornwright, shipped again.

So: **every emplacement is now unlocked by the room that supplies it.** The menu offers a
tanglenet once you own a ropery, a mast once you own a cellwright. It is a fact about the tower
rather than about the player, which is the same rule the opening ladder runs on (§6.11), and it
makes the weapon menu self-explaining — *you unlock a gun by building the thing that feeds it.*

The thorn gun has no gate, and that is why it is the starter: it eats raw bamboo, so it is the
one weapon a tower can always feed.

#### The garden was making a dead material

`produce` had exactly one consumer — the bombary — whose output needs a seed thrower, which
cost **two mechanisms**, which need a salvage rig, a berth, a sun-forge and a fitter.

The garden is **rung one of the opening ladder**. It is the first thing the game makes a player
build, it demands `crew_required: 2` of a three-person crew, and what it grew had no consumer
within reach of a run. The repo's own fixture says as much: *"nobody is posted to the farm, so
the farm does not run… posting two of three crew to it would quietly take two thirds of the
tower's hands away from hauling."* The garden was a tollgate.

The mechanisms are gone from the seed thrower. That row read *"the mechanisms are what make it
tier two"*, which was right when a run was two to four hours; a run is **31–36 minutes** (§6.19),
so "tier two" had quietly become "not in the game". Produce → bombary → seed bombs → thrower is
now entirely tier one, and the sun axis buys something a run can reach.

#### And a panel to see it from inside the game

Both findings above were **invisible from inside the game**. You learned that produce had no
consumer by building a garden and watching nothing happen to it; you learned a lantern mast
could not be fed by building one and watching it never fire. The economy was legible only by
reading the content pack.

`the chain` (a toggle above the sidebar) draws it. Materials are the nodes, laid out left to
right so that each column is made from the one before it; rooms are the **edges**, listed
underneath with what they take and what they give. Live, from the same snapshot the simulation
runs on — there is no second model of the economy to drift out of step with the first, which
matters more than it sounds, because a diagram that lies is worse than no diagram.

What it marks:

- **A material nothing consumes** is dashed and dimmed. That is the §6.27 finding made visible
  rather than needing a review to notice.
- **A material a room you own is taking** is outlined. Everything else is a chain you have not
  built yet.
- **A room's row** is bright if it is running, warns if it is stalled, and carries the stall
  reason from §6.26 — so *nowhere to put what it makes* appears beside the room that cannot
  place it.

Four kinds of consumer had to be gathered for the graph to be honest — a recipe, a burner's
fuel, an emplacement's ammunition, and the crew's dinner. Missing any one makes a material look
like a dead end when it is not, so `burner_fuel` and `meal_item` are on the wire now; the first
version had bamboo looking terminal.

**The lanes were clipped and nothing said so.** `overflow-x: auto` makes a flex child
shrinkable below its content, so the long room list underneath squeezed the graph to three rows
and quietly dropped scrap, rope and seed bombs — a graph hiding a third of itself while looking
tidy. It is `flex: none` now, and a smoke spec walks every material the pack moves and asserts
each one is on screen.

#### What the review did not change

- **Bamboo carries four consumers** — mill, canteen, burner, thorn gun — and that contention is
  the design (§6.10), not a fault.
- **The canteen keeps eating bamboo**, tempting as it was to feed it produce. `BALANCE.md`
  measures that demand as the thing which makes terrain yields *felt* at all, and moving food
  onto the sun axis would put a deep-canopy tower one bad route from starving.
- **Poles look like a dead end and are not.** A tower that out-produces its own building fills
  every shelf with poles and stops — measured in §6.26 — but that is a rich tower, and the
  escape is to spend.

### 6.28 A mark over their head

**What somebody is doing, on the person rather than in a panel.**

The roster has said it in words since M4 — *"seeing something out"*, *"held up at the stairs"* —
but the roster is a panel on the right and the crew are in the cross-section in the middle. So
watching the tower meant guessing, and reading what anybody was up to meant looking away from
them.

One glyph over the name: asleep, eating, mending, at a post, seeing something out, waiting for
a way up, climbing, carrying. Nothing new crosses the bridge — `CrewStateTag` has been on
`view.crew` since M0.

**Walking and idle wear nothing, and the blank is the design.** They are the two commonest
states in the game: walking is already legible from the movement, and a badge over every idle
person during a quiet minute is a screen full of punctuation. A mark on everybody at all times
is wallpaper, and wallpaper is not a signal. What earns a glyph is a state you would otherwise
have opened a panel to learn.

### 6.29 A recruit is a person, not a purchase

**§6.9's first open question, closed.** Recruiting was: berth at a settlement, pay the price,
receive the next name off `names.ron`. Forty traits existed (§6.25) and a player never chose
between them, because nobody was ever *shown* before the money changed hands.

The settlement offers somebody now. The board says who they are and what is true about them —
*Marek · sleeps through daylight, and does not mind the dark* — and `Recruit` hands over that
person rather than a fresh roll.

**Drawn once when the tower berths and held until it walks on.** Rolling every tick would let a
player stand still and watch the names cycle until an *Ox* came up; drawing on arrival means
the person standing there is the person standing there, and **passing costs you the visit
rather than nothing** — a region has one to three recruits in it and walking on spends the
chance.

There is no separate decline verb, deliberately. Not recruiting *is* declining, and a button
that said so would only be a second way to do nothing.

The trait roll moved out of `add_crew` into `GameState::roll_trait`, so an offer can be drawn
without anybody joining, and `next_crew_name` reads the name the next person will carry rather
than rolling one — names come from the pack by index and never from a stream (`DECISIONS.md`
§2), so showing one costs no entropy at all.

### 6.30 The shaft question, answered: width

**§6.9 asked whether a thirty-minute run can contain a shaft. It can, and the answer is width
rather than money.**

§6.19 filed the question after measuring the golden recorder — a tower that buys a canteen, two
burners and a bunk before any transport, and reaches the Refugia four poles short of a lift.
That is one buying order. `a_tower_that_wants_a_shaft_can_have_one` asks the other: chain first,
nothing merely nice, earning every pole.

**Seven of seven rooms and a lift at tick 46,170 — 26 minutes**, inside a 31–36 minute run.

#### What the probe had to be taught, and each lesson was already written down

1. **Answer the fork.** `journey.rs`'s own header says every harness that steps the engine must,
   or it measures a parked tower. The probe did not: full charge, `walking: true`, `strode:
   false`, forty-five minutes and twelve thousand paces short of the edge.
2. **The mill before anything optional.** It is the only source of poles in the game. Buying a
   fiber comb first — same unlock, same price — spent the opening's last poles on a room whose
   output nothing could yet use: 17 bamboo, **40 fiber**, zero poles, and no way ever to afford
   the four-pole mill that would have started it earning. **That trap is live for a player.**
3. **Width, not height.** With sixty poles and five floors the tower still could not place a
   comb, because both ground floors were full once the Heartseed, a bed, a gun, the ladder, a
   mill and the shaft's reserved column were down.

#### Width is relief for space, and §6.16 asked the wrong question

§6.16 swept hull width and concluded **"width is not relief for the climb"** — which is true,
and is not what width is for. The rooms that reach the ground carry `max_floor: 1`: a comb, a
rig, a cutter arm. **They have two floors for ever, however tall the tower gets.** Width is the
only thing that gives them more room, and it is therefore the only thing that opens the rope
chain, and the rope chain is the shaft.

The chain-first tower widens to **fourteen slots and stays two floors tall**, and fits
everything. That is a shape nothing in this project had predicted, and it makes widening the
purchase that decides whether a run has a shaft in it — from a room that measured, on
throughput alone, as buying four percent.

#### What is not settled

One seed, one buying order. And a player is given no reason to think width unlocks anything:
the card says *"2 more slots, at the back"*, which is true and says nothing about the only
thing it is really for. Carried into §6.9.

### 6.31 The game as tools an agent can use

**WebMCP**: a page declares what it can do, and an agent in the browser calls those
declarations instead of clicking pixels. `web/src/mcp/provider.ts` registers Understory's verbs
through `navigator.modelContext` when it exists, and always mirrors them onto
`window.__webmcp` — the API is a proposal and is not in most browsers, so without the mirror
the tools would exist only where the spec has shipped and could never be exercised.

#### The tools are the player's verbs, not the engine's

`window.__understory` has exposed `step`, `grant` and a raw command channel since M0, and
**none of it is here.** Those are debug hooks: an agent handed `grant` does not play the game,
it edits the save, and whatever it then tells you about the design is worthless. Every tool is
something a player can do with a mouse, and the refusals are the same refusals — `Locked`,
`InsufficientStock`, `NotAtTheFront`.

**The refusal is the interesting half.** `engine::commands` rejects with a typed error that
says exactly what was wrong, and an agent that only hears "failed" cannot correct itself.
Passing it through verbatim is most of why a tool beats a click.

And one `look` rather than twenty getters, for the reason `DECISIONS.md` §3 gives about the
bridge: twenty small reads is twenty round trips and a model that has to remember which it
called.

#### Playing it found two bugs in the first call

`e2e/dogfood.spec.ts` plays a run through `__webmcp` alone — no test hooks, no grants. The very
first `look` printed:

```
Day 1, Morning · 0 paces · brownout
Charge 800/800
...
Can build: Bunk, Garden, Heartseed, Thorn Gun
```

**A freshly opened game announced a brown-out with a full bank.** `halt_reason` read "the
player wants to walk and the tower did not move" as a brown-out, and at tick 0 nothing has
moved because nothing has run. `power.brownout` was false throughout, so the two facts the
snapshot ships disagreed — and the legs and the audio both read `journey.halt` (§4.6), so two
subsystems were telling the player different things.

It reads `power.brownout` now, which makes them agree by construction. **The first attempt
guarded on `speed != Paused` and broke a fixture**, because `debugStep` runs ticks *while*
paused — the speed is not a proxy for whether time is running, and the existing test caught it.

**And the build list offered the Heartseed**, which is unique and already standing, so it could
only ever produce a rejection. The sidebar has filtered it since M0; the tool did not, because
`view.unlocked` answers "what is not locked" rather than "what is worth showing".

Neither was visible from inside the game. The halt appears as a leg animation and the Heartseed
never reaches the sidebar — it took printing the whole situation as *text*, next to itself, for
either to be obvious.

#### Playing a whole run found three more

`e2e/dogfood.spec.ts` builds a seven-room chain through the tools alone, and asserts two things:
that the plan *can* be built that way, and that the tool layer and the command layer never
disagree.

**A player sees where a room may go; an agent had nothing.** The first run walked every slot on
every floor by hand and spent about thirty refused calls per room. Refusals are cheap for the
simulation and ruinous for a model — they fill its context with `SlotOccupied`.
`understory_where_can_it_go` shares `placementFits` with the renderer, so the answer cannot
drift from the ghost highlights. **The log went from about two hundred refusals to five, and the
run from 52 seconds to 12.**

**An agent had no way to let time pass.** Nothing about the tower changes on the agent's turn;
it changes because time passed, and without a wait the loop spent its whole budget being told
the same price it could not meet — five of seven rooms, and a log that was one refusal repeated.
`understory_wait` runs the tower for a few seconds and reports what changed, which is the shape
of almost every turn a player takes.

**And the two layers did disagree.** `where` offered floor 0 slot 0, the agent built there, and
the next `where` offered it again — because `viewForTool` returned the last snapshot published
to React, which is behind any command the agent itself just sent. It reads fresh now. One extra
`view()` per tool call is nothing at agent cadence; `DECISIONS.md` §3's "one view a frame" is
about the render loop, not about a question asked once every few seconds.

#### Playing past the chain found the surface had missed a whole milestone

The chain is the easy half. Extending the dogfood through the journey — forks, waypoints,
recruits, a lift, creatures — turned up three more, and the third is the one worth remembering.

**An answered fork is not a question.** `world.fork` keeps its answer until the tower crosses the
split, because the choice stays changeable that whole time (§3.3). So a reading of *is there a
fork* is not a reading of *is anything being asked*, and `look` printed `A FORK is ahead … the
tower halts until you choose` for the entire approach. **A dogfood agent answered the same fork
398 times in 400 turns and never got on with the run.** The fork panel had this right from M3 —
it says "the way is chosen, and stays changeable until the tower crosses" — and the tool was
written from the snapshot field rather than from the panel that already interpreted it.

**A shaft's question is harder than a room's and had no tool at all.** A room needs one free
footprint on one floor, which an agent can work out from a text dump; a shaft needs the same
column free on *every* floor it spans, which no dump makes legible. `where_can_it_go` takes a
`shaft` now and reports the tallest legal span per column.

**And the tool surface had none of M6's verbs.** The milestone is *entirely* about what a player
can do while a wave is landing, and the agent could read a list of species and do nothing about
it: no focus, no charge priority, no reinforcing, no work order, no second car. Worse, `look`
printed the species and nothing else — not how far off they were, not whether anything was being
chewed — so the two things a player reads instantly off the cross-section were both invisible.
The wave now reports id, state, distance and health per creature, plus the tower's integrity, the
mending bill, the attention level and where the charge goes; and the five verbs are tools.

Nothing failed while they were missing, which is the point: **a tool surface falls behind the
game silently.** `dogfood.spec.ts` names the verbs that must exist, so the next milestone that
adds one and forgets the tool fails a spec rather than being discovered a year later.

#### And then a whole run, played to its ending, priced the rope chain

With those in place a run went the distance through the tools alone: **arrived at the far edge on
day 7, 38,969 paces, nine of eleven planned rooms, and no lift.** The ending state is the most
useful thing this project has produced in a while, because it is the first time anyone has looked
at a finished tower from a player's seat rather than through an instrument:

```
Shelves: Meals 24, Rope 89
Burner (unfuelled) · Canteen (starved) · Mill (backedup) · Thorn Gun (unarmed)
Can build: … every single entry marked "cannot pay"
```

**Eighty-nine rope, no poles, and nothing affordable.** An elevator wants two rope. The tower had
forty-four times that and could not buy one, because a lift also costs poles and there were none.

`examples/glut.rs` was written to find out why, and the answer is **competition at the source**:
the fiber comb is a second mouth on bamboo, and **the rope chain costs the tower 56% of its
poles** — 39 milled without it, 17 with it, over the same twenty minutes. `tanglenet.ron` and
`BALANCE.md`'s ropery row have both carried "rope's only consumer is a one-off" as a known
problem since M5; this is the first time it has had a number on it.

**Two harness traps were walked into getting there, and both are ones this repo had already
written down.** Widening looked like an unbounded pole sink and is not — the hull runs 10 to 16
in steps of two, so it is three purchases and both towers reach the ceiling, which produced a
confident "3 and 3, the rope chain is free" that was a reading of `max_slots`. And `RoomView.
stalled` means *waiting on an input* **or** *backed up on an output*; reading it as the second
when it was the first kept a shelf-jam theory alive for an hour. The shelves finish a third full
and nothing is stuck in a buffer — explanation two is measurably wrong, and the columns that
disprove it are kept in the instrument on purpose.

**Both stalls are real, in different towers, and they want opposite fixes.** The played tower's
mill was genuinely backed up: three crew, two floors, no lift, so poles were made and never
carried — §6.6's binding constraint, seen from the inside. The probe's mill is starved of bamboo.
`snapshot.rs` tells them apart correctly; a reader has to as well.

The other thing that run showed: a tower that builds all seven chain rooms by Day 1 afternoon
**browns out at 16/800 charge** — one burner against a mill, a comb and a ropery, all wanting the
bamboo the burner also eats. That is `charge_per_burn` and `provocation_per_burn` doing exactly
what the note at the top of `AGENTS.md` says they do.

#### And the settlements were invisible — to a player, not just to an agent

Chasing why the played run never berthed turned up the largest single thing this pass found, and
it is not about tools at all. **Nothing anywhere drew a settlement.** `journey.enclave_ahead` has
been on the snapshot since M3 and is plumbed all the way into the UI's state object at
`Game.ts`; it is read by nothing. The board panel appears only once `at_enclave` is already true.
So a settlement existed only once you were standing in it — and standing in it means having
stopped inside a window 160 paces wide that nothing on screen marked.

The tools' first attempt made the shape of the problem obvious: the agent stopped the tower at
136 paces, which is outside the 80-pace reach, and it stood there for the rest of the run berthed
at nothing with no sign anything was wrong.

`scene.ts` draws it now, with the same compression the fork uses and for the same reason — low
roofs and lit windows on the ground ahead, a lantern over them, and the name beside it. Hearth
colour rather than white, because a fork is a decision about ground and this is a place people
live. The windows brighten as the tower closes and brighten again when it berths, and **that is
the whole of the feedback**: no ring, no in-range readout, no marker. A player who stops next to
it is next to it (`DECISIONS.md` §8).

**`understory_wait` stops early now.** The berth window is 160 paces wide and the tower covers
288 in four seconds at 4×, so an agent could ask for four seconds and go from "a settlement is
300 paces ahead" to "it is behind you" without ever being offered the choice. A player watching
the screen simply sees it coming; this is the agent's version of looking up. It interrupts for a
settlement in reach, an unanswered fork closing, a waypoint alongside, somebody at the door,
something chewing the tower, a brown-out, and the run ending — and says which.

**And the view was handing over an enclave's contents without its identity.** `offers`,
`recruits` and `shell_work` all come from `nearest_enclave`, which walks *forward* to the first
settlement not yet behind the tower — so a tower past its own region's board is already being
shown the next region's. Three separate readers were naming it from `journey.region`, which is a
different question: the board panel, the tools' `look`, and the new marker. `journey.enclave_at`
carries the answer now.

#### Two facts wearing one phrase, and a green suite lying

The next pass found the dogfood reporting `0 waypoints` on every run it had ever done, and the
cause was in the tool rather than the harness. `look` marked unaffordable rooms in the build
menu with `— cannot pay` **and** marked an unaffordable waypoint `(cannot pay for it)`, so
anything scanning the document for that phrase got the wrong answer about the wrong thing. The
tower had walked past every free pole in the game — the Fallen Carrier gives five — while
starving for poles. A waypoint states its own terms now, on its own line, in words nothing else
uses: what it costs, what it gives, the ground, the attention, and whether walking on is the only
thing available. **Seven waypoints a run instead of none, and the wave verbs went from five beats
to fourteen**, because the branch that swallowed them was swallowing those too.

**And the browser had been running a WASM that predated the source for the whole of the previous
change.** `make e2e` rebuilds it; `npx playwright test` does not. `journey.enclave_at` was added
to `snapshot.rs`, the whole suite passed green, and every reader of the field silently took its
`undefined` branch — so the settlement panel reported "no trades left" for a settlement with
three, and the tools named the wrong settlement, which is the exact bug `enclave_at` was added to
fix. Rebuilt, the fix verifies: a tower past Ropewalk is told about **High Water**, the next
region's, by name.

`dogfood.spec.ts` names the view fields the tools depend on and fails with "the WASM predates
this source" rather than mystifying the next person. It is the cheap version of a contract check
and it would have caught this in one run.

#### The beats had no picture either

§6.14 exists because "between one fork and the next the tower walked through scenery and decided
nothing" — and a waypoint had **no representation in the world at all**. The card describes a
walking tower down on its side, or clean water under the roots, and none of it was anywhere: the
beat arrived as text from nowhere and left the same way. A decision you cannot see is still
scenery.

Four silhouettes now, keyed off the content id rather than the index — an index is a fact about
load order, and a pack with a fifth beat in the middle would silently repaint the other four. An
unrecognised id draws a neutral marker rather than nothing, because a pack is allowed to add
beats before the renderer knows about them. The carrier is a hull lying down with its legs
snapped out under it; the seep pool is the only cool thing in the set, because it is the only
beat that *sheds* attention; the thicket is taller than it is wide and in the way; the tangle is
brass loops low in the roots. Drawn small and never competing with the tower, because ignoring a
beat is free and is the default (§6.14) — this is not a warning a player needs, it is the route
having things in it.

**And `waypoint_ahead` was a distance with no identity**, which is exactly the shape that had
just gone wrong with the enclave. Two parallel `Option`s that must agree is the bug; it is one
`Option<WaypointAheadView>` carrying both now. Nothing read the old field, so there was nothing
to break — which is its own finding: it had been on the snapshot since M6, documented as being
there so "the renderer needs this to draw the place coming", and read by nobody.

**What the berth showed is a balance finding and it is not a small one.** A tower that reaches
Ropewalk on schedule arrives holding **`Rope 12` and nothing else**, and *every* item on the
board is out of reach: 6 poles for rope, 4 poles for darts, 4 scrap for 3 poles, 18 poles for a
person — no poles, no scrap — and Ropewalk has no shell work at all. The one trade that would
answer the pole famine costs scrap, which comes from salvaging ruins the tower did not stop at.
Walking 7,300 paces to a board you can do nothing with is worth somebody deciding about
deliberately.

### 6.9 Open questions

0. **Is the ladder legible, or merely short?** §6.11 can show the opening is *buildable* —
   the golden recorder walks it every time it runs, farm at tick 900 and a mill by 3,000. Nobody
   has shown it is *readable*: that a player who has never seen the game works out that the farm
   wants two people in it, or that the menu growing is a reward rather than a bug. That is the
   same stranger-at-the-keyboard criterion this project has carried open since M5, and it is now
   load-bearing for the first five minutes rather than only for balance.
1. **Answered by §6.29.** A settlement offers somebody in particular now, drawn on arrival and
   held until the tower walks on. What is *not* settled is whether the choice is interesting at
   one to three recruits a region — a decision you make twice a run may be too rare to learn
   from, and that wants a person rather than an instrument.
2. **Answered by §6.26, and this question had the mechanism backwards.** It read: *a tower
   that harvests from every floor with one mill jams — bamboo claims every shelf and poles have
   nowhere to land.* Measured, it is the other way round: the mill converts everything, every
   shelf ends up holding **poles**, bamboo reads zero, and the tower stops *cleanly* rather
   than jamming — a rich tower holding a hundred poles, whose escape is to spend. Kept here
   because the lesson is the method: that mechanism was asserted and never measured, in a repo
   whose own guidance says to measure your instruments first. What was genuinely missing is now
   §6.26: a quiet room could not say **which** silence it was in.
3. **Is the weapon set a loadout or a checklist?** §6.22 gave weapons an approach they answer
   and three specialists to go with the generalists. The filter is measured — a mast leaves a
   skitter alone, a ward leaves a leaper alone. What is not measured is whether *choosing*
   between them is interesting: two front slots a floor against three kinds of trouble should
   be a real squeeze, and it might instead be a checklist a tall tower simply completes. That
   needs a person and a wave, not an instrument.
4. **What actually binds a tower, if not crew?** *Answered by 6.24*: one cutter arm, because max_floor and front_only together allowed exactly one. Kept because the method is the lesson - the plateau was visible for a milestone and read as crew not mattering. §6.20's sweep went looking for the pressure
   that would make a second car necessary and found the opposite: three crew to eight buys
   **+4% hauls**, and hauls plateau near 225 however much transport is thrown at them. Every
   design argument in this project rests on transport contention being the constraint
   (`DESIGN.md` insight 1), and on this tower it is not. Either the harness plan is too small
   to show it — it holds the room count fixed while varying the crew — or the binding
   constraint moved and nothing noticed. Until this is answered, "add crew, add pressure" is a
   claim rather than a mechanism.
5. **Answered by §6.30: yes, and the answer is width.** A chain-first tower widens to fourteen
   slots, stays two floors tall, fits its whole chain, and has a lift at 26 minutes. What is
   left is that **nothing tells a player this**: the widen card reads "2 more slots, at the
   back", which is true and silent about the only thing width is really for — the rooms that
   reach the ground have two floors for ever, and width is their only relief. The original
   question follows, because the tower §6.19 measured still cannot afford one.
5b. **Can a thirty-minute run contain a shaft?** §6.19 hit thirty minutes and did not ship it:
   at that length a tower reaches the Refugia holding four poles of the ten a lift costs, and
   never gets the rest, because arriving ends its income. Four compensations were measured and
   all four were worse. The choices are a slightly longer run, a cheaper shaft, or mid-run
   income that rises without jamming the shelves — and the third is the one nothing has found
   yet. This is the largest open balance question in the project.
6. **Is ten percent a rank the right ten percent?** §6.17's practice was measured as a
   maximum against a zero: two towers, one seed, one of them starting at the ceiling, and
   the veterans get more done. That answers *does it reach the tower* and not *is this the
   number*. Nobody has played a run at 20% or at 5%, and the failure mode to watch for is
   the one the modesty is guarding against — a run won by parking one person on one job
   from the first pace, which would mean the bonus is large enough to be a build order.
7. **Where should a shaft go, and is there a decision left at all?** §6.16's width sweep
   turned `lift.rs`'s oldest hypothesis into a measurement — the elevator's value fell from
   +91% to +48% as the hull widened, entirely through crew walking further to reach it. §6.18
   then folded the dumbwaiter in, and the merged shaft holds +251% to +271% across the same
   range, so the *placement* cost is much smaller. What replaced it is a thinner question:
   there is now one built shaft that goes up, so "which shaft" is not a choice any more.
   Whether, where and how tall are what is left, and the game still teaches none of them and
   offers no way to move a shaft once built.
8. **Does a mother read as territory or as a boss fight?** §6.15 argues the first and an
   instrument cannot tell them apart — the numbers are sized against the kill-shot table and
   nobody has met one. The tell is whether a player who fells one goes looking for the next,
   because that is the jungle becoming a gallery, which `DECISIONS.md` §8 rules out.
9. **Is a beat every 1,100 paces a rhythm or a metronome?** §6.14 answers "the journey is a
   screensaver" by putting something in front of the player roughly once a minute, and the
   failure mode of that fix is the opposite complaint: a prompt often enough to become
   wallpaper. The tell is whether anybody reads the second one. Note also that the streaming
   window (900 ahead, 300 behind) is *narrower* than the interval, so beats can appear without
   being seen coming — deliberate for now, and the first thing to change if they read as
   pop-ups.
10. **Is a ten-wide floor a quietly easier floor?** §6.13 widened it to decouple the weapon
   edge from the shaft column, which is a placement fix — but every layout puzzle now has two
   more answers, and `floor_slots`' own row is explicit that its value was chosen for scarcity.
   Nobody has played a ten-wide tower against an eight-wide one. It is on the difficulty pass's
   list and it is the change on that list most likely to have made the game softer by accident.
11. **Does the push make stationing redundant?** §6.12 gives the player a verb that does most
   of what a posting does and cleans up after itself. If nobody ever uses the permanent form
   once they have the temporary one, that is not two verbs, it is one verb and a trap — and
   the tell is whether anybody posts somebody *for the run* rather than *for the minute*.
12. **What stops a tower that loses its only cutter arm?** Nothing, currently. §6.10 records the
   spiral: repair wants poles, poles want the mill, the mill wants bamboo, bamboo wants the arm.
   The sails used to fund enough slack that it never came up; `starting_stock` now buys exactly
   one mend of margin. The candidate answers are a second intake room the opening tower can
   afford, a repair path that does not cost the material the dead room makes, or accepting it as
   a loss condition and *saying so* — which is the one thing the current version does not do.
13. **Is stationing a decision or a default?** `manned_work_pct` is 150 and the price is a porter,
   but a tower with a spare person has no reason not to post them. The tell is whether anybody
   ever *un*-posts somebody, and nothing measures that.
2. **Does the charge ranking ever get touched?** It defaults to the old order and behaves
   identically, which is safe and may also be invisible. A ranking nobody reorders is a panel that
   should not exist.
3. **Is a kit a decision about a person, or a strictly-correct upgrade?** The harness and the
   mender's kit are one-of-each at the coast, so their scarcity is real; the lamp is craftable and
   may not be. If a tower ends every run with three lamps and no thought about who carries them,
   the kit is a stat block after all.
4. **`repelled` now excludes shooing deliberately**, and there is no counter for a creature seen
   out without violence. That is defensible — the tower keeps no score of it either way — and it
   also means the game's only defensive number under-reports what a well-run tower does.
5. **Still nothing is `PLAYTESTED`.** Every constant M6 added is `MEASURED`: an instrument
   confirms the effect it exists to produce. None has been played with, or played with against its
   neighbours, and `docs/PLAYTEST.md` remains the work that closes it.
