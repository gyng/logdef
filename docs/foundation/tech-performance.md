Project: SUPPLY LINE | Technical Performance & Development Workflow

> **Status:** This document describes v1 only.

## I. Runtime Performance Budgets

Target: 144fps on mid-range hardware (2020-era laptop, dedicated GPU, Chrome/Firefox). Graceful fallback to 60fps on integrated GPUs. The game must never drop below 30fps during worst-case scenarios (max enemies, full tower, all transport running).

Note: simulation runs at a fixed **30hz** timestep regardless of frame rate. Higher frame rates mean smoother rendering and input polling, not faster simulation. At 144fps the renderer interpolates between simulation states. (30hz gives generous CPU headroom per tick while interpolation keeps visuals smooth.)

### Frame budget

At 144fps, total frame time = 6.94ms. Budget allocation:

| Phase | Budget | Notes |
|-------|--------|-------|
| Simulation tick | 1.5ms | Fixed 30hz — runs 0 or 1 tick per frame at 144fps. Averaged: ~0.5ms/frame. Per tick: up to 6ms budget. |
| Render snapshot generation | 0.5ms | Transform GameState → RenderSnapshot (+ interpolation state) |
| wgpu rendering | 3ms | Sprite batching, draw calls, GPU submit |
| React UI update | 0.5ms | HUD refresh only during combat — React updates throttled to 30hz for non-critical UI |
| Audio event processing | 0.2ms | Web Audio API calls from JS |
| Bridge overhead (WASM↔JS) | 0.5ms | Serialization, data transfer |
| Headroom | 0.7ms | Buffer for GC, browser overhead, variance |

At 60fps (fallback): total frame time = 16.67ms. Same budget ratios but with ~10ms additional headroom. Comfortable.

### Simulation vs. rendering — decoupled rates

| Rate | Purpose | Who controls it |
|------|---------|----------------|
| 30hz (fixed) | Simulation, physics, game logic | Rust game loop with fixed timestep accumulator |
| Up to 144hz (variable) | Rendering, input polling, visual smoothness | requestAnimationFrame / display refresh |
| 30hz (throttled) | React UI updates (non-critical HUD) | React layer, intentionally throttled |
| Per-event | Audio (Web Audio API) | Triggered by SoundEvents, processed by browser audio thread |

**Simulation: fixed 30hz (33.33ms per tick).** Every tick advances the game by exactly 1/30th of a second. Physics, economy, runner AI, enemy movement, projectile arcs — all step in fixed increments. This is deterministic: same inputs at the same tick produce the same results. This rate never changes regardless of frame rate. 30hz gives generous CPU headroom (~6ms budget per tick) while interpolation keeps visuals smooth. Can be bumped to 60hz by changing one constant if input latency or collision precision demands it.

**Rendering: variable, up to 144fps.** Renders as fast as the display allows. At 144fps the renderer draws ~4.8 frames per simulation tick. At 60fps it's 2:1. At 30fps it's 1:1.

**Interpolation bridges the gap.** At 144fps, entities would visually "jump" every 33ms and freeze between ticks without interpolation. The renderer blends between two consecutive simulation states based on how far through the current tick interval the frame falls:

```
Tick 0 (t=0ms):      enemy at x=100
Tick 1 (t=33.33ms):  enemy at x=116

Rendered frames between tick 0 and tick 1:
  Frame at t=0ms:    draw at x=100  (tick 0 state)
  Frame at t=7ms:    draw at x=103  (interpolated 21%)
  Frame at t=14ms:   draw at x=107  (interpolated 42%)
  Frame at t=21ms:   draw at x=110  (interpolated 63%)
  Frame at t=28ms:   draw at x=113  (interpolated 84%)

Tick 1 arrives at t=33.33ms: confirm position x=116
```

Result: visually smooth motion at any frame rate, with game logic locked to 30hz.

**Projectile collision at 30hz:** fast projectiles could tunnel past enemies between ticks (33ms gap). Mitigated with sweep collision detection — check the entire path the projectile traveled during the tick, not just its current position. This is a standard technique and adds negligible cost.

