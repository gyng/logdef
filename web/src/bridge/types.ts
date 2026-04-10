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

export interface ValidationWarning {
  severity: "Info" | "Warning" | "Critical";
  message: string;
}

export interface SoundEvent {
  type: string;
  [key: string]: unknown;
}
