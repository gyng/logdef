# Understory — Decisions

> Cross-cutting engineering rules. Numbered so code comments can cite them —
> `// see DECISIONS.md §4` next to a command handler is a legitimate comment. These are
> rules about *how* the game is built; what the game *is* lives in `DESIGN.md`, what's
> *built so far* lives in `SYSTEMS.md`, and the numbers live in `BALANCE.md`.
>
> Each section says what breaks if you ignore it. That's not decoration — if you can't
> say what breaks, the rule probably shouldn't be a rule.

---

## §1 Determinism

No floating point appears in `GameState`, in any system, or in any `GameCommand`. Every
sub-integer quantity is `Fx` — a signed Q8.8 fixed-point value over `i32` (`crates/core/src/fx.rs`).
Item counts and costs are `i64`. Floor and slot indices are `u8`. Tick durations are `u32`
counts of fixed 30 Hz ticks, never wall-clock floats. `Fx` multiplication and division widen
through `i64` and truncate toward zero, so the result is bit-identical on every target with
64-bit integers — that's the entire reason it exists instead of `f32`.

Iteration is ordered everywhere: `Vec`, sorted where lookup matters (`Content` interns
items/rooms/terrain into `Vec`s sorted by string ID and binary-searches them), never a
`HashMap`. This is enforced structurally, not just by convention — `disallowed_types = "deny"`
in the workspace clippy lints (`Cargo.toml`) makes `HashMap`/`HashSet` a build failure, not a
review comment.

System order within a tick is fixed: stride → intake → production → haul
(`crates/core/src/systems.rs`). Haul runs last deliberately, so crew react to the buffers
this tick actually produced rather than last tick's. Reordering, even moving one system
earlier "because it seems more correct," invalidates every golden replay recorded against
the old order.

The guarantee this all buys: same seed plus the same command stream produces the same XXH3
hash of serialized `GameState` (`replay::hash_state`), on native and in wasm, every time.
`crates/core/src/tests/determinism.rs` asserts this directly — same seed/same state,
different seeds diverge, batching ticks differently doesn't change the result, a rejected
command changes nothing, save/load round-trips exactly, and (structurally, by walking the
serialized JSON) no field is ever a non-integer number.

**What breaks if you violate this:** a flaky determinism test is not flaky — it is a P0 bug
in whatever changed most recently. Any float that leaks into state makes hashes
platform-dependent, which silently breaks replays, seed sharing, and the native/wasm parity
gate in §5, usually in a way that only shows up on one platform and looks like a browser bug
instead of a determinism bug.

---

## §2 Named RNG streams and the cosmetic firewall

`RngStreams` (`crates/core/src/rng.rs`) splits the run seed into three independent
xorshift64 streams via a splitmix64 conditioner: `world` (terrain and region generation),
`sim` (anything the economy rolls for), and `cosmetic` (presentation-only jitter — currently
just the per-crew `fidget` value handed to the renderer at spawn).

The firewall is a *drawing* rule, not a type rule: nothing stops `cosmetic` from being
passed to a system that also touches `sim`, so the discipline is "no system reads
`cosmetic` and writes anything another system reads," enforced by review and by
`cosmetic_stream_never_moves_the_economy` in `tests/determinism.rs`, which burns 500 draws
from `cosmetic` and asserts every economic outcome (stats, world distance) is untouched.

**What breaks if you violate this:** the day a bark system in M4 draws from `sim` instead
of `cosmetic` to pick a line, every economic roll downstream of that draw shifts. A shared
seed stops reproducing the same run for two people who saw different bark text along the
way — non-reproducible in a way that looks like it has nothing to do with barks, and is
brutal to bisect.

---

## §3 Bridge patterns

