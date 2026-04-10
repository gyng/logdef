# v1 Implementation Progress

> Tracks what's actually implemented vs the v1 scope. This is a moving snapshot, not a contract.

**Last updated:** 2026-04-10

## Headline numbers

- **58** content RON files (weapons, enemies, trinkets, modifiers, buildings, heroes, chapters)
- **43** Rust unit/integration tests passing
- **Playwright smoke test** passes end-to-end (chapter 1: map → prep → 3 combats → victory) in ~15s
- **3 chapters** authored, full progression to Victory works

## What's playable

A complete game loop runs from start to finish:

1. **Map**: pick a node from the current chapter (Combat, EliteCombat, Merchant, Rest, Mystery, Boss)
2. **Prep**: build floors (wood/stone), place 7 building types (Fletcher/Forge/Quarry/Lumberyard/Smelter/Alchemist/Enchanter), allocate hero stat points, see validation warnings, march
3. **Combat**: hero aims and fires (click), 2 starting companions auto-fire, weapon ability (Q) fires a 3-shot burst, hero skill (E) does class-specific effect (Focus / Overclock / Rally), Tab swaps weapon
4. **Post-combat**: gold + XP awarded, level up grants stat points
5. **Continue**: back to map, advance through chapter, beat boss, advance to next chapter
6. **Victory** after chapter 3 boss
7. **Game Over** if foundation HP reaches 0

Side mechanics:
- **Merchant nodes**: shop with 3 trinkets at 20g each, ContinueJourney to leave
- **Rest nodes**: free panel repair + bonus ticks
- **Mystery nodes**: random event from a 6-outcome pool
- **Save / Load** to localStorage via top-right menu
- **Audio**: synthesized oscillator tones for fire / hit / death / breach / wave / victory

## v1 scope coverage

### ✅ Hero classes (3/3)
Archer (Focus), Engineer (Overclock), Commander (Rally) — all have data files, starting weapons, and class-specific E skill. 5-stat allocation system working.

### ✅ Weapons (11/11)
- **Bows** (3): shortbow, longbow, composite_bow
- **Crossbows** (2): hand_crossbow, heavy_crossbow
- **Staves** (2): wand, staff
- **Thrown** (2): javelin, bomb
- **Melee** (2): dagger, sword

Modifier system has typed effect variants (16+ effects), 13 modifier RON files authored. Effects don't yet apply at runtime — that's the next layer.

### ✅ Trinkets (10/10)
All 10 v1 trinkets authored with typed effect enums. Buy via merchant. Effect runtime application not yet implemented.

### ✅ Enemies (10/10 archetypes with data)
Grunt, Runner, Armored, Climber, HovererFlyer, Catapult, Ram, Sapper, BossGround, BossClimber.

Distinct behaviors:
- Climber transitions to Climbing state at tower face, ascends floors
- Catapult/Flyer use stop_distance (350/200) to bombard from range
- Sapper destroys first building/cache on contact, then dies
- Bosses get +50% damage in phase 2 (HP < 66%)

### ✅ Buildings (7/7)
Fletcher, Forge, Quarry, Lumberyard (T1), Smelter, Alchemist, Enchanter (T2). Production rates from data, MVP auto-deliver to hero ammo.

### ✅ Chapters (3/3)
- Chapter 1 "The Road" — 4 nodes linear (combat → combat → boss)
- Chapter 2 "The Wilds" — 7 nodes branching (combat / merchant / rest / elite / boss)
- Chapter 3 "The Frontier" — 9 nodes branching (combat / mystery / merchant / rest / elite / boss)

### ✅ Game state machine
MapView → Travel → Encounter → PostCombat → MapView (or Merchant → MapView), Victory after final chapter boss, GameOver on foundation death.

### ✅ UI screens (5)
MapPage (combined map + prep), CombatPage (Canvas2D combat), PostCombatPage, MerchantPage, GameOver/Victory inline.

### ✅ Save / Load
Bridge.save() / load() through localStorage. Top-right menu in non-combat phases.

### ✅ Audio
SoundEvent → AudioManager oscillator synth. 14 event variants mapped. Rate-limited per kind.

### ✅ Determinism
Same seed + same commands = same state. Enforced via clippy `disallowed_types` denying HashMap. 43 tests including determinism roundtrips.

### ✅ Data-driven content
Plan 001 phases 1-5 implemented. RON files load via include_dir at compile time, full Registry validation, hot-reload feature gated for native dev.

## What's still rough

These are gaps from full v1 polish — the loop works without them:

- **Companion management UI**: companions are auto-assigned at game start; no UI to swap, equip, or change orders.
- **Inventory / weapon swap UI**: EquipWeapon command exists but no screen to choose new weapons.
- **Perk tree UI**: stat allocation works, perk system not exposed.
- **Trinket / modifier effect runtime**: data is loaded and equippable but the engine doesn't yet read effect variants. This is the next layer of mechanical depth.
- **Full transport/runner system**: runners exist as state but the v1 system uses an auto-deliver shortcut from production output to hero ammo. The full pathfinding/lift/chute system is post-v1 polish.
- **Boss visual phase transitions**: phase 2 damage buff works but no visual indicator.
- **Procedural map generation**: chapters are hardcoded RON. Procgen can be layered later without changing the registry shape.
- **Companion dialogue / barks**: no triggered text in any system.
- **Visual polish**: combat is colored rectangles on a Canvas2D — wgpu sprite renderer is post-v1.

## How to play

```
make dev
# open http://localhost:3000
```

In combat: click to fire, Tab to swap weapon, Q for weapon ability, E for hero skill.

## Run tests

```
make check          # fmt + clippy + cargo test
cd web && npm run e2e   # Playwright smoke test (full game loop)
```