**Input polling** runs at the render rate (up to 144hz). Aiming is polled at 144hz — smooth mouse tracking. Shots are processed at the next simulation tick (30hz). Input latency is up to 33ms — acceptable for this genre (not a twitch shooter). 30 potential shots per second is more than enough for any weapon's fire rate.

**The fixed timestep accumulator:**

```rust
const FIXED_DT: f32 = 1.0 / 30.0;  // 33.33ms

pub struct GameLoop {
    accumulator: f32,
    previous_state: Option<RenderSnapshot>,  // for interpolation
    current_state: RenderSnapshot,
}

impl GameLoop {
    pub fn update(&mut self, real_dt: f32, state: &mut GameState, commands: &[GameCommand]) {
        self.accumulator += real_dt;

        // Run 0, 1, or 2 simulation ticks depending on accumulated time
        while self.accumulator >= FIXED_DT {
            self.previous_state = Some(self.current_state.clone());
            game_tick(state, commands, FIXED_DT);
            self.current_state = generate_snapshot(state);
            self.accumulator -= FIXED_DT;
        }

        // Remainder is the interpolation alpha (0.0 to 1.0)
        // Passed to renderer for smooth blending
    }

    pub fn interpolation_alpha(&self) -> f32 {
        self.accumulator / FIXED_DT
    }
}
```

The renderer receives `previous_state`, `current_state`, and `alpha`. It draws each entity at `lerp(previous_position, current_position, alpha)`. Smooth at any frame rate.

### Simulation tick budget (6ms at 30hz — runs once per ~4.8 render frames at 144fps)

The simulation runs all systems in fixed order. Budget per system:

| System | Budget | Scaling concern |
|--------|--------|----------------|
| production | 0.3ms | Scales with building count (~10 max) |
| transport | 1.0ms | Scales with runner count × transport connections. Runner pathfinding is the hottest path. |
| companion_ai | 0.5ms | Scales with companion count (~6 max) × enemy count |
| hero | 0.2ms | Single entity, minimal work |
| projectiles | 0.8ms | Scales with active projectile count. Sweep collision detection at 30hz. |
| enemies | 1.0ms | Scales with enemy count (~30-50 max per wave). Pathfinding, climbing, state transitions. |
| damage | 0.3ms | Scales with enemies-at-panels count |
| economy | 0.1ms | Simple arithmetic |
| encounter_flow | 0.1ms | Wave transitions, timers |
| interior_raiders | 0.1ms | Few raiders, simple timer |
| loot | 0.1ms | Gravity roll, few items |
| **Total** | **4.5ms** | 1.5ms headroom within 6ms tick budget |

### Render budget (6ms)

| Operation | Budget | Notes |
|-----------|--------|-------|
| Sprite batching | 1ms | Sort by texture/layer, build vertex buffer |
| Background | 0.5ms | 1-2 draw calls (landscape layers) |
| Tower interior | 1.5ms | ~20-30 sprites (buildings, transport, runners, buffers) |
| Tower exterior | 1ms | ~10-15 sprites (panels, balconies, racks, fighters) |
| Enemies | 1ms | ~30-50 sprites, instanced where possible |
| Projectiles + effects | 0.5ms | ~20-40 sprites (arrows in flight, particles) |
| GPU submit + present | 0.5ms | wgpu overhead |

### Memory budget

| Category | Budget | Notes |
|----------|--------|-------|
| WASM heap (Rust) | 32MB | GameState (~1MB), system working memory, snapshot buffer |
| Audio buffers (JS) | 16MB | Pre-decoded audio assets (~100 sounds × ~150KB avg) |
| Texture atlas (GPU) | 16MB | Sprite sheets, UI textures. Source art is SVG (vector), but runtime assets are rasterized PNG atlases (see Section V Asset Pipeline). Atlas size stays small because the art style uses flat colors and limited detail. |
| React DOM | 8MB | UI components, React context, cached snapshots |
| **Total** | **~72MB** | Comfortable for any modern browser |

### Worst-case scenario

Maximum load: 8-floor tower, 6 companions, 4 runners, 50 enemies on screen, 30 projectiles in flight, 3 active transport types, all buildings producing, breach in progress.

