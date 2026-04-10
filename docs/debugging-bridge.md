# Debugging the WASM Bridge

Systematic guide for diagnosing issues at the Rust ↔ JS boundary. Written for both human engineers and AI agents.

## Architecture recap

```
React                           Rust (WASM)
─────                           ──────────
send_command(json) ──────────►  GameCommand (deserialized)
                                    │
                                    ▼
                                GameEngine.send_command()
                                    │
                                    ▼
get_hud_state() ◄──────────────  HudSnapshot (serialized to JSON string)
get_tower_state() ◄────────────  TowerSnapshot
tick(real_dt) ◄────────────────  Vec<SoundEvent> (serialized)
```

Every bridge call crosses the WASM boundary via `wasm-bindgen`. Data is serialized as JSON strings in both directions. The bridge layer lives in `crates/bridge/src/lib.rs`. TS types live in `web/src/bridge/types.ts`.

## Decision tree

Start here when something is wrong at the bridge.

```
Is the WASM module loading?
├─ No  → §1 WASM initialization failures
├─ Yes
│   Does send_command() return an error?
│   ├─ Yes, JSON parse error    → §2 Command serialization mismatch
│   ├─ Yes, CommandError        → §3 Command validation failure
│   ├─ No (returns Ok)
│   │   Is the snapshot data wrong/missing?
│   │   ├─ Yes, field missing   → §4 Snapshot schema drift
│   │   ├─ Yes, wrong values    → §5 Snapshot logic bugs
│   │   ├─ No
│   │   │   Is tick() not advancing?
│   │   │   ├─ Yes              → §6 Tick/accumulator issues
│   │   │   ├─ No
│   │   │   │   Are sounds missing?
│   │   │   │   ├─ Yes          → §7 SoundEvent issues
│   │   │   │   └─ No           → §8 Performance / bridge overhead
```

---

## §1 WASM initialization failures

**Symptoms:** `initBridge()` throws, blank screen, "WASM bridge not yet built" error.

**Check:**
1. Was `wasm-pack build --target web` run in `crates/bridge/`?
2. Does `web/pkg/` exist with `.wasm` and `.js` files?
3. Is the import path in `web/src/bridge/index.ts` correct?
4. Browser console: look for `CompileError` (WASM binary corrupt) or `LinkError` (missing imports).

**Agent workflow:**
```bash
# Verify wasm-pack output exists
ls web/pkg/supply_line_bridge*.{js,wasm}

# Rebuild if missing
cd crates/bridge && wasm-pack build --target web --out-dir ../../web/pkg

# Check the JS glue compiles
cd web && npx tsc --noEmit
```

**Common causes:**
- Forgot to rebuild WASM after Rust changes.
- `wasm-pack` target mismatch (`--target web` for Vite, not `--target bundler`).
- WASM binary too large for synchronous compile — needs `await init()`.

---

## §2 Command serialization mismatch

**Symptoms:** `send_command()` returns `{"error":"Invalid command: ..."}`. The JSON was malformed or doesn't match the Rust `GameCommand` enum.

**Check:**
1. Print the JSON string being sent from JS before calling `send_command()`.
2. Compare against `crates/core/src/command.rs` — serde's default enum serialization is `{"VariantName": {fields}}` (externally tagged).
3. Check field names: Rust uses `snake_case`, serde serializes as `snake_case` by default.

**Agent workflow:**
```bash
# Find the Rust enum variant
grep -n "pub enum GameCommand" crates/core/src/command.rs

# Check serde attributes — any #[serde(rename = ...)] or #[serde(tag = ...)]?
grep -n "serde" crates/core/src/command.rs

# Verify the TS side is constructing the right shape
grep -rn "send_command" web/src/
```

**Example of correct command JSON:**
```json
{"BuildFloor": {"material": "Wood"}}
{"AimAt": {"direction": {"x": 1.0, "y": 0.0}}}
{"Fire"}  // ← WRONG: unit variants serialize as "Fire", not {"Fire"}
"Fire"    // ← CORRECT for unit variant
```

**Critical gotcha:** Serde's default for unit variants (no fields) serializes them as plain strings (`"Fire"`, `"March"`), while struct variants serialize as `{"Name": {fields}}`. If you're wrapping all commands in `{"command_type": ...}`, that's wrong — send the serde-native format.

