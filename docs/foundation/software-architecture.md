Project: SUPPLY LINE | Software Architecture

> **Status:** v1 and post-v1 vision. Check [implementation-decisions.md](implementation-decisions.md) and [v1-scope.md](v1-scope.md) before implementing any feature described here.

## I. System Overview

Supply Line is a two-layer application: a Rust game core compiled to WASM and a React/TypeScript frontend. They communicate through wasm-bindgen. The Rust side owns all game state and simulation. The React side handles UI rendering, input collection, and frame orchestration.

**React's role is broader than "pure view."** During prep, React is genuinely a view/input layer: it renders snapshots and sends commands. During combat, React also drives the frame loop (requestAnimationFrame), collects per-frame input, calls `bridge.tick(dt)`, and synchronizes React context/state — effectively acting as the application's main loop coordinator. This is the correct architecture for a browser-first game (the browser's rAF is the only reliable frame clock), but it means React is an orchestration layer during combat, not just a renderer. (See [implementation-decisions.md §18](implementation-decisions.md) for the state management decision; see [debugging-bridge.md](/mnt/c/Users/gng/w/logdef/docs/foundation/debugging-bridge.md) for bridge troubleshooting.)

```
┌──────────────────────────────────────────────────────┐
│                      Browser                          │
│                                                      │
│  ┌───────────────────────────────────────────────┐   │
│  │              React (TypeScript)                │   │
│  │                                               │   │
│  │  ┌─────────┐ ┌──────────┐ ┌──────────────┐   │   │
│  │  │  Pages  │ │Organisms │ │  React        │   │   │
│  │  │         │ │          │ │  Context      │   │   │
│  │  │ Prep    │ │ Tower    │ │              │   │   │
│  │  │ Combat  │ │ Editor   │ │ selections   │   │   │
│  │  │ Map     │ │ Map      │ │ panel state  │   │   │
│  │  │ Menu    │ │ HUD      │ │ cached       │   │   │
│  │  │ ...     │ │ ...      │ │ snapshots    │   │   │
│  │  └────┬────┘ └──────────┘ └──────┬───────┘   │   │
│  │       │                          │            │   │
│  │       │    useGameCommand()      │            │   │
│  │       │    useRustSync()         │            │   │
│  │       ▼                          ▼            │   │
│  │  ┌────────────────────────────────────────┐   │   │
│  │  │         wasm-bindgen Bridge            │   │   │
│  │  │                                        │   │   │
│  │  │  send_command(json) → Result           │   │   │
│  │  │  get_hud_state() → HudSnapshot       │   │   │
│  │  │  get_tower_state() → TowerSnapshot     │   │   │
│  │  │  get_journey_state() → JourneySnapshot │   │   │
│  │  │  tick(dt) → void                       │   │   │
│  │  │  new_game(config) → void               │   │   │
│  │  │  save() → json                         │   │   │
│  │  │  load(json) → void                     │   │   │
│  │  └────────────────┬───────────────────────┘   │   │
│  └───────────────────┼───────────────────────────┘   │
│                      │                                │
│  ┌───────────────────┼───────────────────────────┐   │
│  │              Rust (WASM)                       │   │
│  │                      │                         │   │
│  │  ┌──────────────────▼─────────────────────┐   │   │
│  │  │            GameEngine                   │   │   │
│  │  │                                        │   │   │
│  │  │  ┌──────────┐  ┌───────────────────┐   │   │   │
│  │  │  │ Command  │  │    GameState      │   │   │   │
│  │  │  │ Queue    │──▶   (source of      │   │   │   │
│  │  │  │          │  │    truth)         │   │   │   │
│  │  │  └──────────┘  └────────┬──────────┘   │   │   │
│  │  │                         │              │   │   │
│  │  │  ┌──────────────────────▼──────────┐   │   │   │
│  │  │  │          Systems                │   │   │   │
│  │  │  │  (fixed order, deterministic)   │   │   │   │
│  │  │  │                                 │   │   │   │
│  │  │  │  1. production                  │   │   │   │
│  │  │  │  2. transport                   │   │   │   │
│  │  │  │  3. companion_ai                │   │   │   │
│  │  │  │  4. projectiles                 │   │   │   │
│  │  │  │  5. combat                      │   │   │   │
│  │  │  │  6. economy                     │   │   │   │
│  │  │  └─────────────────────────────────┘   │   │   │
│  │  │                                        │   │   │
│  │  │  ┌─────────────────────────────────┐   │   │   │
│  │  │  │         Renderer (wgpu)         │   │   │   │
│  │  │  │  sprite batching, camera,       │   │   │   │
│  │  │  │  effects, canvas output         │   │   │   │
│  │  │  └─────────────────────────────────┘   │   │   │
│  │  │                                        │   │   │
│  │  │  ┌─────────────────────────────────┐   │   │   │
│  │  │  │         Audio (Web Audio API     │   │   │   │
│  │  │  │          via JS/React)          │   │   │   │
│  │  │  │  spatial, layered, reactive     │   │   │   │
│  │  │  └─────────────────────────────────┘   │   │   │
│  │  └────────────────────────────────────────┘   │   │
│  └───────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────┘
```

### Deployment targets

| Target | Rendering | Windowing | Audio | Distribution |
|--------|-----------|-----------|-------|--------------|
| Browser (WASM) | wgpu (WebGPU/WebGL2 fallback) | winit (web) | Web Audio API (JS) | itch.io, custom site |
| Desktop (Tauri) | wgpu (Vulkan/Metal/DX12) | winit (via webview) | Web Audio API (JS) | Steam, standalone |

Same Rust core compiles to both. React UI runs in both (embedded webview for desktop via Tauri, or native browser for web).

---

## II. Core Principles

### 1. GameState is the single source of truth

The entire authoritative game state lives in one serializable struct. Derived data exists elsewhere (RenderSnapshot, UI store cached snapshots, renderer interpolation state), but these are read-only projections — they never feed back into the simulation. Save/load = serialize/deserialize GameState. Replay = record commands that produce GameState. No presentation-layer cache is authoritative.

### 2. Command pattern for all mutations

No system, UI callback, or input handler mutates GameState directly. All changes flow through `GameCommand` variants that are validated before application. This provides:
- **Validation:** illegal actions rejected with error (not enough ticks, invalid floor, wrong phase)
- **Testing:** feed commands to GameState, assert outcomes
- **Replay:** record command stream, replay identically (enables seed sharing, bug reproduction)
- **Undo:** store previous state or inverse command

These are v1 benefits that justify the pattern today. The same pattern would also enable multiplayer command sync if that becomes a product direction — but that's not a v1 requirement, and no architecture decisions should be made solely to pre-pay for it.

### 3. Deterministic simulation

Same commands + same delta-time = same result. Achieved by:
- Fixed system execution order (no scheduler, no parallelism)
- No floating-point non-determinism (use fixed-point or careful f32 with identical operations)
- Ordered iteration over all collections (Vec, not HashMap)
- No external randomness during simulation (RNG seeded and stepped deterministically)

### 4. Separation of concerns: simulation vs. presentation

The simulation (GameState + systems) knows nothing about rendering. It doesn't know about sprites, animations, or screen coordinates. Presentation layers consume read-only projections of GameState, never the state itself.

**The full `RenderSnapshot` never crosses the WASM bridge.** It stays in Rust and is consumed directly by the wgpu renderer. Only compact, typed accessors cross the bridge to React. See [implementation-decisions.md](implementation-decisions.md) §3 for the canonical decision.

**Multiple read models, one source of truth.** The bridge exposes several compact read projections for React:

| Read model | Consumer | When called | What it contains |
|-----------|----------|-------------|-----------------|
| `get_hud_state()` → HudSnapshot | React combat HUD only | Once per sim tick (30hz) | Ammo count, cooldowns, wave counter, companion status, gold. ~20 values. Compact. |
| `get_tower_state()` → TowerSnapshot | React prep UI | After each prep command | Floors, buildings, transport, buffers, panels |
| `get_journey_state()` → JourneySnapshot | React map UI | On map open | Chapter maps, nodes, paths, current position |
| `get_hero_state()` → HeroSnapshot | React hero screen | On hero screen open | Stats, perks, equipment, XP |
| `get_inventory()` → InventorySnapshot | React inventory | On inventory open | Weapons, trinkets, materials |
| `get_merchant_state()` → MerchantSnapshot | React merchant UI | At merchant nodes | Stock, prices, comparison data |
| `get_validation_warnings()` | React prep UI | Before march | Mismatches, missing transport, etc. |

These are all derived from GameState — none is authoritative. The renderer crate depends only on RenderSnapshot types, not core game logic. This means:
- The renderer could be swapped (wgpu → PixiJS → Canvas2D) without touching the simulation
- The simulation could run headless for testing or replay verification
- Prep-phase React never needs the full RenderSnapshot (which contains combat data), and combat never needs the full tower detail view

### 5. Single-player architecture with clean boundaries

The simulation runs locally. All game logic, command processing, and state live in one process. The architecture's clean boundaries (commands in, snapshots out) exist because they make the v1 game better: testable, replayable, debuggable, and modular. They are not pre-payment for multiplayer.

```
v1 (single player):
  Client: simulation + renderer + UI + audio
  (everything local, single process)
```

If multiplayer or co-op ever becomes a product direction, the command/snapshot boundary would make the transition feasible — but that's a side effect of good single-player architecture, not a design goal driving decisions today.

### 6. Hero as entity, not singleton

The hero is a struct on the tower, not a hardcoded global. Hero and companions share targeting, damage, and positioning logic where possible — not because of future co-op, but because shared logic means fewer bugs and simpler systems. The hero does have unique systems (player input, weapon abilities, stat allocation) that companions don't share.

---

## III. GameState — Data Model

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameState {
    pub phase: GamePhase,
    pub tower: Tower,
    pub encounter: Option<EncounterState>,
    pub journey: JourneyState,
    pub meta: MetaState,
    pub economy: EconomyState,
    pub rng: DeterministicRng,
    pub tick: u64,
    pub elapsed: f32,  // total elapsed time in current encounter
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GamePhase {
    MainMenu,
    ClassSelect,
    MapView,
    Travel,       // prep phase — tower stopped, player building
    Encounter,    // combat phase — real-time simulation
    PostCombat,   // summary screen
    Merchant,
    GameOver,
    Victory,
}
```

### Tower

```rust
/// The tower owns the hero and companions because they are persistent across encounters.
/// During combat, the hero and companions are also combat actors (they shoot, take damage,
/// use abilities). Systems read from tower.hero/tower.companions and write combat state
/// (cooldowns, aim direction) back to the same structs. There is no separate "combat entity"
/// for the hero — the tower's hero IS the combat hero. This avoids sync bugs between
/// two representations of the same character, at the cost of the Tower struct containing
/// both persistent data (stats, equipment) and transient combat data (aim_direction, cooldowns).
pub struct Tower {
    pub floors: Vec<Floor>,
    pub foundation: Foundation,
    pub warehouse: Warehouse,
    pub runner_quarters: Vec<RunnerQuarters>,
    pub runners: Vec<Runner>,
    pub width: TowerWidth,
    pub balconies: Vec<Balcony>,
    pub hero: Hero,
    pub companions: Vec<Companion>,
}

pub struct Floor {
    pub index: usize,
    pub building: Option<Building>,
    pub cache: Option<DepotCache>,
    pub panel: WallPanel,
    pub material: FloorMaterial,      // Wood, Stone, Iron
    pub transport_segments: Vec<TransportId>,  // IDs of transport passing through
    pub floor_width_used: f32,        // how much horizontal space is consumed
    pub floor_width_max: f32,         // determined by tower width
}

pub struct Building {
    pub building_type: BuildingType,
    pub tier: ProductionTier,         // T1, T2, T3
    pub output_buffer: ResourceBuffer,
    pub input_buffers: Vec<ResourceBuffer>,  // T2/T3 buildings have input requirements
    pub production_rate: f32,         // crates per minute
    pub operating_cost: u32,          // gold per encounter
    pub is_active: bool,              // false if starved or wrecked
}

pub struct ResourceBuffer {
    pub resource: ResourceType,
    pub current: u32,                 // crate count
    pub max: u32,
}

pub struct WallPanel {
    pub current_hp: f32,
    pub max_hp: f32,
    pub is_breached: bool,
}

pub struct Foundation {
    pub leg_type: LegType,            // Chicken, Spider, Treads, Hover
    pub current_hp: f32,
    pub max_hp: f32,
    pub max_floors: usize,
    pub maintenance_cost: u32,        // gold per encounter
}

pub struct Warehouse {
    pub slots: Vec<ResourceBuffer>,   // one per resource type
    pub capacity_per_slot: u32,
}
```

### Transport

```rust
pub enum Transport {
    BuiltInStairs,                    // always exists, left edge, no width cost
    Ladder { connects: (usize, usize) },
    Dumbwaiter { connects: (usize, usize), cargo: Option<Crate>, state: DumbwaiterState },
    Chute { from_floor: usize, to_floor: usize, diverters: Vec<usize> },
    CargoLift { floor_range: (usize, usize), settings: LiftSettings, cars: Vec<LiftCar> },
    ExpressLift { stops: Vec<usize>, settings: LiftSettings, cars: Vec<LiftCar> },
    Conveyor { floor: usize },
    PneumaticTube { connects: (usize, usize) },
}

pub struct LiftSettings {
    pub car_count: u8,                // 1-3
    pub departure_mode: DepartureMode, // Immediate | Batch
}

pub struct LiftCar {
    pub current_floor: usize,
    pub target_floor: Option<usize>,
    pub cargo: Option<Crate>,
    pub rider: Option<RunnerId>,
    pub state: LiftCarState,          // Idle, Moving, Loading, Unloading
}

pub enum DepartureMode {
    Immediate,  // go as soon as one runner boards
    Batch,      // wait for full load
}
```

### Runners

```rust
pub struct RunnerQuarters {
    pub floor: usize,
    pub capacity: u8,                 // how many runners housed
    pub salary_per_runner: u32,       // gold per encounter
}

pub struct Runner {
    pub id: RunnerId,
    pub quarters_id: QuartersId,
    pub current_floor: usize,
    pub state: RunnerState,
    pub carried: Option<Crate>,
    pub speed: f32,
    pub carry_capacity: u8,           // 1 or 2 (with upgrade)
}

pub enum RunnerState {
    Idle { at_floor: usize },
    Moving { from: usize, to: usize, progress: f32, via: TransportId },
    Loading { at_floor: usize, timer: f32 },
    Unloading { at_floor: usize, timer: f32 },
    Queued { at_transport: TransportId, position_in_queue: usize },
}
```

### Combat Entities

```rust
pub struct Hero {
    pub id: EntityId,
    pub position: BalconyId,
    pub class: HeroClass,
    pub stats: HeroStats,
    pub level: u32,
    pub xp: u32,
    pub perks: Vec<Perk>,
    pub weapon_primary: Weapon,
    pub weapon_secondary: Weapon,
    pub active_weapon: WeaponSlot,    // Primary | Secondary
    pub trinket: Option<Trinket>,
    pub personal_ammo: u32,           // emergency reserve (10-15 shots)
    pub aim_direction: Vec2,
    pub weapon_ability_cooldown: f32,
    pub hero_skill_cooldown: f32,
    pub hero_skill: HeroSkill,
}

pub struct HeroStats {
    pub precision: u32,
    pub draw_power: u32,
    pub tempo: u32,
    pub grit: u32,
    pub salvage: u32,
    pub unspent_points: u32,
}

pub struct Companion {
    pub id: EntityId,
    pub name: String,
    pub position: Option<BalconyId>,  // None if displaced
    pub passive: CompanionPassive,
    pub accuracy: f32,                // 0.0 - 1.0, grows from experience
    pub combat_xp: u32,
    pub weapon: Weapon,
    pub trinket: Option<Trinket>,
    pub target_order: TargetOrder,
    pub fire_discipline: FireDiscipline,
    pub wage: u32,                    // gold per encounter
    pub injured: bool,                // reduced accuracy for N encounters
    pub injury_remaining: u32,        // encounters until healed
}

pub enum CompanionPassive {
    PinningShots,     // hit enemies slowed
    Wall,             // blocks climbers, doesn't shoot
    Mark,             // hit enemies take more damage
    Patch,            // repairs tower exterior between encounters
    Splash,           // AoE shots, 2x ammo
}

pub struct Balcony {
    pub id: BalconyId,
    pub floor: usize,
    pub rack: AmmoRack,
    pub cover_level: CoverLevel,      // Exposed | Partial | Sheltered
    pub occupant: Option<EntityId>,   // hero or companion ID
}

pub struct AmmoRack {
    pub resource: ResourceType,       // auto-configured from occupant's weapon
    pub current: u32,                 // crate count
    pub max: u32,                     // 2-3
    pub destroyed: bool,
}
```

### Encounter State

```rust
pub struct EncounterState {
    pub enemies: Vec<Enemy>,
    pub projectiles: Vec<Projectile>,
    pub loot_on_ground: Vec<LootDrop>,
    pub waves: Vec<Wave>,
    pub current_wave: usize,
    pub wave_state: WaveState,        // Active | Lull { timer: f32 }
    pub terrain_modifier: Option<TerrainModifier>,
    pub interior_raiders: Vec<InteriorRaider>,
}

pub struct Enemy {
    pub id: EnemyId,
    pub archetype: EnemyArchetype,
    pub position: EnemyPosition,
    pub hp: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub state: EnemyState,
    pub stuck_arrows: Vec<StuckProjectile>,  // visual: arrows sticking out
}

pub enum EnemyPosition {
    Ground { x: f32 },                // approaching from right
    Climbing { floor: f32 },          // on tower face, fractional floor
    Flying { x: f32, y: f32 },        // airborne
    AtBase,                           // reached tower base
    AtPanel { floor: usize },         // reached a defended position, attacking panel
}

pub enum EnemyState {
    Approaching,
    Climbing,
    AttackingPanel { floor: usize },
    Dying { timer: f32 },
    Dead,
}

pub enum EnemyArchetype {
    Grunt,
    Runner,             // fast ground
    Armored,            // high HP
    Climber,            // starts climbing immediately
    HovererFlyer,       // ranged attacks on panels
    DiveBomber,         // fast dive attack
    Catapult,           // long range, targets specific floors
    Ram,                // targets foundation
    SiegeTower,         // docks and disgorges climbers
    Sapper,             // targets infrastructure
    Boss(BossType),
}

pub struct Projectile {
    pub id: ProjectileId,
    pub source: EntityId,             // who fired it
    pub weapon_type: WeaponBaseType,
    pub position: Vec2,
    pub velocity: Vec2,
    pub gravity: f32,                 // weapon-dependent
    pub damage: f32,
    pub modifier: Option<WeaponModifier>,
    pub state: ProjectileState,       // Flying | Stuck { in_entity: EnemyId } | OnGround
}

pub struct InteriorRaider {
    pub floor: usize,
    pub timer_remaining: f32,         // 15-20 seconds, then expelled
    pub damage_dealt: Vec<InfrastructureDamage>,
}
```

### Journey State

```rust
pub struct JourneyState {
    pub destination: DestinationId,
    pub current_chapter: usize,       // 1-5
    pub chapters: Vec<ChapterMap>,
    pub current_node: NodeId,
    pub visited_nodes: Vec<NodeId>,
    pub available_companions: Vec<CompanionTemplate>,
}

pub struct ChapterMap {
    pub nodes: Vec<MapNode>,
    pub edges: Vec<MapEdge>,
    pub boss_node: NodeId,
}

pub struct MapNode {
    pub id: NodeId,
    pub node_type: NodeType,
    pub column: usize,                // for layout
    pub difficulty: Option<u8>,       // 1-3 for combat nodes
    pub visited: bool,
    pub rewards: NodeRewards,
}

pub struct MapEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub terrain: TerrainType,
    pub distance: PathDistance,        // Short | Medium | Long → tick bonus
}

pub enum NodeType {
    Combat { modifier: Option<EliteModifier> },
    Elite { modifier: EliteModifier },
    Merchant { archetype: MerchantArchetype },
    Rest,
    Mystery,
    Boss { boss_type: BossType },
    CompanionRecruitment { companion: CompanionTemplate },
    ForgeSite { material: SpecialtyMaterial },
    TollGate { cost: u32 },
}
```

### Economy

```rust
pub struct EconomyState {
    pub gold: u32,
    pub ticks_remaining: u32,
    pub ticks_max: u32,
    pub per_encounter_costs: PerEncounterCosts,
    pub lifetime_gold_earned: u32,    // for meta tracking
}

pub struct PerEncounterCosts {
    pub building_operations: u32,     // sum of all Tier 1 operating costs
    pub runner_salaries: u32,
    pub companion_wages: u32,
    pub foundation_maintenance: u32,
    pub total: u32,
}
```

---

## IV. Command System

### Command enum

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameCommand {
    // --- Prep phase ---
    BuildFloor { floor_index: usize, material: FloorMaterial },
    BuildBuilding { floor_index: usize, building_type: BuildingType },
    BuildBalcony { floor_index: usize },
    BuildTransport { transport: Transport },
    BuildCache { floor_index: usize, resource: ResourceType },
    DestroyBuilding { floor_index: usize },
    RearrangeBuilding { from_floor: usize, to_floor: usize },
    UpgradeBuilding { floor_index: usize },
    UpgradeQuarters { quarters_id: QuartersId },
    WidenTower,
    UpgradeFoundation { leg_type: LegType },
    RepairPanel { floor_index: usize },
    RebuildFloor { floor_index: usize },
    HireCompanion { companion_id: CompanionId, gold_cost: u32 },
    DismissCompanion { companion_id: CompanionId },
    AssignPosition { entity_id: EntityId, balcony_id: BalconyId },
    ConfigureOrders { companion_id: CompanionId, target: TargetOrder, discipline: FireDiscipline },
    ConfigureCache { floor_index: usize, resource: ResourceType },
    ConfigureLift { lift_id: TransportId, settings: LiftSettings },
    EquipWeapon { entity_id: EntityId, weapon_id: WeaponId, slot: WeaponSlot },
    EquipTrinket { entity_id: EntityId, trinket_id: TrinketId },
    AllocateStat { stat: HeroStat },
    ChoosePerk { perk: Perk },
    ChooseMapNode { node_id: NodeId },
    March,  // depart to next node

    // --- Combat phase ---
    AimAt { direction: Vec2 },
    Shoot,
    StopShooting,
    UseWeaponAbility,
    UseHeroSkill,
    SwapWeapon,
    SetPriority { resource: ResourceType },

    // --- Merchant ---
    BuyItem { item_id: ItemId },
    SellItem { item_id: ItemId },
    SellResource { resource: ResourceType, amount: u32 },

    // --- Enchanter ---
    RerollModifier { weapon_id: WeaponId },
    ApplyModifier { weapon_id: WeaponId, modifier: WeaponModifier },

    // --- Meta ---
    NewGame { class: HeroClass, destination: DestinationId },
    SaveGame,
    LoadGame { save_data: String },
}
```

### Validation

```rust
pub fn validate_command(state: &GameState, cmd: &GameCommand) -> Result<(), CommandError> {
    match cmd {
        GameCommand::BuildFloor { floor_index, material } => {
            require_phase(state, GamePhase::Travel)?;
            require_ticks(state, tick_cost_floor(*material))?;
            require_materials(state, material_cost_floor(*material))?;
            require_floor_available(state, *floor_index)?;
            require_foundation_supports(state, *floor_index)?;
            Ok(())
        }
        GameCommand::Shoot => {
            require_phase(state, GamePhase::Encounter)?;
            require_ammo(state)?;  // rack or personal reserve
            Ok(())
        }
        // ... etc
    }
}

pub fn apply_command(state: &mut GameState, cmd: &GameCommand) -> Result<CommandResult, CommandError> {
    validate_command(state, cmd)?;
    match cmd {
        GameCommand::BuildFloor { floor_index, material } => {
            state.economy.ticks_remaining -= tick_cost_floor(*material);
            deduct_materials(&mut state.tower.warehouse, material_cost_floor(*material));
            state.tower.floors.insert(*floor_index, Floor::new(*material));
            Ok(CommandResult::FloorBuilt { floor_index: *floor_index })
        }
        // ... etc
    }
}
```

### Command results

```rust
pub enum CommandResult {
    Ok,
    FloorBuilt { floor_index: usize },
    BuildingPlaced { floor_index: usize, building_type: BuildingType },
    CompanionHired { companion_id: CompanionId },
    ItemBought { item: Item },
    ItemSold { gold_received: u32 },
    ProjectileFired { projectile_id: ProjectileId },
    AbilityActivated { cooldown: f32 },
    WeaponSwapped,
    GameSaved { save_data: String },
    // ... etc
}
```

---

## V. Systems — Simulation Loop

During the encounter phase, systems run every tick in fixed order. Each system is a pure function taking `&mut GameState` and `dt: f32`.

```rust
pub fn game_tick(state: &mut GameState, commands: &[GameCommand], dt: f32) {
    // 1. Process commands
    let results: Vec<Result<CommandResult, CommandError>> = commands
        .iter()
        .map(|cmd| apply_command(state, cmd))
        .collect();

    // 2. Run systems (encounter phase only)
    if state.phase == GamePhase::Encounter {
        system_production(state, dt);
        system_transport(state, dt);
        system_companion_ai(state, dt);
        system_hero(state, dt);
        system_projectiles(state, dt);
        system_enemies(state, dt);
        system_damage(state, dt);
        system_economy(state, dt);
        system_encounter_flow(state, dt);  // wave transitions, lulls, end conditions
        system_interior_raiders(state, dt);
        system_loot(state, dt);
    }

    state.tick += 1;
    state.elapsed += dt;
}
```

### System responsibilities

| System | Reads | Writes | Purpose |
|--------|-------|--------|---------|
| `production` | buildings, input buffers | output buffers | Advance production timers, consume inputs, produce outputs |
| `transport` | runners, transport, buffers, caches, racks | runner state, buffer contents, cache contents, rack contents | Runner AI: pick destination, choose transport, move, load/unload. Lift/dumbwaiter/chute movement. Queue management |
| `companion_ai` | companions, enemies, orders, racks | companion state, racks (ammo consumption) | Target selection per orders, fire/miss rolls (accuracy), ammo deduction, passive ability effects |
| `hero` | hero, aim_direction, weapon | hero state (cooldowns) | Process aim input, apply stat modifiers (precision aim assist, tempo fire rate) |
| `projectiles` | projectiles | projectile positions, enemy HP, rack contents (refund on Double Tap), panel HP (vampiric) | Advance positions with gravity, check collisions, apply damage, apply modifiers (flaming DOT, frost slow, etc.) |
| `enemies` | enemies, tower layout, balcony positions | enemy positions, enemy state | Movement AI: approach, climb, target selection, attack panels, flyer patterns, siege behavior |
| `damage` | enemies at panels, catapult projectiles, rams | panel HP, foundation HP, breaches, interior raider spawns | Apply damage to panels/foundation, trigger breaches, spawn interior raiders |
| `economy` | kill events, loot drops | gold, resources | Award kill bounties, process scavenged drops |
| `encounter_flow` | wave definitions, enemy counts | wave state, lull timers, encounter end flag | Advance waves, trigger lulls, check encounter end (all enemies dead) |
| `interior_raiders` | interior raiders, floor infrastructure | raider timers, infrastructure destruction | Tick raider timers, destroy infrastructure, expel raiders when timer expires |
| `loot` | loot on ground, tower position | loot positions, warehouse contents | Gravity-roll loot toward tower base, deposit arrived loot into warehouse |

### Fixed timestep

Combat uses a fixed timestep of 1/30s (33.33ms). This matches the rate established in tech-performance.md — 30hz gives generous CPU headroom per tick while interpolation keeps visuals smooth at any frame rate.

```rust
const FIXED_DT: f32 = 1.0 / 30.0;  // 33.33ms — matches tech-performance.md

pub struct GameLoop {
    accumulator: f32,
}

impl GameLoop {
    pub fn update(&mut self, real_dt: f32, state: &mut GameState, commands: &[GameCommand]) {
        self.accumulator += real_dt;
        while self.accumulator >= FIXED_DT {
            game_tick(state, commands, FIXED_DT);
            self.accumulator -= FIXED_DT;
        }
        // Remainder in accumulator used for render interpolation
    }
}
```

---

## VI. Render Pipeline

### Snapshot generation

After each tick, the simulation produces a `RenderSnapshot` — a flat, presentation-oriented struct that the renderer and React consume. This decouples simulation from presentation.

```rust
pub struct RenderSnapshot {
    // Tower structure
    pub floors: Vec<FloorVisual>,
    pub transport_visuals: Vec<TransportVisual>,
    pub panel_visuals: Vec<PanelVisual>,

    // Entities
    pub runners: Vec<RunnerVisual>,
    pub enemies: Vec<EnemyVisual>,
    pub projectiles: Vec<ProjectileVisual>,
    pub loot_drops: Vec<LootVisual>,

    // Fighters
    pub hero: HeroVisual,
    pub companions: Vec<CompanionVisual>,
    pub balconies: Vec<BalconyVisual>,

    // Status
    pub warehouse_levels: Vec<(ResourceType, u32, u32)>,  // (type, current, max)
    pub wave_info: WaveVisual,
    pub gold: u32,
    pub foundation_hp: (f32, f32),

    // Effects
    pub particles: Vec<ParticleVisual>,
    pub damage_events: Vec<DamageEvent>,
    pub sound_events: Vec<SoundEvent>,
}

pub struct EnemyVisual {
    pub position: Vec2,       // screen-space
    pub sprite: SpriteId,
    pub animation_frame: u32,
    pub hp_ratio: f32,        // 0.0-1.0 for health bar
    pub stuck_arrows: Vec<Vec2>,  // positions of stuck projectiles
    pub status_effects: Vec<StatusEffect>,  // burning, frozen, etc.
}
```

### wgpu renderer

A thin custom 2D renderer:

```rust
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    sprite_pipeline: wgpu::RenderPipeline,
    sprite_batcher: SpriteBatcher,
    camera: Camera2D,
    asset_store: AssetStore,  // loaded textures/sprites
}

impl Renderer {
    pub fn render(&mut self, snapshot: &RenderSnapshot) {
        self.sprite_batcher.clear();

        // Layer 0: Background (landscape, sky)
        self.render_background(snapshot);

        // Layer 1: Tower interior (floors, buildings, transport, runners)
        self.render_tower_interior(snapshot);

        // Layer 2: Tower exterior (panels, balconies, racks)
        self.render_tower_exterior(snapshot);

        // Layer 3: Fighters (hero, companions on balconies)
        self.render_fighters(snapshot);

        // Layer 4: Enemies (ground, climbing, flying)
        self.render_enemies(snapshot);

        // Layer 5: Projectiles
        self.render_projectiles(snapshot);

        // Layer 6: Effects (particles, damage numbers, loot)
        self.render_effects(snapshot);

        self.sprite_batcher.flush(&self.device, &self.queue);
    }
}
```

The renderer knows about sprites and screen coordinates. It does NOT know about game rules. All visual decisions (which sprite, which animation frame, which color) are made during snapshot generation, not during rendering.

---

## VII. React ↔ Rust Bridge

> For diagnosing bridge issues (serialization mismatches, snapshot drift, tick problems, performance), see [debugging-bridge.md](/mnt/c/Users/gng/w/logdef/docs/foundation/debugging-bridge.md).

### Exposed API (wasm-bindgen)

```rust
#[wasm_bindgen]
pub struct GameBridge {
    engine: GameEngine,
}

#[wasm_bindgen]
impl GameBridge {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self;

    /// Send a command (JSON-serialized GameCommand). Returns JSON result or error.
    pub fn send_command(&mut self, command_json: &str) -> String;

    /// Advance simulation by dt seconds. Call on requestAnimationFrame during combat.
    pub fn tick(&mut self, dt: f32);

    /// Get compact HUD state for React combat overlay (~20 values: ammo,
    /// cooldowns, wave, companion status, gold). The full RenderSnapshot
    /// stays in Rust for the wgpu renderer — it never crosses this bridge.
    pub fn get_hud_state(&self) -> String;

    /// Get sound events from the last tick. Consumed by JS AudioManager.
    pub fn get_sound_events(&self) -> String;

    /// Get tower state for React prep UI. JSON.
    pub fn get_tower_state(&self) -> String;

    /// Get journey/map state for React map UI. JSON.
    pub fn get_journey_state(&self) -> String;

    /// Get hero state (stats, perks, equipment) for React hero UI. JSON.
    pub fn get_hero_state(&self) -> String;

    /// Get inventory (all weapons, trinkets, materials). JSON.
    pub fn get_inventory(&self) -> String;

    /// Get merchant stock (during merchant node). JSON.
    pub fn get_merchant_state(&self) -> String;

    /// Get current phase.
    pub fn get_phase(&self) -> String;

    /// Get validation warnings for current tower state. JSON array of strings.
    pub fn get_validation_warnings(&self) -> String;

    /// Save game to JSON.
    pub fn save(&self) -> String;

    /// Load game from JSON.
    pub fn load(&mut self, save_json: &str) -> String;

    /// Start new game.
    pub fn new_game(&mut self, config_json: &str);
}
```

### React hooks

```typescript
// Hook: send commands to Rust
function useGameCommand() {
    const bridge = useGameBridge();  // access to GameBridge wasm instance
    return useCallback((command: GameCommand) => {
        const result = JSON.parse(bridge.send_command(JSON.stringify(command)));
        if (result.error) {
            // show error notification
        }
        // refresh relevant snapshot
        return result;
    }, [bridge]);
}

// Hook: sync Rust state into React context/state
function useRustSync() {
    const bridge = useGameBridge();
    const [phase] = useGamePhase(); // from React context

    useEffect(() => {
        if (phase === 'encounter') {
            // During combat: sync every frame via requestAnimationFrame
            let raf: number;
            let lastTime = performance.now();
            const loop = (time: number) => {
                const dt = (time - lastTime) / 1000;
                lastTime = time;
                bridge.tick(dt);
                const snapshot = JSON.parse(bridge.get_hud_state());
                setCombatSnapshot(snapshot); // React state setter
                raf = requestAnimationFrame(loop);
            };
            raf = requestAnimationFrame(loop);
            return () => cancelAnimationFrame(raf);
        } else {
            // During prep: sync on demand (after each command)
            const tower = JSON.parse(bridge.get_tower_state());
            const journey = JSON.parse(bridge.get_journey_state());
            setTowerSnapshot(tower);
            setJourneySnapshot(journey);
        }
    }, [phase, bridge]);
}
```

### Data flow during combat

```
1. Browser fires requestAnimationFrame
2. React hook calculates dt
3. React collects input (mouse position, click events, key presses)
4. React sends combat commands: AimAt, Shoot, UseAbility, etc.
5. bridge.send_command() for each → Rust validates & applies
6. bridge.tick(dt) → Rust runs all systems + renders game canvas via wgpu
   (RenderSnapshot generated and consumed INSIDE Rust — never crosses bridge)
7. bridge.get_hud_state() → compact HudSnapshot (~20 values) crosses to React
8. bridge.get_sound_events() → sound events cross to JS AudioManager
9. React updates context/state with HUD snapshot
10. React HUD components re-render (ammo, cooldowns, wave)
```

### Data flow during prep

```
1. Player clicks "Build Fletcher on Floor 3" in React UI
2. React calls bridge.send_command({ BuildBuilding: { floor: 3, type: "Fletcher" } })
3. Rust validates: correct phase? enough ticks? floor available? materials?
4. If valid: apply, return success. If invalid: return error with reason.
5. React calls bridge.get_tower_state() to refresh tower display
6. React updates state, TowerEditor re-renders
7. Tick counter decrements
```

---

## VIII. Audio Architecture

**Web Audio API only.** No Rust audio dependency. Audio lives entirely in the JS/React layer.

```
Rust Simulation → SoundEvent list + AudioState (in RenderSnapshot)
    │
    ▼
JS AudioManager (Web Audio API) → audio output
    Works in browser natively + Tauri/Electron desktop
```

The simulation produces `SoundEvent` data and `AudioState` snapshots as part of the RenderSnapshot. A TypeScript `AudioManager` class consumes these and plays sounds via the Web Audio API. React `useAudioManager` hook manages the lifecycle.

**Why Web Audio everywhere:** the game runs in a web context on all platforms (browser natively, desktop via Tauri/Electron webview). Web Audio API works identically in both. No need for Rust audio crates (no kira, no rodio, no cpal). Fewer dependencies, simpler build, one audio codebase.

Sound events are generated during simulation (in the RenderSnapshot) and consumed by the JS audio manager. The Rust simulation never calls audio APIs directly. See [audio-direction.md](audio-direction.md) for full implementation details, SoundEvent enum, and AudioState struct.

---

## IX. Save/Load

The entire `GameState` is serializable via serde. Save = `serde_json::to_string(&state)`. Load = `serde_json::from_str(json)`.

```rust
pub fn save(state: &GameState) -> String {
    serde_json::to_string(state).expect("GameState must be serializable")
}

pub fn load(json: &str) -> Result<GameState, SaveError> {
    let state: GameState = serde_json::from_str(json)?;
    validate_save_version(&state)?;  // handle schema migration
    Ok(state)
}
```

Save files are JSON (human-readable, debuggable). For desktop: stored in user data directory. For browser: stored in IndexedDB (not localStorage — localStorage has a ~5MB limit across the origin, and save files with telemetry history may grow beyond that. IndexedDB has no practical size limit and supports structured data natively). Autosave between encounters. The bridge exposes `save()` and `load()` as JSON strings; the storage backend (IndexedDB vs. filesystem) is a JS/Tauri concern, not a Rust concern.

Save versioning: `GameState` includes a `save_version: u32` field. When the schema changes, a migration function transforms old saves to new format.

---

## X. Testing Strategy

### Unit tests (Rust)

Each system is a pure function. Test by constructing a GameState, running the system, asserting the result.

```rust
#[test]
fn test_runner_delivers_to_cache() {
    let mut state = create_test_state_with_fletcher_and_cache();
    // Fletcher has full output buffer, cache is empty
    system_transport(&mut state, 1.0);
    // Runner should have picked up from fletcher, delivered to cache
    assert_eq!(state.tower.floors[3].cache.unwrap().current, 1);
}
```

### Integration tests (Rust)

Feed a sequence of commands, run N ticks, assert game state outcomes.

```rust
#[test]
fn test_full_encounter_flow() {
    let mut state = create_standard_chapter1_state();
    state.phase = GamePhase::Encounter;
    // Run 300 ticks (10 seconds at 30hz)
    for _ in 0..300 {
        game_tick(&mut state, &[], FIXED_DT);
    }
    // All grunts should be dead if hero was shooting
    assert!(state.encounter.unwrap().enemies.iter().all(|e| e.state == EnemyState::Dead));
}
```

### Determinism tests

Run the same command sequence twice, assert identical final states.

```rust
#[test]
fn test_determinism() {
    let commands = generate_random_command_sequence(100);
    let state_a = run_with_commands(commands.clone());
    let state_b = run_with_commands(commands);
    assert_eq!(state_a, state_b);
}
```

### React component tests

Use React Testing Library. Mock the wasm bridge. Test that components render correctly from snapshots and send correct commands on interaction.

---

## XI. Project Structure

```
supply-line/
├── Cargo.toml                    # Rust workspace root
├── crates/
│   ├── core/                     # Game simulation (no rendering, no IO)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── state.rs          # GameState and all sub-structs
│   │       ├── command.rs        # GameCommand, validate, apply
│   │       ├── systems/
│   │       │   ├── mod.rs
│   │       │   ├── production.rs
│   │       │   ├── transport.rs
│   │       │   ├── companion_ai.rs
│   │       │   ├── hero.rs
│   │       │   ├── projectiles.rs
│   │       │   ├── enemies.rs
│   │       │   ├── damage.rs
│   │       │   ├── economy.rs
│   │       │   ├── encounter_flow.rs
│   │       │   ├── interior_raiders.rs
│   │       │   └── loot.rs
│   │       ├── tower/
│   │       │   ├── mod.rs
│   │       │   ├── floor.rs
│   │       │   ├── building.rs
│   │       │   ├── transport.rs
│   │       │   ├── warehouse.rs
│   │       │   └── runner.rs
│   │       ├── combat/
│   │       │   ├── mod.rs
│   │       │   ├── enemy.rs
│   │       │   ├── projectile.rs
│   │       │   └── companion.rs
│   │       ├── journey/
│   │       │   ├── mod.rs
│   │       │   ├── map.rs
│   │       │   ├── encounter_gen.rs
│   │       │   └── terrain.rs
│   │       ├── economy/
│   │       │   ├── mod.rs
│   │       │   ├── production.rs
│   │       │   └── merchant.rs
│   │       ├── equipment/
│   │       │   ├── mod.rs
│   │       │   ├── weapon.rs
│   │       │   ├── trinket.rs
│   │       │   ├── modifier.rs
│   │       │   └── loot_table.rs
│   │       ├── hero/
│   │       │   ├── mod.rs
│   │       │   ├── stats.rs
│   │       │   ├── perks.rs
│   │       │   └── classes.rs
│   │       ├── meta.rs
│   │       └── snapshot.rs       # RenderSnapshot generation
│   │
│   ├── bridge/                   # wasm-bindgen layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs            # GameBridge, exposed API
│   │
│   └── renderer/                 # wgpu rendering
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── renderer.rs       # main render loop
│           ├── sprites.rs        # sprite batching
│           ├── camera.rs         # 2D camera, viewport
│           ├── assets.rs         # texture/sprite loading
│           └── effects.rs        # particles, screen shake
│
├── web/                          # React frontend
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── eslint.config.js
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── bridge/
│       │   ├── index.ts          # wasm-bindgen bridge interface
│       │   └── types.ts          # TypeScript types matching Rust structs
│       ├── components/           # Atomic design system (see design-system.md)
│       │   ├── atoms/
│       │   ├── molecules/
│       │   ├── organisms/
│       │   └── pages/
│       ├── hooks/
│       │   └── useGameCommand.ts
│       ├── audio/
│       │   └── AudioManager.ts   # Web Audio API, consumes SoundEvents
│       ├── context/              # React context (UI state only, see impl-decisions §18)
│       └── styles/
│           ├── tokens.css        # CSS custom properties (design tokens)
│           └── global.css        # reset, font imports, base styles
│
├── assets/                       # Game assets (sprites, sounds)
│   ├── sprites/
│   ├── sounds/
│   └── fonts/
│
└── docs/
    └── foundation/
        ├── art-direction.md
        ├── design-system.md
        └── software-architecture.md   # this document
```

### Crate separation

| Crate | Dependencies | Purpose |
|-------|-------------|---------|
| `core` | serde, glam, rand | Pure game simulation. No IO, no rendering, no platform code. Testable in isolation. |
| `bridge` | core, wasm-bindgen, serde_json | Thin wasm-bindgen wrapper. Converts JSON ↔ Rust types. Calls core. |
| `renderer` | core (for RenderSnapshot), wgpu, winit, glam | 2D rendering. Reads snapshots, draws frames. |

The `core` crate is the heart. It compiles to native (for tests, potential desktop headless server) and to WASM (via `bridge`). It has zero platform dependencies. Everything it needs (random numbers, time, etc.) is passed in from outside.