`GameEngine::view()` returns one `ViewSnapshot` — the entire presentable state — as one
JSON document per frame, built fresh from `GameState` each call. `crates/bridge/src/lib.rs`
exposes exactly this shape: `frame()` once per animation frame to advance the sim, `view()`
once after it. v1 made a dozen separate typed accessor calls at 30 Hz; every wasm↔JS
crossing pays a serialization cost whether or not anything changed, so that's a cost paid
for nothing. `catalog()` is a second, separate call for content definitions — fetched once
at startup, static for the life of a content pack, and must not be polled.

JSON is the transport today because it's debuggable and irrelevant at cozy scale (8–14
floors, single-digit crew counts). It is not the intended end state: the snapshot shapes in
`snapshot.rs` are already grouped the way a Worker-plus-SoA/transferable-buffer transport
would want them — `TowerView { floors, shafts }`, `Vec<CrewView>`, flat `Vec<StockView>` —
so swapping the transport later is a change to *how* the same data crosses, not a redesign
of *what* crosses.

`Fx::to_f32` exists solely for the presentation boundary and is legal to call in exactly one
place: `snapshot.rs`. Nowhere in a system, and nowhere in `command.rs`.

**What breaks if you violate this:** adding a second per-subsystem accessor (say, a
`get_crew_state()` called separately from `view()`) reintroduces the v1 chattiness this
architecture exists to avoid, and it will not show up as a bug — it'll show up as a
framerate regression nobody traces back to the bridge. Calling `Fx::to_f32` inside a
system reintroduces floating point into a code path that §1's determinism guarantee assumes
is float-free; the compiler won't catch it, only review and the "presentation only" comment
on the function will.

---

## §4 Command pattern

Every mutation to `GameState` is a `GameCommand` (`crates/core/src/command.rs`), applied
through `engine::commands::apply` (`crates/core/src/engine/commands.rs`). Validation fully
precedes mutation in every handler: `place_room` checks the room exists, the slot range
fits, the floor restriction, uniqueness, and slot occupancy — all against the *current*
state — before it ever calls `spend()`. `build_floor` checks the floor cap and calls
`check_stock` before `spend`, never the reverse. This isn't incidental ordering; it's the
property that makes a rejected command a byte-identical no-op, asserted directly by
`a_rejected_command_changes_nothing` in `tests/determinism.rs`.

Only accepted commands are recorded: `GameEngine::send` records into the replay only inside
the `Ok(())` branch (`engine.rs`). A rejected command never touches the recorder, because it
never changed anything a replay would need to reproduce.

**What breaks if you violate this:** if a handler spends stock and *then* discovers the
room can't be placed, a rejected `PlaceRoom` silently mutates state. Two consequences:
the state hash test above starts failing (correctly), and — worse, if that test is ever
weakened — a replay that never recorded the failed command would reconstruct a *different*
tower than the one the player actually had, because the real run spent materials that the
replay never sees spent.

---

## §5 Replay-as-the-format

`Replay` (`crates/core/src/replay.rs`) is `{ seed, content_hash, commands: [{tick, cmd}],
checkpoints: [{tick, hash}], final_tick }`. Semantics are exact: a command stamped tick *T*
applies *before* tick *T* runs; a checkpoint at tick *T* is the state hash *after* T ticks
have executed, so checkpoint 0 is the freshly seeded state before anything runs.
Checkpoints land every `CHECKPOINT_INTERVAL` (30 ticks — one second) so a divergence is
caught within a second of simulated time and reported by tick, not just at the end.

One format serves four jobs: the save file, the regression fixture, the seed-sharing
format, and — if it's ever needed — netcode. There is deliberately no second format for any
of these.

`assets/replays/golden.json` is embedded into the binary with `include_str!`
(`replay::GOLDEN_REPLAY`), so the native `cargo test` run and the browser's Playwright smoke
test verify literally the same bytes. That identity — not "an equivalent replay," the same
JSON — is the native/wasm parity gate: if `verify_golden_replay()` passes both places, wasm
and native agree on every checkpointed state hash for that recording. Regenerate it with
`cargo run -p understory-core --example record_golden` (`crates/core/examples/record_golden.rs`)
whenever a deliberate simulation change makes the old fixture stale — and if it drifts when
you *didn't* intend a simulation change, that's a determinism bug per §1, not a fixture to
regenerate away.

