//! `GameState` — the single source of truth.
//!
//! Split by domain: the world the tower walks through, the tower
//! itself, and the crew inside it. Everything here serialises, contains
//! no floating point, and iterates in a fixed order.

pub mod clock;
pub mod crew;
pub mod power;
pub mod siege;
pub mod tower;
pub mod world;

use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::fx::Paces;
use crate::ids::{CrewId, ItemIdx, RoomId, ShaftId};
use crate::rng::RngStreams;

pub use clock::Clock;
pub use crew::{Crew, CrewState, HaulDestination, HaulPickup, HaulTask, RepairJob};
pub use power::Power;
pub use siege::{DamageTarget, Enemy, EnemyState, Health, Siege};
pub use tower::{
    Car, CarDir, CarState, Floor, Room, Shaft, ShaftPriority, ShaftProgram, Shelf, Stack, Tower,
};
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
    /// Items taken in trade at an enclave.
    pub traded: u64,
    pub hp_repaired: u64,
    /// Poles consumed putting the tower back together. Tracked rather
    /// than derived, because a shift that finishes a nearly-mended
    /// panel heals less than a full shift but still costs one.
    pub repair_poles_spent: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub tick: u64,
    pub seed: u64,
    pub speed: SimSpeed,
    pub rng: RngStreams,
    pub clock: Clock,
    pub power: Power,
    pub world: World,
    pub tower: Tower,
    pub siege: Siege,
    pub crew: Vec<Crew>,
    /// Whether the legs are running. Halting banks the charge striding
    /// would have burned — the bank-or-burn decision in its simplest
    /// form. M3 turns this into a continuous throttle.
    pub walking: bool,
    /// Whether the legs actually ran last tick. `walking` is the
    /// player's intent; a tower that cannot afford the charge still
    /// stands still, and things clinging to it are not shaken off.
    pub strode: bool,
    /// How far the legs actually carried the tower last tick, in Q8.8
    /// paces. Zero whenever `strode` is false.
    ///
    /// The quantitative sibling of `strode`, and read the same way:
    /// one tick late. Intake accrues against ground covered
    /// (`SYSTEMS.md` §3.6) but runs fourth in the tick, while stride
    /// runs eleventh — and stride's place at the end is not
    /// negotiable, because tick order *is* charge priority and walking
    /// is the first thing a tower short of power gives up (§1.6). So
    /// stride writes this and intake reads it on the following tick.
    /// The lag is deterministic and imperceptible at 30 Hz; moving
    /// stride earlier to close it would reorder charge priority and
    /// invalidate every golden replay.
    pub paces_last: Paces,
    /// The tower has reached the far edge of the last region.
    ///
    /// One of the two ways a run ends, and the only one that is not a
    /// loss. Presented as an arrival rather than a victory — the game
    /// reports where the tower got to, it does not grade it
    /// (`DECISIONS.md` §8).
    pub arrived: bool,
    /// What the enclave has left, one entry per authored offer, and how
    /// many people are still willing to come aboard.
    ///
    /// Held in state rather than read from content because a trade
    /// spends it: an enclave is somewhere a run passes through once,
    /// not a shop that restocks.
    pub enclave_stock: Vec<i64>,
    pub enclave_recruits: u8,
    /// How many more times a settlement will plate the shell.
    pub shell_work_left: u8,
    pub stats: RunStats,
    /// Monotonic allocators. Never reuse an ID, even after removal —
    /// a stale reference should fail to resolve, not silently alias.
    pub next_room_id: u32,
    pub next_crew_id: u32,
    pub next_shaft_id: u32,
    pub next_enemy_id: u32,
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
            clock: Clock::new(),
            power: Power::new(balance.power.starting_charge),
            world: World::new(&mut rng.world, content),
            tower: Tower::new(content),
            siege: Siege::new(),
            crew: Vec::new(),
            walking: true,
            strode: false,
            paces_last: 0,
            stats: RunStats::default(),
            rng,
            next_room_id: 1,
            next_crew_id: 1,
            next_shaft_id: 2,
            next_enemy_id: 1,
            arrived: false,
            // Sized from whichever region carries the enclave. One
            // region has one, so there is one list; when a second
            // enclave lands this becomes a list per enclave and the
            // commands index by berth rather than by offer.
            enclave_stock: content
                .regions
                .iter()
                .find_map(|region| region.enclave.as_ref())
                .map(|enclave| enclave.offers.iter().map(|offer| offer.stock).collect())
                .unwrap_or_default(),
            enclave_recruits: content
                .regions
                .iter()
                .find_map(|region| region.enclave.as_ref())
                .map_or(0, |enclave| enclave.recruits),
            shell_work_left: content
                .regions
                .iter()
                .find_map(|region| region.enclave.as_ref())
                .and_then(|enclave| enclave.reinforce.as_ref())
                .map_or(0, |work| work.times),
        };

        state.place_starting_rooms(content);
        state.place_starting_crew(content);
        state.stock_starting_shelves(content);

        let _ = balance;
        state
    }

    /// The opening tower, laid out so the very first haul is a climb:
    /// the cutter arm is on the ground and the mill it feeds is two
    /// floors up. Sails go on the roof, where they will keep needing to
    /// be moved every time the tower grows.
    ///
    /// The slots are chosen to leave awkward gaps rather than tidy
    /// ones. Rooms are one, two or three wide, the stairs take slot 0
    /// off every floor, and what fits beside what is the layout
    /// question the whole tower pillar rests on — a starting tower that
    /// packed perfectly would teach the player it never has to be
    /// thought about.
    ///
    /// There is exactly one three-wide gap in the opening tower, on
    /// floor 1. That is deliberate: the salvage rig is three wide and
    /// wants a low floor, so a player who decides to work ruins finds
    /// there is precisely one place for it and has to give that place
    /// up for something else later. The ground floor has no such gap at
    /// all — the Heartseed and a cutter arm see to that — so the rig
    /// competes with the chain for the same scarce low deck.
    ///
    /// Nothing sits at slot 7 on any floor. A shaft costs a slot column
    /// on every floor it spans (`DESIGN.md` pillar 2), and the outboard
    /// edge is where one naturally goes — a starting tower that blocked
    /// it would make the first elevator a demolition job.
    fn place_starting_rooms(&mut self, content: &Content) {
        const LAYOUT: [(&str, u8, u8); 6] = [
            ("room.heartseed", 0, 1),
            ("room.cutter_arm", 0, 5),
            ("room.storeroom", 1, 2),
            ("room.cell_bank", 1, 1),
            ("room.mill", 2, 3),
            ("room.canopy_sails", 3, 4),
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
        for _ in 0..content.balance.crew.starting_crew {
            self.add_crew();
        }
    }

    /// Take somebody aboard, wherever they came from.
    ///
    /// Shared by the starting crew and by the enclave's recruit, so a
    /// hired hand is the same kind of thing as one the tower set out
    /// with — no second construction path to drift.
    pub fn add_crew(&mut self) {
        // Names are placeholders until M4 gives the crew personalities.
        const NAMES: [&str; 8] = [
            "Wren", "Odile", "Bakri", "Sena", "Toma", "Ilay", "Rook", "Mira",
        ];
        let id = CrewId(self.next_crew_id);
        self.next_crew_id += 1;
        let index = (self.next_crew_id as usize).saturating_sub(2) % NAMES.len();
        let fidget = (self.rng.cosmetic.next_u32() & 0xFFFF) as u16;
        self.crew
            .push(Crew::new(id, NAMES[index].to_string(), fidget));
    }

    /// Put `amount` of an item on whatever shelves will take it.
    /// Returns how much landed; the rest stays where it was.
    pub fn shelve(&mut self, item: ItemIdx, amount: i64) -> i64 {
        let mut remaining = amount;
        for floor in &mut self.tower.floors {
            for room in &mut floor.rooms {
                remaining -= room.shelve(item, remaining);
                if remaining == 0 {
                    return amount;
                }
            }
        }
        amount - remaining
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

    pub fn alloc_enemy_id(&mut self) -> crate::ids::EnemyId {
        let id = crate::ids::EnemyId(self.next_enemy_id);
        self.next_enemy_id += 1;
        id
    }
}
