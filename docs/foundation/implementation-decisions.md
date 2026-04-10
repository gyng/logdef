Project: SUPPLY LINE | Implementation Decisions

> **Status:** Canonical, overrides all other docs. When any other foundation doc conflicts with this file, this file wins.

Canonical answers to every cross-doc contradiction. When any other doc conflicts with this file, this file wins.

---

## 1. Simulation tick rate

**Decision: 30hz (33.33ms per tick).**

Not 60hz. The simulation advances in fixed 1/30th-second increments regardless of frame rate. Rendering interpolates between ticks for visual smoothness at any display refresh rate.

Can be changed to 60hz by modifying one constant (`FIXED_DT`). No architectural change needed. Start at 30hz, profile, bump if input latency or collision precision demands it.

**Projectile tunneling at 30hz:** mitigated with sweep collision detection (check the full travel path per tick, not just current position).

---

## 2. Rendering ownership

**Decision: Rust owns the game canvas. React owns the UI overlay.**

Rust (wgpu) renders the tower cross-section, battlefield, enemies, projectiles, effects — everything on the game canvas. React renders the prep UI panels, map, inventory, companion management, HUD overlays — everything that's HTML/CSS.

The game canvas is a `<canvas>` element managed by Rust/wgpu. React components render on top of it via absolute positioning. They do not share a rendering pipeline.

---

## 3. Bridge model — snapshot boundary

**Decision: one `RenderSnapshot` struct for the game canvas. Separate typed accessors for React UI.**

During combat, Rust produces a single `RenderSnapshot` per tick containing all visual state (entity positions, sprite IDs, health bars, buffer levels, particle data). The wgpu renderer consumes this directly in Rust — it never crosses the WASM bridge. React's combat HUD reads a SMALL subset (ammo count, cooldowns, wave counter, companion status) via a separate `get_hud_state()` call that returns only what the HUD needs. This is NOT the full snapshot — it's a compact struct of ~20 values.

During prep, React calls typed accessors:
- `get_tower_state()` → tower layout, floors, buildings, transport, caches
- `get_hero_state()` → stats, perks, equipment, level
- `get_journey_state()` → map, current node, available paths
- `get_inventory()` → weapons, trinkets, materials
- `get_companion_roster()` → all companions with stats, orders, relationships
- `get_merchant_state()` → merchant stock (at merchant nodes)
- `get_validation_warnings()` → pre-departure warnings

These are separate typed endpoints, not one monolithic snapshot. Each returns only what that UI panel needs. They are called on-demand (after each command), not every frame.

**The full `RenderSnapshot` never crosses the WASM bridge.** It stays in Rust. Only the compact HUD state and typed UI accessors cross.

---

## 4. Game loop ownership

**Decision: Rust owns the game loop. React requests frames.**

During combat:
1. React calls `requestAnimationFrame`
2. In the RAF callback, React collects input (mouse position, clicks, key presses)
3. React sends input commands to Rust via `send_commands(json)`
4. React calls `tick(real_dt)` — Rust's accumulator decides whether to run 0 or 1 simulation ticks
5. Rust renders the game canvas (wgpu) — this happens inside `tick()`, on the Rust side
6. React calls `get_hud_state()` to update the combat HUD
7. Next RAF

React drives the frame timing (via RAF) but does NOT run the simulation or render the canvas. Rust does both. React is the clock, Rust is the engine.

During prep:
- No RAF loop. React calls `send_command()` per player action, then calls the relevant typed accessor to refresh the UI. Rust does not tick during prep.

---

## 5. Audio ownership

**Decision: Web Audio API only. JavaScript/React owns all audio.**

No Rust audio crates (no kira, no rodio). Audio lives entirely in the JS layer. Rust produces `SoundEvent` data as part of the tick output. A JS `AudioManager` class consumes these events and plays sounds via Web Audio API. Works identically in browser and Tauri/Electron desktop.

The `SoundEvent` list is returned from `tick()` alongside the HUD state, or via a separate `get_sound_events()` call.

---

## 6. UI update cadence

**Decision: combat HUD at 30hz (matching sim tick rate). Prep UI on-demand.**