**What breaks if you violate this:** a second bespoke format for saves or for fixtures means
two serialization paths to keep in sync, and the parity gate stops meaning anything the
moment "the fixture" and "a save" aren't provably the same mechanism. Forgetting to rebuild
after regenerating the fixture means the embedded copy is stale and the test passes against
old bytes while looking green.

---

## §6 Content packs

Designers author string IDs in RON (`"room.mill"`, `"item.bamboo"`). `Content::load`
(`crates/core/src/content.rs`) interns every one exactly once, at load, into a dense `u16`
index (`ItemIdx`, `RoomIdx`, `TerrainIdx` — `crates/core/src/ids.rs`), by sorting each
definition list by its string ID and letting position *be* the index. That means indices
are a pure function of pack content, never of directory-walk order, and lookups after load
are `binary_search_by` on the sorted `Vec`, not a hash map. Every room's recipe, build cost,
and intake are pre-resolved into `RoomRuntime` at load time, so no system ever compares a
string during a tick — the tick loop only ever sees `ItemIdx`/`RoomIdx`.

The pack's bytes (path plus content, in the sorted load order) are hashed into
`content_hash` via XXH3, and that hash is stamped into every replay. `verify_replay` checks
it before replaying a single tick and fails loudly with the two hashes if they don't match,
rather than replaying a recording against data it wasn't recorded against and producing
confusing divergences that look like a determinism bug.

**What breaks if you violate this:** a system that compares `&str` room IDs mid-tick pays a
string comparison 30 times a second per room, and worse, a designer renaming a string ID
would silently break nothing at *load* time but could still corrupt in-flight tasks that
cached the old ID. Skipping the content-hash check on replay means a renamed or rebalanced
pack replays against a recording made for different data and produces a divergence report
that points at the wrong tick.

---

## §7 Balance provenance

Every constant in `assets/data/balance.ron` is described in `BALANCE.md` with a reasoning
note and a grade: `DESIGNED` (reasoned about, unproven) or `PLAYTESTED` (played against
neighboring values and won). `crates/core/src/tests/balance_doc.rs` enforces this in both
directions — `every_balance_field_is_documented_with_a_grade` fails if `balance.ron` gains a
field the doc doesn't grade, and `the_doc_does_not_grade_constants_that_no_longer_exist`
fails if the doc still grades a field that's gone. Content constants (room and item numbers
in `assets/data/rooms/`) are graded as a group in `BALANCE.md` rather than individually,
since they're tuned as a set.

M5 does not ship until every row reads `PLAYTESTED` (`v2-plan.md` §9).

**What breaks if you violate this:** without the bidirectional test, `BALANCE.md` rots the
first time someone tunes a number in `balance.ron` during a playtest session and forgets the
doc — at which point "provenance-graded" is a claim the repo makes about itself that isn't
true, which is worse than not grading at all.

---

## §8 Tone guardrails

Carried forward from v1's `art-direction.md`, re-skinned Ghibli-European → solarpunk-tropical.
Solarpunk warmth, not gunmetal. "The tower is a home, not a war machine" — defenders, not
soldiers. Creatures defend their territory; the tower is passing through their world, not
clearing it. See `DESIGN.md` §3 pillar 5 for the design argument; this section is the
enforceable version.

Diegetic UI is primary, numbers are secondary. Breakage, silence, and queue stress *are*
the interface: a stalled production room renders quiet (`RoomView.stalled` in
`snapshot.rs`, driven by starved inputs or a backed-up output, never by a warning banner); a
crew member blocked past `stress_ticks` tints red in the cross-section
(`CrewView.stressed`, driven purely by `wait_ticks`); an intake room that fills its buffer
visibly stops rather than discarding overflow. None of these states are reported through a
dashboard number first — the number is available on hover, for the player who wants
precision, but the diegetic signal is what the game leads with. `state/crew.rs` states this
directly: `wait_ticks` is "the only bottleneck instrument in the game... there is no
throughput dashboard and there will not be one."

