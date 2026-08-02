//! `GameState` — the single source of truth.
//!
//! Split by domain: the world the tower walks through, the tower
//! itself, and the crew inside it. Everything here serialises, contains
//! no floating point, and iterates in a fixed order.

pub mod crew;
pub mod tower;
pub mod world;

use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::ids::{CrewId, ItemIdx, RoomId, ShaftId};
use crate::rng::RngStreams;

pub use crew::{Crew, CrewState, HaulDestination, HaulTask};
pub use tower::{Floor, Room, Shaft, ShaftKind, Shelf, Stack, Tower};
pub use world::{Feature, TerrainBand, World};

/// How fast wall-clock time maps to simulation ticks. Has no effect on
/// the content of a tick — it only decides how many run per frame — but
/// it lives in state so a save restores it and a replay records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimSpeed {
    Paused,
    X1,
    X2,
    X4,
}

impl SimSpeed {
    #[must_use]
    pub const fn multiplier(self) -> u32 {
        match self {
            SimSpeed::Paused => 0,
            SimSpeed::X1 => 1,
            SimSpeed::X2 => 2,
            SimSpeed::X4 => 4,
        }
    }
}

/// Counters the player never spends but the game reports on.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStats {
    pub hauls_completed: u64,
    pub crafts_completed: u64,
    pub items_harvested: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub tick: u64,
    pub seed: u64,
    pub speed: SimSpeed,
    pub rng: RngStreams,
    pub world: World,
    pub tower: Tower,
    pub crew: Vec<Crew>,
    pub stats: RunStats,
    /// Monotonic allocators. Never reuse an ID, even after removal —
    /// a stale reference should fail to resolve, not silently alias.
    pub next_room_id: u32,
    pub next_crew_id: u32,
    pub next_shaft_id: u32,
}

impl GameState {
    /// A fresh run: the starting tower, its starting crew, and the
    /// first stretch of terrain already streamed in.
    #[must_use]
    pub fn new(seed: u64, content: &Content) -> Self {
        let mut rng = RngStreams::new(seed);
        let balance = &content.balance;

        let mut state = Self {
            tick: 0,
            seed,
            speed: SimSpeed::Paused,
            world: World::new(&mut rng.world, content),
            tower: Tower::new(content),
            crew: Vec::new(),
            stats: RunStats::default(),
            rng,
            next_room_id: 1,
            next_crew_id: 1,
            next_shaft_id: 1,
        };

        state.place_starting_rooms(content);
        state.place_starting_crew(content);
        state.stock_starting_shelves(content);

        let _ = balance;
        state
    }

    /// The opening tower, laid out so the very first haul is a climb:
    /// the cutter arm is on the ground and the mill it feeds is two
    /// floors up.
    fn place_starting_rooms(&mut self, content: &Content) {
        const LAYOUT: [(&str, u8, u8); 4] = [
            ("room.heartseed", 0, 1),
            ("room.cutter_arm", 0, 4),
            ("room.storeroom", 1, 3),
            ("room.mill", 2, 3),
        ];
        for (room_id, floor, slot) in LAYOUT {
            let Some(idx) = content.room_idx(room_id) else {
                continue;
            };
            let id = RoomId(self.next_room_id);
            self.next_room_id += 1;
            let room = Room::new(id, idx, slot, content);
            if let Some(target) = self.tower.floors.get_mut(floor as usize) {
                target.rooms.push(room);
                target.rooms.sort_by_key(|r| r.slot);
            }
        }
    }

    fn place_starting_crew(&mut self, content: &Content) {
        // Names are placeholders until M4 gives the crew personalities.
        const NAMES: [&str; 8] = [
            "Wren", "Odile", "Bakri", "Sena", "Toma", "Ilay", "Rook", "Mira",
        ];
        for i in 0..content.balance.crew.starting_crew {
            let id = CrewId(self.next_crew_id);
            self.next_crew_id += 1;
            let fidget = (self.rng.cosmetic.next_u32() & 0xFFFF) as u16;
            self.crew.push(Crew::new(
                id,
                NAMES[i as usize % NAMES.len()].to_string(),
                fidget,
            ));
        }
    }

    fn stock_starting_shelves(&mut self, content: &Content) {
        for entry in &content.balance.tower.starting_stock {
            let Some(item) = content.item_idx(&entry.item) else {
                continue;
            };
            let mut remaining = entry.amount;
            for floor in &mut self.tower.floors {
                for room in &mut floor.rooms {
                    remaining -= room.shelve(item, remaining);
                    if remaining == 0 {
                        break;
                    }
                }
                if remaining == 0 {
                    break;
                }
            }
        }
    }

    /// Total of `item` across every storeroom shelf. This is what
    /// construction spends: the chain pays for the tower.
    #[must_use]
    pub fn stock_of(&self, item: ItemIdx) -> i64 {
        self.tower
            .floors
            .iter()
            .flat_map(|f| f.rooms.iter())
            .flat_map(|r| r.shelves.iter())
            .filter(|s| s.item == Some(item))
            .map(|s| s.count)
            .sum()
    }

    /// Remove `amount` of `item` from shelves, lowest floor first.
    /// Callers must have checked [`Self::stock_of`] already; this
    /// returns how much it actually took.
    pub fn take_stock(&mut self, item: ItemIdx, amount: i64) -> i64 {
        let mut remaining = amount;
        for floor in &mut self.tower.floors {
            for room in &mut floor.rooms {
                for shelf in &mut room.shelves {
                    if remaining == 0 {
                        return amount;
                    }
                    if shelf.item != Some(item) {
                        continue;
                    }
                    let taken = shelf.count.min(remaining);
                    shelf.count -= taken;
                    remaining -= taken;
                    if shelf.count == 0 {
                        shelf.item = None;
                    }
                }
            }
        }
        amount - remaining
    }

    pub fn alloc_room_id(&mut self) -> RoomId {
        let id = RoomId(self.next_room_id);
        self.next_room_id += 1;
        id
    }

    pub fn alloc_shaft_id(&mut self) -> ShaftId {
        let id = ShaftId(self.next_shaft_id);
        self.next_shaft_id += 1;
        id
    }
}
