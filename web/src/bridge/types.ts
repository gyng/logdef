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
  width: string;
}

export interface FloorSnapshot {
  index: number;
  building: BuildingSnapshot | null;
  panel_hp_fraction: number;
  material: string;
}

export interface BuildingSnapshot {
  building_type: string;
  tier: string;
  output_buffer: ResourceBuffer;
  production_rate: number;
  operating_cost: number;
  is_active: boolean;
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

export interface EconomySnapshot {
  gold: number;
  materials: ResourceBuffer[];
  ticks_remaining: number;
}
