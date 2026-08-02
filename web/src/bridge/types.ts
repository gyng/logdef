/** Mirrors Rust GamePhase enum. */
export type GamePhase =
  | "MainMenu"
  | "ClassSelect"
  | "MapView"
  | "Travel"
  | "Encounter"
  | "PostCombat"
  | "Merchant"
  | "GameOver"
  | "Victory";

export type HeroClass = "archer" | "engineer" | "commander";

export type WeaponSlot = "Primary" | "Secondary";

export interface HudSnapshot {
  ammo_primary: number;
  ammo_secondary: number;
  personal_ammo: number;
  weapon_ability_cooldown: number;
  hero_skill_cooldown: number;
  active_weapon: WeaponSlot;
  current_wave: number;
  total_waves: number;
  gold: number;
  companion_statuses: CompanionHudStatus[];
  hero_hp_fraction: number;
  tower_hp_fraction: number;
  enemies_remaining: number;
}

export interface CompanionHudStatus {
  name: string;
  ammo: number;
  injured: boolean;
  displaced: boolean;
}

export interface TowerSnapshot {
  floors: FloorSnapshot[];
  warehouse: WarehouseSnapshot;
  runners: RunnerSnapshot[];
  balconies: BalconySnapshot[];
  runner_quarters: RunnerQuartersSnapshot[];
  companions: CompanionSnapshot[];
  transports: TransportInstanceSnapshot[];
  width: string;
}

export interface TransportInstanceSnapshot {
  id: number;
  kind: "Stairs" | "Ladder" | "Dumbwaiter" | "Chute";
  low_floor: number;
  high_floor: number;
  speed_mul: number;
  capacity: number;
  occupancy: number;
  direction: "Both" | "DownOnly" | "UpOnly";
  slot: number;
}

export interface CompanionSnapshot {
  id: number;
  name: string;
  position: number | null;
  passive: string;
  accuracy: number;
  combat_xp: number;
  weapon: { sub_type: string; damage: number; fire_rate: number; base_type: string };
  trinket: { id: string; name: string } | null;
  target_order: string;
  fire_discipline: string;
  wage: number;
  injured: boolean;
  injury_remaining: number;
}

export interface FloorSnapshot {
  index: number;
  building: BuildingSnapshot | null;
  cache: DepotCacheSnapshot | null;
  panel_hp_fraction: number;
  material: string;
  slots: number;
}

export interface BuildingSnapshot {
  building_type: string;
  tier: string;
  output_buffer: ResourceBuffer;
  input_buffers: ResourceBuffer[];
  production_rate: number;
  operating_cost: number;
  is_active: boolean;
  production_progress: number;
  slot: number;
  width_slots: number;
}

export interface DepotCacheSnapshot {
  slots: ResourceBuffer[];
  slot: number;
}

export interface WarehouseSnapshot {
  slots: ResourceBuffer[];
  capacity_per_slot: number;
}

export interface BalconySnapshot {
  id: number;
  floor: number;
  rack: AmmoRackSnapshot;
  cover_level: string;
  occupant: number | null;
}

export interface AmmoRackSnapshot {
  resource: string;
  current: number;
  max: number;
  destroyed: boolean;
}

export interface RunnerQuartersSnapshot {
  floor: number;
  capacity: number;
  salary_per_runner: number;
}

/** Mirrors the Rust Runner struct (state.rs). */
export interface RunnerSnapshot {
  id: number;
  quarters_id: number;
  current_floor: number;
  state: RunnerState;
  carried: { resource: string } | null;
  speed: number;
  carry_capacity: number;
  task: RunnerTask | null;
  current_slot: number;
}

export type RunnerState =
  | { Idle: { at_floor: number } }
  | {
      Moving: {
        from: number;
        to: number;
        progress: number;
        via: number;
        from_slot: number;
        to_slot: number;
      };
    }
  | { Loading: { at_floor: number; timer: number } }
  | { Unloading: { at_floor: number; timer: number } }
  | { Queued: { at_transport: number; position_in_queue: number } };

