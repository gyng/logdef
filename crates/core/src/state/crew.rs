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
    /// Standing at a shaft column waiting for capacity.
    Boarding {
        shaft: ShaftId,
        to_floor: FloorIdx,
    },
    /// On the shaft, moving between floors.
    Climbing {
        shaft: ShaftId,
        to_floor: FloorIdx,
    },
    Loading {
        ticks_left: u32,
    },
    Unloading {
        ticks_left: u32,
    },
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
    /// Cosmetic-stream draw. Renderer-only: idle animation phase.
    pub fidget: u16,
}

impl Crew {
    #[must_use]
    pub fn new(id: CrewId, name: String, fidget: u16) -> Self {
        Self {
            id,
            name,
            floor_fx: Fx::ZERO,
            slot_fx: Fx::ZERO,
            carrying: None,
            task: None,
            state: CrewState::Idle,
            wait_ticks: 0,
            fidget,
        }
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
