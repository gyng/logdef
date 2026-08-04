//! The crew — the tower's porters, and its pulse.
//!
//! A crew member is a small state machine walking an L-shaped path:
//! along a floor to a shaft column, up or down the shaft, along the
//! destination floor. Each leg is one state, and position is kept in
//! fixed point so the renderer gets smooth motion without the
//! simulation ever touching a float.
//!
//! `wait_ticks` is the only bottleneck instrument in the game. When a
//! crew member is blocked at a shaft it climbs, and the cross-section
//! tints them toward red. There is no throughput dashboard and there
//! will not be one: the queue diagnoses itself.

use serde::{Deserialize, Serialize};

use crate::content::Shift;
use crate::fx::Fx;
use crate::ids::{CrewId, FloorIdx, ItemIdx, RoomId, ShaftId, SlotIdx};

/// Where a haul is going. Priority order is the enum order — feeding a
/// live recipe beats stockpiling, always.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HaulDestination {
    /// A room's input buffer. The chain is hungry.
    Inbox { room: RoomId, input: u8 },
    /// A storeroom shelf. Overflow, and the construction stock.
    Shelf { room: RoomId },
    /// A chute. The load leaves the tower and does not come back.
    ///
    /// Last in the enum and last in priority, because this is where
    /// something goes when there is nowhere for it to go. A tower with a
    /// free shelf never spills — see `haul::find_destination`.
    Spill { shaft: ShaftId },
}

/// Where a load is collected from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaulPickup {
    pub room: RoomId,
    pub floor: FloorIdx,
    pub slot: SlotIdx,
}

/// One committed end-to-end delivery.
///
/// `pickup` is `None` when the crew member is already holding the load
/// — which happens when a destination fills up mid-trip and they need
/// somewhere new to put it. Nothing a crew member picks up is ever
/// destroyed; worst case they stand there holding it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaulTask {
    pub item: ItemIdx,
    pub amount: i64,
    pub pickup: Option<HaulPickup>,
    pub to_floor: FloorIdx,
    pub to_slot: SlotIdx,
    pub destination: HaulDestination,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrewState {
    Idle,
    /// Walking along the current floor toward `to_slot`.
    Walking {
        to_slot: SlotIdx,
    },
    /// Standing at a shaft column waiting for a way up: either capacity
    /// on the stairs, or a car going their way. This is the state the
    /// stress tint reads, and the one that makes a bottleneck visible.
    Boarding {
        shaft: ShaftId,
        to_floor: FloorIdx,
    },
    /// Climbing stairs under their own power.
    Climbing {
        shaft: ShaftId,
        to_floor: FloorIdx,
    },
    /// Aboard a car. Position is driven by the car, not by the crew
    /// member — they are cargo until the doors open.
    Riding {
        shaft: ShaftId,
        car: u8,
        to_floor: FloorIdx,
    },
    /// Working on damage. Reuses the same walking and climbing legs
    /// as a haul, because repair competes for the same crew and the
    /// same shafts — that competition is the point.
    Repairing {
        target: crate::state::siege::DamageTarget,
        ticks_left: u32,
    },
    Loading {
        ticks_left: u32,
    },
    Unloading {
        ticks_left: u32,
    },
    /// Sat down to a meal, wherever the meal was. Shaped exactly like a
    /// repair — go somewhere, stand there, spend ticks — because eating
    /// is not a haul and is not worth a second pathfinder.
    Eating {
        ticks_left: u32,
    },
    /// Standing in a room, working it.
    ///
    /// Open-ended, unlike every other state here: no `ticks_left`,
    /// because a posting ends when the player ends it, the room goes, or
    /// a need pulls the person away. That is the whole trade — somebody
    /// at a station is somebody not on the stairs.
    Manning {
        room: RoomId,
    },
    /// Off shift. In a bunk if one was free, on the deck where they
    /// stopped if not; which it is depends on the errand, not on this
    /// tag. A sleeper takes no tasks, advances no legs, mends nothing,
    /// and accrues no `wait_ticks` — a red-tinted sleeper would make
    /// the only bottleneck instrument in the game lie.
    Sleeping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crew {
    pub id: CrewId,
    pub name: String,
    /// Q8.8 floor coordinate — fractional only while climbing.
    pub floor_fx: Fx,
    /// Q8.8 slot coordinate — fractional only while walking.
    pub slot_fx: Fx,
    pub carrying: Option<(ItemIdx, i64)>,
    pub task: Option<HaulTask>,
    pub state: CrewState,
    /// Consecutive ticks blocked from making progress.
    pub wait_ticks: u32,
    /// Somewhere to be that is not a haul, and where to stand for it.
    /// Held separately from `task` because none of the three is a haul
    /// and pretending otherwise would put an `Option` inside an
    /// `Option` in every scoring path.
    pub errand: Option<Errand>,
    /// Ticks since the last meal. Rises every tick, awake or asleep; a
    /// meal resets it to zero, because a meal is a meal and somebody
    /// who ate late does not carry the deficit forward.
    pub hunger: u32,
    /// Ticks of work left in them. Counts down while awake and up while
    /// asleep. Tiredness is a scheduling problem the way hunger is a
    /// supply problem: there is no mid-shift nap, and the only thing
    /// that refills this is being off shift.
    pub rested: u32,
    /// Which half of the rota they work. The player sets it; the
    /// simulation never does.
    pub shift: Shift,
    /// A room this person has been told to stand in and work.
    ///
    /// **A standing order, not an errand.** The errand is how they get
    /// there and clears on arrival like every other; this is *why* they
    /// went, and it outlives arriving, eating, sleeping and a wave.
    /// Clearing it is the only thing that sends them back to hauling.
    ///
    /// A room id rather than a coordinate, because a room can be
    /// demolished under somebody's feet and a coordinate would
    /// re-resolve to whatever took its place — the trap `DamageTarget`
    /// documents. An id that no longer exists simply ends the posting.
    pub stationed: Option<RoomId>,
    /// Cosmetic-stream draw. Renderer-only: idle animation phase, and
    /// (frontend-side) which face and which of several equivalent bark
    /// lines are this person's.
    pub fidget: u16,
}

/// A place to stand, and a reason to be there.
///
/// A repair, a meal and a bed are the same shape — walk, climb, stand,
/// spend ticks — so they share one router (`haul::errand_leg`) rather
/// than three parallel `Option`s alongside `task`, which would
/// duplicate the routing three times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Errand {
    /// Damage this crew member has been assigned to.
    Repair {
        target: crate::state::siege::DamageTarget,
        floor: FloorIdx,
        slot: SlotIdx,
    },
    /// Somewhere holding at least one meal — a canteen's outbox, or any
    /// shelf a meal has been hauled to. If it is gone on arrival the
    /// errand clears and they look again, the same failure path
    /// `Loading` already has when somebody else got there first.
    Meal {
        room: RoomId,
        floor: FloorIdx,
        slot: SlotIdx,
    },
    /// A bed with space in it. `None` for this errand means no bed was
    /// free, which is not an error — it means sleeping on the deck.
    Bunk {
        room: RoomId,
        floor: FloorIdx,
        slot: SlotIdx,
    },
    /// The room this person has been posted to. Ends in `Manning`
    /// rather than in a timed job, so they stay put.
    Station {
        room: RoomId,
        floor: FloorIdx,
        slot: SlotIdx,
    },
}

