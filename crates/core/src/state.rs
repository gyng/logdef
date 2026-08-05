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

use crate::content::{Content, Shift};
use crate::fx::Paces;
use crate::ids::{CrewId, ItemIdx, RoomId, ShaftId};
use crate::rng::RngStreams;

pub use clock::Clock;
pub use crew::{Crew, CrewState, Errand, HaulDestination, HaulPickup, HaulTask, Job};
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
    /// Meals eaten. The kitchen chain's own throughput figure, and the
    /// one number that says whether a tower is feeding itself — a
    /// starving tower and a tower with no canteen at all look identical
    /// from every other counter.
    pub meals_eaten: u64,
    /// What was harvested, per item.
    ///
    /// **`items_harvested` stopped being a useful number at M5**, and
    /// this is why: with bamboo from the shade, produce from the sun and
    /// fiber from the middle, two routes that harvest completely
    /// different things now report the same *total*. Measured on
    /// `examples/journey.rs`, a shade-seeking run and a sun-seeking one
    /// came out at 369 against 374 — which reads as "the route does not
    /// matter" and means the opposite. A sum over materials cannot see a
    /// change in the mix, and the mix is the whole of what M5 added.
    ///
    /// Indexed by `ItemIdx`, sized at run start, so it stays a pure
    /// function of the content pack.
    pub harvested_by_item: Vec<u64>,
    /// Loads taken out of an outbox by a thief. Not damage and not a
    /// haul — work the tower did and did not get to keep.
    pub items_stolen: u64,
    /// Crew-ticks spent asleep. What sleep actually costs the economy,
    /// which is the largest single unknown M4 introduces
    /// (`SYSTEMS.md` §4.10) and not something any existing counter sees.
    pub crew_ticks_asleep: u64,
    /// Fuel put up the chimney.
    ///
    /// **A sink the conservation test could not see.** M6 cut the sails
    /// and put a burner on the opening roof, which made bamboo vanish
    /// legitimately for the first time in a tower nobody had built
    /// anything in — `hauling_never_creates_or_destroys` read it as
    /// items being destroyed, which is exactly what it is for.
    /// Crafting was already accounted; burning was not, because until
    /// M6 no starting tower burned.
    ///
    /// It is also the number that says what charge cost this run,
    /// which no other counter reports: `charge` is a level, not a bill.
    pub fuel_burned: u64,
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
    /// What each enclave has left, one board per region and one entry
    /// per authored offer, and how many people are still willing to come
    /// aboard at each.
    ///
    /// Held in state rather than read from content because a trade
    /// spends it: an enclave is somewhere a run passes through once, not
    /// a shop that restocks.
    ///
    /// **Indexed by region, from M5.** It was a single board while there
    /// was a single enclave, and the comment here said it would become a
    /// list per enclave when a second landed. A second landed.
    pub enclave_stock: Vec<Vec<i64>>,
    pub enclave_recruits: Vec<u8>,
    /// Who the settlement the tower is standing at is offering.
    ///
    /// **A recruit is a person now, not a purchase** (`SYSTEMS.md`
    /// §6.29). It used to be: pay the price, receive the next name off
    /// the list. Forty traits existed and you never chose between them,
    /// because you never saw one before you paid.
    ///
    /// Drawn once when the tower berths and held until it walks on, so
    /// it cannot be rerolled by stepping away and back — the person
    /// standing there is the person standing there. `None` away from a
    /// settlement, or once its recruits are spent.
    #[serde(default)]
    pub recruit_offer: Option<crate::ids::TraitIdx>,
    /// How many more times each settlement will plate the shell. Only
    /// one of them does any, but the shape follows the others.
    pub shell_work_left: Vec<u8>,
    /// Which shift the clock says is awake. Held only so the handover
    /// can be noticed and sounded once for the tower rather than once
    /// per crew member; every other reader derives it from the daypart.
    pub shift_now: Shift,
    /// What kind of work an idle crew member reaches for first.
    ///
    /// **One order for the whole tower, not a rota per person.** A
    /// per-person matrix is the shape that turns crew into a
    /// spreadsheet, and it answers a question the player is rarely
    /// asking: what they want to say is *stop mending and get the
    /// harvest in*, which is one sentence about the tower. Somebody who
    /// should be doing one specific thing has `stationed` for that
    /// already, and it is per-person precisely because it is the
    /// exception.
    ///
    /// Needs are not in here — see `Job`. `serde(default)` is not
    /// enough for a `Vec` that must be a full permutation, so old saves
    /// get the default order through `work_order` below.
    #[serde(default = "default_work_order")]
    pub work: Vec<Job>,
    pub stats: RunStats,
    /// Monotonic allocators. Never reuse an ID, even after removal —
    /// a stale reference should fail to resolve, not silently alias.
    pub next_room_id: u32,
    pub next_crew_id: u32,
    pub next_shaft_id: u32,
    pub next_enemy_id: u32,
}

