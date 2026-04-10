use serde::{Deserialize, Serialize};

use crate::rng::DeterministicRng;
use crate::types::*;

// ---------------------------------------------------------------------------
// Top-level GameState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub phase: GamePhase,
    pub tower: Tower,
    pub encounter: Option<EncounterState>,
    pub journey: JourneyState,
    pub meta: MetaState,
    pub economy: EconomyState,
    pub rng: DeterministicRng,
    pub tick: u64,
    pub elapsed: Scalar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePhase {
    MainMenu,
    ClassSelect,
    MapView,
    Travel,
    Encounter,
    PostCombat,
    Merchant,
    GameOver,
    Victory,
}

// ---------------------------------------------------------------------------
// Tower
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TowerWidth {
    Narrow,
    Standard,
    Wide,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Floor {
    pub index: usize,
    pub building: Option<Building>,
    pub cache: Option<DepotCache>,
    pub panel: WallPanel,
    pub material: FloorMaterial,
    pub transport_segments: Vec<TransportId>,
    pub floor_width_used: Scalar,
    pub floor_width_max: Scalar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FloorMaterial {
    Wood,
    Stone,
    Iron,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub building_type: BuildingType,
    pub tier: ProductionTier,
    pub output_buffer: ResourceBuffer,
    pub input_buffers: Vec<ResourceBuffer>,
    pub production_rate: Scalar,
    pub operating_cost: u32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingType {
    Fletcher,
    Quarry,
    Smelter,
    Alchemist,
    Enchanter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductionTier {
    T1,
    T2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBuffer {
    pub resource: ResourceType,
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    Arrows,
    Bolts,
    Mana,
    Thrown,
    Stone,
    Iron,
    Gold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepotCache {
    pub slots: Vec<ResourceBuffer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallPanel {
    pub current_hp: Scalar,
    pub max_hp: Scalar,
    pub is_breached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Foundation {
    pub leg_type: LegType,
    pub current_hp: Scalar,
    pub max_hp: Scalar,
    pub max_floors: usize,
    pub maintenance_cost: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegType {
    Chicken,
    Spider,
    Treads,
    Hover,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warehouse {
    pub slots: Vec<ResourceBuffer>,
    pub capacity_per_slot: u32,
}

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transport {
    BuiltInStairs,
    Ladder {
        connects: (usize, usize),
    },
    Dumbwaiter {
        connects: (usize, usize),
        cargo: Option<Crate>,
        state: DumbwaiterState,
    },
    Chute {
        from_floor: usize,
        to_floor: usize,
        diverters: Vec<usize>,
    },
    CargoLift {
        floor_range: (usize, usize),
        settings: LiftSettings,
        cars: Vec<LiftCar>,
    },
    ExpressLift {
        stops: Vec<usize>,
        settings: LiftSettings,
        cars: Vec<LiftCar>,
    },
    Conveyor {
        floor: usize,
    },
    PneumaticTube {
        connects: (usize, usize),
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crate {
    pub resource: ResourceType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DumbwaiterState {
    Idle,
    Moving,
    Loading,
    Unloading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiftSettings {
    pub car_count: u8,
    pub departure_mode: DepartureMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiftCar {
    pub current_floor: usize,
    pub target_floor: Option<usize>,
    pub cargo: Option<Crate>,
    pub rider: Option<RunnerId>,
    pub state: LiftCarState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiftCarState {
    Idle,
    Moving,
    Loading,
    Unloading,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepartureMode {
    Immediate,
    Batch,
}

// ---------------------------------------------------------------------------
// Runners
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerQuarters {
    pub floor: usize,
    pub capacity: u8,
    pub salary_per_runner: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runner {
    pub id: RunnerId,
    pub quarters_id: QuartersId,
    pub current_floor: usize,
    pub state: RunnerState,
    pub carried: Option<Crate>,
    pub speed: Scalar,
    pub carry_capacity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunnerState {
    Idle { at_floor: usize },
    Moving { from: usize, to: usize, progress: Scalar, via: TransportId },
    Loading { at_floor: usize, timer: Scalar },
    Unloading { at_floor: usize, timer: Scalar },
    Queued { at_transport: TransportId, position_in_queue: usize },
}

// ---------------------------------------------------------------------------
// Combat entities
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub active_weapon: WeaponSlot,
    pub trinket: Option<Trinket>,
    pub personal_ammo: u32,
    pub aim_direction: Vec2,
    pub weapon_ability_cooldown: Scalar,
    pub hero_skill_cooldown: Scalar,
    pub hero_skill: HeroSkill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeroClass {
    Archer,
    Engineer,
    Commander,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroStats {
    pub precision: u32,
    pub draw_power: u32,
    pub tempo: u32,
    pub grit: u32,
    pub salvage: u32,
    pub unspent_points: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponSlot {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeroSkill {
    Focus,
    Overclock,
    Rally,
}

// Placeholder types — will be fleshed out per registries.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Perk {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    pub base_type: WeaponBaseType,
    pub sub_type: String,
    pub damage: Scalar,
    pub fire_rate: Scalar,
    pub modifiers: Vec<WeaponModifier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponBaseType {
    Bow,
    Crossbow,
    Staff,
    Thrown,
    Melee,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponModifier {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trinket {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Companion {
    pub id: EntityId,
    pub name: String,
    pub position: Option<BalconyId>,
    pub passive: CompanionPassive,
    pub accuracy: Scalar,
    pub combat_xp: u32,
    pub weapon: Weapon,
    pub trinket: Option<Trinket>,
    pub target_order: TargetOrder,
    pub fire_discipline: FireDiscipline,
    pub wage: u32,
    pub injured: bool,
    pub injury_remaining: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompanionPassive {
    PinningShots,
    Wall,
    Mark,
    Patch,
    Splash,
    FieldMedic,
    Suppression,
    Anchor,
    JuryRig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetOrder {
    Closest,
    Strongest,
    Weakest,
    Climbers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FireDiscipline {
    AtWill,
    Conserve,
    HoldFire,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balcony {
    pub id: BalconyId,
    pub floor: usize,
    pub rack: AmmoRack,
    pub cover_level: CoverLevel,
    pub occupant: Option<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmoRack {
    pub resource: ResourceType,
    pub current: u32,
    pub max: u32,
    pub destroyed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverLevel {
    Exposed,
    Partial,
    Sheltered,
}

// ---------------------------------------------------------------------------
// Encounter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterState {
    pub enemies: Vec<Enemy>,
    pub projectiles: Vec<Projectile>,
    pub loot_on_ground: Vec<LootDrop>,
    pub waves: Vec<Wave>,
    pub current_wave: usize,
    pub wave_state: WaveState,
    pub terrain_modifier: Option<TerrainModifier>,
    pub interior_raiders: Vec<InteriorRaider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enemy {
    pub id: EnemyId,
    pub archetype: EnemyArchetype,
    pub position: EnemyPosition,
    pub hp: Scalar,
    pub max_hp: Scalar,
    pub speed: Scalar,
    pub state: EnemyState,
    pub stuck_arrows: Vec<StuckProjectile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyArchetype {
    Grunt,
    Runner,
    Armored,
    Climber,
    HovererFlyer,
    DiveBomber,
    Catapult,
    Ram,
    SiegeTower,
    Sapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnemyPosition {
    Ground { x: Scalar },
    Climbing { floor: Scalar },
    Flying { x: Scalar, y: Scalar },
    AtBase,
    AtPanel { floor: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyState {
    Approaching,
    Climbing,
    AttackingPanel,
    Dying,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projectile {
    pub id: ProjectileId,
    pub source: EntityId,
    pub weapon_type: WeaponBaseType,
    pub position: Vec2,
    pub velocity: Vec2,
    pub gravity: Scalar,
    pub damage: Scalar,
    pub modifier: Option<WeaponModifier>,
    pub state: ProjectileState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectileState {
    Flying,
    Stuck { in_entity: EnemyId },
    OnGround,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StuckProjectile {
    pub weapon_type: WeaponBaseType,
    pub offset: Vec2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootDrop {
    pub position: Vec2,
    pub contents: LootContents,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LootContents {
    Gold(u32),
    Material(ResourceType, u32),
    Weapon(Weapon),
    Trinket(Trinket),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave {
    pub enemies: Vec<WaveEnemy>,
    pub spawn_delay: Scalar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveEnemy {
    pub archetype: EnemyArchetype,
    pub spawn_time: Scalar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaveState {
    Active,
    Lull { timer: Scalar },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainModifier {
    Fog,
    Rain,
    Night,
    Wind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteriorRaider {
    pub floor: usize,
    pub timer_remaining: Scalar,
    pub damage_dealt: Vec<InfrastructureDamage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureDamage {
    pub target: String,
    pub amount: Scalar,
}

// ---------------------------------------------------------------------------
// Journey
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyState {
    pub destination: DestinationId,
    pub current_chapter: usize,
    pub chapters: Vec<ChapterMap>,
    pub current_node: NodeId,
    pub visited_nodes: Vec<NodeId>,
    pub available_companions: Vec<CompanionTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionTemplate {
    pub name: String,
    pub passive: CompanionPassive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMap {
    pub nodes: Vec<MapNode>,
    pub edges: Vec<MapEdge>,
    pub boss_node: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapNode {
    pub id: NodeId,
    pub node_type: NodeType,
    pub column: usize,
    pub difficulty: Option<u8>,
    pub visited: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Combat,
    EliteCombat,
    Merchant,
    Rest,
    Mystery,
    Boss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapEdge {
    pub from: NodeId,
    pub to: NodeId,
}

// ---------------------------------------------------------------------------
// Meta & Economy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaState {
    pub seed: u64,
    pub run_number: u32,
    pub unlocked_classes: Vec<HeroClass>,
    pub unlocked_weapons: Vec<WeaponBaseType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyState {
    pub gold: u32,
    pub materials: Vec<ResourceBuffer>,
    pub ticks_remaining: u32,
    pub ticks_per_prep: u32,
}