**Automated check (add as test):**
```rust
#[test]
fn command_roundtrip() {
    let cmd = GameCommand::Fire;
    let json = serde_json::to_string(&cmd).unwrap();
    let back: GameCommand = serde_json::from_str(&json).unwrap();
    // If this fails, serialization format changed
}
```

---

## §3 Command validation failure

**Symptoms:** `send_command()` returns `CommandResult::Error(CommandError::...)`. The command was well-formed but rejected by the engine.

**Check:**
1. What `CommandError` variant? Read the error fields.
2. `WrongPhase` — sending a combat command during prep, or vice versa. Check `GameState.phase`.
3. `InsufficientTicks` / `InsufficientGold` / `InsufficientMaterials` — resource check failed.
4. `InvalidFloor` / `FloorOccupied` — structural constraint violated.

**Agent workflow:**
```bash
# Find where the command is validated
grep -n "apply_command\|fn validate" crates/core/src/engine.rs

# Check the current phase
# (from JS console or by reading get_hud_state/get_tower_state)

# Find the specific error variant
grep -n "CommandError" crates/core/src/command.rs
```

**This is not a bridge bug.** The bridge worked correctly — the game logic rejected the command. Fix the caller or the game state, not the bridge.

---

## §4 Snapshot schema drift

**Symptoms:** TS code gets `undefined` for a field that should exist, or a runtime type error when reading snapshot data. The Rust struct and TS interface have diverged.

**Check:**
1. Compare `crates/core/src/snapshot.rs` structs against `web/src/bridge/types.ts` interfaces field-by-field.
2. Did someone add a field to the Rust struct without updating the TS type?
3. Did a serde rename or skip attribute change?

**Agent workflow:**
```bash
# Diff the Rust snapshot fields against TS types
grep "pub " crates/core/src/snapshot.rs | head -40
grep -A 20 "interface HudSnapshot" web/src/bridge/types.ts

# Find all snapshot structs
grep "pub struct.*Snapshot" crates/core/src/snapshot.rs

# Find all TS interfaces
grep "export interface" web/src/bridge/types.ts
```

**Prevention:** Snapshot tests. Serialize a known GameState to JSON, assert the shape matches. If the shape changes, the test breaks before the TS code does.

```rust
#[test]
fn hud_snapshot_shape() {
    let engine = GameEngine::new(42, HeroClass::Archer);
    let hud = engine.get_hud_state();
    let json = serde_json::to_value(&hud).unwrap();
    // Assert expected keys exist
    assert!(json.get("ammo_primary").is_some());
    assert!(json.get("companion_statuses").is_some());
}
```

---

## §5 Snapshot logic bugs

**Symptoms:** Snapshot returns but values are wrong (always 0, stale, missing entries).

**Check:**
1. Is the accessor reading from the right part of GameState?
2. Is the data being computed or just stubbed? Search for `// TODO` in the accessor.
3. Is the snapshot being called at the right time? (After tick for combat, after command for prep.)

**Agent workflow:**
```bash
# Read the accessor implementation
grep -A 30 "fn get_hud_state" crates/core/src/engine.rs

# Find TODOs in accessors
grep -n "TODO" crates/core/src/engine.rs

# Check if the data source is populated
grep -n "ammo_primary\|personal_ammo" crates/core/src/engine.rs
```

**Common causes:**
- Accessor reads from `tower.hero.personal_ammo` but the value is never decremented by the combat system.
- Companion statuses read `ammo: 0` because the rack lookup isn't implemented yet.
- `enemies_remaining` filter uses wrong state comparison.

---

## §6 Tick / accumulator issues

**Symptoms:** Game appears frozen, simulation doesn't advance, tick count doesn't increment, or simulation runs too fast/slow.

**Check:**
1. Is `tick(real_dt)` being called from the RAF loop?
2. What value is `real_dt`? Should be seconds since last frame (typically 0.006–0.033).
3. Is the accumulator draining? If `real_dt` is always less than `FIXED_DT` (0.0333), no sim tick runs.
4. Is the phase `Encounter`? Tick is a no-op outside encounter phase.

