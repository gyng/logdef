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
