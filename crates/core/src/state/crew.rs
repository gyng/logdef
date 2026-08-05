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
use crate::ids::{CrewId, FloorIdx, ItemIdx, RoomId, ShaftId, SlotIdx, TraitIdx};

/// A kind of work, and the unit the player's work order ranks.
///
/// **Four, and needs are not among them.** Sleeping and eating outrank
/// every one of these and are not reorderable, because they are not
/// jobs — a player who could rank hauling above dinner would only be
/// building the starvation trap, and offering it as a setting would be
/// the game pretending a mistake is a strategy.
///
/// The enum order is the default order, and it is an argument rather
/// than a habit: something happening now (a thief in the outbox) beats
/// something that already happened (a wrecked panel), which beats a
/// standing order (a post), which beats the background work that is
/// always there (a haul). A player who disagrees can say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Job {
    /// Go and stand between a thief and what it is taking.
    Answer,
    /// Mend damage.
    Mend,
    /// Go to the room you were posted to, and work it.
    Man,
    /// Carry something somewhere.
    Haul,
}

impl Job {
    /// Every job, in the default order. See the type doc for the
    /// argument the order makes.
    pub const ALL: [Self; 4] = [Self::Answer, Self::Mend, Self::Man, Self::Haul];

    /// This job's slot in a practice array.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Answer => 0,
            Self::Mend => 1,
            Self::Man => 2,
            Self::Haul => 3,
        }
    }

    /// What this variant serialises as — the spelling a `SetWorkOrder`
    /// has to use. Shipped in the catalog beside the display name so the
    /// frontend never keeps its own list of enum spellings to drift.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Answer => "Answer",
            Self::Mend => "Mend",
            Self::Man => "Man",
            Self::Haul => "Haul",
        }
    }

    /// What the tower calls it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Answer => "answering",
            Self::Mend => "mending",
            Self::Man => "working a post",
            Self::Haul => "hauling",
        }
    }

    /// The job somebody in this state is practising, if any.
    ///
    /// **Walking and climbing are hauling, wherever they are going.**
    /// What a porter learns is the building — which stair is quicker
    /// with a crate on, where the landings are — and that knowledge does
    /// not evaporate because this particular trip is toward a broken
    /// panel. Standing still and working is the job you are stood in.
    ///
    /// Idle, eating and sleeping teach nothing, which is why a tower
    /// with nothing to do is a tower that is not getting better at
    /// anything.
    #[must_use]
    pub const fn practised_by(state: &CrewState) -> Option<Self> {
        match state {
            CrewState::Walking { .. }
            | CrewState::Boarding { .. }
            | CrewState::Climbing { .. }
            | CrewState::Riding { .. }
            | CrewState::Loading { .. }
            | CrewState::Unloading { .. } => Some(Self::Haul),
            CrewState::Repairing { .. } => Some(Self::Mend),
            CrewState::Manning { .. } => Some(Self::Man),
            CrewState::Shooing { .. } => Some(Self::Answer),
            CrewState::Idle | CrewState::Eating { .. } | CrewState::Sleeping => None,
        }
    }
}

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
    /// Standing in a room something is taking from, until it leaves.
    ///
    /// **Nobody fights.** `DECISIONS.md` §8 has defenders rather than
    /// soldiers, and creatures defending their territory rather than a
    /// gallery to clear — so what a person does about a thief is *be
    /// there*. A crow that finds somebody in the room goes, the way it
    /// would if you walked into your own kitchen.
    Shooing {
        ticks_left: u32,
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
    /// Does the posting end when this person runs out of energy?
    ///
    /// **The difference between a job and a push.** M6's stationing is a
    /// standing order: somebody works a room until the player says
    /// otherwise. This is the other thing a player wants — *everybody
    /// on the mill, now* — and it would be a trap if it were also
    /// permanent, because the moment you stop looking it is still true
    /// and half the crew are standing in a room nobody remembers
    /// sending them to.
    ///
    /// So it expires by itself, on the one clock that already means
    /// "this person has given what they have": `tired_ticks`. A push
    /// lasts the rest of somebody's shift and no longer.
    ///
    /// `serde(default)` so replays recorded before it still load.
    #[serde(default)]
    pub post_until_tired: bool,
    /// A kit this person is carrying, if the tower has lent them one.
    ///
    /// **Held, not consumed.** The item leaves the shelves while it is
    /// in somebody's hands and goes back when they hand it in, so a kit
    /// is a decision you can take back rather than a purchase you
    /// regret. It survives sleeping, eating and a wave — it is theirs
    /// until you say otherwise.
    pub kit: Option<ItemIdx>,
    /// Ticks of practice at each job, indexed by `Job::index`.
    ///
    /// **Earned by doing, never spent, and capped.** There is no screen
    /// where a player assigns points, because the moment there is one
    /// the crew are a build to optimise rather than people who have
    /// been here a while. Practice accrues from the work somebody
    /// actually did, tops out, and is read as a rank rather than as a
    /// number the player is meant to be counting.
    ///
    /// It cannot be lost. Somebody who spent a week on the stairs and is
    /// now stood at a gun still knows the stairs — that is what makes it
    /// a fact about a person rather than a slider.
    ///
    /// `serde(default)` so replays recorded before it still load.
    #[serde(default)]
    pub practice: [u32; 4],
    /// What is true about this person, drawn when they came aboard.
    ///
    /// **Not a cosmetic draw**, unlike `fidget` below, and the
    /// difference is the firewall (`DECISIONS.md` §2): a trait changes
    /// how fast somebody gets hungry and how much they carry, so it is
    /// economic and rolls on the `sim` stream. A name is cosmetic and a
    /// trait is not, and putting them on the same stream would let
    /// renaming somebody move an economic roll.
    ///
    /// A `Vec` holding at most one today. The shape is here so a second
    /// trait is a content change rather than a save-format change.
    ///
    /// `serde(default)` so replays recorded before it still load.
    #[serde(default)]
    pub traits: Vec<TraitIdx>,
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
    /// A room something is taking from. Go and stand in it.
    Shoo { floor: FloorIdx, slot: SlotIdx },
}