impl Errand {
    #[must_use]
    pub const fn floor(&self) -> FloorIdx {
        match self {
            Self::Repair { floor, .. }
            | Self::Meal { floor, .. }
            | Self::Bunk { floor, .. }
            | Self::Station { floor, .. } => *floor,
        }
    }

    #[must_use]
    pub const fn slot(&self) -> SlotIdx {
        match self {
            Self::Repair { slot, .. }
            | Self::Meal { slot, .. }
            | Self::Bunk { slot, .. }
            | Self::Station { slot, .. } => *slot,
        }
    }

    /// The room this errand names, for the two arms that name one.
    /// Bunk occupancy is counted by scanning these rather than stored on
    /// the room, so there is no counter to get out of step with reality.
    #[must_use]
    pub const fn room(&self) -> Option<RoomId> {
        match self {
            Self::Repair { .. } => None,
            Self::Meal { room, .. } | Self::Bunk { room, .. } | Self::Station { room, .. } => {
                Some(*room)
            }
        }
    }

    #[must_use]
    pub const fn is_bunk(&self) -> bool {
        matches!(self, Self::Bunk { .. })
    }
}

impl Crew {
    #[must_use]
    pub fn new(id: CrewId, name: String, fidget: u16, rested: u32) -> Self {
        Self {
            id,
            name,
            floor_fx: Fx::ZERO,
            slot_fx: Fx::ZERO,
            carrying: None,
            task: None,
            state: CrewState::Idle,
            wait_ticks: 0,
            errand: None,
            hunger: 0,
            rested,
            // Everybody starts on the day shift, so a player who never
            // opens the roster has a tower that works in daylight and
            // sleeps at night. The rota is a decision offered, not one
            // demanded before the first pace.
            shift: Shift::Day,
            stationed: None,
            fidget,
        }
    }

    /// The damage they are assigned to, if that is what they are up to.
    #[must_use]
    pub const fn repair_target(&self) -> Option<crate::state::siege::DamageTarget> {
        match self.errand {
            Some(Errand::Repair { target, .. }) => Some(target),
            _ => None,
        }
    }

    /// Asleep, in the sense that matters to every system that assigns
    /// work: they are unavailable, and handing them a job would be
    /// handing it to somebody who is not there.
    #[must_use]
    pub const fn is_asleep(&self) -> bool {
        matches!(self.state, CrewState::Sleeping)
    }

    /// Current floor, rounded down. Exact except mid-climb.
    #[must_use]
    pub fn floor(&self) -> FloorIdx {
        self.floor_fx.floor_int().max(0) as FloorIdx
    }

    /// Current slot, rounded down. Exact except mid-walk.
    #[must_use]
    pub fn slot(&self) -> SlotIdx {
        self.slot_fx.floor_int().max(0) as SlotIdx
    }

    #[must_use]
    pub const fn is_carrying(&self) -> bool {
        self.carrying.is_some()
    }

    /// Snap to an exact floor/slot, clearing accumulated fraction. Used
    /// when a leg completes so positions never drift.
    pub fn snap_to(&mut self, floor: FloorIdx, slot: SlotIdx) {
        self.floor_fx = Fx::from_int(i32::from(floor));
        self.slot_fx = Fx::from_int(i32::from(slot));
    }
}
