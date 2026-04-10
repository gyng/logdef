use serde::{Deserialize, Serialize};

use crate::state::*;
use crate::types::*;

/// Compact HUD state sent to React during combat (30hz).
/// ~20 values. This is NOT the full RenderSnapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudSnapshot {
    pub ammo_primary: u32,
    pub ammo_secondary: u32,
    pub personal_ammo: u32,
    pub weapon_ability_cooldown: Scalar,
    pub hero_skill_cooldown: Scalar,
    pub active_weapon: WeaponSlot,
    pub current_wave: usize,
    pub total_waves: usize,
    pub wave_state: WaveState,
    pub gold: u32,
    pub companion_statuses: Vec<CompanionHudStatus>,
    pub hero_hp_fraction: Scalar,
    pub tower_hp_fraction: Scalar,
    pub enemies_remaining: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionHudStatus {
    pub name: String,
    pub ammo: u32,
    pub injured: bool,
    pub displaced: bool,
}

/// Tower state sent to React during prep.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerSnapshot {
    pub floors: Vec<FloorSnapshot>,
    pub warehouse: Warehouse,
    pub runners: Vec<Runner>,
    pub width: TowerWidth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorSnapshot {
    pub index: usize,
    pub building: Option<Building>,
    pub cache: Option<DepotCache>,
    pub panel_hp_fraction: Scalar,
    pub material: FloorMaterial,
}

/// Journey/map state sent to React.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneySnapshot {
    pub current_chapter: usize,
    pub chapters: Vec<ChapterMap>,
    pub current_node: NodeId,
    pub visited_nodes: Vec<NodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomySnapshot {
    pub gold: u32,
    pub materials: Vec<ResourceBuffer>,
    pub ticks_remaining: u32,
}

/// Hero state sent to React.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroSnapshot {
    pub class: HeroClass,
    pub level: u32,
    pub xp: u32,
    pub stats: HeroStats,
    pub perks: Vec<Perk>,
    pub weapon_primary: Weapon,
    pub weapon_secondary: Weapon,
    pub trinket: Option<Trinket>,
}

/// Validation warnings shown before marching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub severity: WarningSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningSeverity {
    Info,
    Warning,
    Critical,
}

/// Compact encounter state for Canvas2D rendering.
/// Sent to React during combat — positions, HP fractions, wave info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterSnapshot {
    pub enemies: Vec<EnemySnapshot>,
    pub projectiles: Vec<ProjectileSnapshot>,
    pub current_wave: usize,
    pub total_waves: usize,
    pub enemies_remaining: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerfMetricSnapshot {
    pub calls: u64,
    pub last_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemPerfSnapshot {
    pub production: PerfMetricSnapshot,
    pub transport: PerfMetricSnapshot,
    pub companion_ai: PerfMetricSnapshot,
    pub projectiles: PerfMetricSnapshot,
    pub combat: PerfMetricSnapshot,
    pub economy: PerfMetricSnapshot,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimPerfSnapshot {
    pub ticks_last_frame: u32,
    pub frame_sim: PerfMetricSnapshot,
    pub tick_total: PerfMetricSnapshot,
    pub systems: SystemPerfSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemySnapshot {
    pub id: EnemyId,
    pub archetype: EnemyArchetype,
    pub x: Scalar,
    pub hp_fraction: Scalar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectileSnapshot {
    pub id: ProjectileId,
    pub x: Scalar,
    pub y: Scalar,
}

/// Sound events produced by the simulation for the JS AudioManager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SoundEvent {
    WeaponFire {
        weapon_type: WeaponBaseType,
        position: Vec2,
    },
    ProjectileHit {
        weapon_type: WeaponBaseType,
        position: Vec2,
    },
    ProjectileMiss {
        position: Vec2,
    },
    EnemyDeath {
        archetype: EnemyArchetype,
        position: Vec2,
    },
    EnemySpawn {
        archetype: EnemyArchetype,
    },
    PanelHit {
        floor: usize,
    },
    PanelBreach {
        floor: usize,
    },
    BuildingProduce {
        building_type: BuildingType,
    },
    RunnerPickup,
    RunnerDeliver,
    WaveStart {
        wave_number: usize,
    },
    WaveComplete,
    EncounterVictory,
    EncounterDefeat,
}
