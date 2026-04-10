# Agent Guidelines — Supply Line

This document guides AI agents (and human engineers) working on Supply Line. It covers project conventions, software practices, and pillar-specific best practices for game development.

---

## I. Project Context

### What this is

Supply Line is a single-player roguelike where you build a supply chain inside a walking tower and personally defend its exterior in real-time combat. Rust/WASM core + React/TypeScript UI. Browser-first (itch.io), desktop via Tauri.

### Doc hierarchy

Before implementing anything:

1. **`docs/foundation/implementation-decisions.md`** — overrides everything else
2. **`docs/foundation/v1-scope.md`** — if a feature isn't tagged `v1`, don't build it
3. **`docs/foundation/registries.md`** — canonical names, stats, unlock order
4. **`docs/foundation/balance-config.md`** — concrete numbers for all systems
5. Detail docs (game-design, controls, equipment, etc.) — full specs, but the above 4 win on conflict

### Architecture in one paragraph

Rust owns all game state and simulation (30hz fixed tick). React owns UI, input collection, and frame orchestration (rAF). They communicate through wasm-bindgen. The full RenderSnapshot never crosses the WASM bridge — React only sees compact typed accessors (`get_hud_state()`, `get_tower_state()`, etc.). Audio is Web Audio API, owned by JS. All mutations flow through `GameCommand` variants. GameState is the single source of truth.

### What v1 ships

3 hero classes, 11 weapon sub-types (5 base types), 16 modifiers, 10 trinkets, 10 companions (8 exterior + 2 interior), 3-chapter journey to The Harbor, full tower building with Tier 1-2 production, seeded procedural generation. No between-run persistence, no relationship system, no pacing adaptation. See `v1-scope.md` for the complete list.

---

## II. Software Engineering Best Practices

### Test-driven development

- **Write the test first.** For game systems especially — the command pattern makes this natural. Feed commands to GameState, assert outcomes.
- **Test at the system boundary.** Don't test internal helper functions in isolation unless they contain complex logic. Test the public interface: command in, state out.
- **Property-based testing for procedural generation.** Map gen, loot gen, encounter composition — these should satisfy invariants (no orphan nodes, threat budget respected, anti-frustration rules hold) across thousands of seeds.
- **Determinism is testable.** Same seed + same commands = same result. If a test is flaky, determinism is broken — treat as a P0 bug.
- **Snapshot tests for bridge outputs.** The typed accessors (`get_hud_state()`, etc.) return compact structs. Snapshot-test their shape to catch unintentional bridge changes.

### Code organization

- **Separation of concerns is structural, not aspirational.** The simulation crate knows nothing about rendering. The renderer knows nothing about React. React knows nothing about game logic. If you find yourself importing across these boundaries, stop — you're violating the architecture.
- **Command pattern is non-negotiable.** No system, callback, or input handler mutates GameState directly. All changes go through `GameCommand` variants that are validated before application. This is what makes the game testable, replayable, and debuggable.
- **Prefer data over code.** Enemy stats, production rates, weapon parameters — these live in the balance config, not in code. If you're hardcoding a number that a designer might want to tweak, put it in the config.
- **Ordered iteration everywhere.** `Vec`, not `HashMap`. If you need key lookup, use a sorted `Vec` or `BTreeMap`. HashMap iteration order breaks determinism.
- **No floating-point surprises.** Use identical operations in identical order. No `HashMap` iteration feeding into f32 accumulation. If determinism breaks, suspect floating point first.

### Error handling

- **Validate at the command boundary.** `GameCommand` processing validates legality (enough ticks? valid floor? correct phase?). Reject bad commands with errors. Don't silently ignore them.
- **Panic on impossible states.** If the simulation reaches a state that should be structurally impossible (negative HP, missing floor, orphaned runner), panic with a descriptive message. These are bugs, not edge cases.
- **Degrade gracefully in presentation.** If the renderer gets unexpected data, render a fallback — don't crash. If audio gets an unknown SoundEvent, skip it. The simulation is authoritative; presentation layers are resilient.

### Performance discipline

- **Profile before optimizing.** The frame budget is generous (6.94ms at 144fps). Don't optimize speculatively. When you do optimize, measure before and after.
- **Allocation-free hot paths.** The simulation tick and render loop should not allocate. Pre-allocate buffers, reuse Vec capacity, avoid String construction in the tick.
- **Batch bridge calls.** Each WASM↔JS crossing has overhead. Don't call `get_hud_state()` 30 times per second if you can call it once per tick and cache.

### Version control

