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

use crate::content::{Content, RoomCategory, ShaftKind};
use crate::fx::Fx;
use crate::ids::{
    CrewId, DaypartIdx, FloorIdx, ItemIdx, RoomId, RoomIdx, ShaftId, ShaftIdx, SlotIdx,
};
use crate::state::siege::Health;

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
    /// Switched on. Mostly matters for the burner, where "should this
    /// be running right now" is a real decision every night — but any
    /// room can be shut down to stop it eating charge or inputs.
    pub active: bool,
    /// Damage. A hurt room works; a wrecked one does not.
    pub health: Health,
}

impl Room {
    #[must_use]
    pub fn new(id: RoomId, def: RoomIdx, slot: SlotIdx, content: &Content) -> Self {
        let room_def = content.room(def);
        let rt = content.room_rt(def);

        let mut inputs: Vec<Stack> = rt
            .recipe_inputs
            .iter()
            .map(|(item, _, buffer_max)| Stack::new(*item, *buffer_max))
            .collect();
        // A burner eats from an inbox like any other room, so the crew
        // have to keep it fed — which is what makes lighting it compete
        // with the mill for exactly the same bamboo.
        if let Some((item, buffer_max)) = rt.burner_fuel {
            inputs.push(Stack::new(item, buffer_max));
        }
        // A battery's magazine is an input stack too, so the crew feed
        // it with exactly the machinery that feeds a mill.
        if let Some(item) = rt.defence_ammo {
            inputs.push(Stack::new(item, rt.defence_buffer_max));
        }

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
            active: true,
            health: Health::full(if room_def.category == RoomCategory::Heart {
                content.balance.siege.heartseed_hp
            } else {
                content.balance.siege.room_hp
            }),
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

    /// Is this room damaged past the point of working at all?
    #[must_use]
    pub fn is_wrecked(&self, content: &Content) -> bool {
        self.health.permille() < content.balance.siege.wrecked_permille
    }

    /// Can this room do its job on this tick?
    ///
    /// Damage slows a room rather than stopping it: at half health it
    /// works half the ticks. Deterministic, because it is a function of
    /// the tick number, and proportional, so the player sees a visible
    /// decline before the room goes silent instead of a cliff.
    #[must_use]
    pub fn is_working(&self, content: &Content, tick: u64) -> bool {
        if !self.active || self.is_wrecked(content) {
            return false;
        }
        let health = self.health.permille();
        health >= 1000 || ((tick % 1000) as i64) < health
    }

    /// Total shelf space this room could still take for `item`.
    #[must_use]
    pub fn shelf_space_for(&self, item: ItemIdx) -> i64 {
        self.shelves.iter().map(|s| s.space_for(item)).sum()
    }
}

/// Which way a car is sweeping. `Idle` means it is parked and has no
/// direction yet — not that it is between floors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CarDir {
    Idle,
    Up,
    Down,
}

impl CarDir {
    #[must_use]
    pub const fn toward(from: FloorIdx, to: FloorIdx) -> Self {
        if to > from {
            CarDir::Up
        } else if to < from {
            CarDir::Down
        } else {
            CarDir::Idle
        }
    }

    #[must_use]
    pub const fn reversed(self) -> Self {
        match self {
            CarDir::Up => CarDir::Down,
            CarDir::Down => CarDir::Up,
            CarDir::Idle => CarDir::Idle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CarState {
    /// Parked, waiting for enough demand to be worth departing.
    Idle,
    Moving,
    /// Stopped at a floor with the doors open.
    Dwelling {
        ticks_left: u32,
    },
}

/// One elevator or dumbwaiter car.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Car {
    /// Fractional floor position, so the renderer gets smooth travel.
    pub pos: Fx,
    pub dir: CarDir,
    pub state: CarState,
    /// Crew aboard. Their position is driven by the car while riding.
    pub riders: Vec<CrewId>,
    /// Items aboard. Dumbwaiters only — elevator freight rides in the
    /// hands of the crew carrying it.
    pub freight: Vec<Stack>,
    /// Floors someone aboard wants. Kept sorted and deduplicated.
    pub stops: Vec<FloorIdx>,
    /// Where a dumbwaiter is taking its load.
    pub target: Option<FloorIdx>,
    /// Ticks parked with demand outstanding. Feeds the dispatch
    /// threshold: a car does not leave for a single caller instantly.
    pub idle_ticks: u32,
}

impl Car {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pos: Fx::ZERO,
            dir: CarDir::Idle,
            state: CarState::Idle,
            riders: Vec::new(),
            freight: Vec::new(),
            stops: Vec::new(),
            target: None,
            idle_ticks: 0,
        }
    }

    #[must_use]
    pub fn floor(&self) -> FloorIdx {
        self.pos.floor_int().max(0) as FloorIdx
    }

    /// Exactly at a floor, rather than between two.
    #[must_use]
    pub fn at_floor(&self) -> bool {
        self.pos.frac_raw() == 0
    }

    pub fn add_stop(&mut self, floor: FloorIdx) {
        if let Err(at) = self.stops.binary_search(&floor) {
            self.stops.insert(at, floor);
        }
    }

    pub fn clear_stop(&mut self, floor: FloorIdx) {
        self.stops.retain(|stop| *stop != floor);
    }