**Profiling target** for worst-case frame time: ~5-6ms (within 6.94ms budget at 144fps). This is an estimate based on the per-system budgets above, not a demonstrated measurement. Actual worst-case performance depends on browser overhead, GC pressure, bridge serialization cost, and GPU driver behavior — all of which require profiling under real load to validate. The quality scaling below exists precisely because this target may not hold on all hardware. If the frame drops below 144fps, the game seamlessly falls to the next refresh tier (120 → 60). If exceeded even at 60fps:

**Automatic quality scaling:**
1. Reduce particle effects (fewer particles, shorter lifetimes)
2. Reduce stuck-arrow rendering (cap at 3 per enemy instead of unlimited)
3. Reduce resource flow particles (the subtle trails showing goods moving)
4. Lower enemy animation quality (skip frames on distant/background enemies)

Quality scaling is automatic and invisible. No settings menu for "graphics quality" — the game just maintains 60fps.

---

## II. Scaling Concerns & Mitigations

### Simulation

**Runner pathfinding** is the most expensive per-tick operation. Runners evaluate all available transport options, estimate queue times, and choose optimal routes. With 4 runners and 8 floors:

Mitigation:
- Cache transport graph between ticks (only recalculate when infrastructure changes — during prep, not combat)
- Runner re-evaluates route only when: arriving at a new floor, current route becomes blocked, or a new demand signal appears
- Limit route evaluation to 3 best options (not all permutations)

**Enemy pathfinding** (climbing, target selection):

Mitigation:
- Enemies don't pathfind every tick. They evaluate once on spawn, once when reaching the tower base, and once every 30 ticks (0.5s) thereafter
- Climbing is simple: go up one floor toward target. No complex pathfinding needed.
- Target selection: nearest occupied balcony. Pre-compute and cache balcony positions (only changes during prep)

**Projectile collision detection:**

Mitigation:
- Broad phase: check only projectiles near enemies (spatial partitioning — simple grid for 2D)
- Narrow phase: circle-circle intersection (cheap)
- Cap active projectiles at 50. Oldest projectile on the ground despawns when cap is reached.
- Projectiles that miss and hit the ground: remove after 2 seconds (visual lingers, then despawns)

### Rendering

**Sprite count scaling:** worst case ~150 sprites on screen (tower + enemies + projectiles + effects). A properly batched sprite renderer handles 1000+ sprites easily on WebGPU. This is not a concern.

**Draw call count:** target < 20 draw calls per frame. Achieved by:
- One texture atlas for all game sprites (1-2 textures total)
- One draw call per render layer (7 layers = 7 draw calls)
- UI rendered by React (separate from game draw calls)

### WASM↔JS Bridge

**The full RenderSnapshot never crosses the bridge.** It stays in Rust and is consumed directly by the wgpu renderer. Only compact typed accessors cross to React. See [implementation-decisions.md](implementation-decisions.md) §3.

What DOES cross the bridge during combat:
- `get_hud_state()` → ~20 values (ammo count, cooldowns, wave counter, companion status, gold). Tiny. ~200 bytes JSON.
- `get_sound_events()` → sound event list from the last tick. Variable size, typically ~500 bytes.
- `send_commands()` → player input commands (aim, shoot, ability). ~100 bytes per frame.

**Serialization overhead is minimal** because only compact data crosses. The old concern about "10-20KB RenderSnapshot per frame" is eliminated by keeping rendering in Rust.

**Bridge transport rules:**

| Data flow | Frequency | Format | Notes |
|-----------|-----------|--------|-------|
| HudSnapshot (Rust→JS) | Once per sim tick (30hz) | JSON | ~200 bytes. Negligible. |
| SoundEvents (Rust→JS) | Once per sim tick (30hz) | JSON | ~500 bytes typical. |
| InputCommands (JS→Rust) | Every frame (up to 144hz) | JSON | ~100 bytes. |
| PrepCommand response (JS→Rust→JS) | Per player action | JSON | Low frequency, JSON permanently. |
| Typed prep accessors (Rust→JS) | On demand during prep | JSON | get_tower_state(), get_hero_state(), etc. Called after each command, not every frame. |
| UI state for React HUD (Rust→JS) | 30hz throttled | JSON | Bridge serialize > 1ms at 30hz | Binary if needed |

