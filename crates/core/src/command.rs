//! Commands — the only way `GameState` changes.
//!
//! Nothing in the simulation, no input handler, and no snapshot builder
//! mutates state directly. Every change is a `GameCommand`, validated
//! before it applies, and rejected with a typed error rather than
//! silently ignored. That is what makes the game replayable: a run is
//! its seed plus this list.

use serde::{Deserialize, Serialize};

use crate::ids::{FloorIdx, SlotIdx};
use crate::state::SimSpeed;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameCommand {
    /// Pause or set the time multiplier. Recorded in replays; has no
    /// effect on the content of any individual tick.
    SetSpeed { speed: SimSpeed },

    /// Add a floor on top and extend the stairs to reach it. Paid for
    /// out of storeroom stock.
    BuildFloor,

    /// Place a room from the content pack.
    PlaceRoom {
        /// Authored string ID, e.g. `"room.mill"`. Interned on apply.
        room: String,
        floor: FloorIdx,
        slot: SlotIdx,
    },

    /// Remove whatever room covers this slot. No refund — you committed
    /// the materials when you built it.
    RemoveRoom { floor: FloorIdx, slot: SlotIdx },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandResult {
    Ok,
    Error(CommandError),
}

impl CommandResult {
    #[must_use]
    pub const fn is_ok(&self) -> bool {
        matches!(self, CommandResult::Ok)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandError {
    /// The content pack has no room with that ID.
    UnknownRoom { room: String },
    /// Floor index past the top of the tower.
    NoSuchFloor { floor: FloorIdx },
    /// The slot range runs off the edge of the floor.
    SlotOutOfRange {
        slot: SlotIdx,
        width: u8,
        floor_slots: u8,
    },
    /// A room or shaft column already occupies part of the range.
    SlotOccupied { floor: FloorIdx, slot: SlotIdx },
    /// This room may only be placed on lower floors.
    FloorTooHigh { floor: FloorIdx, max_floor: u8 },
    /// Only one of these may exist in a tower.
    AlreadyPlaced { room: String },
    /// Not enough on the shelves. The chain pays for the tower.
    InsufficientStock {
        item: String,
        needed: i64,
        available: i64,
    },
    /// The tower is as tall as its legs will carry.
    FloorLimit { max_floors: u8 },
    /// Nothing occupies that slot.
    NoRoomThere { floor: FloorIdx, slot: SlotIdx },
    /// The Heartseed cannot be torn out.
    Undemolishable { room: String },
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::UnknownRoom { room } => write!(f, "no such room: {room}"),
            CommandError::NoSuchFloor { floor } => write!(f, "no floor {floor}"),
            CommandError::SlotOutOfRange {
                slot,
                width,
                floor_slots,
            } => write!(
                f,
                "slots {slot}..{} run past the floor width of {floor_slots}",
                *slot as u16 + *width as u16
            ),
            CommandError::SlotOccupied { floor, slot } => {
                write!(f, "floor {floor} slot {slot} is already occupied")
            }
            CommandError::FloorTooHigh { floor, max_floor } => {
                write!(f, "cannot go above floor {max_floor}; asked for {floor}")
            }
            CommandError::AlreadyPlaced { room } => write!(f, "{room} is already placed"),
            CommandError::InsufficientStock {
                item,
                needed,
                available,
            } => write!(f, "need {needed} {item}, have {available}"),
            CommandError::FloorLimit { max_floors } => {
                write!(f, "the legs will not carry more than {max_floors} floors")
            }
            CommandError::NoRoomThere { floor, slot } => {
                write!(f, "nothing at floor {floor} slot {slot}")
            }
            CommandError::Undemolishable { room } => write!(f, "{room} cannot be removed"),
        }
    }
}