**Agent workflow:**
```bash
# Check the tick implementation
grep -A 20 "pub fn tick" crates/core/src/engine.rs

# Check FIXED_DT value
grep "FIXED_DT" crates/core/src/types.rs

# Check the systems::tick guard
grep -A 5 "pub fn tick" crates/core/src/systems.rs
```

**Common causes:**
- `real_dt` passed in milliseconds instead of seconds (16.67 vs 0.01667).
- RAF callback not running (tab backgrounded, `requestAnimationFrame` not called).
- Phase is `Travel` (prep) — tick intentionally does nothing.
- Accumulator overflow: if `real_dt` is huge (e.g., after a tab switch), many ticks run at once. Cap `real_dt` to `FIXED_DT * 4` to prevent spiral of death.

---

## §7 SoundEvent issues

**Symptoms:** No audio during combat, sounds play at wrong times, or `tick()` returns empty arrays.

**Check:**
1. `tick()` returns serialized `Vec<SoundEvent>`. Parse the JSON and inspect.
2. Systems are stubs — are the system `run()` functions actually pushing SoundEvents?
3. Is `AudioManager.play()` being called with the parsed events?
4. Web Audio: is the `AudioContext` in `running` state? (Browsers require user gesture to start.)

**Agent workflow:**
```bash
# Check which systems emit sounds
grep -rn "sounds.push" crates/core/src/systems/

# Check if AudioManager.play() is called
grep -rn "AudioManager\|\.play(" web/src/

# Check AudioContext state handling
grep -n "AudioContext\|resume\|suspend" web/src/audio/AudioManager.ts
```

**Common cause:** Systems are stubs that don't push any SoundEvents yet. The bridge is fine — the simulation isn't producing sounds.

---

## §8 Performance / bridge overhead

**Symptoms:** Sluggish frame rate, visible hitching, high JS-to-WASM call overhead.

**Check:**
1. How many bridge calls per frame? Should be: `send_commands()` (batched), `tick()`, `get_hud_state()`. Three calls max during combat.
2. Is `get_tower_state()` being called every frame during combat? It shouldn't — only during prep.
3. Are snapshots being serialized/deserialized unnecessarily? Cache on the JS side.
4. Profile with browser DevTools Performance tab — look for long `wasm-function` entries.

**Agent workflow:**
```bash
# Count bridge calls in the combat loop
grep -n "bridge\.\|getBridge()" web/src/ -r

# Check for per-frame calls to prep-only accessors
grep -n "get_tower_state\|get_journey_state\|get_hero_state" web/src/ -r
```

**Rules from the architecture:**
- Combat: `tick()` + `get_hud_state()` at 30hz. That's it.
- Prep: `send_command()` + relevant accessor, on-demand per user action.
- Never call the same accessor twice per tick.
- The full `RenderSnapshot` never crosses the bridge — it stays in Rust for wgpu.

---

## Adding bridge instrumentation

For systematic debugging, add optional logging to the bridge layer:

```rust
// In crates/bridge/src/lib.rs
pub fn send_command(&mut self, json: &str) -> String {
    #[cfg(debug_assertions)]
    log::debug!("bridge::send_command: {}", json);

    let cmd: GameCommand = match serde_json::from_str(json) { ... };
    let result = self.engine.send_command(cmd);

    #[cfg(debug_assertions)]
    log::debug!("bridge::result: {:?}", result);

    serde_json::to_string(&result).unwrap_or_else(|e| format!(r#"{{"error":"{e}"}}"#))
}
```

On the JS side, wrap `getBridge()` calls during development:

```typescript
function debugBridge<T>(name: string, fn: () => string): T {
  const raw = fn();
  console.debug(`bridge::${name}`, raw.slice(0, 200));
  return JSON.parse(raw) as T;
}
```

Remove or gate behind a flag before shipping.

---

## Checklist for bridge changes

When modifying any struct that crosses the bridge:

- [ ] Update the Rust struct in `crates/core/src/snapshot.rs` or `command.rs`
- [ ] Update the TS type in `web/src/bridge/types.ts`
- [ ] Update the accessor in `crates/core/src/engine.rs` if snapshot fields changed
- [ ] Add or update a snapshot shape test
- [ ] Run `make check` — catches type mismatches on both sides