**Migration line:** JSON is the default until profiling shows bridge serialization exceeding its budget (0.5ms per frame). The performance alert at 2ms (see Section III) is the trigger to investigate binary format. Delta encoding (only sending changed entities) is the second optimization, triggered if binary alone doesn't bring serialization under budget. Neither optimization should be built speculatively — profile first.

---

## III. Profiling & Monitoring

### Built-in performance metrics (always available, dev build)

```rust
pub struct PerfMetrics {
    // Per-tick timing
    pub tick_time_ms: f32,
    pub system_times: HashMap<SystemId, f32>,  // per-system breakdown
    pub snapshot_gen_time_ms: f32,

    // Per-frame timing (renderer)
    pub render_time_ms: f32,
    pub draw_calls: u32,
    pub sprite_count: u32,

    // Bridge timing
    pub bridge_serialize_ms: f32,
    pub bridge_deserialize_ms: f32,

    // Memory
    pub wasm_heap_bytes: usize,
    pub entity_counts: EntityCounts,
}

pub struct EntityCounts {
    pub enemies: u32,
    pub projectiles: u32,
    pub runners: u32,
    pub active_transport: u32,
    pub loot_drops: u32,
}
```

### Dev overlay (toggle with F1)

A transparent overlay showing real-time performance data during gameplay:

```
┌─────────────────────────────┐
│ FPS: 60  Frame: 11.2ms     │
│ Tick: 2.1ms  Render: 5.8ms │
│ Bridge: 0.8ms  Audio: 0.3ms│
│                             │
│ Systems:                    │
│   transport: 0.42ms        │
│   enemies:   0.38ms        │
│   projectiles: 0.31ms      │
│   ...                      │
│                             │
│ Sprites: 142  Draws: 12    │
│ Enemies: 34  Projs: 22     │
│ WASM: 18.2MB               │
└─────────────────────────────┘
```

Available in dev builds. Stripped from production builds. Toggle with F1.

### Performance alerts (dev console)

Automatic warnings when thresholds are breached:

| Threshold | Alert |
|-----------|-------|
| Frame time > 16ms (3 consecutive) | `[PERF] Frame budget exceeded: {avg}ms` |
| Tick time > 6ms | `[PERF] Simulation tick slow: {time}ms. Hottest system: {system}` |
| Projectile count > 50 | `[PERF] Projectile cap reached, despawning oldest` |
| Enemy count > 60 | `[PERF] Enemy count high: {count}. Check encounter generation.` |
| Bridge serialize > 2ms | `[PERF] Bridge serialization slow: {time}ms. Consider binary format.` |
| WASM heap > 48MB | `[PERF] Memory high: {size}MB. Check for leaks.` |

### External profiling tools

| Tool | Use case | Setup |
|------|----------|-------|
| Chrome DevTools Performance | Frame timing, JS profiling, Web Audio timing | Built-in |
| Chrome DevTools Memory | JS heap snapshots, GC pressure | Built-in |
| `console.time` / `performance.mark` | Custom timing points in JS audio/bridge code | Instrumented |
| `wasm-bindgen-test` + `cargo bench` | Rust-side benchmarks for simulation systems | Cargo workspace |
| RenderDoc | GPU debugging (via wgpu native, not WASM) | Desktop only |

---

## IV. Testing Strategy

### Unit tests (Rust, `cargo test`)

Every system is a pure function. Test by constructing a GameState, running the system, asserting the result.

```rust
#[test]
fn runner_delivers_to_cache() {
    let mut state = test_state_with_fletcher_and_cache();
    // Fletcher has full output buffer, cache is empty
    for _ in 0..60 { system_transport(&mut state, FIXED_DT); }  // 2 seconds at 30hz
    // Runner should have delivered
    assert!(state.tower.floors[3].cache.unwrap().current > 0);
}

#[test]
fn breach_destroys_building() {
    let mut state = test_state_with_building_on_floor(3);
    state.tower.floors[3].panel.current_hp = 0.0;
    state.tower.floors[3].panel.is_breached = true;
    system_damage(&mut state, FIXED_DT);
    assert!(state.tower.floors[3].building.is_none());
}

#[test]
fn projectile_arc_hits_ground_enemy() {
    let mut state = test_state_with_hero_and_grunt();
    // Fire at correct angle
    apply_command(&mut state, &GameCommand::AimAt { direction: vec2(1.0, -0.3) });
    apply_command(&mut state, &GameCommand::Shoot);
    // Run until projectile reaches enemy
    for _ in 0..120 { game_tick(&mut state, &[], FIXED_DT); }  // 4 seconds at 30hz
    assert!(state.encounter.unwrap().enemies[0].hp < state.encounter.unwrap().enemies[0].max_hp);
}
```

