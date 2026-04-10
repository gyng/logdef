# Plan 001 — Data-Driven Config

> **Status:** Plan, not yet implemented.
> **Revised:** addresses Codex review feedback on loader strategy, system config access, UI string flow, and scope framing.

## Goal

Move **balance numbers, content definitions, and chapter/level data** out of Rust source into editable RON files. Move **localized UI copy** out of TSX components into a JSON locale bundle owned by the frontend. Designers and developer-in-design-mode can tweak values without recompiling. Hot reload in native dev. Strict validation in CI.

This is *not* a goal to remove every literal from the codebase. System logic, type definitions, simulation invariants, debug labels, and dev-only diagnostic strings stay in code. See [What stays as code](#what-stays-as-code) for the precise line.

## Format choices

| Surface | Format | Owner | Why |
|---|---|---|---|
| Balance, entities, chapters | **RON** | Rust core | Serde-native, real enums, comments, trailing commas. `kind: Bow(draw_time: 0.8)` round-trips to a Rust enum. |
| Localized UI copy | **JSON** | Frontend | JS owns translation; JSON parses with built-in `JSON.parse`, no extra crate, no bridge round-trip per key. One file per locale. |

Two formats is intentional. RON is great for typed game data Rust deserializes; JSON is the right call for a flat key-value bundle JavaScript consumes directly.

Enable these RON extensions in every hand-authored game-data file:

```ron
#![enable(implicit_some)]
#![enable(unwrap_newtypes)]
#![enable(unwrap_variant_newtypes)]
#![enable(explicit_struct_names)]
```

Reference: [Veloren](https://book.veloren.net) is the canonical Rust game shipping RON at scale; this plan borrows their patterns.

## Directory layout

```
assets/data/                 # Rust-owned game data (RON, embedded into WASM)
  balance.ron                # all tuning numbers (one big file)
  entities/
    heroes/{archer,engineer,commander}.ron
    weapons/
      bow/{shortbow,longbow}.ron
      crossbow/hand_crossbow.ron
      melee/dagger.ron
    enemies/{grunt,runner,armored}.ron
    buildings/{fletcher,forge}.ron
  chapters/
    chapter_1.ron            # nodes, edges, encounter compositions
    chapter_2.ron
    chapter_3.ron

web/public/strings/          # Frontend-owned locale bundles (JSON, fetched at runtime)
  en.json
  # future: ja.json, de.json
```

**Why this layout:**
- `balance.ron` is one flat file because numbers form a coherent whole.
- Entity content is one-file-per-entity — cleaner diffs, fewer merge conflicts as content grows.
- Chapters are separate from entities because levels are *compositions* of entity references.
- Strings live under `web/public/` so Vite serves them statically. They never cross the WASM bridge.

## ID scheme

Use **enum variants** for fixed sets and **string ID newtypes** for instances:

| Thing | Type | Example |
|---|---|---|
| Weapon base type (5 variants, fixed) | Rust enum | `WeaponBaseType::Bow` |
| Weapon instance (open-ended) | String ID newtype | `WeaponId("weapon.bow.shortbow")` |
| Enemy archetype (fixed set) | Rust enum | `EnemyArchetype::Grunt` |
| Companion instance (10 in v1) | String ID newtype | `CompanionId("companion.kael")` |
| Hero class (3 variants, fixed) | Rust enum | `HeroClass::Archer` |
| Map node ID (per-chapter) | u32 newtype | `NodeId(1)` |

Newtype-wrap every string ID:

```rust
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeaponId(pub String);
```

Cross-references between RON files use these string IDs:

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

This was underspecified in the previous draft. Here is the canonical strategy.

### Single source of file bytes: `include_dir!`

The full `assets/data/` tree is embedded into the core crate at compile time using the [`include_dir`](https://docs.rs/include_dir) macro:

```rust
use include_dir::{include_dir, Dir};

static EMBEDDED_DATA: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../assets/data");
```

`include_dir!` walks the directory at build time and embeds every file. The resulting `Dir` supports recursive iteration and path-based lookup, both at runtime, with no I/O. This works identically on native, WASM, and in tests — it is the **same code path everywhere**.

### Loader trait abstracts the byte source

```rust
pub trait DataSource {
    /// Read raw bytes for a path relative to assets/data/.
    fn read(&self, path: &str) -> Result<Cow<'_, [u8]>, LoadError>;
    /// List all files under a directory prefix, recursively, in sorted order.
    fn list(&self, prefix: &str) -> Vec<String>;
}
```

Two implementations:

```rust
/// Production: zero-I/O, baked into the binary.
pub struct EmbeddedSource;
impl DataSource for EmbeddedSource { /* delegates to EMBEDDED_DATA */ }

/// Native dev only: reads from disk every call so file edits are live.
#[cfg(all(feature = "hot_reload", not(target_arch = "wasm32")))]
pub struct DiskSource { root: PathBuf }
```

The `Registry::load(source: &dyn DataSource)` function uses the trait — it doesn't know or care which implementation it has. For tests and WASM, pass `&EmbeddedSource`. For native dev with `--features hot_reload`, pass `&DiskSource::new("assets/data")`.

### Discovery without a manifest file

`Registry::load()` walks the tree using `source.list()`:

```rust
pub fn load(source: &dyn DataSource) -> Result<Registry, Vec<LoadError>> {
    let mut errors = vec![];
    let balance = parse_one::<Balance>(source, "balance.ron", &mut errors);
    let weapons = parse_dir::<WeaponDef>(source, "entities/weapons", &mut errors);
    let enemies = parse_dir::<EnemyDef>(source, "entities/enemies", &mut errors);
    let heroes  = parse_dir::<HeroDef>(source, "entities/heroes", &mut errors);
    let buildings = parse_dir::<BuildingDef>(source, "entities/buildings", &mut errors);
    let chapters = parse_dir::<ChapterDef>(source, "chapters", &mut errors);

    let registry = Registry { balance, weapons, enemies, heroes, buildings, chapters };
    if errors.is_empty() { validate(&registry).map(|_| registry) } else { Err(errors) }
}
```

`parse_dir` recursively lists `*.ron` files under a prefix, parses each into the target type, keys it by the `id` field, and inserts into a `BTreeMap`. Sorted iteration is enforced by `BTreeMap` and by `list()` returning sorted paths — load order is deterministic.

### Hot reload as a separate layer (native dev only)

Hot reload is **not** a property of the loader. It's a separate layer on top:

```rust
#[cfg(all(feature = "hot_reload", not(target_arch = "wasm32")))]
pub struct HotReloadingRegistry {
    current: ArcSwap<Registry>,
    source: DiskSource,
    _watcher: notify::RecommendedWatcher,
}
```

A `notify` watcher on `assets/data/` calls `Registry::load(&disk_source)` on every change. On success, `current.store(Arc::new(new))`. On failure, the previous Arc stays in place — designers don't crash the game by saving a syntax error.

The game loop reads `registry.load()` once per tick boundary, never mid-tick. Single Arc clone, no locks.

### Why not `assets_manager` or `rust-embed`?

`assets_manager` is the more game-specific solution but ships its own opinions about reload semantics, asset references, and async loading we don't need. `rust-embed` overlaps with `include_dir` but couples embedding and disk-fallback into a single feature flag instead of letting us layer them. `include_dir` + manual `notify` is ~50 lines of code we control fully and can debug end-to-end.

## How systems access the registry

This was the second underspecified piece. Here is the explicit answer.

**Approach: thread `&Registry` through the simulation tick as an explicit parameter.** This is option A from the design discussion — the most boring and most explicit choice.

### Current system signature

```rust
// crates/core/src/systems/combat.rs (today)
pub fn run(state: &mut GameState, dt: Scalar, sounds: &mut Vec<SoundEvent>) { ... }
```

### Target system signature

```rust
pub fn run(
    state: &mut GameState,
    registry: &Registry,
    dt: Scalar,
    sounds: &mut Vec<SoundEvent>,
) { ... }
```

### Tick orchestrator change

`crates/core/src/systems.rs::tick()` already takes `&mut GameState`. It will be extended to take `&Registry`:

```rust
pub fn tick(state: &mut GameState, registry: &Registry, dt: Scalar) -> TickResult {
    // ... pass registry into each system::run call
}
```

`GameEngine::tick()` holds the `Arc<Registry>` and passes `&*self.registry` into `systems::tick`.

### Why not store the registry on `GameState`?

`GameState` is fully serialized for save/load. The registry is not state — it's data. Adding it to `GameState` would mean either serializing it (huge save files; coupled save schema to data schema) or marking it `#[serde(skip)]` and re-attaching on load (works but is implicit). Threading the parameter is more honest about the dependency and keeps the save schema small.

### Why not a `SimContext` struct?

Bundling `&mut GameState`, `&Registry`, `dt`, and `&mut Vec<SoundEvent>` into a single `SimContext<'a>` is cleaner ergonomically but adds an abstraction we don't need yet. If the parameter list grows past what's comfortable, refactoring to a context struct is a localized change. **Defer until needed.**

### What about callers outside `systems::tick`?

`apply_command()` in `engine.rs` is the other place that needs registry data (for `BuildFloor` material costs, `PlaceBuilding` building stats, `March`'s encounter generation). It already runs on `&mut self` of `GameEngine`, so it can read `self.registry` directly without a parameter change.

**Rule:** systems take `&Registry` as a parameter; engine methods read `self.registry`.

## Registry layout

```rust
pub struct Registry {
    pub balance: Balance,
    pub heroes: BTreeMap<HeroId, HeroDef>,
    pub weapons: BTreeMap<WeaponId, WeaponDef>,
    pub enemies: BTreeMap<EnemyDefId, EnemyDef>,
    pub buildings: BTreeMap<BuildingDefId, BuildingDef>,
    pub chapters: Vec<ChapterDef>,
}
```

Note: **no `strings` field**. Localized copy is owned by the frontend and never enters the Rust registry.

`BTreeMap` everywhere, never `HashMap`. Iteration order is part of the determinism contract and is already enforced by clippy `disallowed_types`.

## Determinism contract

This is **load-bearing** for the deterministic-simulation requirement (see [implementation-decisions.md §19](../foundation/implementation-decisions.md)):

1. **`Registry` is immutable during a tick.** Loaded once into `Arc<Registry>`, passed by reference into systems. Never mutated.
2. **Hot reload swaps the Arc between ticks, never during.** The game loop checks for a pending swap at tick boundaries only.
3. **Same RON files + same seed + same commands = same result.** A snapshot test serializes the post-load `Registry` to a stable canonical form and compares against a checked-in golden file. Any unintentional content drift fails CI.
4. **`BTreeMap` everywhere**, never `HashMap` (already enforced by clippy `disallowed_types`).
5. **Sorted file enumeration.** `DataSource::list()` must return paths in lexicographic order so iteration order is independent of OS filesystem semantics.

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

**3. CI integration test:** `cargo test data_files_load` parses the full registry via `EmbeddedSource` (so the test runs on the same bytes as production) and runs validation. Catches broken refs before merge. Use `ron::error::SpannedError` to point at exact `file:line:col` on parse failure.

## Frontend strings: a separate, simpler story

UI copy doesn't need RON, doesn't need bridge calls, and doesn't need Rust to know about it.

### Layout

```
web/public/strings/
  en.json
```

```json
{
  "ui.map.chapter": "Chapter {chapter}",
  "ui.map.gold": "Gold: {amount}",
  "ui.prep.build_wood_floor": "Build Wood Floor ({ticks}t, {wood} wood)",
  "ui.prep.march": "March!",
  "ui.combat.wave": "Wave {current}/{total}",
  "ui.post_combat.title": "Encounter Complete",
  "ui.post_combat.continue": "Continue"
}
```

### Loading and use

```ts
// web/src/i18n/index.ts
let strings: Record<string, string> = {};

export async function loadLocale(locale: string) {
  const res = await fetch(`/strings/${locale}.json`);
  strings = await res.json();
}

export function t(key: string, params: Record<string, string | number> = {}) {
  const template = strings[key] ?? key; // missing key returns the key itself
  return template.replace(/\{(\w+)\}/g, (_, p) => String(params[p] ?? `{${p}}`));
}
```

`loadLocale("en")` runs once at app boot, before the bridge initializes. `t()` is synchronous after that.

**Zero bridge involvement.** Rust never sees a string. Adding a locale = adding a JSON file + a language switcher in the UI. No recompile, no WASM rebuild.

### What this replaces

The previous draft proposed `bridge.get_string(key)` per call. That's the wrong shape — it's a chatty WASM crossing for data the frontend should own end-to-end. The architecture has been moving toward compact coarse-grained accessors; per-key string lookups are the opposite.

## What gets migrated, what stays as code

### Migrated (out of Rust into RON)

- All ~93 numbers in `crates/core/src/balance.rs`
- Hardcoded enemy stats in `engine.rs::generate_encounter()`
- Hardcoded weapon stats in `engine.rs::Fire` handler and `GameState::new()`
- The hardcoded 3-node chapter map in `GameState::new()`
- Encounter composition rules (which enemies appear at which difficulty)
- Building production rates and operating costs
- Hero starting stats per class

### Migrated (out of TSX into JSON)

- All shipped, user-facing UI copy in `web/src/components/pages/*.tsx`

### Stays as code (intentional)

- **Type definitions:** all `enum` and `struct` declarations in `state.rs`, `command.rs`, `snapshot.rs`. These are the schema.
- **Engine constants:** `FIXED_DT` (30hz tick rate), `MAX_TICKS_PER_FRAME` (4), starting `next_id` value. These are simulation invariants, not tuning.
- **System logic:** the rules of how things interact (sweep collision, wave progression, kill bounty calculation) live in code. Data files provide *parameters* for these rules.
- **Bridge accessors:** the `get_*_state()` methods. These are the API contract.
- **Debug labels, dev-only diagnostic strings, console messages, error messages thrown to developers.** These are not localized copy.
- **Visual config (canvas colors, sizes):** stays in CSS tokens. Designers editing colors want CSS, not RON.

**Rule of thumb:** if a designer would want to tweak it, it's data. If a programmer needs to change it, it's code. If only a developer ever sees it, it's code.

## Migration phases

Each phase keeps the engine compiling and tests passing.

### Phase 1: Loader infrastructure (no migration yet)

- Add `ron` and `include_dir` deps to `crates/core/Cargo.toml`
- Add `notify` and `arc-swap` behind a `hot_reload` feature
- Create `crates/core/src/registry/` module: `Registry`, `DataSource` trait, `EmbeddedSource`, optional `DiskSource` + `HotReloadingRegistry`
- Create `assets/data/` directory with a minimal `balance.ron` containing one stub field
- Wire `Arc<Registry>` into `GameEngine::new()` via `Registry::load(&EmbeddedSource)`
- Add `data_files_load` integration test that loads the registry and asserts validation passes
- Extend `systems::tick()` signature to take `&Registry` (passing through but not yet read)

**Done:** `cargo test` passes, engine carries a real (mostly empty) Registry, `&Registry` is threaded through every system, hot reload feature compiles on native.

### Phase 2: Migrate `balance.rs` → `balance.ron`

All ~93 numbers from `crates/core/src/balance.rs` move to `assets/data/balance.ron`. Each consumer:

- In systems: read from the `&Registry` parameter (`registry.balance.combat.wave_delay`)
- In `engine.rs`: read from `self.registry.balance.*`
- Delete `crates/core/src/balance.rs` at the end

**Done:** `balance.rs` deleted, all 40 tests still pass, hot reload demonstrably works in dev.

### Phase 3: Entity definitions

Move hardcoded weapon/enemy/building stats out of `engine.rs`. Each entity becomes a RON file with a string ID.

`generate_encounter()` reads enemy stats by archetype from the registry. `GameState::new()` reads hero starting weapons by ID.

```ron
// assets/data/entities/enemies/grunt.ron
EnemyDef(
    id: "enemy.grunt",
    archetype: Grunt,
    hp: 30.0,
    speed: 60.0,
    damage: 8.0,
    attack_rate: 1.0,
    bounty: 3,
    threat: 1,
)
```

**Done:** No inline weapon/enemy numbers in `engine.rs`; lookups go through the registry.

### Phase 4: Chapter data

The hardcoded 3-node map in `GameState::new()` moves to `chapters/chapter_1.ron`. Encounter compositions become data.

```ron
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

### Phase 5: Frontend localization

- Create `web/public/strings/en.json` with every shipped UI string
- Create `web/src/i18n/index.ts` with `loadLocale()` and `t()`
- Call `loadLocale("en")` in `App.tsx` before bridge init
- Replace shipped UI copy in TSX files with `t("ui.map.chapter", { chapter })` etc.

**Done:** All shipped, user-facing UI copy lives in `en.json`. Adding a locale = new JSON file. Debug labels and dev-only strings stay inline — no rule against developer-facing literals.

### Phase 6 (deferred): Visual config

Canvas colors and dimensions in `CombatPage.tsx` stay in CSS tokens. Only revisit if a theme system or runtime palette swap becomes a requirement.

## Open questions

1. **Snapshot test format for Registry.** Probably bincode or stable RON-pretty. Decide during Phase 1.
2. **Asset baking pipeline.** A `build.rs` step that pre-validates RON at compile time would catch errors before runtime. The current plan validates at first load, which is fine for v1 because tests run the loader. Reconsider if WASM bundle size becomes a concern.
3. **String interpolation for plurals/genders.** Plain `{key}` substitution covers v1. ICU MessageFormat is overkill until we ship to a language that needs it.
4. **Designer-facing editor.** Long-term, a browser-based RON editor with live game preview is the dream. Out of scope for v1.
5. **Save versioning.** When entity field shapes change, old saves break. v1 has no save migration; if we need it, add a `version` field to `GameState` and gate `load()` on it.

## References

- [ron-rs/ron](https://github.com/ron-rs/ron) — format docs
- [include_dir crate](https://docs.rs/include_dir) — compile-time directory embedding
- [notify crate](https://docs.rs/notify) — file watcher for hot reload
- [arc-swap crate](https://docs.rs/arc-swap) — lock-free Arc swapping
- [Veloren book — adding weapons](https://book.veloren.net/contributors/guides/adding-weapons/guide.html) — best public RON-in-Rust-game example
- [implementation-decisions.md §19](../foundation/implementation-decisions.md) — determinism contract this plan must respect
