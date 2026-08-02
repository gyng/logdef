//! The tower: floors, the rooms on them, and the shafts between them.
//!
//! Two structural decisions carry the whole design:
//!
//! * A floor holds **many** rooms. v1's `Option<Building>` per floor
//!   made the interesting layout question — what shares a floor with
//!   what — impossible to ask. It is fixed in the data model here, on
//!   day one, before anything depends on the old shape.
//! * A shaft costs a slot column on **every** floor it spans. Vertical
//!   transport is not free real estate; the price of circulation is
//!   floor space, and that tension is the game.

use serde::{Deserialize, Serialize};

use crate::content::{Content, RoomCategory};
use crate::fx::Fx;
use crate::ids::{FloorIdx, ItemIdx, RoomId, RoomIdx, ShaftId, SlotIdx};

/// A typed pile of one item with a ceiling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stack {
    pub item: ItemIdx,
    pub count: i64,
    pub max: i64,
}

impl Stack {
    #[must_use]
    pub const fn new(item: ItemIdx, max: i64) -> Self {
        Self {
            item,
            count: 0,
            max,
        }
    }

    #[must_use]
    pub const fn space(&self) -> i64 {
        self.max - self.count
    }

    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.count >= self.max
    }

    /// Add up to `amount`, returning how much fit.
    pub fn deposit(&mut self, amount: i64) -> i64 {
        let taken = amount.min(self.space()).max(0);
        self.count += taken;
        taken
    }

    /// Remove up to `amount`, returning how much came out.
    pub fn withdraw(&mut self, amount: i64) -> i64 {
        let given = amount.min(self.count).max(0);
        self.count -= given;
        given
    }
}

/// A storeroom shelf: empty until something lands on it, then dedicated
/// to that item until it empties again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shelf {
    pub item: Option<ItemIdx>,
    pub count: i64,
    pub max: i64,
}

impl Shelf {
    #[must_use]
    pub const fn empty(max: i64) -> Self {
        Self {
            item: None,
            count: 0,
            max,
        }
    }

    #[must_use]
    pub const fn accepts(&self, item: ItemIdx) -> bool {
        match self.item {
            None => true,
            Some(held) => held.0 == item.0 && self.count < self.max,
        }
    }

    #[must_use]
    pub const fn space_for(&self, item: ItemIdx) -> i64 {
        match self.item {
            None => self.max,
            Some(held) if held.0 == item.0 => self.max - self.count,
            Some(_) => 0,
        }
    }

    pub fn deposit(&mut self, item: ItemIdx, amount: i64) -> i64 {
        let space = self.space_for(item);
        let taken = amount.min(space).max(0);
        if taken > 0 {
            self.item = Some(item);
            self.count += taken;
        }
        taken
    }
}

/// One placed room instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Room {
    pub id: RoomId,
    pub def: RoomIdx,
    pub slot: SlotIdx,
    pub width: u8,
    /// Recipe inputs, in definition order.
    pub inputs: Vec<Stack>,
    /// Recipe outputs and intake produce, in definition order.
    pub outputs: Vec<Stack>,
    /// Storeroom shelves.
    pub shelves: Vec<Shelf>,
    /// Ticks accumulated toward the current craft.
    pub progress: u32,
    /// Sub-item intake accumulation, Q8.8.
    pub intake_acc: Fx,
}

impl Room {
    #[must_use]
    pub fn new(id: RoomId, def: RoomIdx, slot: SlotIdx, content: &Content) -> Self {
        let room_def = content.room(def);
        let rt = content.room_rt(def);

        let inputs = rt
            .recipe_inputs
            .iter()
            .map(|(item, _, buffer_max)| Stack::new(*item, *buffer_max))
            .collect();

        let mut outputs: Vec<Stack> = rt
            .recipe_outputs
            .iter()
            .map(|(item, _, buffer_max)| Stack::new(*item, *buffer_max))
            .collect();
        if let Some(item) = rt.intake_item {
            outputs.push(Stack::new(item, rt.intake_buffer_max));
        }

        let shelves = (0..rt.shelves)
            .map(|_| Shelf::empty(rt.per_shelf))
            .collect();

        Self {
            id,
            def,
            slot,
            width: room_def.width,
            inputs,
            outputs,
            shelves,
            progress: 0,
            intake_acc: Fx::ZERO,
        }
    }

    /// Slot range occupied: `[slot, end)`.
    #[must_use]
    pub const fn end_slot(&self) -> u16 {
        self.slot as u16 + self.width as u16
    }

    #[must_use]
    pub const fn covers(&self, slot: SlotIdx) -> bool {
        slot >= self.slot && (slot as u16) < self.end_slot()
    }

    /// Where a crew member stands to collect from this room: the right
    /// edge, so a left-anchored shaft means a walk across the floor.
    #[must_use]
    pub const fn outbox_slot(&self) -> SlotIdx {
        (self.end_slot() - 1) as SlotIdx
    }

    /// Put items on the first shelf that will take them. Returns how
    /// many were shelved.
    pub fn shelve(&mut self, item: ItemIdx, amount: i64) -> i64 {
        let mut remaining = amount;
        // Prefer a shelf already holding this item so shelves don't
        // fragment across identical stock.
        for shelf in self.shelves.iter_mut().filter(|s| s.item == Some(item)) {
            remaining -= shelf.deposit(item, remaining);
            if remaining == 0 {
                return amount;
            }
        }
        for shelf in self.shelves.iter_mut().filter(|s| s.item.is_none()) {
            remaining -= shelf.deposit(item, remaining);
            if remaining == 0 {
                return amount;
            }
        }
        amount - remaining
    }