**Test coverage targets:**
- Command validation: 100% (every command variant has valid + invalid tests)
- System functions: >80% (critical paths covered, edge cases for transport/combat)
- Snapshot generation: basic coverage (produces valid output)
- Economy calculations: 100% (math must be correct)

### Integration tests (Rust)

Full encounter simulations. Feed commands, run N ticks, verify outcomes.

```rust
#[test]
fn full_chapter1_encounter() {
    let mut state = create_chapter1_start();
    // Simulate a full encounter with auto-fire
    let commands = generate_auto_fire_commands(&state);
    for tick in 0..1800 {  // 60 seconds at 30hz (1800 ticks)
        let cmds = if tick % 6 == 0 { &commands } else { &[] };
        game_tick(&mut state, cmds, FIXED_DT);
        if state.encounter.is_none() { break; }
    }
    // Should survive chapter 1
    assert!(state.encounter.is_none());  // encounter ended
    assert!(state.tower.foundation.current_hp > 0.0);  // tower survived
}
```

### Determinism tests (Rust)

Verify that identical inputs produce identical outputs. Determinism is a robustness property: it enables seed sharing, replay, automated balance testing, and bug reproduction. (It would also be prerequisite for any future multiplayer, but that's not a current product direction.)

```rust
#[test]
fn simulation_is_deterministic() {
    let commands = generate_random_commands(200);
    let state_a = run_simulation(commands.clone(), 1000);
    let state_b = run_simulation(commands.clone(), 1000);
    assert_eq!(
        serde_json::to_string(&state_a).unwrap(),
        serde_json::to_string(&state_b).unwrap()
    );
}
```

Run this test with multiple random seeds. Any failure = non-determinism bug (float ordering, HashMap iteration, uncontrolled randomness).

### Balance tests (Rust)

Automated playtests that verify balance targets from telemetry-balance.md:

```rust
#[test]
fn chapter1_winrate_above_90_percent() {
    let results: Vec<bool> = (0..100)
        .map(|seed| simulate_ai_run(seed, HeroClass::Archer, 1))
        .map(|result| result.reached_chapter >= 2)
        .collect();
    let win_rate = results.iter().filter(|&&w| w).count() as f32 / 100.0;
    assert!(win_rate > 0.85, "Chapter 1 win rate too low: {}", win_rate);
}

#[test]
fn economy_doesnt_spiral_in_chapter2() {
    let results: Vec<RunTelemetry> = (0..50)
        .map(|seed| simulate_ai_run(seed, HeroClass::Archer, 2))
        .collect();
    let avg_gold_at_ch2_end = results.iter()
        .map(|r| r.final_gold as f32)
        .sum::<f32>() / 50.0;
    assert!(avg_gold_at_ch2_end > 50.0, "Players going broke in Ch2: avg {}", avg_gold_at_ch2_end);
}
```

These require a simple AI player (auto-fire, auto-build basic tower, auto-march). Not meant to play well — meant to verify that the game is completable and the economy doesn't collapse.

### React component tests

React Testing Library. Mock the WASM bridge. Verify components render from snapshots and send correct commands.

```typescript
test('FloorPanel shows building info', () => {
    const snapshot = mockTowerSnapshot({ floors: [{ building: { type: 'Fletcher', buffer: { current: 3, max: 5 } } }] });
    render(<FloorPanel floor={snapshot.floors[0]} />);
    expect(screen.getByText('Fletcher')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '3');
});

test('BuildMenu sends BuildBuilding command', async () => {
    const sendCommand = jest.fn();
    render(<BuildMenu floor={0} onCommand={sendCommand} />);
    await userEvent.click(screen.getByText('Fletcher'));
    expect(sendCommand).toHaveBeenCalledWith({ BuildBuilding: { floor_index: 0, building_type: 'Fletcher' } });
});
```

### End-to-end tests

Playwright or Cypress. Run the actual game in a browser, interact with the UI, verify game state through the bridge.

```typescript
test('can complete a basic encounter', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="new-game"]');
    await page.click('[data-testid="class-archer"]');
    // Wait for encounter to start
    await page.waitForSelector('[data-testid="wave-counter"]');
    // Auto-fire for 30 seconds (game should handle Chapter 1 encounter)
    await page.mouse.move(800, 400);  // aim right
    await page.mouse.down();
    await page.waitForTimeout(30000);
    await page.mouse.up();
    // Should see post-combat summary
    await expect(page.locator('[data-testid="post-combat-summary"]')).toBeVisible();
});
```

---

## V. Development Workflow

### Project setup

```
supply-line/
├── Cargo.toml              # Rust workspace
├── crates/
│   ├── core/               # Game simulation (pure, no IO)
│   ├── bridge/             # wasm-bindgen API
│   └── renderer/           # wgpu rendering
├── web/                    # React frontend
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
└── scripts/
    ├── dev.sh              # Start everything for development
    ├── build.sh            # Production build
    └── test.sh             # Run all tests
```

### Dev server setup

Two processes running in parallel:

1. **Rust WASM watch build:** `cargo watch -s 'wasm-pack build crates/bridge --target web --dev'`
   - Rebuilds WASM on Rust file changes
   - Dev build (debug symbols, no optimization)
   - Incremental: ~3-5 seconds rebuild

2. **Vite dev server:** `cd web && npm run dev`
   - Serves React app with HMR
   - Picks up WASM changes automatically
   - React changes: instant HMR (<1 second)

Single command to start both: `./scripts/dev.sh` (runs both in parallel, kills both on Ctrl+C).

### Compile time targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Rust incremental (core crate) | < 3s | Most common during gameplay iteration |
| Rust incremental (bridge crate) | < 5s | Includes wasm-pack processing |
| Rust incremental (renderer) | < 4s | Shader changes may be slower |
| Rust clean build | < 30s | Full workspace from scratch |
| React HMR | < 1s | Vite, instant for component changes |
| Full production build | < 2min | Optimized WASM + minified React + assets |

**Compile time mitigations:**
- Split Rust code into 3 crates (core, bridge, renderer). Changing core doesn't recompile renderer.
- Use `mold` linker on Linux, `lld` on Windows for faster linking
- Minimize proc-macro dependencies (serde_derive is the big one — accept the cost)
- `cargo-workspace-hack` if dependency graph gets deep
- Consider `wasm-opt` only on production builds (slow but reduces WASM size)

### Hot reload strategy

| What changed | Reload behavior |
|--------------|-----------------|
| React component | HMR, instant. State preserved. |
| React context | HMR, state reset. |
| Rust simulation logic | WASM rebuild (~5s), page auto-reloads. Game state lost. |
| Rust renderer | WASM rebuild (~5s), page auto-reloads. |
| Game balance values (Rust constants) | WASM rebuild. Consider extracting to JSON config loaded at runtime for faster iteration. |
| Audio assets | Vite detects change, reloads. |
| Art assets | Vite detects change, reloads. Sprite atlas may need rebuild. |

**Balance values as runtime config:** numbers like enemy HP, production rates, tick costs, gold amounts should live in a JSON/TOML file loaded at startup, NOT compiled into Rust. This allows balance iteration without recompiling WASM. The config file is loaded by the bridge crate on init.

```rust
#[derive(Deserialize)]
pub struct BalanceConfig {
    pub grunt_hp: f32,
    pub grunt_speed: f32,
    pub fletcher_production_rate: f32,
    pub tick_cost_build_floor_wood: u32,
    pub runner_salary: u32,
    // ... all balance-affecting constants
}
```

Changed a number → save JSON → game reloads config → test immediately. No compile step.

### Debug tools

**In-game (dev build only):**
- F1: performance overlay (frame times, entity counts, system times)
- F2: toggle tower debug view (show transport connections, buffer values, runner assignments as text overlays)
- F3: toggle enemy debug view (show pathfinding targets, HP values, state names)
- F4: toggle economy debug view (show gold income/expense per encounter, production rates)
- F5: skip encounter (instant win, for testing prep/map flow)
- F6: give resources (add 99 of each material to warehouse, for testing builds)
- F7: spawn enemy (click to spawn a specific enemy type for testing combat)
- F8: time controls (0.5x, 1x, 2x, 4x speed. Pause. Step one tick.)
- F9: save current state to clipboard as JSON (for bug reports / test case creation)
- F10: load state from clipboard (restore a saved state for reproduction)

**Console commands (browser dev console):**
```javascript
window.debug.setGold(999);
window.debug.setTicks(99);
window.debug.spawnEnemy('armored', { x: 800, y: 300 });
window.debug.setPhase('encounter');
window.debug.toggleInvincible();  // panels don't take damage
window.debug.dumpState();  // full GameState to console
window.debug.metrics();  // current PerfMetrics
```

These are available in dev builds only. Production builds strip all debug code.

### CI/CD pipeline

```yaml
# On every push:
- cargo fmt --check          # formatting
- cargo clippy -- -D warnings  # lints
- cargo test --workspace     # all Rust tests (unit, integration, determinism, balance)
- cd web && npm run lint     # React linting
- cd web && npm run test     # React component tests
- wasm-pack build crates/bridge --target web  # verify WASM compiles

# On PR merge to main:
- Full production build
- Run E2E tests (Playwright)
- Build desktop package (Tauri)
- Deploy web build to staging

# On release tag:
- Production build with optimizations
- wasm-opt on WASM binary
- Deploy to production (itch.io / custom site)
- Build + sign desktop packages (Windows, Mac, Linux via Tauri)
```

### Asset pipeline

**Sprites/art:**
- Source: SVG files (vector art)
- Build step: rasterize SVGs to PNG sprite sheets at target resolution
- Sprite atlas packed by `texture-packer` or equivalent tool
- Atlas JSON (sprite positions) loaded by renderer at startup
- Dev workflow: edit SVG → save → asset pipeline auto-rebuilds atlas → Vite picks up change

**Audio:**
- Source: WAV files (uncompressed)
- Build step: encode to OGG (lossy, ~128kbps) for production
- Dev: load WAV directly (larger but no encode step)
- Audio manifest JSON lists all sound IDs and file paths

**Balance config:**
- Source: `balance.toml` or `balance.json` in assets directory
- Loaded at runtime by Rust bridge on init
- No build step — edit and reload

### Version control conventions

- `main` branch: always deployable
- Feature branches: `feature/combat-system`, `feature/lift-programming`
- Conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `perf:`, `test:`
- Game design docs (docs/foundation/*): versioned alongside code. Design changes require doc updates.
- Balance config changes: committed with a note explaining the reasoning ("Increased grunt HP from 10 to 15 — too easy in Ch1 based on telemetry")

---

## VI. Production Optimization Checklist

Before release, verify:

- [ ] WASM binary optimized with `wasm-opt -O3`
- [ ] Unused Rust code eliminated by LTO (link-time optimization)
- [ ] React bundle tree-shaken, code-split by page
- [ ] Audio assets compressed to OGG
- [ ] Sprite atlas packed, no wasted space
- [ ] All debug tools stripped from production build
- [ ] Performance overlay removed
- [ ] Console debug commands removed
- [ ] Balance config: embed default values in WASM binary (no separate fetch needed), but support an override file for post-release hotfixes. The embedded config is the shipping default; the override file (if present) patches specific values without a full rebuild. This preserves the runtime-iteration workflow from dev while eliminating the separate file dependency for most players.
- [ ] Source maps generated but not deployed publicly
- [ ] Tested on: Chrome, Firefox, Safari, Edge
- [ ] Tested on: low-end hardware (4GB RAM, integrated GPU)
- [ ] Total download size < 20MB (WASM + assets + React)
- [ ] Initial load time < 5 seconds on broadband
- [ ] No memory leaks over 30+ minute session (verified with heap snapshots)