impl Errand {
    #[must_use]
    pub const fn floor(&self) -> FloorIdx {
        match self {
            Self::Repair { floor, .. }
            | Self::Meal { floor, .. }
            | Self::Bunk { floor, .. }
            | Self::Station { floor, .. }
            | Self::Shoo { floor, .. } => *floor,
        }
    }

    #[must_use]
    pub const fn slot(&self) -> SlotIdx {
        match self {
            Self::Repair { slot, .. }
            | Self::Meal { slot, .. }
            | Self::Bunk { slot, .. }
            | Self::Station { slot, .. }
            | Self::Shoo { slot, .. } => *slot,
        }
    }

    /// The room this errand names, for the two arms that name one.
    /// Bunk occupancy is counted by scanning these rather than stored on
    /// the room, so there is no counter to get out of step with reality.
    #[must_use]
    pub const fn room(&self) -> Option<RoomId> {
        match self {
            Self::Repair { .. } | Self::Shoo { .. } => None,
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
            post_until_tired: false,
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
            kit: None,
            practice: [0; 4],
            traits: Vec::new(),
            fidget,
        }
    }

    /// How practised this person is at a job, in ranks.
    ///
    /// Zero is "new to it", and the ceiling is `max_rank`. Integer
    /// division, so the rank a player sees and the bonus the simulation
    /// applies are the same fact — a pip on the card is not a rounding
    /// of something finer going on underneath.
    #[must_use]
    pub fn rank(&self, job: Job, content: &crate::content::Content) -> u8 {
        let balance = &content.balance.crew;
        if balance.practice_per_rank == 0 {
            return 0;
        }
        u8::try_from(self.practice[job.index()] / balance.practice_per_rank)
            .unwrap_or(u8::MAX)
            .min(balance.max_rank)
    }

    /// Put a tick of practice in, up to the ceiling.
    ///
    /// Stopping at the ceiling rather than letting the count run on is
    /// deliberate: an uncapped counter is a number that keeps going up
    /// forever with nothing attached to it, which is exactly the kind of
    /// stat this design does not want lying around in a save file.
    pub fn practise(&mut self, job: Job, content: &crate::content::Content) {
        let balance = &content.balance.crew;
        let ceiling = balance
            .practice_per_rank
            .saturating_mul(u32::from(balance.max_rank));
        let slot = &mut self.practice[job.index()];
        *slot = slot.saturating_add(1).min(ceiling);
    }

    /// Fold every trait's percentage into one figure, as a percentage.
    ///
    /// Multiplicative rather than additive, so two traits that each
    /// halve something halve it twice — and so a pack that adds a
    /// sixth trait cannot accidentally make a stat go negative.
    #[must_use]
    pub fn trait_pct(
        &self,
        content: &crate::content::Content,
        of: impl Fn(&crate::content::TraitDef) -> i64,
    ) -> i64 {
        self.traits
            .iter()
            .filter_map(|idx| content.traits.get(idx.get()))
            .fold(100, |acc, def| acc * of(def).max(0) / 100)
    }

    /// Items this person carries on top of `carry_capacity`, from their
    /// traits. May be negative: somebody who is a poor porter is
    /// somebody you post to a room.
    #[must_use]
    pub fn trait_carry_bonus(&self, content: &crate::content::Content) -> i64 {
        self.traits
            .iter()
            .filter_map(|idx| content.traits.get(idx.get()))
            .map(|def| def.carry_bonus)
            .sum()
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
