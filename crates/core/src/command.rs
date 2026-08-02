use serde::{Deserialize, Serialize};

use crate::state::*;
use crate::types::*;

/// All mutations to GameState flow through GameCommand variants.
/// Commands are validated before application — illegal actions are
/// rejected with an error, never silently ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameCommand {
    // -- Game lifecycle --
    NewGame {
        seed: u64,
        class: HeroClass,
    },
    SaveGame,
    LoadGame {
        data: String,
    },

    // -- Map navigation --
    SelectNode {
        node: NodeId,
    },
    March,

    // -- Prep / tower building --
    BuildFloor {
        material: FloorMaterial,
    },
    PlaceBuilding {
        floor: usize,
        slot: u8,
        building_type: BuildingType,
    },
    RemoveBuilding {
        floor: usize,
    },
    PlaceCache {
        floor: usize,
        slot: u8,
    },
    RemoveCache {
        floor: usize,
    },
    InstallTransport {
        transport: Transport,
    },
    RemoveTransport {
        id: TransportId,
    },
    /// Build a ladder spanning two adjacent floors at a specific slot
    /// column. Faster to install than dumbwaiters but slower per ride.
    BuildLadder {
        low_floor: usize,
        slot: u8,
    },
    /// Build an autonomous dumbwaiter at a slot column spanning two
    /// adjacent floors.
    BuildDumbwaiter {
        low_floor: usize,
        slot: u8,
    },
    HireRunner,
    /// Spend 1 tick to run a synthetic logistics drill in prep mode.
    /// Production + transport tick for `seconds`, with the hero rack
    /// draining every second to model combat demand. XP reward at end.
    RunDrill {
        seconds: u32,
    },
    AssignCompanion {
        companion: EntityId,
        balcony: BalconyId,
    },
    UnassignCompanion {
        companion: EntityId,
    },
    SetCompanionOrders {
        companion: EntityId,
        target_order: TargetOrder,
        fire_discipline: FireDiscipline,
    },

    // -- Hero management --
    EquipWeapon {
        slot: WeaponSlot,
        weapon: Weapon,
    },
    EquipTrinket {
        trinket: Trinket,
    },
    AllocateStat {
        stat: StatType,
    },
    SelectPerk {
        perk_id: String,
    },

    // -- Combat input --
    AimAt {
        direction: Vec2,
    },
    SetDrawPower {
        power: Scalar,
    },
    Fire,
    SwitchWeapon,
    UseWeaponAbility,
    UseHeroSkill,
    MoveToBalcony {
        balcony: BalconyId,
    },

    // -- Merchant --
    BuyItem {
        item_index: usize,
    },
    SellItem {
        item_index: usize,
    },
    RerollModifier {
        weapon_slot: WeaponSlot,
    },

    // -- Post-combat --
    CollectLoot {
        index: usize,
    },
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
    WrongPhase {
        expected: String,
        actual: String,
    },
    InsufficientTicks {
        needed: u32,
        available: u32,
    },
    InsufficientGold {
        needed: u32,
        available: u32,
    },
    InsufficientMaterials {
        resource: ResourceType,
        needed: u32,
        available: u32,
    },
    InvalidFloor {
        index: usize,
    },
    FloorOccupied {
        index: usize,
    },
    InvalidTarget,
    NotUnlocked {
        feature: String,
    },
    InvalidCommand {
        reason: String,
    },
}
