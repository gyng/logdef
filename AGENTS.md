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
(`DESIGNED`/`MEASURED`/`PLAYTESTED`) — check there before hardcoding a number a designer might
want to tune, and add a graded row for any new constant (`DECISIONS.md` §7 explains why a test
enforces this).

**`MEASURED` is not `PLAYTESTED` and the difference is load-bearing.** All 140 rows are
`MEASURED`: an instrument confirms the effect the constant exists to produce, and the limit of
that measurement is written into the row. None is `PLAYTESTED`, which this file defines as
somebody having played with it *and with neighbouring values*. Do not promote a row without
doing that — closing a criterion by redefining a word is the failure the grade exists to
prevent, and `SYSTEMS.md` §5.11 open question 4 predicted it by name.

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

M0 ("The Stride"), M1 ("The Chain"), M2 ("The Siege"), M3 ("The Journey") and M4 ("The
Home") are shipped. That is: the deterministic chassis and streaming terrain; the day clock,
charge, elevators and dumbwaiters, and the crafting chain; creatures, infrastructure damage,
emplacements and repair; regions, route forks, berthing at ruins, an enclave, and a run that
ends two ways; and now named crew with two needs — meals from a canteen chain and sleep in a
bunk — plus the art pass and the whole audio subsystem. A run
can be played from the first pace to the last, and the tower is somewhere people live.

M5 ("The Refugia") is most of the way there: the tier-two chains and the materials under
them, the chute, region 3 and the coast, the two creatures that complete the taxonomy, three
enclaves, the journal that carries unlocks between runs, and the run log. A run can be played
from the first pace to an arrival at the Refugia.

M6 ("The Watch") is shipped: the verbs a player has while a wave is landing, plus the sails
being cut out of the game entirely (§6.10) and the opening rebuilt around a build ladder
(§6.11).

**The starting tower is two floors, three crew, a Heartseed, a bed and one thorn gun.** Everything else —
the cutter arm, the mill, the burner, the storeroom — is something the player builds, gated
behind `RoomDef.unlocked_by`: farm, then cutter arm, then burner, and then the whole menu.
That gate is validated in `engine::commands`, so **a harness that places a canteen on turn
one now gets `CommandError::Locked` rather than a tower.** Use `tests::engine` (walks the
ladder, returns a working tower), `tests::opening` (the shipped one), `harness::chain_tower`
in `examples/`, or `debug_grant` in the browser specs.

**Weapons go on the leading edge** (§6.13). `front_only` is validated in
`engine::commands`, the front is `floor_slots - width`, and the floor is ten slots wide — so
columns 8-9 are the weapons' and 7 is still the shaft's. A harness that places a battery
mid-floor gets `NotAtTheFront`; one that puts a shaft on column 8 collides with a cutter arm.
`tests::disarm` strips the gun *and* the arm from a fixture that needs an **undefended** tower,
because the arm deals melee damage now and a tower with one is not undefended — four tests had
quietly started measuring the gun instead of their own subject.

Charge priority
handed over — it used to *be* the tick order — a creature the emplacements can be told to
prefer, a person posted to a room, kit that belongs to somebody named, the berth given its own
halt, and a thief answered by somebody standing in the room. **An instrument's prose goes stale where its table cannot.** Three were caught in one
session, all the same way: `needs.rs` printed "a third of a person's life spent hungry" above
a table reading 14.6%, `haulcycle.rs` printed "queueing is 3.4%" above one reading 5.6%, and
`chain.rs` quoted *another instrument's* idle share as 0.2% when it was 3.8%. The computed
numbers were right every time. The sentences underneath were literals from an older run — and
the sentence is what gets quoted, so ten `BALANCE.md` rows agreed with each other by all
copying the same wrong words. `haulcycle.rs` interpolates now; `chain.rs` names the other
instrument instead of quoting it. **If you write a figure into prose that your own code also
computes, interpolate it or delete it.**

**The shift rota was cut** (`SYSTEMS.md` §6.32). Crew go to bed when `rested` reaches
`tired_ticks` and get up when it is full; there is no `Shift`, no `SetShift`, and no roster
toggle. Every split of the old rota cost the tower 28-44% of its poles and the *mixed* splits
were the worst option on the board, which made it a menu whose every non-default option was a
trap. What replaced it covers the tower's nights (night work 1 -> 29 poles) and switched on
thirteen rest traits that had only ever fed a hidden work-rate multiplier. **It also moved
shaft affordability from 26 minutes to 32** in a 31-36 minute run — recorded, not tuned away,
and §6.19's problem rather than a new one.

**Read `SYSTEMS.md` §6 before
adding anything to a wave**, because the tone gate is what shaped every one of them: weapon
loadouts and crew fighting boarders were cut rather than softened, and what survived is the
shape of *attention* rather than the shape of a fight.

A run is now **31–36 minutes at 1×** (`SYSTEMS.md` §6.19), down from 37–44 — the journey layer
scaled again and nothing else moved. **Thirty was the target and thirty does not work yet**:
0.73 delivers 28–32 minutes and a tower that reaches the Refugia four poles short of the ten a
lift costs, permanently, because arriving ends its income. Four compensations were measured
and all four were worse; §6.19 lists them. That gap is now the largest open balance question
in the project. §6.6 records the three things tried instead and why each made it worse; the
short version is that crew hauling *was* the binding constraint, so speeding production only
filled shelves the crew cannot clear.

**Where the constraint sits moved, but less than the previous version of this paragraph
claimed.** Cutting the rota (§6.32) gave the tower ~64% crew-awake against 58%, and `chain.rs`
now measures the mill **6.7% backed up against 86.8% starved** where it used to be 43.1%
against 45.2% — near enough half and half. So the tower flipped from *nobody empties the mill*
to *nothing arrives at it*, which want opposite fixes (§II rule 5).

**This paragraph then said "the constraint sits on intake, not hands", and that overstated
it** (§6.34). Twelve seeds say more crew still buy +25% more hauls. Both are true and
consistent: hauling is how bamboo reaches the mill, so a starved mill is partly a hauling
result. What is safe to say is that the compensations §6.6 rejected were rejected against a
tower that no longer exists, and that anything reaching for "add hands" should re-measure
rather than assume either the old +63% or the newer overcorrection.

**And the lift is worth about a quarter of what §6.18 records** — +11% at five floors, +54% at
eight, against +119% and +256%. Both sides of that comparison moved and in opposite directions.
It bears directly on §6.19: the question may not be whether a tower can afford a lift, but
whether one is worth affording at the heights a run reaches.

Not built: the difficulty pass. **Every balance row is `MEASURED`** — checked against an
instrument, section by section — and **none is `PLAYTESTED`**, because nobody has played with a
value and its neighbours. That gap is the difficulty pass, and it is a person's work rather than
an agent's. The itch.io release cut works and has not been shown to a stranger. Role priorities
are cut for good rather than deferred (`SYSTEMS.md` §5.10).

**A tower could own exactly one cutter arm until the last change of M6** (`SYSTEMS.md` §6.24).
`max_floor: 1` plus `front_only` allowed one arm per floor's leading edge across two floors,
and the opening gun already held floor 0's. Every economy number measured before that uncap was
taken against that ceiling — hauls plateaued near 225 whatever crew or cars a tower was given,
which read for a whole milestone as "crew do not matter". **Anything in `BALANCE.md` measured
through `lift.rs` or `chain.rs` predates this**, and §6.16's width sweep is explicitly
invalidated by it.

**This line used to say "three to eight crew is now +63% at three cars" and that number came
from an instrument quoting itself wrong.** `lift.rs` printed a hardcoded "+4%" under a table
its own run had moved to +21%, and this file had copied a third figure from an older run
still. **Resolved at twelve seeds (§6.34): 314, 328, 392 hauls at three, five and eight crew —
monotone, and +25%.** Crew still matter; the number was simply too big. `lift.rs` computes that
sentence off its own grid now and prints UNRESOLVED when the curve is not monotone.

**Nine of the seventeen instruments run on a single seed**, including `chain.rs`,
`haulcycle.rs` and `needs.rs` — the three that feed `BALANCE.md`'s Crew rows and
`PLAYTEST.md` criterion 3. `needs.rs` sweeps eight now and prints ranges; it turned out
stable, but the figures published from it were the *worst* seed of eight rather than the
figure, and one run could not have told anybody that. **Two instruments disagree about the elevator and neither is wrong.** `throughput.rs` puts
a four-floor tower at **+88%** hauls with a shaft; `lift.rs` puts a five-floor one at
**+11%**. Both are eight-seed means with non-overlapping ranges, so neither is noise —
they are measuring different towers (one buys a canteen and a bunk over a fixed window,
the other holds a room plan fixed and grows the hull). Which difference accounts for it is
open, and both headers say so. **Quote the tower, not the verb**, and do not put "what an
elevator is worth" in a balance row until somebody has closed it.

**And a stable instrument is worth as much as a swept one, once you know which it is.**
`chain.rs` gives 6.7%/86.8% to the digit on five seeds because a three-day window covers
34,000 paces and the terrain mix converges — so its figure needs no range. `haulcycle.rs`
does not: its idle share runs **0.0% to 9.1%**, meaning saturation is a property of the
*run* rather than of the tower, and one seed cannot tell you which end you got. A
stationary tower is also seed-stable, so check the ground covered before reading
stability as convergence.

**Check an instrument's seed count
before quoting it**, and widen it before you quote it twice.

**Three seeds was never a cost decision.** The whole sweep runs in forty seconds at twelve and
nine at three, and nobody had checked. Two claims went into this file on three seeds and both
were wrong, in opposite directions.

**Read `SYSTEMS.md` §6.10 before touching terrain, yields, charge or a chain — and §5.11
open question 0 after it, for the method.** Question 0 was the largest open finding in the
project: `yield_pct` and `sun_pct` were **one constant in two columns**, deliberately opposed,
cancelling to within two percent on every tower shape measured. That analysis is correct and
the system it analysed is **gone** — M6 cut the canopy sails, so `sun_pct` no longer pays a
tower any charge at all. It now sets the garden's rate, decides when the lamps come on, and
lights the scene. Read question 0 for how to measure your own instruments before believing
them; it is still the best short lesson in this repo.

**Charge now has exactly two sources and they are not symmetrical.** Burners, which a tower
builds and which eat the same bamboo the mill wants; and the Heartseed's trickle, which is a
*floor* rather than an income. The trickle exists because cutting the sails created a deadlock
with no way out — every source of charge required already having charge, and a tower that ran
dry parked for 160,000 measured ticks with no path back.
`tests/power.rs::a_tower_that_runs_completely_dry_can_still_crawl_out` is that property, and
**any change that makes an income depend on an output of that income needs it to still pass.**

**`charge_per_burn` and `provocation_per_burn` are the new one-constant-in-two-columns.**
Efficiency and smoke: raise one alone and you silently retune the siege. `provocation_per_burn`
is what decided whether the M6 opening tower lived at all.

Four confident wrong answers were filed against that question before the right one — chains
terminating in buffers, crew scarcity, a shelf jam, a 9.7% route gap — and every single one
was a property of the harness rather than of the game. The section records them all, and it
is the best short lesson in this repo about measuring your own instruments first.

**`docs/SYSTEMS.md` is the exact, current boundary of what exists.** Read the milestone
section for whatever you are about to touch, and its "Deferred out of" list, before
assuming a system is live.

Three things worth knowing before you touch balance or an instrument, all of them M4's:

1. **A measurement window is a whole number of days, or it is a measurement of what time it
   started.** Crew sleep through the night band now, so a tower does not queue, haul or craft
   at the same rate around the clock. `SYSTEMS.md` §4.8 has the case that taught this — an
   elevator that measured as worthless because two thirds of the window was night.
2. **A harness tower needs a standing reason to want poles.** A tower that has worked through
   a finite shopping list stops consuming, every buffer fills, and harvest stops dead at the
   tower's total buffer capacity — which is a property of the tower and identical whatever the
   ground underfoot was. Both `siege_run.rs` and `journey.rs` have been wrong this way.
3. **`BALANCE.md`'s Journey section has been re-derived and says which rows were not.**
   Region 1 is 12,754–16,303 paces and 12–15 minutes at 1×, `halted 0`; a walking tower earns
   115–142 bamboo across it; berthing beats never stopping on 7 seeds of 12 recklessly and 3
   carefully, spread −97.5% to +186.1%. The two scrap rows are marked *not re-derived*, because
   their figures predate the harness repair and cannot be compared across it. Read the block at
   the top of the section before trusting a number in it.
4. **`BALANCE.md`'s Siege section has been re-measured and says so.** Read the block at the top
   of it before trusting a `PLAYTESTED` grade there. The plating comparison produced the same
   false finding **four** times before the cause was found, and it is worth knowing what the
   cause was, because the shape recurs: **never compare two towers on a quantity whose maximum
   is the thing you are testing.** `reinforce` raises `panel_hp`, so any column counting panels
   makes the plated tower look worse for having more to lose — as a fraction *or* as an
   absolute. It counts rooms and shafts now, whose maxima are identical either way.

   The second half is worth as much: it had been running at a provocation level where **nothing
   ever reached a room on either tower**. Eight seeds, zero damage, both shapes, and a
   confident conclusion drawn from it. Before believing that something does nothing, check that
   the run gave it something to do.

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
        world.rs            # World, TerrainBand, Feature, the journey roll, forks
        tower.rs            # Tower, Floor, Room, Shaft, Stack, Shelf
        crew.rs             # Crew, CrewState, HaulTask — the porter state machine
        clock.rs            # the day cycle and its dayparts
        power.rs            # charge: the bank, and who is allowed to spend it
        siege.rs            # Enemy, DamageTarget, Health, provocation
      command.rs             # GameCommand, CommandResult, CommandError
      engine.rs               # GameEngine: frame/step/send/view, thin by design
      engine/
        commands.rs           # command validation + application — §4
      systems.rs               # tick() — the fixed system order
      systems/
        stride.rs              # the legs, terrain streaming, region and fork crossings
        intake.rs               # harvesting ground covered, and extracting from ruins
        production.rs           # crafting rooms
        transport.rs            # elevator cars, dumbwaiters, queues
        power.rs                # sun, sails, burners, and the charge priority order
        siege.rs                # waves, approach, damage, wardens
        defence.rs              # emplacements, fed off the same shelves as everything
        repair.rs               # putting the tower back together, for poles and crew time
        haul.rs                  # crew state machine + task assignment/scoring
        needs.rs                 # hunger, rest, who is tired enough to sleep, work multiplier
      snapshot.rs                # presentation boundary — the only place Fx::to_f32 runs
      replay.rs                  # Replay, Recorder, hash_state, embedded golden fixture
      tests.rs, tests/            # tests grouped by topic (determinism, haul, journey, ...)
    examples/
      record_golden.rs            # regenerate assets/replays/golden.json
      throughput.rs               # M1's instrument: does an elevator earn its slot?
      siege_run.rs                # M2's: three towers, five days, one seed
      journey.rs                  # M3's: twelve seeds, three policies, and a whole run
      lift.rs                     # M6's: does a shaft ever pay, swept over tower height.
                                  #   Reads its pack from UNDERSTORY_PACK when set, so a
                                  #   tuning pass costs seconds rather than a rebuild
      rest.rs                     # did cutting the shift rota pay? Every split of it cost the
                                  #   tower 28-44% of its poles and the mixed rotas were the
                                  #   worst option on the board (SYSTEMS.md 6.32), so the rota
                                  #   went and sleep became need-driven. Now the after: night
                                  #   work 1 -> 29 poles, run length unchanged
      orders.rs                   # does the work order move the numbers? No — the third verb
                                  #   in a row that does not (SYSTEMS.md 6.33). Demoting Mend
                                  #   below Haul makes the tower mend MORE, because hauling
                                  #   first is what funds the repairs. Contains its own
                                  #   control: "mend first" is the shipped default
      watch.rs                    # do M6's wave verbs move the numbers? Focus does not
                                  #   (<=1%, five policies, two tower shapes) and charge
                                  #   priority does not either (five orders, identical to
                                  #   the digit) — and in both cases the code says why.
                                  #   Carries the validity check worth copying: one policy
                                  #   reproduces the engine's own default and must match
                                  #   the control
      glut.rs                     # does the rope chain pay for itself? (it costs 56% of
                                  #   the tower's poles). Written from a played run's
                                  #   ending, and it documents two harness traps it fell
                                  #   into first
  bridge/                         # understory-bridge: wasm-bindgen entry points (cdylib)
    src/lib.rs
assets/
  data/                           # RON content pack: balance.ron, items/, rooms/,
                                  # terrain/, enemies/, regions/
  replays/golden.json          # the embedded native/wasm parity fixture
web/                              # React/TypeScript frontend (Vite)
  src/
    engine/                       # the custom WebGL2 renderer — not React
      AudioManager.ts             # loops from view(), one-shots from frame() — M4 §4.6
    ui/                           # React chrome: panels, cards, readouts
    bridge/                       # the wasm boundary and its TypeScript contract
  e2e/smoke.spec.ts               # Playwright smoke test
  e2e/capture.spec.ts             # screenshot harness — how visual questions get answered
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
make instruments            # run every harness in crates/core/examples — see below
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

# Mutation testing (occasional, not part of make check — an hour a file)
cargo mutants --package understory-core --file crates/core/src/systems/siege.rs

# Frontend (from web/) — ts7 (tsgo) / oxfmt / oxlint, not tsc/prettier/eslint
npm run typecheck             # tsgo — type-checks without emitting
npm run lint                  # oxlint
npm run format                # oxfmt — write
```

See `DECISIONS.md` §10 for why the frontend toolchain is ts7/oxfmt/oxlint rather than the
tsc/prettier/eslint stack v1 used, and why the renderer is custom WebGL2 rather than SVG.

### The instruments, and why `make check` does not run them

`crates/core/examples/*` are the measuring instruments — they answer the design questions
the tests cannot. **`make check` does not run them, and in one session three of them turned
out to be dead or lying:**

- `throughput.rs` **panicked on startup** and had since M5 gated the elevator on rope.
- `siege_run.rs`'s plating comparison counted a total whose maximum was the thing under
  test, producing the same false finding for the fourth time.
- The same harness's whole "battery + darts" tower **built nothing at all** — a dart battery
  costs rope the pressure tower has none of, and nothing checked the return value. It was
  byte-for-byte the bare tower, and M2's exit criterion rested on that comparison.
- `journey.rs` built **no cutter arm and no mill**, from M6 emptying the starting tower until
  it was found. Every one of the twelve seeds and all four policies reported bamboo 0,
  produce 0 and meals 0, halted three quarters of the run because an incomeless tower cannot
  afford to walk, and concluded that no berthing policy ever beat another — which was a fact
  about the shopping list. Fixed with `harness::chain_tower` plus an assertion that the run
  finished owning both rooms; region 1's walker floor moved from 57–76 minutes to 14–18.
  **The tell was three zeroes in a table that otherwise looked fine.**

Clippy's `--all-targets` compiles them, so a type error is caught; nothing runs them, so a
runtime failure is invisible until somebody asks a question. `make instruments` runs the lot.
It is not in `check` because `journey.rs` alone is minutes.

**Read the output, not the exit code.** An instrument measuring the wrong thing exits zero.
Three rules the wreckage taught:

1. **Assert your setup.** If a harness builds a room, assert the build succeeded. Both
   `siege_run` failures were a silent `false` from a place-a-room helper.
2. **Never compare two towers on a quantity whose maximum is the thing you are testing.**
   Plating raises `panel_hp`, so any column counting panels makes the plated tower look worse
   for having more to lose.
3. **Check the run gave the mechanism something to do.** The plating comparison ran at a
   provocation where nothing ever reached a room on either tower — eight seeds, zero damage,
   and a confident conclusion drawn from it.
4. **A sink with a ceiling is not a sink.** `glut.rs` compared two towers on widenings
   bought, which looked unbounded and is three purchases — the hull runs 10 to 16 in steps of
   two. Both towers reached the cap and it reported "identical, the rope chain is free", which
   was a reading of `max_slots`. **Before comparing on a quantity, check it can still go up.**
5. **`RoomView.stalled` is two facts wearing one name** — waiting on an input, *or* backed up
   on an output — and they want opposite fixes. Reading it as the second when it was the first
   kept a wrong theory alive for an hour. `RoomView.stall` (the `StallTag`) says which; use it.
6. **A harness measures its own policy as readily as the game's.** `lift.rs`'s affordability
   column gave three different answers to one question depending on whether its tower grew,
   whether it owned a chute, and what order it bought in — because a tower that spends every
   pole the moment it has one almost never *holds* a surplus, and the column asked what it
   held. Removing the material it was supposedly gated on moved the number not at all, which
   is the tell. Its header says so rather than shipping the figures.

### Mutation testing, and two ways it will lie to you

Scope it to a file — a whole-crate run is hours. Read `mutants.out/missed.txt` rather than
the tail of the command, which truncates.

**Never pass `--in-place`.** It mutates your working tree instead of a copy, so a run that
is interrupted can leave a mutant in your source. There is no reason to want this.

**A survivor may be a stale report.** Results are only as fresh as the tree the run copied
at *start*, and a long run finishing after you have added tests will list mutants those
tests already kill. Before believing a survivor, hand-apply it and run the tests: that takes
a minute and settles it. Doing exactly this is how the note in `BALANCE.md` about the
`discard_generation_past` mutants got corrected — the tool said alive, the hand check said
caught.

**And a survivor is often not a gap.** A guard no shipped content can trigger, a comparison
where validation already forbids the other side, an arithmetic change that a
self-correcting loop absorbs — all report as missed and none of them are worth a test. The
useful question is not "is this caught" but "would a player notice if this were wrong".
Write down the ones you decide to leave, and why.

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
- **A snapshot field with no reader is a bug or a lie, and they are cheap to find.** Two were
  found this way in one session — `enclave_ahead` and `waypoint_ahead`, both documented as
  existing so the renderer could draw a thing coming, both read by nobody, and both hiding a
  real gap. The sweep is one throwaway script: pull every field name out of
  `web/src/bridge/types.ts` and grep the rest of `web/src` for it. Of 18 hits, 2 were bugs and
  16 were deliberate — the Defence card omits damage, rate and range *on purpose* (§8), and
  `crew_required` is `SYSTEMS.md` §6.9 question 0's own subject. **Read the comment before
  "fixing" a field: this codebase writes down what it left out.**
- **`npx playwright test` does not rebuild the WASM, and `make e2e` does.** A field added to
  `snapshot.rs` is simply `undefined` in the browser until somebody rebuilds, every reader of
  it silently takes its fallback branch, and **the whole suite passes green**. That happened:
  `journey.enclave_at` was added and the settlement panel reported "no trades left" for a
  settlement with three. `e2e/dogfood.spec.ts` names the fields the tools depend on and fails
  with "the WASM predates this source" rather than mystifying you. If you touched Rust, run
  `wasm-pack build crates/bridge --target web --out-dir ../../web/pkg --release` before
  believing a browser result.
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
- **Growing taller has a real cost beyond `floor_cost`.** It used to be that a new top
  floor displaced the canopy sail deck (`v2-plan.md` §6.3) — a wall, measured at *zero*
  income four floors up. M6 cut the sails (`SYSTEMS.md` §6.10) and that wall became a
  slope: a fourteen-floor tower's lamps cost 2,268 charge a day against a four-floor
  tower's 648, and one burner leaves it 488 short. The rule is unchanged — don't let floor
  addition become a free action just because the ticks/materials are paid — but the cost is
  now lamps, poles and haul distance rather than a cliff.

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

Built at M4 (`SYSTEMS.md` §4.6); `web/src/engine/AudioManager.ts` is the whole of it, and it
is JS-side entirely — the plumbing had been crossing the bridge since M0.

**The boundary is fixed: Rust decides *that* something happened, JS decides whether and how
it sounds.** `SoundEvent`s are fire-and-forget — emitted during a tick, played or dropped,
never read back into the simulation — and nothing about the mix, the volume, the voice count,
or whether audio is enabled may reach `GameState`.

**The two inputs are not interchangeable, and telling them apart is the whole design.**
`frame()`'s event list drives one-shots: things that *happened*. `view()`'s snapshot drives
loops: things that are *ongoing*. A starved mill going quiet is not an event at all — it is
the absence of a loop, and the fact behind it is `RoomView.stalled`. Getting this backwards is
how a project ends up with a warning beep where a silence belonged.

**A starved production loop goes silent; it does not gain a warning sound.** No alarm on a
stalled room, no beep on a queue, no sting on a full buffer. `wait_ticks` and the red tint are
the bottleneck instrument (`DECISIONS.md` §8) and audio's contribution to them is the mill you
can no longer hear. What keeps absence readable is the beds underneath — the jungle, the day
and night soundscapes, the electrical hum thinning as the bank drains — so silence reads
against a floor rather than against nothing.

Two traps already paid for: the legs read `journey.halt`'s five states and **not**
`power.walking`, because a tower that stopped and a tower that cannot afford to move must not
sound the same; and one-shots are coalesced by kind within a frame and rate-limited per kind,
because `frame()` runs many ticks at 4× and three mills finishing on one tick is one sound.

### Narrative / crew tone

Crew are named individuals with jobs, not stat blocks (`DESIGN.md` §2 structural call 4).
When crew-facing content (barks, portraits, names) lands, keep it off the `sim` RNG stream
— use `cosmetic` (`DECISIONS.md` §2), so that adding or editing a bark can never perturb an
economic roll. Tone follows `DECISIONS.md` §8: solarpunk warmth, defenders not soldiers,
creatures defending territory rather than a target gallery to clear. If a piece of crew
dialogue or a creature description reads as militaristic, that's a tone bug worth flagging
even if it's mechanically inert.