    /// Is there a reason to keep going this way?
    #[must_use]
    pub fn has_stop_beyond(&self, floor: FloorIdx, dir: CarDir) -> bool {
        match dir {
            CarDir::Up => self.stops.iter().any(|stop| *stop > floor),
            CarDir::Down => self.stops.iter().any(|stop| *stop < floor),
            CarDir::Idle => false,
        }
    }
}

impl Default for Car {
    fn default() -> Self {
        Self::new()
    }
}

/// Whose calls a car answers first when both are waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShaftPriority {
    Balanced,
    /// Crew carrying a load board first. The freight lane.
    FreightFirst,
    /// Empty-handed crew board first — commutes over cargo.
    CrewFirst,
}

/// What a shaft does during one daypart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShaftProgram {
    /// Indexed by floor. A floor the car does not serve is one it will
    /// not stop at, however loudly someone is calling.
    pub served: Vec<bool>,
    pub priority: ShaftPriority,
}

impl ShaftProgram {
    #[must_use]
    pub fn all_floors(count: usize) -> Self {
        Self {
            served: vec![true; count],
            priority: ShaftPriority::Balanced,
        }
    }

    #[must_use]
    pub fn serves(&self, floor: FloorIdx) -> bool {
        self.served.get(floor as usize).copied().unwrap_or(false)
    }
}

/// Vertical transport: stairs, a dumbwaiter, or an elevator.
///
/// A shaft occupies one slot column on **every** floor it spans. That
/// is the price of circulation, and it is what stops "add another
/// shaft" from being a free answer to every bottleneck.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shaft {
    pub id: ShaftId,
    pub def: ShaftIdx,
    /// Cached from the definition so the hot path never indexes content
    /// to answer "is this an elevator".
    pub kind: ShaftKind,
    pub low: FloorIdx,
    pub high: FloorIdx,
    pub slot: SlotIdx,
    /// Stairs: bodies on the flight at once. Elevator: car capacity in
    /// units.
    pub capacity: u8,
    /// Crew on the stairs right now.
    pub riders: u8,
    pub cars: Vec<Car>,
    /// One program per daypart, so the night shift can run a different
    /// pattern from the day.
    pub programs: Vec<ShaftProgram>,
    /// Damage. A severed column splits the tower's circulation in two,
    /// which is the signature emergency of the whole design.
    pub health: Health,
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
        self.riders < self.capacity && !self.health.is_broken()
    }

    /// Chewed through. Nothing travels on it until it is repaired, and
    /// route-finding has to notice — that reroute is the moment the
    /// whole siege design exists to produce.
    #[must_use]
    pub const fn is_severed(&self) -> bool {
        self.health.is_broken()
    }

    #[must_use]
    pub fn program(&self, daypart: DaypartIdx) -> &ShaftProgram {
        self.programs
            .get(daypart.get())
            .or_else(|| self.programs.first())
            .expect("a shaft always has at least one program")
    }

    /// Does this shaft serve both ends of the trip during `daypart`?
    #[must_use]
    pub fn serves_trip(&self, from: FloorIdx, to: FloorIdx, daypart: DaypartIdx) -> bool {
        if self.is_severed() || !self.covers_trip(from, to) {
            return false;
        }
        match self.kind {
            // Stairs go everywhere they span; there is nothing to
            // program on a staircase.
            ShaftKind::Stairs => true,
            _ => {
                let program = self.program(daypart);
                program.serves(from) && program.serves(to)
            }
        }
    }

    /// Units currently aboard a car: one per crew member, one more for
    /// anything they are carrying.
    #[must_use]
    pub fn car_load(&self, car: usize, crew: &[crate::state::Crew]) -> u8 {
        let Some(car) = self.cars.get(car) else {
            return 0;
        };
        car.riders
            .iter()
            .filter_map(|id| crew.iter().find(|member| member.id == *id))
            .map(|member| if member.is_carrying() { 2u8 } else { 1u8 })
            .sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Floor {
    pub index: FloorIdx,
    pub slots: u8,
    /// Sorted by `slot`, never overlapping.
    pub rooms: Vec<Room>,
    /// The outer wall. A breached panel lets things inside.
    pub panel: Health,
}

impl Floor {
    #[must_use]
    pub const fn new(index: FloorIdx, slots: u8, panel_hp: i64) -> Self {
        Self {
            index,
            slots,
            rooms: Vec::new(),
            panel: Health::full(panel_hp),
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
            .map(|index| Floor::new(index, balance.floor_slots, content.balance.siege.panel_hp))
            .collect();

        // Built-in stairs at the left edge, spanning everything. One
        // body at a time, so two crew already make a visible queue.
        let stairs_def = content
            .shafts
            .iter()
            .position(|def| def.kind == ShaftKind::Stairs)
            .map_or(ShaftIdx(0), |i| ShaftIdx(i as u16));

        let shafts = vec![Shaft {
            id: ShaftId(1),
            def: stairs_def,
            kind: ShaftKind::Stairs,
            low: 0,
            high: balance.starting_floors.saturating_sub(1),
            slot: 0,
            capacity: balance.stairs_capacity,
            riders: 0,
            cars: Vec::new(),
            // Sized for the tallest the tower can get, so growing does
            // not need every program rewritten.
            programs: vec![
                ShaftProgram::all_floors(balance.max_floors as usize);
                content.dayparts.len().max(1)
            ],
            health: Health::full(content.balance.siege.shaft_hp),
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
