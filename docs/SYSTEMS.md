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
- **One emplacement.** The dart battery is the only thing that shoots back, so "which
  defence to build" is not yet a decision — only "whether."
- **A creature giving up sounds like a creature dying.** `EnemyState::Leaving` and
  `EnemyState::Dying` are distinct in the state and in the snapshot (§2.2, above), but the
  audio layer has no separate cue yet, so walking a wave off and shooting it down sound the
  same.
- **Per-daypart elevator programs still have no UI**, carried forward unchanged from M1's
  deferred note (§1.7, above).
