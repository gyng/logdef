use serde::{Deserialize, Serialize};

use crate::state::*;
use crate::types::*;

/// All mutations to GameState flow through GameCommand variants.
/// Commands are validated before application — illegal actions are
/// rejected with an error, never silently ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameCommand {
    // -- Game lifecycle --
    NewGame { seed: u64, class: HeroClass },
    SaveGame,
    LoadGame { data: String },

    // -- Map navigation --
    SelectNode { node: NodeId },
    March,

    // -- Prep / tower building --
    BuildFloor { material: FloorMaterial },
    PlaceBuilding { floor: usize, building_type: BuildingType },
    RemoveBuilding { floor: usize },
    PlaceCache { floor: usize },
    RemoveCache { floor: usize },
    InstallTransport { transport: Transport },
    RemoveTransport { id: TransportId },
    HireRunner,
    AssignCompanion { companion: EntityId, balcony: BalconyId },
    UnassignCompanion { companion: EntityId },
    SetCompanionOrders { companion: EntityId, target_order: TargetOrder, fire_discipline: FireDiscipline },

    // -- Hero management --
    EquipWeapon { slot: WeaponSlot, weapon: Weapon },
    EquipTrinket { trinket: Trinket },
    AllocateStat { stat: StatType },
    SelectPerk { perk_id: String },

    // -- Combat input --
    AimAt { direction: Vec2 },
    Fire,
    SwitchWeapon,
    UseWeaponAbility,
    UseHeroSkill,
    MoveToBalcony { balcony: BalconyId },

    // -- Merchant --
    BuyItem { item_index: usize },
    SellItem { item_index: usize },
    RerollModifier { weapon_slot: WeaponSlot },

    // -- Post-combat --
    CollectLoot { index: usize },
    ContinueJourney,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatType {
    Precision,
    DrawPower,
    Tempo,
    Grit,
    Salvage,
}

/// Result of processing a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandResult {
    Ok,
    Error(CommandError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandError {
    WrongPhase { expected: String, actual: String },
    InsufficientTicks { needed: u32, available: u32 },
    InsufficientGold { needed: u32, available: u32 },
    InsufficientMaterials { resource: ResourceType, needed: u32, available: u32 },
    InvalidFloor { index: usize },
    FloorOccupied { index: usize },
    InvalidTarget,
    NotUnlocked { feature: String },
    InvalidCommand { reason: String },
}