    /// Total shelf space this room could still take for `item`.
    #[must_use]
    pub fn shelf_space_for(&self, item: ItemIdx) -> i64 {
        self.shelves.iter().map(|s| s.space_for(item)).sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShaftKind {
    /// Free, always present, slow, one body at a time.
    Stairs,
}

/// Vertical transport. In M0 there is only one kind; the dumbwaiter and
/// the elevator car simulation arrive in M1 and slot in here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shaft {
    pub id: ShaftId,
    pub kind: ShaftKind,
    pub low: FloorIdx,
    pub high: FloorIdx,
    pub slot: SlotIdx,
    pub capacity: u8,
    /// Crew currently on the shaft.
    pub riders: u8,
}

impl Shaft {
    #[must_use]
    pub const fn spans(&self, floor: FloorIdx) -> bool {
        floor >= self.low && floor <= self.high
    }

    #[must_use]
    pub const fn covers_trip(&self, from: FloorIdx, to: FloorIdx) -> bool {
        let low = if from < to { from } else { to };
        let high = if from < to { to } else { from };
        self.low <= low && self.high >= high
    }

    #[must_use]
    pub const fn has_room(&self) -> bool {
        self.riders < self.capacity
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Floor {
    pub index: FloorIdx,
    pub slots: u8,
    /// Sorted by `slot`, never overlapping.
    pub rooms: Vec<Room>,
}

impl Floor {
    #[must_use]
    pub const fn new(index: FloorIdx, slots: u8) -> Self {
        Self {
            index,
            slots,
            rooms: Vec::new(),
        }
    }

    #[must_use]
    pub fn room_at(&self, slot: SlotIdx) -> Option<&Room> {
        self.rooms.iter().find(|r| r.covers(slot))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tower {
    pub floors: Vec<Floor>,
    pub shafts: Vec<Shaft>,
}

impl Tower {
    #[must_use]
    pub fn new(content: &Content) -> Self {
        let balance = &content.balance.tower;
        let floors = (0..balance.starting_floors)
            .map(|index| Floor::new(index, balance.floor_slots))
            .collect();

        // Built-in stairs at the left edge, spanning everything. One
        // rider at a time, so two crew already make a visible queue.
        let shafts = vec![Shaft {
            id: ShaftId(1),
            kind: ShaftKind::Stairs,
            low: 0,
            high: balance.starting_floors.saturating_sub(1),
            slot: 0,
            capacity: balance.stairs_capacity,
            riders: 0,
        }];

        Self { floors, shafts }
    }

    #[must_use]
    pub fn top_floor(&self) -> FloorIdx {
        self.floors.len().saturating_sub(1) as FloorIdx
    }

    #[must_use]
    pub fn floor(&self, index: FloorIdx) -> Option<&Floor> {
        self.floors.get(index as usize)
    }

    pub fn floor_mut(&mut self, index: FloorIdx) -> Option<&mut Floor> {
        self.floors.get_mut(index as usize)
    }

    #[must_use]
    pub fn shaft(&self, id: ShaftId) -> Option<&Shaft> {
        self.shafts.iter().find(|s| s.id == id)
    }

    pub fn shaft_mut(&mut self, id: ShaftId) -> Option<&mut Shaft> {
        self.shafts.iter_mut().find(|s| s.id == id)
    }

    /// Does anything already occupy `[slot, slot + width)` on `floor`?
    /// Checks rooms on that floor and every shaft column crossing it.
    #[must_use]
    pub fn slot_range_blocked(&self, floor: FloorIdx, slot: SlotIdx, width: u8) -> bool {
        let end = slot as u16 + width as u16;
        let Some(target) = self.floor(floor) else {
            return true;
        };
        if end > target.slots as u16 {
            return true;
        }
        let start = slot as u16;
        if target
            .rooms
            .iter()
            .any(|room| start < room.end_slot() && (room.slot as u16) < end)
        {
            return true;
        }
        // A shaft occupies exactly one slot on every floor it spans.
        self.shafts.iter().any(|shaft| {
            shaft.spans(floor) && (shaft.slot as u16) < end && start <= shaft.slot as u16
        })
    }

    /// Find a room by floor and any slot it covers.
    #[must_use]
    pub fn find_room(&self, floor: FloorIdx, slot: SlotIdx) -> Option<&Room> {
        self.floor(floor).and_then(|f| f.room_at(slot))
    }

    /// Is there a heart room anywhere in the tower?
    #[must_use]
    pub fn has_category(&self, content: &Content, category: RoomCategory) -> bool {
        self.floors
            .iter()
            .flat_map(|f| f.rooms.iter())
            .any(|r| content.room(r.def).category == category)
    }

    /// Count of placed rooms with a given definition.
    #[must_use]
    pub fn count_of(&self, def: RoomIdx) -> usize {
        self.floors
            .iter()
            .flat_map(|f| f.rooms.iter())
            .filter(|r| r.def == def)
            .count()
    }
}