/// The order an untouched tower works in. See `Job` for the argument.
fn default_work_order() -> Vec<Job> {
    Job::ALL.to_vec()
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
            clock: Clock::new(content),
            power: Power::new(balance.power.starting_charge),
            world: World::new(&mut rng.world, content),
            tower: Tower::new(content),
            siege: Siege::new(),
            crew: Vec::new(),
            walking: true,
            strode: false,
            paces_last: 0,
            // `Clock::new` starts the run exactly at the handover onto
            // the day shift, so this is Day by construction — and
            // seeding it correctly is what stops tick 1 emitting a
            // spurious `ShiftChange` for a handover that already
            // happened before the run began.
            shift_now: Shift::Day,
            work: default_work_order(),
            stats: RunStats {
                // One slot per item in the pack, so a harvest counter is
                // never a lookup that can miss.
                harvested_by_item: vec![0; content.items.len()],
                ..RunStats::default()
            },
            rng,
            next_room_id: 1,
            next_crew_id: 1,
            next_shaft_id: 2,
            next_enemy_id: 1,
            arrived: false,
            // One entry per region, whether or not that region has a
            // settlement in it. Indexing by region rather than by "the
            // nth enclave" means a command never has to work out which
            // enclave it is talking about — the world already knows
            // which region the tower is standing in.
            enclave_stock: content
                .regions
                .iter()
                .map(|region| {
                    region.enclave.as_ref().map_or_else(Vec::new, |enclave| {
                        enclave.offers.iter().map(|offer| offer.stock).collect()
                    })
                })
                .collect(),
            recruit_offer: None,
            enclave_recruits: content
                .regions
                .iter()
                .map(|region| region.enclave.as_ref().map_or(0, |e| e.recruits))
                .collect(),
            shell_work_left: content
                .regions
                .iter()
                .map(|region| {
                    region
                        .enclave
                        .as_ref()
                        .and_then(|e| e.reinforce.as_ref())
                        .map_or(0, |work| work.times)
                })
                .collect(),
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
    ///
    /// **The burner sits beside the mill, and it is the only income
    /// there is.** M6 cut the sails, so nothing arrives for free — the
    /// same bamboo the mill wants is what keeps the lamps on, and the
    /// two of them share a floor and an argument about every stalk the
    /// arm cuts. The starting charge buys about 2,800 ticks of grace
    /// before that haul has to be working.
    ///
    /// **Beside the mill and not on the roof**, which is where the
    /// sails were and where the first version put it. The roof is the
    /// most attackable deck there is — leapers land on it — and a
    /// tower whose entire power supply sits up there loses all of it
    /// to one creature. Measured on `examples/probe.rs`: wrecked at
    /// hp 0/260 by tick 7,200, six burns for the whole run, and a
    /// charge curve sliding to nothing. The burner's own `min_floor`
    /// already said 2.
    ///
    /// It also leaves the roof empty, which it never was before. The
    /// first thing a player decides about the top deck is now a
    /// decision rather than a demolition.
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
        for entry in &content.balance.tower.starting_rooms {
            let (room_id, floor, slot) = (entry.room.as_str(), entry.floor, entry.slot);
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

    /// The three the tower sets out with.
    ///
    /// **They have no traits, and that is a decision about the
    /// opening** (`SYSTEMS.md` §6.25). §6.11 rebuilt the first ten
    /// minutes around a build ladder precisely so a new player is not
    /// handed a roll they cannot read, and a run is now half an hour
    /// (§6.19) with a tower already one purchase short of a shaft.
    /// Three crew drawn from a table containing *Heavy-footed* and
    /// *Quick to tire* is a 40% swing on the most fragile part of the
    /// game, decided before the first pace — measured: the golden
    /// recorder's ropery slipped from tick 46,129 to 65,886 on one
    /// unlucky draw, and its lift stopped being affordable at all.
    ///
    /// So variety arrives with the people you *choose* to bring aboard.
    /// The roll still happens and is still discarded, so the `sim`
    /// stream advances identically whether or not this rule changes
    /// again.
    /// Draw one trait, weighted.
    ///
    /// **Weighted, so rare means rare.** A flat roll over forty traits
    /// has no rare ones by definition; the striking ones carry a tenth
    /// of a common one's weight, so a run's roster is mostly quirks with
    /// the occasional person you remember.
    ///
    /// On the `sim` stream, never `cosmetic` — a trait changes how fast
    /// somebody gets hungry and how much they carry, so it is economic
    /// (`DECISIONS.md` §2).
    pub fn roll_trait(&mut self, content: &Content) -> Option<crate::ids::TraitIdx> {
        if content.traits.is_empty() {
            return None;
        }
        let total: u64 = content.traits.iter().map(|def| u64::from(def.weight)).sum();
        let pick = if total == 0 {
            self.rng.sim.next_u32() as usize % content.traits.len()
        } else {
            let mut roll = u64::from(self.rng.sim.next_u32()) % total;
            let mut chosen = content.traits.len() - 1;
            for (at, def) in content.traits.iter().enumerate() {
                let weight = u64::from(def.weight);
                if roll < weight {
                    chosen = at;
                    break;
                }
                roll -= weight;
            }
            chosen
        };
        Some(crate::ids::TraitIdx(u16::try_from(pick).unwrap_or(0)))
    }

    /// The name the next person aboard will carry.
    ///
    /// Read rather than rolled — `add_crew` takes the next name by
    /// index, never from a stream, so an offer can name somebody
    /// without touching any RNG at all.
    #[must_use]
    pub fn next_crew_name(&self, content: &Content) -> String {
        let names = &content.crew_names;
        let index = (self.next_crew_id as usize).saturating_sub(1) % names.len();
        names[index].clone()
    }

    fn place_starting_crew(&mut self, content: &Content) {
        for _ in 0..content.balance.crew.starting_crew {
            self.add_crew(content);
            if let Some(member) = self.crew.last_mut() {
                member.traits.clear();
                member.practice = [0; 4];
            }
        }
    }

    /// Take somebody aboard, wherever they came from.
    ///
    /// Shared by the starting crew and by the enclave's recruit, so a
    /// hired hand is the same kind of thing as one the tower set out
    /// with — no second construction path to drift.
    /// Names come from `crew/names.ron` by index, never from a roll —
    /// see `content::CrewNames`. `fidget` is the one cosmetic draw, and
    /// the renderer's only source of per-person variety.
    pub fn add_crew(&mut self, content: &Content) {
        self.add_crew_with(content, None);
    }

    /// The same, with the trait already decided.
    ///
    /// A settlement's offer is drawn when the tower berths and shown on
    /// the board, so taking it has to hand over *that* person
    /// (`SYSTEMS.md` §6.29) rather than rolling a fresh one — otherwise
    /// the card was a lie. `None` rolls, which is every other caller.
    pub fn add_crew_with(&mut self, content: &Content, given: Option<crate::ids::TraitIdx>) {
        let names = &content.crew_names;
        let id = CrewId(self.next_crew_id);
        self.next_crew_id += 1;
        let index = (self.next_crew_id as usize).saturating_sub(2) % names.len();
        let fidget = (self.rng.cosmetic.next_u32() & 0xFFFF) as u16;
        let rested = content.balance.crew.rested_max_ticks;
        let mut member = Crew::new(id, names[index].clone(), fidget, rested);

        // **One trait, drawn on the `sim` stream** (`SYSTEMS.md` §6.25).
        // Not `cosmetic`, which is where `fidget` above comes from: a
        // trait changes how fast somebody gets hungry and how much they
        // carry, so it is economic, and the firewall in
        // `DECISIONS.md` §2 exists to keep the two apart. Recruiting
        // somebody perturbing the economy stream is correct — recruiting
        // *is* an economic act.
        if let Some(drawn) = given.or_else(|| self.roll_trait(content)) {
            member.traits.push(drawn);
        }

        // And whatever that trait already knows how to do. A rank
        // rather than a full ceiling: they have done this before, not
        // for years (`SYSTEMS.md` §6.17).
        for idx in &member.traits {
            if let Some(def) = content.traits.get(idx.get())
                && let Some(job) = def.practised_at
            {
                member.practice[job.index()] = content
                    .balance
                    .crew
                    .practice_per_rank
                    .saturating_mul(u32::from(def.practice_ranks));
            }
        }

        self.crew.push(member);
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