- **Small, focused commits.** One system per PR. Don't mix combat changes with UI refactors.
- **Commit messages reference the design doc.** "Implement runner pathfinding (docs/foundation/software-architecture.md §V)" makes review easier.
- **Balance config changes get their own commits.** With a note on what metric motivated the change and what the before/after values are.

---

## III. Game Design Best Practices

### The core tension

Every design decision should reinforce: **logistics IS strategy, shooting IS action, they share one screen.** If a feature only affects combat or only affects logistics without creating tension between the two, question whether it belongs.

### Design for feel first, numbers second

- **Prototype the interaction before tuning the numbers.** A weapon that feels bad at any damage value has a design problem, not a balance problem. Get the input model right (draw-hold-release, click-wait, hold-channel), then tune.
- **Balance config exists so you can be wrong fast.** Set a number, playtest, adjust. Don't spend hours calculating the "right" value — play 5 encounters and you'll learn more.
- **Target emotions, not metrics.** Telemetry targets (grunt dies in 1-3s, breach rate 15-30%) are proxies for emotional states (grunts feel like chaff, breaches feel like emergencies). If the metric is met but the emotion is wrong, trust the emotion.

### Roguelike design principles

- **Interesting decisions, not optimal solutions.** If there's one correct build, the system is broken. Multiple viable strategies = healthy design.
- **Information before commitment.** Players should know what a choice costs before making it. Show tick costs, material costs, and consequences. Surprise mechanics are for combat, not for prep decisions.
- **Failure should teach.** When a run ends, the player should understand why. Post-combat tips, companion hints, and the physical state of the tower (which panels broke, which racks ran dry) are the feedback loop.
- **Unlocks expand options, not power.** No stat inflation. A first-run player and a 50-run player have identical baselines. The veteran has more tools, not bigger numbers.

### Encounter design

- **Each enemy type teaches one lesson.** Grunts teach aiming. Climbers teach vertical priority. Sappers teach infrastructure protection. If an enemy type doesn't have a clear teaching purpose, it's clutter.
- **Waves create decision points.** Wave 1 tests current readiness. Wave 2 tests sustainability. Wave 3 tests crisis management. Don't add waves for length — add them for escalation.
- **Anti-frustration is invisible.** No 3+ combat chains without breathing room. First mystery event is always positive. These rules exist in the generator, not in the UI. Players should never know they're being protected.

### Economy design

- **Three currencies, three pressures.** Ticks = what you CAN do. Materials = what you SHOULD do. Gold = what you can SUSTAIN. Keep them independent — no single bottleneck should gate everything.
- **The player should always want one more thing than they can afford.** If they have spare ticks, ticks are too generous. If they can't do anything useful, ticks are too scarce.
- **Operating costs create commitment.** Buildings cost gold every encounter. This means building decisions have ongoing consequences, not just upfront costs.

---

## IV. Rust/WASM Best Practices

### GameState

- **Single source of truth.** If you're storing derived data somewhere, document why and ensure it's read-only. Never let derived state feed back into the simulation.
- **Serialize everything.** GameState must be fully serializable (serde). If you add a field, it must serialize. If it can't serialize (e.g., function pointers, file handles), it doesn't belong in GameState.
- **Clone is cheap or something is wrong.** GameState cloning happens for save/load and undo. If clone becomes expensive, you're storing too much transient data in GameState.

### ECS-adjacent, not full ECS

- The architecture uses a struct-of-arrays approach (Tower owns Floors, Floors have Buildings, etc.) rather than a full ECS. Don't introduce an ECS framework (bevy_ecs, specs, legion). The game's entity count is small (tower + ~20 enemies + ~10 companions + ~50 projectiles) and the fixed system order is a feature, not a limitation.

### Fixed-point considerations

- If determinism breaks across platforms (different f32 results on ARM vs x86), consider fixed-point arithmetic for the simulation. Keep this option open by isolating f32 math behind a type alias (`type Scalar = f32`) that can be swapped.

### WASM-specific

- **No threads in v1.** WASM threading (SharedArrayBuffer) has browser compatibility issues. The simulation is single-threaded. Don't reach for rayon or tokio.
- **Minimize bridge crossings.** Each wasm-bindgen call has serialization overhead. Batch commands, return compound structs, avoid chatty APIs.
- **Watch the WASM binary size.** Dead code elimination is good but not perfect. Avoid pulling in large crates for small features. Prefer hand-rolled solutions for simple algorithms over crate dependencies.

---

## V. React/TypeScript Best Practices

### React is the view layer (mostly)

- **During prep:** pure view + input. Render snapshots from Rust, send commands back. No game logic in React.
- **During combat:** React also orchestrates the frame loop (rAF), collects input, calls `tick()`, and syncs React context/state. This is the correct architecture for browser-first — but keep the orchestration thin. React decides WHEN to tick, not WHAT happens during the tick.

