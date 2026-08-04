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
| 3 | Heartseed, canopy sails, salvage rig | The heart takes the room a hearth takes. Sails are the widest thing the tower carries and roof-only, which is the cost of the charge economy stated in floor space rather than in poles. A rig is a boom long enough to reach into a ruin from the deck. |
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

      **Standing at 62 of 137.** Moved this pass: `reinforce`, once the plating comparison
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
   *(Answered: neither — it is half of a balanced pair. See the end.)*

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
