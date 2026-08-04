//! Commands — the only way `GameState` changes.
//!
//! Nothing in the simulation, no input handler, and no snapshot builder
//! mutates state directly. Every change is a `GameCommand`, validated
//! before it applies, and rejected with a typed error rather than
//! silently ignored. That is what makes the game replayable: a run is
//! its seed plus this list.

use serde::{Deserialize, Serialize};

use crate::content::Shift;
use crate::ids::{CrewId, FloorIdx, ShaftId, SlotIdx};
use crate::state::{ShaftPriority, SimSpeed};

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

    /// Switch a room off or back on. Mostly the burner: "should this be
    /// running right now" is a real decision every night.
    SetRoomActive {
        floor: FloorIdx,
        slot: SlotIdx,
        active: bool,
    },

    /// Build vertical transport. The column costs a slot on every floor
    /// it spans, which is the whole price of circulation.
    BuildShaft {
        /// Authored string ID, e.g. `"shaft.elevator"`.
        shaft: String,
        low: FloorIdx,
        high: FloorIdx,
        slot: SlotIdx,
    },

    /// Tear out a shaft. The built-in stairs cannot go.
    RemoveShaft { id: ShaftId },

    /// Reprogram a shaft for one daypart: which floors its cars serve,
    /// and who boards first when both crew and freight are waiting.
    SetShaftProgram {
        id: ShaftId,
        /// Index into the content pack's dayparts.
        daypart: u16,
        served: Vec<bool>,
        priority: ShaftPriority,
    },

    /// Halt the legs to bank the charge they would have burned, or set
    /// them running again. The bank-or-burn decision, at its simplest.
    SetStriding { walking: bool },
    /// Take one of the enclave's posted offers, once.
    ///
    /// Only while berthed there. Each offer has finite stock, because
    /// an enclave is somewhere a run passes through rather than a shop
    /// that restocks.
    Trade { offer: u8 },
    /// Take somebody aboard, for poles.
    Recruit,
    /// Have the settlement plate the tower's shell, for scrap.
    Reinforce,
    /// Commit to one of the two branches the pending fork offers.
    ///
    /// Legal from the moment the fork appears until the tower crosses
    /// it, and re-sendable until then — the last answer before the line
    /// is the one that counts (`SYSTEMS.md` §3.3).
    TakeFork { branch: u8 },

    /// Put one crew member on the day or the night shift.
    ///
    /// One person per command rather than a bulk setter: commands batch
    /// cheaply (`DECISIONS.md` §3), a rejection then names the crew
    /// member it is about, and the replay reads as a list of decisions
    /// about people.
    ///
    /// This is also the rota's one emergency verb. Because awake means
    /// "the current daypart belongs to my shift", setting a sleeping
    /// day worker to `Night` in the middle of the night wakes them
    /// immediately — unrested, on the slow multiplier — and come
    /// morning they are off shift and will sleep through the day you
    /// needed them for. A real all-hands lever with a real price, built
    /// out of nothing but that definition.
    SetShift { crew: CrewId, shift: Shift },
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
    /// This room may only be placed on higher floors.
    FloorTooLow { floor: FloorIdx, min_floor: u8 },
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
    /// The content pack has no shaft with that ID.
    UnknownShaft { shaft: String },
    /// The route does not split here, or not yet.
    NoForkPending,
    /// A fork offers two ways. That was not one of them.
    NoSuchBranch { branch: u8 },
    /// The tower is not stopped at the enclave.
    NotBerthedAtAnEnclave,
    /// The enclave posts no such offer.
    NoSuchOffer { offer: u8 },
    /// That offer has been taken as often as it is going to be.
    OfferExhausted { offer: u8 },
    /// Nobody else here is willing to come aboard.
    NobodyToRecruit,
    /// The tower has as many people as it can house.
    CrewFull { cap: u8 },
    /// Nobody here does shell work.
    NoShellWorkHere,
    /// They have plated this tower as often as they are going to.
    NoShellWorkLeft,
    /// No shaft with that runtime ID is standing.
    NoSuchShaft { id: ShaftId },
    /// The span is inverted, too short, or too tall for this kind.
    BadSpan {
        low: FloorIdx,
        high: FloorIdx,
        min_span: u8,
        max_span: u8,
    },
    /// The content pack has no daypart at that index.
    NoSuchDaypart { daypart: u16 },
    /// Nobody aboard has that id.
    NoSuchCrew { crew: CrewId },
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
            CommandError::FloorTooLow { floor, min_floor } => {
                write!(f, "cannot go below floor {min_floor}; asked for {floor}")
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
            CommandError::UnknownShaft { shaft } => write!(f, "no such shaft: {shaft}"),
            CommandError::NoForkPending => write!(f, "the route does not split here"),
            CommandError::NotBerthedAtAnEnclave => {
                write!(f, "the tower is not stopped at an enclave")
            }
            CommandError::NoSuchOffer { offer } => write!(f, "no offer {offer} is posted"),
            CommandError::OfferExhausted { offer } => write!(f, "offer {offer} is spent"),
            CommandError::NobodyToRecruit => write!(f, "nobody else here wants to come"),
            CommandError::CrewFull { cap } => write!(f, "the tower houses {cap} already"),
            CommandError::NoShellWorkHere => write!(f, "nobody here works on hulls"),
            CommandError::NoShellWorkLeft => {
                write!(f, "they have plated this tower as often as they will")
            }
            CommandError::NoSuchBranch { branch } => {
                write!(f, "a fork offers two ways; {branch} was not one of them")
            }
            CommandError::NoSuchShaft { id } => write!(f, "no shaft {}", id.0),
            CommandError::NoSuchCrew { crew } => write!(f, "nobody aboard is {}", crew.0),
            CommandError::BadSpan {
                low,
                high,
                min_span,
                max_span,
            } => {
                let ceiling = if *max_span == 0 {
                    "the whole tower".to_string()
                } else {
                    max_span.to_string()
                };
                write!(
                    f,
                    "floors {low}..{high} is not a valid span; needs {min_span} to {ceiling}"
                )
            }
            CommandError::NoSuchDaypart { daypart } => write!(f, "no daypart {daypart}"),
        }
    }
}
