//! Commands — the only way `GameState` changes.
//!
//! Nothing in the simulation, no input handler, and no snapshot builder
//! mutates state directly. Every change is a `GameCommand`, validated
//! before it applies, and rejected with a typed error rather than
//! silently ignored. That is what makes the game replayable: a run is
//! its seed plus this list.

use serde::{Deserialize, Serialize};

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

    /// Rank what keeps running when the bank runs short, best first.
    ///
    /// **The one piece of FTL's reactor the tower already had the wiring
    /// for.** Charge priority used to *be* the tick order and nothing
    /// else — lifts first because transport runs first, legs last
    /// because striding runs last — so it was a constant rather than a
    /// decision. This is the same four draws with the ranking handed to
    /// the player.
    ///
    /// It is a schedule the player writes, the same category as the
    /// shift rota and the per-daypart shaft programs, which is what
    /// makes a panel for it allowable under `DECISIONS.md` §8.
    SetPowerPriority {
        /// All four uses, best first. Rejected unless it is exactly the
        /// four, each once: a partial order would leave the rest ranked
        /// by an accident of list position.
        order: Vec<crate::state::power::PowerUse>,
    },

    /// Ask every emplacement to prefer one creature, or clear the ask.
    ///
    /// **A battery still has no judgement.** `defence.rs` shoots the
    /// nearest thing in range because the player's judgement went into
    /// where they put it; this adds a second moment to supply it, live,
    /// at the cost of attention during a wave. Nothing focused is the
    /// normal case and the old behaviour exactly.
    ///
    /// A focus out of range does not stop an emplacement firing — it
    /// falls back to nearest, because a battery sitting idle while
    /// something chewed on the tower would be a trap rather than a
    /// decision.
    FocusEnemy {
        /// `None` clears it. An unknown id is rejected rather than
        /// stored, so the highlight can never point at nothing.
        enemy: Option<crate::ids::EnemyId>,
    },

    /// Post somebody to a room, or call them back off it.
    ///
    /// **A standing order about somebody's working day**, in the same
    /// category as the shift rota — which is what makes it allowable
    /// under `DECISIONS.md` §8. The room runs at `manned_work_pct` while
    /// they are in it, and the price is not a resource: a tower has
    /// three crew and one staircase, so posting somebody is a standing
    /// decision to take a porter off the stairs.
    ///
    /// Needs still outrank it. A posted person goes to eat when hungry
    /// and to bed when their shift ends, and comes back afterwards — a
    /// station is not a cage.
    /// Put another car in a shaft that already exists.
    ///
    /// **The late-game answer to a queue, and cheaper in the thing that
    /// is actually scarce.** A second shaft costs a slot column on every
    /// floor it spans; a second car costs none. So a tower that has run
    /// out of width can still buy its way out of a queue, and a tower
    /// that has width to spare gets to choose.
    AddCar { shaft: ShaftId },

    /// Set the order idle crew reach for work in.
    ///
    /// **The whole order at once, and it must be a permutation.** A
    /// command that raised one job would be easier to validate and
    /// would let a save hold a work order with a job missing from it,
    /// which is a state with no meaning — every job has to be somewhere
    /// in the list, because every job still has to be *reachable*.
    SetWorkOrder { order: Vec<crate::state::Job> },

    /// Widen the hull.
    ///
    /// **New frame goes on the back, not the front**, and everything
    /// already aboard shifts up a couple of slots to make room. That is
    /// the one arrangement that keeps the *leading edge* where it was:
    /// weapons are `front_only` (`SYSTEMS.md` §6.13), and a tower that
    /// grew at the front would leave every gun it owns two slots inside
    /// its own nose.
    WidenTower,

    /// Take the beat the tower is passing.
    ///
    /// **Only ever the one in range**, so there is no id to get wrong
    /// and no way to reach back down the axis for something already
    /// behind you. Ignoring it is free and is the default — the tower
    /// walks on and the thing goes by (`SYSTEMS.md` §6.14).
    TakeWaypoint,

    StationCrew {
        crew: CrewId,
        /// `None` calls them back to hauling.
        room: Option<crate::ids::RoomId>,
        /// End the posting when they run out of energy.
        ///
        /// A *push* rather than a job — see `Crew::post_until_tired`.
        /// Defaulted so replays recorded before it still load, and so
        /// the plain "post somebody here" command is unchanged.
        #[serde(default)]
        until_tired: bool,
    },

    /// Lend somebody a kit off the shelves, or take it back.
    ///
    /// **What "equip" means in this game.** Not weapons bolted to the
    /// tower — emplacements already have that half, and what you feed a
    /// dart battery *is* the choice. A kit belongs to a *person*, which
    /// is `DESIGN.md` §2 structural call 4 (crew are named individuals,
    /// not stat blocks) given something mechanical to rest on.
    ///
    /// The item leaves the shelves while it is carried and goes back
    /// when it is handed in. Nothing is consumed, so this is a decision
    /// the player can take back — and a kit in somebody's hands is not
    /// on the shelves for anybody else, which is the whole of the
    /// scarcity.
    EquipCrew {
        crew: CrewId,
        /// Authored item id, e.g. `"item.hand_lamp"`. `None` takes back
        /// whatever they are carrying.
        kit: Option<String>,
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
    /// The room is not on the menu yet: something has to be standing
    /// first. See `RoomDef::unlocked_by` and `SYSTEMS.md` §6.11.
    Locked { room: String, needs: String },
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
    /// A weapon, and not on the tower's leading edge.
    NotAtTheFront { slot: SlotIdx, front: SlotIdx },
    /// An ordinary room aimed at the weapons' deck.
    OnTheWeaponsDeck { slot: SlotIdx, deck_from: SlotIdx },
    /// Nothing within reach to take.
    NothingInReach,
    /// A work order that is not a permutation of every job.
    NotAWorkOrder,
    /// This shaft holds as many cars as it can.
    FullOfCars { cars: u8 },
    /// The hull is already as wide as it goes.
    AlreadyWidest { slots: u8 },
    /// Only one of these may exist in a tower.
    AlreadyPlaced { room: String },
    /// A charge ranking that was not all four uses, each exactly once.
    BadPowerPriority { given: usize },
    /// Asked to focus a creature that is not out there.
    NoSuchEnemy { id: crate::ids::EnemyId },
    /// That item exists but is not something a person can carry.
    NotAKit { item: String },
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
            CommandError::AlreadyWidest { slots } => {
                write!(f, "the hull is already {slots} slots across")
            }
            CommandError::NotAWorkOrder => {
                write!(f, "a work order has to list every job exactly once")
            }
            CommandError::FullOfCars { cars } => {
                write!(f, "the shaft already runs {cars} car(s)")
            }
            CommandError::OnTheWeaponsDeck { slot, deck_from } => write!(
                f,
                "slot {slot} is on the weapons' deck, which starts at {deck_from}"
            ),
            CommandError::NothingInReach => {
                write!(f, "nothing the tower is passing is close enough to take")
            }
            CommandError::NotAtTheFront { slot, front } => write!(
                f,
                "a weapon goes on the front of the tower: slot {front}, not {slot}"
            ),
            CommandError::Locked { room, needs } => {
                write!(f, "{room} is not on the menu until a {needs} is standing")
            }
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
            CommandError::BadPowerPriority { given } => {
                write!(f, "a charge ranking must be all four uses, got {given}")
            }
            CommandError::NoSuchEnemy { id } => write!(f, "no creature {} out there", id.0),
            CommandError::NotAKit { item } => write!(f, "{item} is not something to carry"),
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