**What breaks if you violate this:** adding a numeric warning banner on top of a diegetic
signal (say, a popup that reads "Mill 3 is starved") doesn't just clutter the UI — it
undermines the specific claim the whole visual language makes, which is that you can read
the tower's health by looking at it. M4's exit criteria are literally a screenshot test and
an eyes-closed audio test for exactly this property; a stray dashboard number is the kind of
thing that quietly fails both without ever showing up as a bug report.

---

## §9 Process rules

From `v2-plan.md` §10, restated here because they're engineering discipline as much as
project management:

1. **No content type ships until a system consumes it at runtime.** v1 authored sixteen
   modifier effects and applied zero of them. This is now a load-bearing content-validation
   check, not just a policy: `content::validate` rejects any room whose `category` doesn't
   have a matching behavior block wired up (`RoomCategory::Intake` requires `intake.is_some()`,
   `Production` requires `recipe.is_some()`, and so on) — a room with content but no
   consumer is a load error, not a warning.
2. **Core-mechanic depth beats content breadth.** The elevator ships before the eleventh
   anything. M1 (`v2-plan.md` §9) is the sacred milestone for exactly this reason.
3. **Every sprint ends playable:** checks green, replay fixtures green, and the milestone's
   design question answered by actually playing it — not by inspection.
4. **Determinism discipline is CI, not culture.** Replay hash-parity between native and wasm
   runs on every push from M1 onward (§5); it is not a manual step someone remembers to run.
5. **Docs follow play.** `SYSTEMS.md` grows one milestone section at a time, written before
   that milestone's code and not before. `v2-plan.md` is the only whole-game document — this
   file and `DESIGN.md` are cross-cutting, not sequential, and neither one restates the
   sprint schedule.
6. **Constants live in `BALANCE.md`-graded data.** Covered in full in §7.

**What breaks if you violate this:** rules 1–2 are the direct fix for the single biggest
failure of attempt #1 — a large content surface with a thin, mostly-unused mechanical core.
Skipping rule 3 (shipping a sprint that passes its tests but was never actually played)
is how v1 shipped an elevator that existed as types only, with weapon #11 authored instead.

---

## §10 Frontend stack

Two of these are owner decisions that deviate from what `v2-plan.md` §8 originally called
for, made explicitly and recorded here so the divergence is intentional rather than drift.
The third is a continuation of existing practice, not a deviation, and is included because
it's easy to reach for the opposite by habit.

**(a) Custom WebGL2 renderer from M0 — deviation.** `v2-plan.md` §8 called for "TowerViz
SVG center-stage... port to canvas/WebGL only if cozy-scale entity counts ever hurt." The
owner call is to skip the SVG stage entirely: a batched 2.5D cross-section plus parallax
terrain renderer in WebGL2, with a DOM overlay for text, built from M0 rather than
profile-gated later. Reason: SVG was judged too slow before M0 started, so gating the
WebGL2 build behind a profiling threshold that was already known to trip would just be
paying for the SVG implementation twice. The yeettower renderer architecture that
`v2-plan.md` names as the fallback blueprint (single-batcher WebGL2, static chunks rebuilt
on change, dynamic actor layer, DOM text overlay) is the actual starting point, not a
contingency.

**(b) ts7 (tsgo) / oxfmt / oxlint rather than tsc / prettier / eslint — deviation.**
`v2-plan.md` §7's KEEP table carries forward v1's "Makefile, clippy HashMap ban, fmt/lint
configs" as part of "the hard-won 80%," which implied keeping the existing tsc/eslint/prettier
setup as-is. The owner call is to replace the TypeScript toolchain: `npm run typecheck`
runs `tsgo` (ts7), `npm run lint` runs `oxlint`, `npm run format` runs `oxfmt`. Faster
tooling on a project this small is worth the migration cost once, rather than carrying
v1's slower stack forward by default. See `AGENTS.md` for the exact commands.