### State management

- **React state/context for UI state only.** Selections, panel open/closed, hover targets, cached snapshots from Rust. Never authoritative game state. No external state library — see `implementation-decisions.md` §18.
- **Don't mirror GameState in React.** Call the typed accessor, use the result, discard. If you find yourself building a parallel state tree, you're fighting the architecture.
- **Throttle combat HUD updates to 30hz.** The sim ticks at 30hz. Updating the HUD at 144hz is waste. Use a throttled subscription.

### Component architecture

- Follow the atomic design system in `docs/foundation/design-system.md`: atoms (buttons, inputs) → molecules (AmmoRack, BufferDisplay) → organisms (TowerEditor, HUD) → pages (Prep, Combat, Map).
- **No game logic in components.** Components render and dispatch. They don't calculate damage, validate commands, or run pathfinding. That's Rust's job.
- **Memoize expensive renders.** The tower cross-section has many visual elements. Use `React.memo` and stable references to avoid re-rendering the entire tower when one rack changes.

---

## VI. Pillar: Combat System

### Principles

- **The hero is the star.** 50-70% of kills should come from the hero. If companions overshadow the hero, the game becomes a passive idle game.
- **Every shot is visible.** Arrows stick in enemies, misses hit the ground. No invisible hitscan. This is core feel and non-negotiable.
- **Weapon identity lives in the input model.** Bow = draw-hold-release rhythm. Crossbow = click-wait rhythm. Staff = hold-channel flow. If two weapons feel the same to play, one should be cut or redesigned, regardless of their stats.

### Implementation

- **Projectile simulation is sweep-based.** At 30hz, fast projectiles can tunnel. Check the full travel path per tick, not just current position.
- **Hit detection is server-side (Rust).** No client-side hit prediction. The simulation is authoritative. Visual projectiles interpolate to match.
- **Weapon abilities are cooldown-gated, not ammo-gated** (except where noted). The ability button (Q) should always feel available-soon, not resource-constrained.
- **Aim assist is a stat (Precision), not a setting.** Higher Precision = tighter aim cone. This is how accuracy scales with leveling, not through invisible hitbox expansion.

### Testing combat

- Unit test: fire command at known enemy position → enemy takes expected damage.
- Unit test: sweep collision detects hit on fast-moving projectile that would tunnel at point-check.
- Property test: 1000 random encounters with random weapons → hero kill % is 40-80% (broad sanity range).
- Visual test: fire arrow, watch it arc, see it stick in enemy. If the visual doesn't match the simulation, the interpolation is broken.

---

## VII. Pillar: Supply Chain & Logistics

### Principles

- **The supply chain is visible.** You can see crates move through the tower. You can see racks fill and drain. You can see runners queue at stairs. If the player can't see it, it doesn't exist as a strategic layer.
- **Bottlenecks are the game.** The supply chain should always have one bottleneck the player is aware of and trying to fix. If everything flows smoothly, the tower is overbuilt and there's no tension.
- **Runners are characters, not units.** They have visible behavior (walking, carrying, queuing, resting). They make the tower feel alive. Don't optimize them into invisible instant-delivery.

### Implementation

- **Production is per-tick, not continuous.** Buildings produce discrete crates at fixed intervals. This makes the system predictable and visually readable.
- **Buffer overflow is visible.** When a building's output buffer is full and no runner collects, the building idles. Visually: the building stops animating. Audibly: the production loop goes silent. This IS the feedback mechanism — don't add a UI warning on top of the diegetic signal.
- **Runner pathfinding is greedy.** Runners pick the best available task (highest-priority demand), find the fastest route (chute > dumbwaiter > lift > stairs), and go. They don't plan globally — they react to current state. This makes their behavior readable and predictable.
- **Cache placement is the player's main logistics decision.** Where you put caches determines which floors get fast resupply. Don't automate cache placement — that removes the decision.

### Testing logistics

- Unit test: building produces N crates after M ticks at configured rate.
- Unit test: runner picks highest-priority demand when multiple racks are empty.
- Integration test: full supply chain (fletcher → runner → warehouse → runner → cache → rack) delivers ammo within expected time window.
- Property test: 100 random tower configs → no runner deadlocks, no infinite loops, no starvation when production rate exceeds consumption rate.

---

## VIII. Pillar: Tower Building & Prep Phase

### Principles