The combat HUD (ammo, cooldowns, wave counter, companion status) updates once per simulation tick — 30 times per second. Not every render frame (144hz would be wasteful for numbers that only change at 30hz). React components for the HUD use a throttled update tied to tick completion.

Prep UI updates on-demand — after each command is processed. No polling, no interval.

---

## 7. Save model

**Decision: one autosave per encounter boundary. No manual save. No save scumming.**

The game autosaves:
- After each encounter ends (before prep phase begins)
- After "March" is clicked (prep decisions committed)

One save slot per run. Overwritten each time. Loading a save resumes at the last save point. You cannot reload mid-encounter to undo a bad fight.

Saves are JSON (serde). Browser: localStorage. Desktop (Tauri): user data directory. The full `GameState` is serialized — deterministic replay is possible by saving the seed + command log instead, but v1 uses full state serialization for simplicity.

---

## 8. Dynamic pacing (full vision — see §13 for v1 scope)

**Decision: pacing adaptation EXISTS but is explicitly scoped.**

The game does NOT adjust difficulty (enemy HP, damage numbers, economy values). Those are fixed by the balance config and the encounter's difficulty rating.

The game DOES select encounter compositions from within the rated difficulty tier based on recent performance. A difficulty-2 encounter has a pool of possible compositions (some harder, some gentler within the tier). If the player has been struggling (multiple breaches, low gold), the next difficulty-2 encounter draws from the gentler end of the pool. If cruising, from the harder end.

This is called **pacing adaptation**. It affects encounter composition selection, not balance values. It is invisible to the player. It is documented in [telemetry-balance.md](telemetry-balance.md) §X.

The game also provides **player suggestions** (companion dialogue tips based on telemetry) — these are informational only and never change game state.

---

## 9. Metagame honesty

**Decision: unlocks expand strategic capability, not raw stats. This IS a form of power progression. Say so.**

Correct framing: "Each run starts at the same stat baseline. No +5% damage forever. But unlocking new weapon families, building tiers, and transport types widens your solution space. A player with full unlocks has more strategic options than a first-run player. Knowledge is the primary advantage; toolkit breadth is the secondary one."

Do NOT claim "no power progression" unqualified. Claim "no stat inflation" and "identical baseline" — which are true.

---

## 10. Relationship bonus honesty

**Decision: relationship bonuses are strategically meaningful. Say so.**

Correct framing: "Relationship bonuses are moderate — they won't save a bad build or compensate for poor aim. But at higher tiers (Close/Bonded), affinity-dependent bonuses (up to +10% accuracy, +15% damage, +30% panel HP for Guard×Guard) are significant enough to influence companion placement decisions. The bonuses reward long-term investment in a pair without being mandatory for success."

Do NOT claim "narrative-only" or "you can ignore them." They're small enough to ignore on Standard difficulty, meaningful on Ascension.

---

## 11. Diegetic UI rule

**Decision: diegetic by default. Abstract UI wins when precision or time-critical readability demands it.**

The tower cross-section, ammo racks, panel cracks, buffer piles — these are diegetic (the world IS the UI). Numbers, cooldown timers, wave counters, companion status rows — these are abstract overlays because you can't read "3.2 seconds remaining" from a visual alone.

The prep phase is entirely abstract UI (React panels). No attempt to make inventory management or lift programming "diegetic." That would sacrifice usability.

**Scrolling:** allowed in prep-phase panels (inventory lists, companion roster) when content exceeds viewport. NOT allowed for the tower cross-section (always fits on screen). The UI-UX doc's "no scrolling" rule applies to the tower view, not to every UI surface.

**Modals:** never during combat. Allowed during prep for confirmations (dismiss companion, sell equipped gear) and full-screen overlays (map, hero stats).

---

## 12. Interior raider expulsion timer

**Decision: fixed 15 seconds.**

Interior raiders are expelled by the tower spirit after exactly 15 seconds. Not a range, not random. Fixed and learnable — players can assess how much infrastructure damage a breach will cause and plan accordingly. The Tower Heart trinket reduces this to 8 seconds (~47% reduction), which is a meaningful defensive choice at a fixed, predictable value.

