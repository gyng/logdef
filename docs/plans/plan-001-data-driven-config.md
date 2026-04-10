# Data-Driven Config Plan

> **Status:** Plan, not yet implemented. Reference for the migration from hardcoded `balance.rs` constants to RON data files.

## Goal

Move all tuning numbers, content definitions, map data, and UI strings out of Rust source into editable data files. Designers (and the developer in design-mode) should be able to tweak values without recompiling. Hot reload in dev. Strict validation in CI.

## Format: RON

Use [RON (Rusty Object Notation)](https://github.com/ron-rs/ron). It's serde-native, supports comments, trailing commas, real Rust enums, named structs, and tuples — the things JSON/TOML/YAML all lack. Enum support is the deciding factor: `kind: Bow(draw_time: 0.8)` round-trips to a Rust enum without inventing tagged-union conventions.

Enable these RON extensions in every hand-authored file:

```ron
#![enable(implicit_some)]            // omit Some() wrapper
#![enable(unwrap_newtypes)]          // drop newtype parens
#![enable(unwrap_variant_newtypes)]  // flatten enum variant newtypes
#![enable(explicit_struct_names)]    // catch wrong-struct typos
```

Reference: [Veloren](https://book.veloren.net) is the canonical Rust game shipping RON at scale; this plan borrows their patterns. (Gridlock isn't a public reference — Veloren is what we'll lean on.)

## Directory layout

```
assets/data/
  balance.ron              # all tuning numbers (one big file)
  entities/
    heroes/
      archer.ron
      engineer.ron
      commander.ron
    weapons/
      bow/
        shortbow.ron
        longbow.ron
      crossbow/
        hand_crossbow.ron
      melee/
        dagger.ron
    enemies/
      grunt.ron
      runner.ron
      armored.ron
    buildings/
      fletcher.ron
      forge.ron
  chapters/
    chapter_1.ron          # nodes, edges, encounter compositions
    chapter_2.ron
    chapter_3.ron
  strings/
    en.ron                 # English UI strings
    # future: ja.ron, de.ron
```

**Why this layout:**
- `balance.ron` is one flat file because numbers form a coherent whole — designers tune it as a unit.
- Entity content is one-file-per-entity — cleaner diffs, fewer merge conflicts as content grows.
- Strings live in their own dir per locale (i18n-ready).
- `chapters/` separated from entities because levels are composed of entity references.

## ID scheme

Use **enum variants** for fixed sets and **string IDs** for instances:

| Thing | Type | Example |
|---|---|---|
| Weapon base type (5 variants, fixed) | Rust enum | `WeaponBaseType::Bow` |
| Weapon instance (~20+ in v1+post) | String ID newtype | `WeaponId("weapon.bow.shortbow")` |
| Enemy archetype (10 variants, fixed) | Rust enum | `EnemyArchetype::Grunt` |
| Companion instance (10 in v1) | String ID newtype | `CompanionId("companion.kael")` |
| Hero class (3 variants, fixed) | Rust enum | `HeroClass::Archer` |
| Map node ID (per-chapter) | u32 newtype | `NodeId(1)` |

Newtype-wrap all string IDs so the type system still catches mix-ups:

```rust
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeaponId(pub String);
```

Cross-references between files use string IDs:

```ron
// entities/heroes/archer.ron
Hero(
    id: "hero.archer",
    starting_weapon_primary: "weapon.bow.shortbow",
    starting_weapon_secondary: "weapon.melee.dagger",
    starting_stats: HeroStats(precision: 7, draw: 6, tempo: 5, grit: 4, salvage: 4),
    skill: Focus,
)
```

## Loader architecture

Two modes, switched by cfg:

**Native dev (with `hot_reload` feature):** use [`assets_manager`](https://docs.rs/assets_manager) for file-watcher hot reload. Designer saves a `.ron`, loader detects, parses, swaps the new `Arc<Registry>` between ticks. Failed reloads keep the old registry — never crash on bad data.

**WASM and tests:** `include_bytes!` all RON files at compile time. Zero I/O, deterministic, identical native+WASM behavior. Build script bakes the file tree.

```rust
#[cfg(all(feature = "hot_reload", not(target_arch = "wasm32")))]
fn load_registry() -> Arc<Registry> { /* assets_manager */ }

#[cfg(not(all(feature = "hot_reload", not(target_arch = "wasm32"))))]
fn load_registry() -> Arc<Registry> {
    let bytes = include_bytes!("../../assets/data/balance.ron");
    parse_and_validate(bytes).expect("baked-in data must be valid")
}
```

The `Registry` struct holds all loaded data:

```rust
pub struct Registry {
    pub balance: Balance,
    pub heroes: BTreeMap<HeroId, HeroDef>,
    pub weapons: BTreeMap<WeaponId, WeaponDef>,
    pub enemies: BTreeMap<EnemyId, EnemyDef>,
    pub buildings: BTreeMap<BuildingId, BuildingDef>,
    pub chapters: Vec<ChapterDef>,
    pub strings: BTreeMap<String, String>,
}
```

`BTreeMap` not `HashMap` — determinism. Iteration order matters.

## Determinism contract

This is **load-bearing** for the deterministic-simulation requirement (see [implementation-decisions.md §19](foundation/implementation-decisions.md)):

1. **`Registry` is immutable during a tick.** Load once into `Arc<Registry>`, pass by reference into systems. Never mutate.
2. **Hot reload swaps the Arc between ticks, never during.** The game loop checks for a pending reload at tick boundaries only.
3. **Same RON files + same seed + same commands = same result.** Snapshot-test the post-load Registry to catch unintentional content changes.
4. **`BTreeMap` everywhere**, never `HashMap` (already enforced by clippy `disallowed_types`).

## Validation

Three layers, stacked from cheap to comprehensive:

**1. Type-level (free):** `#[serde(deny_unknown_fields)]` on every deserialized struct. Catches typos in field names. Catches missing required fields. Catches wrong types.

**2. Cross-reference pass (load-time):** after parsing all files, walk the registry and verify every ID reference resolves:

```rust
pub fn validate(reg: &Registry) -> Result<(), Vec<ValidationError>> {
    let mut errs = vec![];
    for (id, hero) in &reg.heroes {
        if !reg.weapons.contains_key(&hero.starting_weapon_primary) {
            errs.push(ValidationError::UnknownWeapon {
                referenced_by: id.clone(),
                missing: hero.starting_weapon_primary.clone(),
            });
        }
    }
    // ... walk every cross-reference
    if errs.is_empty() { Ok(()) } else { Err(errs) }
}
```

Collect all errors before returning — designers want to see every problem at once, not fix-rebuild-fix-rebuild.

**3. CI integration test:** `cargo test data_files_load` parses the full registry from disk and runs validation. Catches broken refs before merge. Use `ron::error::SpannedError` to point at exact `file:line:col` on parse failure.

## Migration phases

The hardcoded values in the current MVP map cleanly to phases. Each phase keeps the engine compiling and tests passing.

### Phase 1: Set up the loader infrastructure

- Add `ron` dependency to `crates/core/Cargo.toml`
- Create `crates/core/src/registry.rs` with the `Registry` struct + load function
- Add the `assets/data/` directory and a minimal `balance.ron`
- Wire `Registry` into `GameEngine::new()` — load once, store on engine
- Add a `data_files_load` integration test
- Add a `hot_reload` cargo feature (off by default)

**Done:** `cargo test` passes, engine carries an empty/minimal Registry, tests still work.

### Phase 2: Migrate balance.rs → balance.ron

All ~93 numbers from `crates/core/src/balance.rs` move to `assets/data/balance.ron`. Update every `use crate::balance::*` consumer to read from `engine.registry.balance.*` instead. Delete `balance.rs` at the end.

```ron
// assets/data/balance.ron
#![enable(implicit_some, unwrap_newtypes, unwrap_variant_newtypes)]
Balance(
    economy: Economy(
        starting_gold: 30,
        starting_wood: 5,
        starting_stone: 3,
        chapter_ticks: [6, 8, 10],
        unused_tick_gold: 3,
        encounter_completion_bonus: 15,
        hero_kill_multiplier: 1.5,
        runner_salary: 3,
        companion_wage: 2,
        leg_maintenance: 5,
    ),
    construction: Construction(
        floor_tick_cost: 2,
        wood_floor_material_cost: 3,
        stone_floor_material_cost: 4,
        wood_panel_hp: 80.0,
        stone_panel_hp: 120.0,
        foundation_hp: 300.0,
        max_floors: 4,
    ),
    combat: Combat(
        battlefield_width: 800.0,
        wave_delay: 5.0,
        breach_duration: 15.0,
        difficulty_budgets: [4, 8, 12],
    ),
    hero: HeroBalance(
        personal_ammo: 99,
        ammo_rack_max: 3,
    ),
)
```

**Done:** `balance.rs` deleted, all 40 tests still pass, hot reload works in dev.

### Phase 3: Migrate entity definitions

Move hardcoded weapon/enemy/building stats out of `engine.rs`. Each entity becomes a RON file with a string ID.

`generate_encounter()` reads enemy stats from the registry instead of constants. `GameState::new()` reads hero starting weapons by ID.

```ron
// assets/data/entities/enemies/grunt.ron
#![enable(implicit_some, unwrap_newtypes)]
EnemyDef(
    id: "enemy.grunt",
    archetype: Grunt,        // Rust enum variant
    hp: 30.0,
    speed: 60.0,
    damage: 8.0,
    attack_rate: 1.0,
    bounty: 3,
    threat: 1,
)
```

**Done:** All inline weapon/enemy numbers in `engine.rs` replaced with registry lookups.

### Phase 4: Migrate chapter data

The hardcoded 3-node map in `GameState::new()` moves to `chapters/chapter_1.ron`. Encounter compositions (which enemies appear at which difficulty) become data instead of code.

```ron
// assets/data/chapters/chapter_1.ron
Chapter(
    id: "chapter.1",
    name: "The Road",
    nodes: [
        MapNode(id: 0, node_type: Combat, column: 0, difficulty: None),
        MapNode(id: 1, node_type: Combat, column: 1, difficulty: 1),
        MapNode(id: 2, node_type: Combat, column: 2, difficulty: 2),
        MapNode(id: 3, node_type: Boss,   column: 3, difficulty: 3),
    ],
    edges: [(0, 1), (1, 2), (2, 3)],
    boss_node: 3,
    enemy_pool: ["enemy.grunt", "enemy.runner"],
)
```

**Done:** No hardcoded map in `engine.rs`. Multiple chapters loaded from disk.

### Phase 5: Migrate UI strings

Frontend strings move out of TSX files into `strings/en.ron`. The bridge gets a `get_string(key)` accessor; React uses a small `t()` helper.

```ron
// assets/data/strings/en.ron
{
    "ui.map.chapter": "Chapter {chapter}",
    "ui.map.gold": "Gold: {amount}",
    "ui.prep.build_wood_floor": "Build Wood Floor ({ticks}t, {wood} wood)",
    "ui.prep.march": "March!",
    "ui.combat.wave": "Wave {current}/{total}",
    "ui.post_combat.title": "Encounter Complete",
    "ui.post_combat.continue": "Continue",
}
```

Simple string-keyed map for v1; no plurals, no interpolation engine — just `{placeholder}` substitution.

**Done:** No user-facing string literals in TSX files. Adding a new locale = new RON file.

### Phase 6 (optional): Visual config

Canvas colors and dimensions in `CombatPage.tsx` could move to a `visual.ron` or stay in CSS tokens (current approach). **Recommend leaving visual config in CSS** — designers editing colors want CSS, not RON. Only move it if there's a specific reason (e.g., theme system).

## What stays as code

These are NOT migrated:

- **Type definitions:** all `enum` and `struct` declarations in `state.rs`, `command.rs`, `snapshot.rs`. These are the schema.
- **Engine constants:** `FIXED_DT` (30hz tick rate), `MAX_TICKS_PER_FRAME` (4), starting `next_id` value. These are simulation invariants, not tuning.
- **System logic:** the rules of how things interact (sweep collision, wave progression, kill bounty calculation) live in code. Data files provide *parameters* for these rules.
- **Bridge accessors:** the `get_*_state()` methods. These are the API contract.

Rule of thumb: **if a designer would want to tweak it, it's data. If a programmer needs to change it, it's code.**

## Open questions

1. **String interpolation library?** Plain `{key}` substitution covers v1. ICU MessageFormat is overkill until we have plurals/genders.
2. **Asset baking pipeline?** A `build.rs` that runs `ron::from_str` at compile time and embeds bincode would shrink WASM size and speed up startup. Defer until binary size becomes a real problem.
3. **Designer-facing editor?** Long-term, a browser-based RON editor with live game preview is the dream. Out of scope for v1.
4. **Versioning/migration?** When fields change shape, old saves break. v1 has no save migration; if we need it, add a `version` field to top-level structs.

## References

- [ron-rs/ron repo & docs](https://github.com/ron-rs/ron)
- [Veloren book — adding weapons](https://book.veloren.net/contributors/guides/adding-weapons/guide.html) — best public example of RON in a Rust game
- [`assets_manager` crate](https://docs.rs/assets_manager) — hot-reload loader
- [implementation-decisions.md §19](foundation/implementation-decisions.md) — determinism contract this plan must respect