export interface RunnerTask {
  pickup_floor: number;
  dropoff_floor: number;
  resource: string;
  destination:
    | { Inbox: { floor: number } }
    | { Rack: { balcony: number } }
    | { Cache: { floor: number } }
    | "Warehouse";
  pickup_slot: number;
  dropoff_slot: number;
}

export interface ResourceBuffer {
  resource: string;
  current: number;
  max: number;
}

export interface JourneySnapshot {
  current_chapter: number;
  chapters: ChapterMap[];
  current_node: number;
  visited_nodes: number[];
}

export interface ChapterMap {
  nodes: MapNode[];
  edges: MapEdge[];
  boss_node: number;
}

export interface MapNode {
  id: number;
  node_type: string;
  column: number;
  difficulty: number | null;
  visited: boolean;
}

export interface MapEdge {
  from: number;
  to: number;
}

export interface EncounterSnapshot {
  enemies: EnemySnapshot[];
  projectiles: ProjectileSnapshot[];
  current_wave: number;
  total_waves: number;
  enemies_remaining: number;
}

export interface EnemySnapshot {
  id: number;
  archetype: string;
  x: number;
  hp_fraction: number;
}

export interface ProjectileSnapshot {
  id: number;
  x: number;
  y: number;
}

export interface SoundEvent {
  type: string;
  [key: string]: unknown;
}

export interface ValidationWarning {
  severity: "Info" | "Warning" | "Critical";
  message: string;
}

export interface HeroStats {
  precision: number;
  draw_power: number;
  tempo: number;
  grit: number;
  salvage: number;
  unspent_points: number;
}

export interface HeroSnapshot {
  class: string;
  level: number;
  xp: number;
  stats: HeroStats;
  perks: unknown[];
  weapon_primary: { sub_type: string; damage: number; fire_rate: number };
  weapon_secondary: { sub_type: string; damage: number; fire_rate: number };
  trinket: { id: string; name: string } | null;
  position: number;
}

export interface MerchantItem {
  index: number;
  id: string;
  name: string;
  description: string;
  price: number;
}

export interface MerchantSnapshot {
  items: MerchantItem[];
  gold: number;
}

export interface PerfMetricSnapshot {
  calls: number;
  last_ms: number;
  avg_ms: number;
  max_ms: number;
}

export interface SimPerfSnapshot {
  ticks_last_frame: number;
  frame_sim: PerfMetricSnapshot;
  tick_total: PerfMetricSnapshot;
  systems: {
    production: PerfMetricSnapshot;
    transport: PerfMetricSnapshot;
    companion_ai: PerfMetricSnapshot;
    projectiles: PerfMetricSnapshot;
    combat: PerfMetricSnapshot;
    economy: PerfMetricSnapshot;
  };
}

export interface BridgePerfSnapshot {
  send_command: PerfMetricSnapshot;
  tick: PerfMetricSnapshot;
  interpolation_alpha: PerfMetricSnapshot;
  get_phase: PerfMetricSnapshot;
  get_hud_state: PerfMetricSnapshot;
  get_tower_state: PerfMetricSnapshot;
  get_journey_state: PerfMetricSnapshot;
  get_economy_state: PerfMetricSnapshot;
  get_gold: PerfMetricSnapshot;
  get_hero_state: PerfMetricSnapshot;
  get_merchant_state: PerfMetricSnapshot;
  get_encounter_state: PerfMetricSnapshot;
  get_validation_warnings: PerfMetricSnapshot;
  get_perf_state: PerfMetricSnapshot;
  save: PerfMetricSnapshot;
  load: PerfMetricSnapshot;
}

export interface EconomySnapshot {
  gold: number;
  materials: ResourceBuffer[];
  ticks_remaining: number;
}

export interface DrillSnapshot {
  seconds_remaining: number;
  seconds_total: number;
  deliveries_during_drill: number;
}
