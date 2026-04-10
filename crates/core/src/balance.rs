//! Balance constants — all tuning numbers for the MVP.
//! Source of truth: docs/foundation/balance-config.md
//! These will eventually be loaded from a runtime config file.

use crate::types::Scalar;

// ── Economy ─────────────────────────────────────────────
pub const STARTING_GOLD: u32 = 30;
pub const STARTING_WOOD: u32 = 5;
pub const STARTING_STONE: u32 = 3;

pub const CH1_TICKS: u32 = 6;
pub const CH2_TICKS: u32 = 8;
pub const CH3_TICKS: u32 = 10;

pub const UNUSED_TICK_GOLD: u32 = 3;
pub const ENCOUNTER_COMPLETION_BONUS: u32 = 15;
pub const HERO_KILL_MULTIPLIER: Scalar = 1.5;

pub const RUNNER_SALARY: u32 = 3;
pub const COMPANION_WAGE: u32 = 2;
pub const LEG_MAINTENANCE: u32 = 5;

// ── Construction ────────────────────────────────────────
pub const FLOOR_TICK_COST: u32 = 2;
pub const WOOD_FLOOR_MATERIAL_COST: u32 = 3;
pub const STONE_FLOOR_MATERIAL_COST: u32 = 4;

pub const WOOD_PANEL_HP: Scalar = 80.0;
pub const STONE_PANEL_HP: Scalar = 120.0;
pub const FOUNDATION_HP: Scalar = 300.0;
pub const MAX_FLOORS: usize = 4;

pub const BUILDING_TICK_COST: u32 = 2;
pub const FLETCHER_WOOD_COST: u32 = 2;
pub const FORGE_STONE_COST: u32 = 2;

// ── Production (crates per minute) ──────────────────────
pub const FLETCHER_RATE: Scalar = 6.0;
pub const FLETCHER_OPERATING_COST: u32 = 5;
pub const FLETCHER_BUFFER_MAX: u32 = 4;

pub const FORGE_RATE: Scalar = 5.0;
pub const FORGE_OPERATING_COST: u32 = 6;
pub const FORGE_BUFFER_MAX: u32 = 4;

// ── Weapons ─────────────────────────────────────────────
pub const SHORTBOW_DAMAGE: Scalar = 15.0;
pub const SHORTBOW_FIRE_RATE: Scalar = 1.8;
pub const SHORTBOW_RANGE: Scalar = 250.0;
pub const SHORTBOW_PROJ_SPEED: Scalar = 300.0;

pub const LONGBOW_DAMAGE: Scalar = 35.0;
pub const LONGBOW_FIRE_RATE: Scalar = 0.8;
pub const LONGBOW_RANGE: Scalar = 400.0;
pub const LONGBOW_PROJ_SPEED: Scalar = 350.0;

pub const DAGGER_DAMAGE: Scalar = 8.0;
pub const DAGGER_FIRE_RATE: Scalar = 3.5;
pub const DAGGER_RANGE: Scalar = 30.0;

// ── Enemies ─────────────────────────────────────────────
pub const GRUNT_HP: Scalar = 30.0;
pub const GRUNT_SPEED: Scalar = 60.0;
pub const GRUNT_DAMAGE: Scalar = 8.0;
pub const GRUNT_ATTACK_RATE: Scalar = 1.0;
pub const GRUNT_BOUNTY: u32 = 3;
pub const GRUNT_THREAT: u32 = 1;

pub const RUNNER_ENEMY_HP: Scalar = 20.0;
pub const RUNNER_ENEMY_SPEED: Scalar = 120.0;
pub const RUNNER_ENEMY_DAMAGE: Scalar = 5.0;
pub const RUNNER_ENEMY_ATTACK_RATE: Scalar = 1.2;
pub const RUNNER_ENEMY_BOUNTY: u32 = 4;
pub const RUNNER_ENEMY_THREAT: u32 = 2;

pub const ARMORED_HP: Scalar = 150.0;
pub const ARMORED_SPEED: Scalar = 30.0;
pub const ARMORED_DAMAGE: Scalar = 15.0;
pub const ARMORED_ATTACK_RATE: Scalar = 0.8;
pub const ARMORED_BOUNTY: u32 = 10;
pub const ARMORED_THREAT: u32 = 4;

// ── Combat ──────────────────────────────────────────────
pub const BATTLEFIELD_WIDTH: Scalar = 800.0;
pub const WAVE_DELAY: Scalar = 5.0;
pub const BREACH_DURATION: Scalar = 15.0;

// ── Threat budgets by difficulty ────────────────────────
// MVP: smaller budgets so encounters fit the available ammo + click-fire pace.
// These will scale up once companions/full transport are implemented.
pub const DIFFICULTY_1_BUDGET: u32 = 4;
pub const DIFFICULTY_2_BUDGET: u32 = 8;
pub const DIFFICULTY_3_BUDGET: u32 = 12;

// ── Hero ────────────────────────────────────────────────
pub const HERO_PERSONAL_AMMO: u32 = 99;
pub const AMMO_RACK_MAX: u32 = 3;