---

## 13. Pacing adaptation scope

**Decision: pacing adaptation is post-v1. v1 uses pure random selection from the difficulty pool.**

This document describes pacing adaptation in §8 as a design that EXISTS in the full vision. For v1 implementation, encounter composition is selected randomly from the difficulty tier pool with no performance-based weighting. The pacing adaptation system described in §8 and in [telemetry-balance.md](telemetry-balance.md) §X is deferred to post-v1.

---

## 14. Melee sidearm slot

**Decision: the dagger sidearm occupies one of the hero's two weapon slots. It can be sold or replaced.**

Every hero starts with a dagger in weapon slot 2. It auto-activates when the primary weapon's rack is empty (standard weapon-swap behavior, not a hidden mechanic). The dagger is a normal weapon — it can be sold, replaced at a merchant, or swapped for any other weapon. There is no hidden third slot.

If the player sells/replaces both weapons with ranged types and runs out of ammo, they have no fallback — this is an intentional consequence of their loadout decision.

---

## 15. Companion v1 weapon assignments

**Decision: companion weapon preferences that reference post-v1 weapon types are resolved with v1-available substitutions, except where the post-v1 weapon is added to v1 scope as a companion-only type.**

Two companion preferences reference post-v1 weapons:
- **Rust** prefers Guns → Guns (pistol sub-type) are added to v1 scope as a **companion-only weapon type**. The hero cannot equip guns in v1. Rust uses a pistol with the click-fire input model. This requires: pistol sprite, pistol SFX, gunpowder ammo production (Forge + Alchemist chain, already Tier 2 in v1).
- **Stone** prefers Hammer → Hammer (melee sub-type) is added to v1 scope as a **companion-only weapon type**. Stone uses a hammer with the click-to-swing melee model. No ammo required.

Companion-only weapons do not appear in loot tables, merchants, or the hero's inventory. They are fixed equipment on the companion.

---

## 16. Weapon unlock gating within a run

**Decision: staves and thrown weapons are gated behind the Ch2 boss. They are not available from run start.**

The [registries.md](registries.md) unlock table is canonical. Bows, crossbows, and melee are available from run start. Staves and thrown unlock after beating the Ch2 boss. This applies to both hero weapons (loot drops, merchant stock) and companion weapon availability.

---

## 17. Color hierarchy

1. **Resource type colors** are sacred — arrows are ALWAYS warm brown (#c87941), bolts ALWAYS steel (#7a8fa3), mana ALWAYS purple (#7c5cbf). These never mean anything else.
2. **Status colors** are second priority — healthy green (#3daa5e), warning amber (#d4922a), critical red (#c9433a). These are used for HP bars, buffer states, panel damage.
3. **Rarity colors** use distinct hues that don't overlap with resource or status — common silver (#a0a8b4), uncommon teal-green (#48b066), rare blue (#4a8fd4), legendary warm gold (#e8a830).
4. **UI accent colors** (interactive.focus, text.accent) are tertiary and must not conflict with the above.

Green is overloaded (healthy + uncommon). Resolution: uncommon rarity uses **teal-green** (#48b066), which is bluer than status healthy (#3daa5e). They're distinguishable in context (rarity badges vs. health bars) but the hue gap should be verified in testing.

Gold is overloaded (currency + legendary rarity). Resolution: gold currency in the HUD uses a smaller, less saturated coin icon. Legendary rarity uses a larger, more saturated shimmer border. Context distinguishes them — currency is a number, rarity is a border treatment.

---

## 18. UI state management — no Zustand

**Decision: plain React state and context. No Zustand.**

React is not the source of truth for game state — Rust is. The React layer only needs to cache compact snapshots from the bridge and track transient UI state (selections, panel open/closed, hover targets). This is a small amount of state that React's built-in `useState` and `useContext` handle well without a third-party store.

Other docs reference Zustand in diagrams and code examples. Those references describe the *role* (UI state cache) correctly — the implementation is React context instead of a Zustand store. The role and data flow are identical; only the library is different.
