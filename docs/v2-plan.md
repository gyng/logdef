# Understory — v2 Plan

> Attempt #2. Same architecture (Rust/WASM deterministic sim + React/TS), refocused gameplay:
> **SimTower × Factorio × tower defence — a solarpunk walking tower in a jungle that ate the
> old world.** Attempt #1 ("Supply Line") stays archived on `main`; v2 is built on the `v2`
> branch. This doc is the sprint contract.

**Status:** locked with owner, 2026-08-03. Build mode: big agentic sprints, one per milestone.

**Progress:** M0 ✅ · M1 ✅ · M2 ✅ · M3 ✅ · M4 ✅ · M5 mostly built. The plan below is
unchanged; this line and the notes in §9 are the only edits since it was locked.

**Two things M5 decided that this document guessed at**, recorded here because §11 asked for
them and they are answered in `SYSTEMS.md` §5 rather than by editing the plan: the destination
keeps the name *the Refugia* (§11 open #1), and unlocks arrive as a **journal** rather than as
enclave gifts or Heartseed cultivars (§11 open #2). §6.1's `meals` are made of bamboo rather
than produce, which is a deliberate departure from this document and is argued in
`SYSTEMS.md` §5.2 — M4 measured it, and the measurement is why. Two owner decisions taken during
M0 deviate from §8 and are recorded in `DECISIONS.md` §10 — a custom WebGL2 renderer from
the start rather than SVG-until-profiling-hurts, and ts7/oxfmt/oxlint for the frontend
toolchain.

---

## 0. Decisions locked (2026-08-03)

| Decision | Call |
|---|---|
| Name | **Understory** (jungle layer + storey pun). Repo stays `logdef` for now |
| Theme | Solarpunk, post-apocalyptic tropical jungle. Nausicaä-adjacent: reclamation, territorial creatures, overgrown ruins |
| Repo strategy | Build on `v2` branch; `main` frozen as the v1 archive until v2 overtakes it |
| Platform | Desktop browser first (itch.io); mouse-first UI |
| Structure | **Roguelike runs**: 2–4h, permadeath, seeded, seed-sharing |
| Meta | Unlocks widen the toolkit, never power; identical baselines (built at M5) |
| Player combat verbs | **Fully infrastructural** — priorities, 2–3 tower abilities, build/reroute under fire. No aimed weapons, no hero |
| Energy keystone | **Solar + burner mix**; terrain modulates sun; bank-or-burn day/night cycle |
| Crew | Small named crew with jobs + **meals + sleep/shifts**; personality is barks, not mechanics |
| Scale | **Cozy: 8–14 floors**, whole tower on one screen |
| Art | Readable placeholder (rectangles-with-personality) through M3; real art pass at M4 |
| vs yeettower | Patterns only, no shared crates; **logdef goes first** and debugs the spine |

---

## 1. The pitch

**Understory** is a walking garden-tower striding through the jungle that swallowed the old
world. It drinks sun through canopy sails, strips bamboo and vines as it walks, salvages
alloy from drowned ruins, and crafts its way upward — while the jungle's territorial fauna
and the old world's feral machines test its walls. It is your factory, your fortress, and
your home. And instead of belts, your logistics run on **stairs, dumbwaiters, and
elevators**. Belts never queue. Elevators do.

**The four parents:**

- **SimTower** — the cross-section; vertical transport as *shared, capacity-limited,
  queueing infrastructure*; elevator scheduling as a first-class game; floor space scarcity.
- **Factorio** — recipes, ratios, buffers, backpressure; tiers; the "one more chain" pull.
- **Tower defence** — waves as demand spikes; enemies as spatial threats to specific
  infrastructure; emplacements as the chain's top consumers.
- **Ghibli (Nausicaä more than Howl now)** — the tower *walks*; the streaming world is the
  resource input; the journey is the run; the tone is a home reclaiming a wild world, never
  a war machine.

### The three insights that fuse it

1. **Contention is the game.** In Factorio, transport is dedicated — a belt serves one lane
   forever. Here transport is *shared*: every new chain adds load to the same shafts
   everyone else uses. Your factory's true capacity is its circulation capacity.
2. **Combat is a load test.** A wave is a demand spike — darts to the batteries, repair
   crews to breaches — on the same circulation that runs your economy. You don't win fights
   with reflexes; you win them earlier, when you placed the second shaft.
3. **The world streams past.** Intake is positional: bamboo and fiber under dense canopy,
   sun and scrap in the open ruin-fields — and the two are opposed (shade = biomass-rich,
   sun-poor). Route choice is simultaneously your energy mix, your material mix, and your
   threat profile.

---

## 2. What attempt #1 taught us

Full audit in §7. Headlines:

- **The hero layer ate the game.** v1 shipped 3 classes, 11 weapons, 10 trinkets, and stat
  allocation while the supply chain ran on an auto-deliver shortcut until the final commit.
- **The prep/combat phase split killed the logistics drama.** Rerouting around a severed
  shaft mid-fight — the best moment this design can produce — was structurally impossible.
- **The elevator was never built.** Cars, queues, departure modes existed as types only,
  while weapon #11 got authored. v2 builds the elevator second (M1) and weapons never.
- **The infrastructure held.** Deterministic command-sim spine, RON pipeline, snapshot
  bridge, 56 tests, e2e smoke harness — all survived a one-day sprint and carries forward.
- **11,800 lines of docs written before play.** v2 docs stay just-ahead-of-implementation:
  each sprint writes only its own milestone's spec.

---

## 3. The four structural calls

1. **No hero.** Emplacement tower defence: dart batteries and seed-bomb mortars, crewed and
   ammo-fed by the chain. Player combat verbs: targeting priorities, 2–3 tower-level
   cooldown abilities (lurch, vent), and *logistics triage under fire*. Skill expression is
   infrastructural, not ballistic.
2. **Continuous sim.** Pause-and-plan + 1×/2×/4×, RimWorld/Factorio-style. Building during
   a fight is allowed but slow and exposed. No prep/combat phase gate — this is what makes
   "combat is a load test" true.
3. **Continuous world.** No node map. Terrain bands stream past within regions; occasional
   route forks (canopy passage vs ruin-field); enclaves as waystations (trade, recruit,
   rest). Map screen is a route overview, not a mode.
4. **Crew, not companions.** Named crew (start 3, cap ~8) who *are* the porters, gunners,
   and repair hands. Jobs + two needs (meals, sleep) + shift rota. Barks and personality as
   tone; no relationship mechanics.

## 4. Design pillars

1. **The tower is the factory, the fortress, and the home.** One entity, one screen.
2. **Vertical transport is the belt — shared, queued, scheduled, and powered.**
3. **Combat is a load test.** You fight with what your chain delivers, where your shafts
   can deliver it.
4. **The world streams past.** Terrain sets the input rates; the route is the strategy.
5. **Reclaim, don't conquer.** Solarpunk warmth; defence is thorns and seeds, not gunmetal;
   creatures defend their territory and you are the intruder passing through; breakage and
   silence are the UI. (v1's tone guardrails carry forward, re-skinned.)

## 5. Core loop (minute to minute)

The tower strides through a sun-dappled clearing; canopy sails drink; the cutter arms strip
bamboo and the mill hums. You place a thornwright on floor 4, add a dumbwaiter to feed it,
reprogram the elevator to skip floor 2. Dusk. Leapers drop from the overstory onto the
upper decks — batteries open up, dart racks drain, a shaft takes a hit, the cells brown out
and the elevator stalls mid-climb. Pause. Light the burner (smoke will draw more attention)
or hold and route repairs up the stairs? Dawn breaks; the wave scatters; the ruin-field —
and its alloy — glints on the horizon. Stop to salvage, or push for the enclave?

---

## 6. Systems

### 6.1 Resources — small, legible, contested

Data-driven string IDs (interned to dense indices at runtime — never a Rust enum).
**Legibility rules: max 2 inputs per recipe, chain depth ≤ 3.** If a chain can't be read in
the cross-section at a glance, it's too deep.

- **Raw (4):** `bamboo`, `fiber`, `produce`, `scrap`
- **T1:** `poles` (bamboo→mill), `rope` (fiber→ropery), `meals` (produce→kitchen),
  `alloy` (scrap→sun-forge, charge-hungry), `darts` (poles→thornwright) — the ammo staple
- **T2:** `mechanisms` (alloy+poles→fitter), `seed bombs` (produce+fiber→bombary),
  `charge cells` (alloy→cellwright) — batteries are *built*, storage is infrastructure

**Bamboo is the contested material** (v1's wood tension, reborn): it becomes poles
(construction, repair, darts) *or* fuel for the burner. Burn your building material or
build with it.

### 6.2 The keystone: charge

**Charge** is a stored flux (in cell banks), not a crate.

- **Sources:** canopy sails (sun × terrain exposure × daypart — nothing at night, weak
  under dense canopy, strong in clearings/ruin-fields) and the **burner** (eats bamboo;
  smoke raises provocation — the dirty fallback announces you).
- **Sinks:** **striding** (charge per stride — walking speed is a player-set throttle),
  **powered production** (sun-forge, T2 machines), **powered transport** (elevators and
  dumbwaiters draw charge per trip — stairs, ladders, chutes are free), night lighting for
  the night shift.
- **The tension:** bank charge for the night (defence, lights, elevators under attack) or
  spend it walking farther by day. A brown-out during a night assault — elevator stalled,
  forge dark — is the signature emergency alongside the severed shaft.

Route choice closes the loop: shaded jungle = biomass-rich/sun-poor, open ruins =
sun-rich/scrap-rich/exposed. **Your route is your power mix.**

No tick currency. Trade tokens exist only at enclaves. Loss = the **Heartseed** (the
tower's living core, pre-placed) destroyed.

### 6.3 Rooms

Slot-based floors, **multiple rooms per floor** (v1's one-building-per-floor cap is fixed
in the data model from day one). Cozy scale: 8–14 floors, one screen.

- **Intake:** cutter arms (bamboo/fiber/produce from passing terrain; low floors only),
  salvage rig (scrap; works when berthed at a ruin)
- **Production:** mill, ropery, sun-forge, thornwright, kitchen, fitter, bombary, cellwright
- **Energy:** canopy sails (top floor/roof slots — **growing taller displaces your power
  deck**, a real cost of height), burner, cell banks
- **Logistics:** storeroom, floor cache
- **Defence:** dart battery (balcony), seed-bomb mortar (deck), repair workshop
- **Crew:** bunks, canteen
- **Heart:** the Heartseed (unique, pre-placed; loss condition)

### 6.4 Transport — the centerpiece

- **Stairs** — free, always present, slow, crew-only (carry 1). The baseline everyone queues on.
- **Ladder** — cheap, 2 floors, crew-only.
- **Dumbwaiter** — item-only, autonomous, 2–3 floors, small batches, sips charge. The "inserter."
- **Elevator** — THE machine. Real car sim: shaft spans chosen floors and costs a slot
  column on each; car capacity; stop queue; boarding time; **draws charge per trip**;
  **player-programmable policy** (served floors / express / freight-vs-crew priority,
  switchable per daypart). **Don't invent the dispatch algorithm**: port the trace-verified
  SimTower model from `phulin/tower-together`'s specs (bidirectional sweep, dispatch
  threshold, dwell timing, fixed floor queues, daypart schedule tables — see §12), then
  extend for freight (crates board like passengers with different dwell/capacity weights).
- **Chute** — down-only, fast, item-only, free (gravity works even after the apocalypse).
- Post-v2.0: pneumatic tube, exterior hoist (weather/attack-exposed).

**Visible queue stress** (SimTower's best feedback device): crew and crates waiting too
long tint toward red in the cross-section. The bottleneck diagnoses itself — no dashboard.

### 6.5 Combat

Enemies approach on the terrain layer and damage **infrastructure** — panels, rooms,
shafts. Each type teaches one lesson (v1 principle, kept):

- **skitters** (swarm baseline) → ammo drain economics
- **canopy leapers** → drop onto *upper* decks from overhanging trees — top-floor exposure,
  and a jungle-native inversion of v1's climb-from-below
- **root-borers** → gnaw legs and shafts → transport redundancy
- **spitters** → ranged, bombard from cover → priority targeting
- **feral wardens** → armored old-world machines that wake when you salvage ruins →
  provocation has a face
- **night predators** → nocturnal pressure; the reason you bank charge

Emplacements auto-fire by player-set priority, consuming ammo from local racks. Repair
consumes poles + rope + crew time — defence is itself a chain sink. **Provocation** is one
knob: aggressive harvesting, burner smoke, and ruin-salvaging all raise local threat.
Tone-safe by construction: they defend their home; you are the one passing through.

### 6.6 Journey & run structure

**Roguelike runs: 2–4h, permadeath, seeded, shareable seeds.** Three regions to **the
Refugia** (the rumored enclave-haven; name provisional): deep jungle → the drowned city
(ruin belt) → the coast approach. Region = terrain palette + resource/sun mix + threat
table. Enclaves between regions (trade, recruit, repair); forks within them.

**Meta:** unlocks widen the toolkit — new rooms, transports, route options — never
baselines. A first-run tower and a fifty-run tower start identical; the veteran has more
tools, not bigger numbers. Built at M5; delivery design (enclave gifts? Heartseed
cultivars?) decided then.

### 6.7 Crew

Jobs: haul / operate / gun / repair, as role priorities per crew member (RimWorld-lite,
one screen). Two needs: **meals** (kitchen chain — hungry crew slow down) and **sleep**
(bunks + a **shift rota** on the daypart clock: day shift / night watch). Night operations
need light (a charge sink). Crew are the tower's pulse: their commutes load the elevators,
their shifts shape demand, their stress-red queues are your bottleneck alarm. Personality
lives in barks and portraits, not mechanics.

---

## 7. Salvage manifest (from the three-way audit of attempt #1)

### KEEP (carry live)
| Asset | Why |
|---|---|
| Command/engine/tick/RNG determinism spine | Textbook; exactly right for a management sim |
| RON registry + `DataSource` + validation pipeline | Idiomatic, extensible; deepen validation, make items data-driven |
| Snapshot DTO pattern + typed accessors | Right decoupling; drop combat snapshots |
| **TowerViz** (SVG cross-section, slots, transports, runner interpolation, place-mode UX) | Strongest asset in the codebase; the seed of the main view |
| `tokens.css` + i18n `t()` setup | Strings externalized; retint palette for jungle/solarpunk |
| Tooling: Makefile, clippy HashMap ban, fmt/lint configs, Playwright smoke harness, `debugging-bridge.md` | The hard-won 80% |
| Logistics tests (~20: chain delivery, stalls, capacity, slot collisions) + determinism trio | Not coupled to combat; they survive |
| Tone guardrails from `art-direction.md`; diegetic-UI philosophy | Best design writing in v1; re-skin Ghibli-European → solarpunk-tropical |

### KEEP-AS-PATTERN (rewrite, keep the shape)
| Asset | The rewrite |
|---|---|
| Transport/runner system | Slot floors + L-paths + demand priority stay; add elevator *cars* (tower-together model), charge draw, multi-room floors, batch carrying, distance-aware routing |
| Production system | Progress/stall/buffer model stays; wire `amount_per_craft` (dead field in v1), multi-output, data-driven item IDs |
| Bridge (wasm-bindgen, JSON accessors) | Shape stays; single shared TS frame driver; wire the unused `interpolation_alpha`; SoA path later (§8) |
| Combat-canvas frame (enemies advance on terrain toward tower) | Reuse the frame; delete the aim/draw input layer |
| Typed-effect enum pattern (v1 modifiers) | Mimic for room upgrades |
| Companion targeting AI | Reheat as emplacement targeting |

### DISCARD
Hero, weapons, classes, stats, trinkets, modifiers-as-content, node-map/chapter graph,
mystery events, prep-tick economy, relationship system, CombatPage input layer, empty
atomic-design scaffolding, monolithic `global.css`, ~half the v1 doc corpus.

### Known v1 debts — fix in the new model, don't port
- `Floor.building: Option<Building>` → `Vec<Room>` (multi-room floors)
- `ResourceType` enum → interned string IDs
- `engine.rs` 1.9k-line monolith → command-handler + snapshot-builder modules
- `state.rs` 800-line mixed module → split by domain
- Porters carry 1 despite `carry_capacity`; `best_transport_for` ignores horizontal distance

---

## 8. Technical plan

**Branch strategy:** create `v2` from `main`; `main` stays the frozen v1 archive until v2
overtakes it. First commits on `v2`: strip the ARPG layer with the surviving tests green,
then rebuild the middle inside the kept chassis.

**Determinism, upgraded (from the yeettower plan, §12):**
- **Integer-only sim math from day one** — i64 quantities, slot/floor coords + Q8.8
  sub-tile fixed point for movement. No f32 in state; v1's "swap `Scalar` later" escape
  hatch is closed at M0 while the codebase is smallest.
- **Replay as THE format:** `{seed, content-pack hashes} + per-tick command log + periodic
  XXH3 state hashes`. One format serves saves (snapshot + log tail), regression fixtures,
  seed-sharing, and any future netcode. Golden-replay fixtures in CI from M1, run **native
  AND wasm with hash diffing**.
- **Named RNG streams with a cosmetic firewall** — barks and visual jitter can never
  perturb the economy.
- Commands tick-stamped; fixed system order; HashMap ban unchanged; flaky determinism = P0.

**Bridge:** start with v1's proven JSON accessors (debuggable, fine at cozy scale), but
shape snapshots so the end-state slots in without redesign: sim in a Web Worker, flat SoA
buffers (grid cells + packed actor arrays ×2 prev/curr for interpolation) read zero-copy,
events batched per frame, transferable-ArrayBuffer snapshots, no SharedArrayBuffer.
Content: string IDs in data → interned dense u16 at runtime, mapping recorded in the save
header (saves survive content changes).

**Rendering:** one unified game view (no page-per-phase). TowerViz SVG center-stage;
terrain + enemies as a layered parallax strip beneath/behind. Desktop mouse-first. Port to
canvas/WebGL only if cozy-scale entity counts ever hurt — yeettower's renderer architecture
(single-batcher WebGL2, static chunks rebuilt on change, dynamic actor layer, DOM text
overlay) is the ready-made blueprint if so.

**Docs:** new small set on `v2` — `DESIGN.md` (this doc's §1–6 distilled), `SYSTEMS.md`
(grown one milestone at a time, spec-first per sprint), `DECISIONS.md` (determinism rules,
bridge patterns, tone guardrails carried from v1), `BALANCE.md` (all constants, each graded
`DESIGNED` / `PLAYTESTED` — provenance-graded, never hardcoded). `docs/foundation/` stays
on `main` as the archive.

---

## 9. Milestones as sprint briefs

Build mode is **big agentic sprints** — so each milestone is specified as a sprint brief:
scope, non-goals, exit criteria. **Sprint protocol:** (1) sprint starts by writing its
`SYSTEMS.md` section — spec before code, but only for this milestone; (2) sprint ends with
`make check` + smoke + replay fixtures green **and the milestone's design question answered
by actually playing**; (3) anything cut mid-sprint is written down, not silently dropped.

### M0 — The Stride *(chassis)* — ✅ shipped

*Answered: yes. The stride, the parallax, and crew who walk with purpose carry it; the
tower reads as alive on one screen. Cut from scope: nothing. Added beyond scope: a custom
WebGL2 renderer instead of SVG (owner call, `DECISIONS.md` §10).*
**Scope:** `v2` branch; strip ARPG (surviving logistics/determinism tests green); rename
scaffolding to Understory; integer/Q8.8 math swap; replay-format skeleton; one unified
game view: tower strides over a streaming terrain strip, pause/1×/2×/4×, one crew member
hauls a crate up the stairs.
**Non-goals:** no energy, no combat, no recipes beyond one placeholder.
**Exit:** smoke test green end-to-end; a replay file records and replays a session
bit-identically. *Question: does the walking tower feel alive on one screen?*

### M1 — The Chain *(the whole bet)* — ✅ shipped

*Answered: yes, but it took a balance change to get there. Measured over 600 s with and
without an added shaft (`--example throughput`): at two crew the difference was inside the
noise and the question was unanswerable; at three crew it is +90% crafts and 56% fewer
ticks queueing. `starting_crew` is now the first `PLAYTESTED` constant. Cut from scope:
nothing. Deferred: per-daypart elevator programs exist in the data model and the command
layer but have no UI yet — a player can only change them through a replay or a command.*
**Scope:** bamboo→poles→darts across 3+ floors; multi-room floors; dumbwaiter; **elevator
car sim** (tower-together dispatch, freight extension, charge draw, programmable stops);
charge system (sails × terrain sun, burner, cell banks, stride cost); dayparts (sun curve);
stress-red queues; starvation/stall/brown-out all readable (and audible — silence = broken).
Golden-replay fixtures + native/wasm hash-parity CI start here.
**Non-goals:** no enemies, no crew needs, no regions.
**Exit:** a deliberately under-built tower visibly bottlenecks at the shaft; fixing it
visibly fixes throughput. *Question: is elevator contention actually fun? Playtest hard —
everything downstream assumes yes.*

### M2 — The Siege *(the load test)*
**Scope:** skitters + leapers + root-borers on the terrain layer; dart battery + seed-bomb
mortar with priority targeting; infrastructure damage (panels/rooms/shafts); repair chain
(poles+rope+crew); night predators + bank-or-burn (threat follows dayparts); provocation
knob v1; build-under-fire (slow, exposed); loss via Heartseed.
**Non-goals:** no wardens/ruins, no enclaves, no meta.
**Exit:** a severed shaft mid-assault forces a live reroute and it's *legible*; a brown-out
night assault is survivable with banked charge and lethal without.
*Question: does combat-as-logistics-stress produce drama without any aimed weapon?*

### M3 — The Journey *(the run)*
**Scope:** regions 1–2 (deep jungle, drowned city); terrain bands with opposed sun/biomass
rates; ruin berthing + salvage rig + feral wardens; route forks; one enclave (trade,
recruit); walk/stop throttle economics; full run loop: seeded start → death or region 2.
**Non-goals:** no region 3, no unlocks, no art.
**Exit:** two runs on the same seed are identical; two seeds feel meaningfully different;
a 45–60 min session reaches the drowned city.
*Question: run pacing — and does the route-is-your-power-mix tension actually bite?*

### M4 — The Home *(the tone)*
**Scope:** crew names/portraits/barks; meals + sleep + shift rota (night watch);
canteen/bunks; the art pass (flat-vector solarpunk-tropical on TowerViz + terrain: verdigris
palette, overgrowth, warm interiors); two soundscapes (day canopy-hum / night siege-tension)
with diegetic audio (production loops go silent when starved).
**Exit:** the eyes-closed test (can you hear how the tower is doing?) and the
screenshot test (does one frame say "solarpunk home, not war machine"?).
*Question: does it feel like a home reclaiming the world, or a spreadsheet with legs?*

### M5 — The Refugia *(depth within rules)*
**Scope:** T2 chains (mechanisms, seed bombs, cells as built storage); chutes; full enemy
taxonomy + region 3 + the Refugia arrival; enclave economy; toolkit-widening unlocks +
delivery design; difficulty tuning + balance telemetry; itch.io release cut.
**Content gate (absolute):** nothing ships unless an existing system consumes it at runtime.
**Exit:** a full 2–4h run to the Refugia; a shared seed reproduces it; balance constants
all graded `PLAYTESTED`.

---

## 10. Process rules (the anti-v1-failure-mode list)

1. **No content type until a system consumes it at runtime.** (v1: 16 modifier effects
   authored, zero applied.)
2. **The elevator before the eleventh anything.** Core-mechanic depth beats content
   breadth. M1 is sacred.
3. **Every sprint ends playable** — checks green, replay fixtures green, design question
   answered by playing, cuts written down.
4. **Determinism discipline is CI, not culture:** replay hash-parity native+wasm on every
   push from M1.
5. **Docs follow play.** Spec the next milestone only; this plan is the only whole-game doc.
6. **Constants live in `BALANCE.md`-backed data with provenance grades** — be wrong fast,
   and know which numbers have earned trust.

---

## 11. Remaining opens (all deferred on purpose)

1. **"The Refugia"** destination name — placeholder; decide during M3 world-writing.
2. **Unlock delivery** (enclave gifts vs Heartseed cultivars vs journal) — design at M5.
3. **SoA bridge + canvas/WebGL port** — profile-gated, blueprint ready (§8), likely never
   needed at cozy scale.
4. **Repo rename** (`logdef` → `understory`) — cosmetic; whenever convenient.

---

## 12. Cross-pollination with YeetTower (`~/p/yeettower`, WSL)

Sibling project: open-source Yoot Tower / The Tower II reimplementation — same stack
(deterministic Rust sim in WASM + React/TS) applied to the genre Understory descends from.
**Locked: patterns only, no shared crates; Understory goes first** — its M0–M1 debugs the
spine (integer math, replay harness, elevator sim) that yeettower's P2–P3 will need, in
exchange for yeettower's research already de-risking Understory's M1.

**Adopted outright (folded into §6/§8):** integer-only sim math; replay-as-the-format with
XXH3 hashes + native/wasm parity CI; cosmetic-firewalled RNG streams; content packs
(string IDs → interned indices, pack hashes in replay headers); **the elevator dispatch
model** via `phulin/tower-together`'s trace-verified SimTower specs (bidirectional sweep,
dispatch threshold, dwell, fixed floor queues, daypart schedule tables); Worker +
zero-copy SoA bridge as the designed end-state; provenance-graded constants.

**Adopted as design:** visible queue stress (SimTower's stress-red waiting people — pure
diegetic UI); dayparts (Yoot's 7-daypart clock → Understory's sun curve, shift rota, and
per-daypart elevator programs); cohort-staggered entity updates as the perf pattern if
counts ever climb.

**Sequencing rule:** don't build both spines simultaneously. Understory M0–M1 first; port
proven pieces into yeettower afterward (or lessons back, if yeettower's fidelity work finds
better constants).