**(c) No external state library — continuation, not a deviation.** React holds UI state
only (panel open/closed, hover targets, the last snapshot fetched from the bridge); it is
never authoritative game state, and there is no zustand/redux/jotai in the dependency tree.
The renderer owns its own frame loop independent of React's render cycle — React does not
drive per-frame rendering, it drives chrome around the renderer's canvas. This is the same
rule v1 held (`implementation-decisions.md` §18, now archived on `main`) and it isn't
being revisited for v2.

**What breaks if you violate this:** pulling in a state library reintroduces exactly the
failure mode `DESIGN.md`'s "React is the view layer" framing exists to prevent — a second
copy of game state that can drift from what Rust says is true, discovered only when the two
disagree on screen. Reaching for tsc/eslint/prettier out of habit because "that's what v1
used" just means two toolchains half-configured in the repo at once; pick the one this
section names.

---

## §11 Cling timers, not indefinite grip

An attacking creature does not hold on to the tower forever. Each `EnemyDef` (content,
`assets/data/enemies/*.ron`) carries a `cling_ticks` value; once a creature makes contact it
starts a countdown that only runs while the tower is actually striding (`GameState.strode`,
`crates/core/src/systems/siege.rs`) and while it is still in contact with the tower at all —
including a creature that has run out of things to chew and dropped back to circling. When
the countdown reaches zero the creature lets go and is left behind, rather than being
destroyed. The timer is set once, when the creature spawns, and never renewed — an earlier
version reset it on contact, which meant a creature that lost its target and later found
another got a fresh full grip for it, so mending a panel during a wave made the wave last
longer. This makes `SetStriding` (`command.rs`) a real answer to a wave that costs
nothing in poles or darts: keeping the legs moving is a legitimate way to survive one, and
stopping to work mid-assault becomes a genuine risk instead of a free action, which is what
makes triage under fire (`SYSTEMS.md` §2.1) an actual decision rather than a slogan.

The alternative considered and rejected: creatures cling until something kills them, so
every wave is answered by an emplacement or not at all. That was the original, unmeasured
design, and measured directly it made an unanswered wave a certainty rather than a risk: a
probe tower with no defences and no player intervention took integrity from 1000 to 88 with
7 rooms wrecked, and the same 8 creatures were still attached 7,000 ticks later. It also made
"keep walking" mechanically meaningless during a fight, which contradicts pillar 2's premise
that transport — including the decision to keep moving — is a live lever during combat, not
just during peacetime (`DESIGN.md` §3). Cling timers are tuned per creature, not a blanket
reprieve: a root-borer's `cling_ticks` (1,200) is well short of the ~2,333 ticks it needs to
sever a shaft alone (`BALANCE.md`, `shaft_hp`), so one borer on a walking tower gets a column
to roughly half and loses its grip before finishing it, and it takes two overlapping borers
— or a tower that stopped moving — to actually sever one. Emplacements still matter; they
are no longer the *only* answer.

**What breaks if you violate this:** raising `cling_ticks` back toward "effectively
infinite," or letting the countdown run only while a creature is mid-bite instead of
whenever it is in contact, quietly turns every wave back into a mandatory emplacement check
— the failure mode measured above, where a tower with no darts yet is guaranteed to lose
rooms it never had a chance to defend. That contradicts pillar 3 ("combat is a load test,"
not a gate) and removes the one purely-defensive tool — walking — available to a tower that
hasn't built a battery yet. Making the timer run regardless of `state.strode` (rather than
gating on whether the tower actually moved) breaks the tie to the charge economy: a tower
that cannot afford to walk would get the escape-by-motion benefit for free, which undercuts
the bank-or-burn tension `SYSTEMS.md` §1.2 and `DESIGN.md` §5.2 describe.