- **Prep decisions are bets. Combat is the reveal.** You can't rewire logistics during combat. This hard boundary is what makes prep decisions meaningful. Never add mid-combat building.
- **The tower is the HP bar.** Players should look at their tower and immediately understand its health. Cracks = damage. Empty racks = ammo problems. Silent buildings = starved production. Diegetic UI is primary.
- **Every floor has a cost-benefit tradeoff.** A floor for production means a floor not available for companions. A floor with a cache takes width from transport. Floors aren't free — they cost materials, add height (more to defend), and create new logistics demands.

### Implementation

- **Tick costs enforce pacing.** Every action costs ticks. The player always wants more ticks than they have. If they're banking ticks regularly, the budget is too generous.
- **Validation before march.** `get_validation_warnings()` catches mismatches (weapon with no ammo source, companion with no rack, transport gaps). These are warnings, not blockers — the player can march anyway. Don't prevent marching; inform and let them choose.
- **Tower width is a real constraint.** Transport takes floor width. Buildings take floor width. You can't put everything on one floor. Width management is a puzzle — don't trivialize it.

### Testing prep

- Unit test: `BuildFloor` command deducts correct ticks and materials.
- Unit test: `March` command with unassigned companion generates validation warning.
- Integration test: full prep sequence (build floor, place building, assign companion, place cache, march) produces valid GameState.

---

## IX. Pillar: Procedural Generation

### Principles

- **Determinism is sacred.** Same seed = same run. Period. If you break this, you break seed sharing, replay, and bug reproduction. Test determinism explicitly.
- **Anti-frustration rules are invisible constraints.** The generator must guarantee: no 3+ combat chains without breathing room, at least one merchant or rest per chapter, first mystery event is positive. These are hard constraints, not soft preferences.
- **Difficulty is a budget, not a feeling.** Encounter difficulty is a number (threat budget). The generator fills the budget with enemy combinations. "How hard does this feel" is a playtesting question answered by tuning the budget and enemy threat costs, not by adding special-case logic.

### Implementation

- **Seed the RNG once per generation scope.** Run seed → chapter seed → node seed → encounter seed. Don't reseed randomly. Don't use system time. The seed chain must be reproducible.
- **Validate generated content.** After generating a map, verify: all nodes reachable, boss at end, anti-frustration rules met, node type budget respected. Assert these in tests.
- **Loot rarity is weighted random, not tiered thresholds.** Use the weight table in balance-config.md. Don't hardcode "if roll > 0.97 then legendary" — use the configurable weights.

### Testing procgen

- Property test: 10,000 seeds → every map is connected, every map has a boss, no orphan nodes.
- Property test: 1,000 encounters at difficulty 2 → all compositions within threat budget ±10%.
- Determinism test: generate map with seed X, generate again with seed X → identical result.
- Anti-frustration test: 1,000 chapters → no chapter has 3+ consecutive combat nodes on any path.

---

## X. Pillar: UI/UX

### Principles

- **Diegetic by default, abstract when precision demands it.** Crate piles = buffer level. Cracks = damage. These are primary. Numbers are secondary precision overlays. See `implementation-decisions.md` §11.
- **No modals during combat.** Ever. Combat is real-time. Anything that blocks input during combat is a bug.
- **Glance and know roughly. Hover and know exactly.** The tower visual gives approximate state at a glance. Tooltips give exact numbers on hover. This two-layer approach handles both fast combat scanning and careful prep planning.

### Implementation

- **React components are stateless where possible.** Render from the Rust snapshot. Don't build local state that can drift from the source of truth.
- **Prep UI is information-dense.** Inventory, companion management, tower editor — these screens show a lot. Use the design system's organism components. Don't reinvent layout.
- **Combat HUD is minimal.** Ammo count, cooldowns, wave counter, companion status row. That's it. If you're adding more to the combat HUD, justify why the player needs it at 30hz.
- **Transitions are emotional punctuation.** Prep → march → combat → post-combat → prep. Each transition is a beat. Don't skip them for efficiency — they create rhythm.

### Testing UI

- Snapshot test: render each page component with mock data → visual snapshot matches.
- Interaction test: click "March" → command sent to Rust, phase transitions to Travel.
- Accessibility: all interactive elements have keyboard focus, tooltips work via keyboard, color-blind–safe palette verified.

---

## XI. Pillar: Audio

### Principles

- **The tower is alive.** Each building has a production loop sound. Transport creaks and rattles. Runners' footsteps echo. The tower's audio state IS its health indicator. A silent tower is a broken tower.
- **Two soundscapes: prep (warm) and combat (tense).** Prep sounds like a workshop. Combat sounds like a siege. The crossfade between them is a key emotional transition.
- **Silence is a design choice.** A starved building goes silent. An empty rack has no clink. A breached floor has wind whistling through. Use silence to communicate state, not just sound.

### Implementation

- **Web Audio API only.** No Rust audio crates. JS/React owns all audio. Rust produces `SoundEvent` data; the JS `AudioManager` consumes it.
- **Spatial audio is simple stereo in v1.** Left = tower interior. Right = battlefield. This naturally separates logistics sounds from combat sounds. Advanced height-based reverb is post-v1.
- **Sound events are fire-and-forget.** Rust emits them during tick. JS plays them. If a sound can't play (too many concurrent sounds), drop it — don't queue. Priority: weapon fire > enemy death > building production > ambient.

### Testing audio

- Unit test: tick with enemy death → SoundEvent list contains death sound.
- Integration test: AudioManager receives SoundEvent → Web Audio API node created.
- Manual test: play an encounter with eyes closed. Can you tell how the fight is going from sound alone? That's the bar.

---

## XII. Pillar: Animation

### Principles

- **Animation never changes game state.** An animation can make a death look dramatic, but the entity is dead the instant the simulation says so. Presentation interpolates; simulation is truth.
- **Readability over beauty.** If an animation obscures what's happening in combat, simplify it. The player needs to read the battlefield at 30hz. A gorgeous particle system that hides enemies is a net negative.
- **Motion communicates purpose.** Runners walk with purpose (carrying) or idle (waiting). Enemies approach with intent (ground, climbing, flying). Every entity's animation should answer "what is this thing doing right now?" at a glance.

### Implementation

- **v1 uses sprite-swap animation, not bone rigs.** Keep it simple. Each entity state (idle, walk, attack, die) has a sprite sequence. Bone rigs are post-v1.
- **Tweens for all motion.** Position, rotation, scale, opacity — all tweened. Use easing functions (ease-in for launches, ease-out for landings, ease-in-out for UI transitions). Linear tweens feel robotic.
- **Particles are budget-constrained.** Max particle count is capped. When the cap is hit, drop lowest-priority particles (ambient dust before hit sparks before weapon effects). Don't let particles blow the frame budget.
- **Interpolation bridges sim and render.** At 144fps, entities move smoothly between 30hz simulation ticks. If an entity visually teleports, the interpolation is broken.

### Testing animation

- Unit test: entity at position A in tick N, position B in tick N+1 → interpolation at 50% between ticks yields midpoint.
- Visual test: max enemy count + max projectiles + all buildings active → frame rate stays above 60fps.
- Manual test: watch a runner carry a crate from building to warehouse. Does the motion feel purposeful? Does the crate visually transfer? That's the bar.

---

## XIII. Pillar: Narrative & Companions

### Principles

- **Show, don't tell.** No cutscenes, no lore codex. Story lives in companion dialogue, tower visual state, landscape, loot flavor text, and mystery events.
- **The tower is home, not a war machine.** Tone is Ghibli fantasy: warm, whimsical, melancholic. Defenders, not soldiers. Enemies are obstacles, not a target gallery. If a feature makes the game feel militaristic, reframe it.
- **Companions are people, not stat blocks.** Each has a name, personality, and opinion about what's happening. Their dialogue should react to game state (breaches, close calls, victories, failures). Even if the mechanical effect is small, the personality should be visible.

### Implementation

- **Companion dialogue is state-triggered, not scripted.** Define trigger conditions (panel breached, encounter won flawlessly, companion displaced, etc.) and response pools per companion. The system picks contextually.
- **Dialogue doesn't block gameplay.** Companion comments appear as speech bubbles or sidebar text during prep. They never interrupt, never modal, never require a response.
- **v1 companion dialogue is text-only.** No voice acting, no animated portraits. Just text lines tied to personality and state. Keep the system simple so content can be added fast.

### Testing narrative

- Unit test: trigger condition met (panel breached on Kael's floor) → Kael has at least one response in the pool.
- Coverage test: all companions have responses for all v1 trigger conditions (no silent companions).
- Tone test (manual): read 20 random dialogue lines. Do they sound like the character? Do they fit "home under threat"? Flag any that sound militaristic or generic.

---

## XIV. Working with the Design Docs

### When docs conflict

The override chain is: `implementation-decisions.md` > `v1-scope.md` > `registries.md` > detail docs. If you find a conflict not already resolved in implementation-decisions, flag it — don't guess.

### When docs are silent

If a detail isn't specified, check `balance-config.md` for numbers or `registries.md` for content. If still silent, make a reasonable choice, document it in your PR description, and flag for design review.

### When you want to deviate

Sometimes the docs are wrong or outdated. If you believe a design decision should change, propose it — don't silently diverge. The docs are the shared understanding. Changing code without updating docs creates drift that costs everyone.
