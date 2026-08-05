//! The content pack: items, rooms, terrain, and balance, loaded from
//! RON and interned for the simulation.
//!
//! Two rules shape this module.
//!
//! **String IDs in data, dense indices at runtime.** Designers write
//! `"item.bamboo"`; the simulation carries an `ItemIdx(0)`. Interning
//! happens exactly once, here, at load. The pack's byte content is
//! hashed into `content_hash`, which is stamped into every replay so a
//! recording can never be silently verified against different data.
//!
//! **No floating point in the data either.** Rates are expressed the
//! way a designer thinks about them — "ticks per item", "paces per 100
//! ticks", "percent yield" — as integers, and converted to `Fx` at the
//! point of use. See `BALANCE.md`.

use std::borrow::Cow;

use include_dir::{Dir, DirEntry, include_dir};
use ron::de::from_bytes;
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::Xxh3;

use crate::fx::Fx;
// Validation asks the two intake systems what a rate comes out as,
// rather than restating their arithmetic and letting the two drift.
use crate::ids::{
    BranchIdx, DaypartIdx, EnemyIdx, ItemIdx, RegionIdx, RoomIdx, ShaftIdx, TerrainIdx,
};
use crate::systems::intake;

static EMBEDDED_DATA: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../assets/data");

// ---------------------------------------------------------------------------
// Definitions as authored
// ---------------------------------------------------------------------------

/// One material or product. Items are pure identity plus presentation;
/// all behaviour lives in the rooms that make and consume them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDef {
    pub id: String,
    pub name: String,
    /// Single glyph used by the cross-section. Presentation only.
    pub glyph: String,
    /// Sort order in the stock readout. Lower comes first.
    pub order: u16,
    /// What this changes about the person carrying it, if anything.
    #[serde(default)]
    pub kit: Option<KitDef>,
}

/// A tool one named person carries, and what it changes about their day.
///
/// **This is what "equip" means here.** The obvious reading — weapon
/// loadouts bolted to the tower — is the wrong game, and emplacements
/// already have their half of it: what you feed a dart battery *is* the
/// choice. A kit is the other half, and it belongs to a person rather
/// than to the building, because `DESIGN.md` §2 structural call 4 says
/// crew are named individuals and not stat blocks. A kit is the
/// smallest mechanic that makes that true in the simulation rather than
/// only in the fiction.
///
/// One item, one person, drawn off the shelves like any build cost and
/// put back when it is handed in. **Not consumed**: a kit is something
/// the tower owns and lends out, so equipping is a decision you can take
/// back rather than a purchase you regret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KitDef {
    /// Extra items carried per trip, on top of `carry_capacity`.
    #[serde(default)]
    pub carry_bonus: i64,
    /// Repair work done per shift, in percent of normal.
    #[serde(default = "hundred")]
    pub mend_pct: i64,
    /// Whether the dark stops slowing this person down. Answers
    /// `dark_work_pct` directly — the penalty a brown-out applies to
    /// everybody who is not carrying a light.
    #[serde(default)]
    pub lights_the_dark: bool,
}

const fn hundred() -> i64 {
    100
}

/// What kind of thing a room is, for grouping in the build menu and for
/// the handful of places the simulation needs to ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomCategory {
    /// Harvests from the terrain the tower is walking through.
    Intake,
    /// Consumes inputs and emits outputs on a timer.
    Production,
    /// Holds items on shelves. The tower's stock, and what construction
    /// draws from.
    Storage,
    /// Makes, burns for, or stores charge.
    Energy,
    /// Shoots back. Fed by the chain like anything else.
    Defence,
    /// Somewhere to sleep. The only room whose whole purpose is that a
    /// person is in it — no recipe, no stock, no reach.
    Quarters,
    /// The Heartseed. Unique, pre-placed, and the loss condition.
    Heart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostEntryDef {
    pub item: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IoEntryDef {
    pub item: String,
    /// Consumed (or produced) per craft.
    pub amount: i64,
    /// Buffer size for this slot.
    pub buffer_max: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeDef {
    /// Ticks of uninterrupted work per craft.
    pub craft_ticks: u32,
    /// At most two, by the legibility rule in `v2-plan.md` §6.1.
    pub inputs: Vec<IoEntryDef>,
    pub outputs: Vec<IoEntryDef>,
}

/// Where an intake room draws from. **Walking harvests bamboo; stopping
/// harvests scrap** (`SYSTEMS.md` §3.4) — the two sources are exact
/// opposites, and expressing that in content rather than as a special
/// case in `intake::run` is what keeps a third source cheap to author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntakeSource {
    /// Accrues against the ground the tower covers, so a stopped tower
    /// harvests nothing at all. Scaled by the yield of the band
    /// underfoot.
    Terrain { paces_per_item: i64 },
    /// Accrues per tick, and only while the tower is stopped with a
    /// ruin inside `range_paces`. Never scaled by terrain yield — a rig
    /// is not drawing from the band, it is drawing from the ruin.
    Ruin {
        ticks_per_item: u32,
        range_paces: i64,
    },
    /// Accrues per tick, scaled by the sun actually reaching the tower.
    ///
    /// **The first source that does not care whether the tower is
    /// moving**, and that is the whole design of it. `Terrain` measures
    /// ground covered and `Ruin` requires a stop, so between them a
    /// tower is always giving something up: berthing at a ruin costs the
    /// harvest, and walking past one costs the salvage. A `Sun` source
    /// runs at full rate either way, which is what finally makes
    /// standing still cost less than everything (`SYSTEMS.md` §5.2).
    ///
    /// Scaled by *exposure* — sun after terrain — so the same number
    /// that decides what the sails make decides what a garden grows, and
    /// the open branches buy something that is not charge.
    Sun { ticks_per_item: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeDef {
    pub item: String,
    pub source: IntakeSource,
    pub buffer_max: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageDef {
    /// Number of independent shelves. Each takes whichever item lands
    /// on it first and holds only that item until emptied.
    pub shelves: u8,
    pub per_shelf: i64,
}

/// The burner: the dirty fallback. Turns the contested material into
/// power, and from M2 its smoke raises provocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BurnerDef {
    pub fuel: String,
    pub fuel_per_burn: i64,
    pub charge_per_burn: i64,
    pub burn_ticks: u32,
}

/// A cell bank. Storage is infrastructure: capacity is something you
/// build, not something you find.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BankDef {
    pub capacity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomDef {
    pub id: String,
    pub name: String,
    /// Two or three characters for the cross-section label.
    pub short: String,
    pub category: RoomCategory,
    pub width: u8,
    #[serde(default)]
    pub build_cost: Vec<CostEntryDef>,
    /// Highest floor this room may be placed on. Intake reaches the
    /// ground, so cutter arms live low.
    #[serde(default)]
    pub max_floor: Option<u8>,
    /// Lowest floor this room may be placed on.
    ///
    /// **The mirror of `max_floor`, and it exists to give the chain a
    /// direction.** Three intake rooms are pinned to the bottom because
    /// they reach the ground, and until this nothing was pinned to the
    /// top — so a whole chain could sit on floors 0 and 1 beside its own
    /// intake and never move anything vertically. A tower that never
    /// hauls upward has no use for a shaft, whatever a shaft costs.
    ///
    /// **Used sparingly and only where the fiction already says so**, a
    /// smoke stack and a sleeping deck. It is a placement rule, not a
    /// tax: most rooms go anywhere, and the interesting layout question
    /// is which of them you choose to put where.
    #[serde(default)]
    pub min_floor: Option<u8>,
    /// Only works on the tower's top floor. Building above it puts it
    /// in shade — which is the price of height, made concrete.
    #[serde(default)]
    pub top_floor_only: bool,
    /// Only one may exist in a tower.
    #[serde(default)]
    pub unique: bool,
    /// Charge drawn per tick while this room is working.
    #[serde(default)]
    pub power_draw: i64,
    #[serde(default)]
    pub recipe: Option<RecipeDef>,
    #[serde(default)]
    pub intake: Option<IntakeDef>,
    /// The room that has to be standing before this one can be built.
    ///
    /// **The opening five minutes, and the only reason it exists.** A
    /// first-turn build menu with twenty cards on it is the loudest
    /// thing in the game and it says nothing; this makes the menu open
    /// out as the player builds, so the first decision is one decision.
    ///
    /// It is a fact about the *tower*, not about the player — it reads
    /// off `GameState`, so it is deterministic, replay-safe, and
    /// validated at the command boundary like everything else. That is
    /// the difference between this and the journal (§5.7), which is
    /// player-level, never enters `GameState`, and can only ever filter
    /// a menu.
    /// Must be built on the tower's leading edge.
    ///
    /// **What makes a weapon a weapon.** A cutter arm and an
    /// emplacement both reach *out* of the tower, and a thing that
    /// reaches out has to be on the outside — which on a cross-section
    /// means the front, because that is the edge the tower is walking
    /// into and the edge everything arrives from.
    ///
    /// It is also the constraint that makes the weapon bar honest: the
    /// FTL-shaped readout in the UI lists what the tower can point at
    /// something, and a list is only a list if the things on it are
    /// somewhere specific.
    #[serde(default)]
    pub front_only: bool,
    /// Damage this room deals to anything clinging within its reach,
    /// per tick of work.
    ///
    /// **The cutter arm's other half.** It is a blade on a boom that
    /// strips bamboo off the ground; a creature that climbs into that
    /// arc has climbed into a blade on a boom. Dual use costs no new
    /// verb and no new room — the player who built an arm to harvest
    /// has already built the thing that answers a skitter, and finding
    /// that out is a better moment than being sold a weapon.
    ///
    /// Not an emplacement: there is no ammo, no reload and no range
    /// beyond the tower's own skin. It hits what is *on* the tower.
    #[serde(default)]
    pub melee_damage: i64,
    #[serde(default)]
    pub unlocked_by: Option<String>,
    /// Crew who must be posted here for the room to work at all.
    ///
    /// Zero for everything that merely *benefits* from being staffed —
    /// M6's `manned_work_pct` is the bonus, and this is a requirement.
    /// The farm is the one room that has it, because the opening should
    /// teach that rooms are run by people before it teaches anything
    /// else.
    #[serde(default)]
    pub crew_required: u8,
    #[serde(default)]
    pub storage: Option<StorageDef>,
    #[serde(default)]
    pub burner: Option<BurnerDef>,
    #[serde(default)]
    pub bank: Option<BankDef>,
    #[serde(default)]
    pub defence: Option<DefenceDef>,
    #[serde(default)]
    pub quarters: Option<QuartersDef>,
}

/// Beds. `content::validate` requires every category to have a matching
/// behaviour block, and a bunk has no recipe, no storage, no intake and
/// nothing to shoot with — so this exists to be that block, and to say
/// how many people fit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuartersDef {
    pub sleepers: u8,
}

/// Which mechanism moves things up and down a shaft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShaftKind {
    /// Free, always present, slow, crew-only, one body at a time.
    Stairs,
    /// Item-only and autonomous. The inserter.
    Dumbwaiter,
    /// The machine: cars, queues, dwell, and a programmable schedule.
    Elevator,
    /// One way, down, and out.
    ///
    /// No cars, no capacity, no charge and no riders — things fall. It
    /// ends at a spill gate on the ground floor and whatever goes in is
    /// gone, which is the point: `BALANCE.md`'s `storeroom` row has
    /// described the shelf-typing deadlock since M2 and handed the fix
    /// forward twice, and this is it (`SYSTEMS.md` §5.4). A surplus that
    /// has claimed every shelf can be thrown away, visibly, by a piece
    /// of infrastructure the player chose to build.
    Chute,
}

/// A kind of vertical transport, as authored. Making these content
/// rather than an enum arm apiece is what lets M5 add the chute and the
/// pneumatic tube without touching the simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaftDef {
    pub id: String,
    pub name: String,
    pub short: String,
    pub kind: ShaftKind,
    #[serde(default)]
    pub build_cost: Vec<CostEntryDef>,
    /// Floors spanned, inclusive of both ends. `max_span` of 0 means
    /// "as tall as the tower".
    pub min_span: u8,
    pub max_span: u8,
    /// Stairs: crew on the flight at once. Elevator: car capacity in
    /// units, where a crew member is one and a carried load is another.
    pub capacity: u8,
    pub ticks_per_floor: u32,
    #[serde(default)]
    pub charge_per_floor: i64,
    /// Cars in the shaft. Zero for stairs.
    #[serde(default)]
    pub cars: u8,
    /// Items a dumbwaiter moves per trip.
    #[serde(default)]
    pub batch: i64,
}

/// How a creature reaches the tower, and therefore what it threatens.
/// Each arm is a lesson: ground teaches ammo economics, canopy teaches
/// that height is exposure, burrow teaches transport redundancy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Approach {
    /// Walks up to the tower and attacks the lowest floor's panel.
    Ground,
    /// Drops from overhanging trees onto the *upper* decks.
    Canopy,
    /// Goes for the legs and the shaft columns.
    Burrow,
}

/// A creature. They are not a target gallery — they defend their
/// territory, and the tower is the thing passing through it
/// (`DECISIONS.md` §8).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyDef {
    pub id: String,
    pub name: String,
    /// Single glyph for the terrain layer. Presentation only.
    pub glyph: String,
    pub hp: i64,
    /// Approach speed in whole paces per 100 ticks.
    pub speed_paces_per_100_ticks: i64,
    pub damage: i64,
    pub attack_ticks: u32,
    /// How long it will hold on once it has hold of something, while
    /// the tower is striding. A stopped tower shakes nothing off, so
    /// this timer only runs while the legs do — which is what makes
    /// "walk it off" a real answer to a wave and stopping a real risk.
    pub cling_ticks: u32,
    pub approach: Approach,
    /// Cost against a wave's threat budget.
    pub threat: i64,
    /// Provocation at or above which this creature starts appearing.
    /// Keeps the opening of a run gentle without a difficulty setting.
    #[serde(default)]
    pub min_provocation: i64,
    /// Only shows up after dark. The reason you banked charge.
    #[serde(default)]
    pub night_only: bool,
    /// Whether ordinary provocation-driven waves may draw this
    /// creature. Defaults to yes, because opting out is the exception.
    ///
    /// A feral warden is not summoned by attention — it is summoned by
    /// berthing at the ruin it guards (`SYSTEMS.md` §3.4) — so it says
    /// so here rather than being fenced off with an out-of-range
    /// `min_provocation`, which would be a number pretending to be a
    /// rule.
    #[serde(default = "wave_eligible_default")]
    pub wave_eligible: bool,
    /// Earliest region this creature appears in, by `order`.
    ///
    /// A second gate alongside `min_provocation`, and a different kind
    /// of one: provocation is a thing the player earns and a region is a
    /// place they have reached. The mire-hulk is the coast's own
    /// problem, and meeting one in the jungle because a run was loud
    /// would make region 3 less itself rather than more.
    #[serde(default)]
    pub min_region: u16,
    /// What it leaves behind when it goes down.
    ///
    /// **Only the residents carry anything**, and the reason is the
    /// tone gate (`DECISIONS.md` §8): creatures defend territory rather
    /// than being a gallery to clear, and a jungle where every skitter
    /// pays out is a jungle you farm. A drop is not loot — it is what a
    /// very large thing turns out to have been carrying, once, and it
    /// is the only reason to *stand and fight* rather than walk away.
    ///
    /// Which is the point. §11 makes walking the free answer to every
    /// wave, and a free answer with no alternative is not a decision.
    /// This is the alternative.
    #[serde(default)]
    pub drops: Vec<CostEntryDef>,
    /// Takes what is in an outbox instead of damaging what it lands on.
    ///
    /// The one creature shape that attacks the *chain* rather than the
    /// structure (`SYSTEMS.md` §5.5). A thief does no hit points of
    /// harm; what it costs is a morning's harvest, which repair cannot
    /// answer and defence can prevent.
    #[serde(default)]
    pub steals: bool,
    /// Percent the tower's stride is cut to while this is attached.
    ///
    /// Zero means no effect, which is every creature but one. A hulk
    /// takes hold of a leg, and a tower it is riding sheds it more
    /// slowly than it would anything else — so "keep walking", the
    /// answer to every other wave since M2, makes itself worse.
    #[serde(default)]
    pub drag_pct: i64,
}

const fn wave_eligible_default() -> bool {
    true
}

/// An emplacement: dart batteries and the like.
///
/// Ammo is an ordinary input stack, so feeding a battery is the same
/// job as feeding a mill and uses the same crew and the same shafts.
/// That equivalence is the design — an emplacement with its own private
/// supply mechanism would make combat a parallel game instead of a load
/// test on the one you are already playing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefenceDef {
    pub ammo: String,
    pub ammo_per_shot: i64,
    pub buffer_max: i64,
    pub damage: i64,
    pub reload_ticks: u32,
    /// How far out it can reach, in whole paces.
    pub range_paces: i64,
}

/// Which half of the rota a crew member works. Awake is "the current
/// daypart belongs to my shift" and nothing else — which is what makes
/// re-shifting a sleeper mid-night an all-hands lever with a real price,
/// out of nothing but the definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shift {
    Day,
    Night,
}

/// A named stretch of the day. The simulation only uses the index; the
/// name is for the player, and the boundaries are what the elevator's
/// per-daypart programs key off.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaypartDef {
    pub id: String,
    pub name: String,
    /// Per-mille of the day at which this daypart begins.
    pub start_permille: i64,
    /// Which shift is awake through this stretch. Content rather than a
    /// constant in a system, so the handover is a designer's decision —
    /// and validated as one contiguous band per shift, because a rota
    /// with two night stretches is a bug in the pack.
    pub shift: Shift,
}

/// A stretch of terrain with one character. In M0 a band's only
/// mechanical effect is its yield multipliers; M1 hangs sun on it too.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainDef {
    pub id: String,
    pub name: String,
    /// Percent multiplier applied to intake in this band.
    pub yield_pct: i64,
    /// Percent multiplier applied to sunlight reaching the sails.
    ///
    /// Deliberately opposed to `yield_pct`: shade is biomass-rich and
    /// sun-poor, open ruin-field is the reverse. That opposition is the
    /// whole of "your route is your power mix", and it only works if
    /// no band is good at both.
    pub sun_pct: i64,
    /// Features scattered per 100 paces of this band.
    pub features_per_100_paces: i64,
    /// Feature glyphs this band may scatter. Presentation only —
    /// except for the ones named in `ruin_kinds`, below.
    pub feature_kinds: Vec<String>,
    /// Which of `feature_kinds` are drowned ruins with something left
    /// in them (`SYSTEMS.md` §3.4). The canopy's ferns and the
    /// ruin-field's broken frames scatter through the same generator;
    /// this is the only thing that separates them.
    #[serde(default)]
    pub ruin_kinds: Vec<String>,
    /// Whole units of scrap one of this band's ruins holds, before the
    /// owning region's rolled ruin richness scales it. Both ends are
    /// ignored when `ruin_kinds` is empty.
    #[serde(default)]
    pub ruin_salvage_min: i64,
    #[serde(default)]
    pub ruin_salvage_max: i64,
}

// ---------------------------------------------------------------------------
// The journey
// ---------------------------------------------------------------------------

/// One entry in a terrain palette: how often this kind of band comes up
/// while the tower is walking through the region (or branch) that owns
/// the palette.
///
/// This replaced a pack-wide `TerrainDef.weight` at M3. How often a kind
/// comes up is a fact about *where you are*, not about the kind, and
/// leaving both knobs in place would have left one of them to go stale
/// (`SYSTEMS.md` §3.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainWeight {
    pub terrain: String,
    pub weight: i64,
}

/// One of the archetypes a route fork may offer.
///
/// **A branch is a palette override, not a detour** (`SYSTEMS.md` §3.3):
/// taking one replaces the region's palette for `length_paces` past the
/// fork and multiplies its `threat_pct`, then the route rejoins. There
/// is no second distance axis and no route tree in state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BranchDef {
    pub id: String,
    pub name: String,
    /// How far past the fork this palette holds.
    pub length_paces: i64,
    pub palette: Vec<TerrainWeight>,
    /// Multiplier on the owning region's own `threat_pct`.
    pub threat_pct: i64,
}

/// One posted exchange at an enclave. Finite `stock`, so a waystation is
/// a windfall rather than an exchange to farm, and every rate is
/// deliberately worse than the chain's own — the enclave is where a
/// tower that lacks a room buys around the gap once (`SYSTEMS.md` §3.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfferDef {
    /// What the tower hands over.
    pub give: CostEntryDef,
    /// What it gets back.
    pub take: CostEntryDef,
    /// How many times this offer may be taken across the whole run.
    pub stock: i64,
}

/// A settlement the tower walks past. Berthing works exactly as it does
/// at a ruin — stop within range — and the tower that keeps walking
/// loses it, because there is no going back down the axis.
/// Shell work: people who will plate a passing tower, for scrap.
///
/// The only permanent upgrade in the game, and deliberately the one
/// that closes salvage's loop — a ruin's scrap has nowhere else to go
/// but the trade board, so a run that berths late banks metal it cannot
/// spend. Plating turns it into hull, which is what a drowned city's
/// worth of old metal ought to become.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReinforceDef {
    pub cost: Vec<CostEntryDef>,
    /// Added to every floor's panel, present and future.
    pub panel_hp: i64,
    /// How many times this settlement will do it.
    pub times: u8,
}

/// Something the route puts in front of the tower once and then never
/// again.
///
/// **The answer to "the journey is a screensaver".** Between one fork
/// and the next the tower walked through scenery and decided nothing:
/// forks are rare by design (`fork_interval_paces`) and an enclave is a
/// whole settlement. A waypoint is the small beat in between — it comes
/// into range, asks one question, and goes past.
///
/// Three rules make it a beat rather than a chore:
///
/// - **Ignoring it is free.** There is no penalty branch. §11's rule
///   that walking is always available applies here too: the tower
///   walking on is the default and it is never wrong, only sometimes
///   less good.
/// - **It resolves in one click.** No sub-menu, no follow-up. A thing
///   that needs a decision *tree* is an enclave.
/// - **It is gone once passed.** There is no going back down the axis
///   (`SYSTEMS.md` §3.5), so a waypoint behind you is a thing that
///   happened rather than a thing you are still owed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointDef {
    pub id: String,
    pub name: String,
    /// One line, second person, read once as it comes into range.
    pub said: String,
    /// The label on the button.
    pub take: String,
    /// What taking it costs off the shelves. Empty for the ones that
    /// only ever give.
    pub costs: Vec<CostEntryDef>,
    /// What taking it puts on the shelves.
    pub gives: Vec<CostEntryDef>,
    /// Attention it draws. The one difficulty dial in the game
    /// (`SYSTEMS.md` §2.6), so a loud waypoint is a real price.
    pub provocation: i64,
    /// Ground gained, or lost if negative.
    ///
    /// **Negative is the usual case**, because the honest cost of
    /// stopping to do something is the walking you did not do — which
    /// is the currency the whole journey layer is denominated in.
    pub paces: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnclaveDef {
    pub id: String,
    pub name: String,
    /// Offset from the **start of the owning region**, not an absolute
    /// distance — a region's length is rolled per run, so an absolute
    /// figure could not be authored. A short way in rather than on the
    /// boundary, for the reasons argued at length in `SYSTEMS.md` §3.5.
    pub at_paces: i64,
    pub offers: Vec<OfferDef>,
    /// What the settlement will do to the tower's shell, if anything.
    #[serde(default)]
    pub reinforce: Option<ReinforceDef>,
    /// Crew available to hire here, across the whole run.
    pub recruits: u8,
    pub recruit_cost: Vec<CostEntryDef>,
}

/// What a creature leaves behind, interned.
///
/// A table of its own rather than a string lookup at death: `DECISIONS.md`
/// §6 is that content is authored as `"item.alloy"` and the simulation
/// only ever sees an `ItemIdx`, and "it only happens when something
/// dies" is exactly the reasoning that lets a string creep into a
/// system.
#[derive(Debug, Clone, Default)]
pub struct EnemyRuntime {
    pub drops: Vec<(ItemIdx, i64)>,
}

/// A waypoint's costs and gifts, interned.
#[derive(Debug, Clone, Default)]
pub struct WaypointRuntime {
    pub costs: Vec<(ItemIdx, i64)>,
    pub gives: Vec<(ItemIdx, i64)>,
}

/// A stretch of the journey with one character.
///
/// Regions are traversed in order and **sorted by `order` rather than by
/// `id`**, so `RegionIdx` is both the interned index and the position in
/// the journey. That is the second deliberate exception to the
/// sort-by-string-ID rule in `DECISIONS.md` §6, for the same reason
/// dayparts are the first: a list whose meaning is a sequence must be
/// stored in that sequence, or the index lies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionDef {
    pub id: String,
    pub name: String,
    /// Position in the journey, 0-based. Validation requires the pack's
    /// `order` values to be a contiguous run from zero with no
    /// duplicates.
    pub order: u8,
    /// Length is **rolled once per run** from this range, off the
    /// `world` stream. Fork spacing stays fixed, so what the roll
    /// changes is how many decisions a region contains.
    pub length_min_paces: i64,
    pub length_max_paces: i64,
    /// Likewise rolled once per run: a percentage applied to what each
    /// of this region's ruins holds. One run's drowned city is picked
    /// over and grudging, the next one's is worth berthing at.
    pub ruin_richness_min_pct: i64,
    pub ruin_richness_max_pct: i64,
    /// At least three kinds with positive weight — see
    /// `validate_palette` for why two is a bug.
    pub palette: Vec<TerrainWeight>,
    /// Multiplier on a wave's provocation-scaled threat budget.
    pub threat_pct: i64,
    /// Distance between route forks. 0 for a region with no forks.
    pub fork_interval_paces: i64,
    /// The archetypes this region's forks draw their two options from.
    #[serde(default)]
    pub branches: Vec<BranchDef>,
    /// Where in the region, if anywhere, people live.
    #[serde(default)]
    pub enclave: Option<EnclaveDef>,
    /// Roughly how far apart this region scatters waypoints.
    ///
    /// Per region rather than global, so a region can have its own
    /// rhythm — the deep jungle is thick with things to poke at and the
    /// coast is empty, and that difference is most of what makes them
    /// feel unlike each other from the walker's seat.
    ///
    /// **Zero means none**, which is a real authoring choice rather
    /// than an oversight: a stretch with nothing to stop for is a
    /// legitimate thing for a route to have, and it is what makes the
    /// stretches that do have something read as busy.
    #[serde(default)]
    pub waypoint_interval_paces: i64,
}

// ---------------------------------------------------------------------------
// Balance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Balance {
    pub world: WorldBalance,
    pub tower: TowerBalance,
    pub crew: CrewBalance,
    pub clock: ClockBalance,
    pub power: PowerBalance,
    pub transport: TransportBalance,
    pub siege: SiegeBalance,
    pub journey: JourneyBalance,
}

/// The handful of journey-wide constants that belong to no single room
/// and no single creature. See `SYSTEMS.md` §3.3–§3.5.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JourneyBalance {
    /// A fork this close to either end of its region, or to the
    /// region's enclave, is skipped — so a decision never lands on top
    /// of a boundary or a berth and competes with it for the same
    /// stretch of horizon.
    pub fork_edge_margin_paces: i64,
    /// How near an enclave the tower must stop to be berthed at it.
    /// Docking at a settlement is not the salvage rig's job, so unlike
    /// a ruin's reach this is not a property of a room.
    pub enclave_berth_paces: i64,
    /// Threat budget a roused ruin spends on wardens, per 100 units of
    /// salvage it held at the moment of rousing. Floored at one warden.
    pub warden_threat_per_100_salvage: i64,
    /// How far out of the ruin a warden wakes, so there is a few
    /// seconds between the ground moving and the first bite.
    pub warden_wake_paces: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiegeBalance {
    /// How long a creature on its way out stays on screen, whether it
    /// was shot down or simply left behind.
    pub enemy_fade_ticks: u32,
    /// Ticks somebody has to stand in a room before a thief gives up on
    /// it.
    ///
    /// **Nobody fights.** `DECISIONS.md` §8 has defenders rather than
    /// soldiers, so what a person does about a crow in the pantry is be
    /// in the pantry. The price is their time: a porter shooing is a
    /// porter not on the stairs, which is the same trade every other
    /// errand in the game makes.
    pub shoo_ticks: u32,
    /// Hit points of a floor's outer panel, a room, and a shaft column.
    /// Damage attaches to the things the player built, because that is
    /// what makes it legible.
    pub panel_hp: i64,
    pub room_hp: i64,
    pub shaft_hp: i64,
    pub heartseed_hp: i64,
    /// How far ahead of the tower creatures appear.
    pub spawn_paces_ahead: i64,
    /// Ticks between wave checks.
    pub wave_interval_ticks: u32,
    /// Threat budget per point of provocation, per 100 points.
    pub threat_per_100_provocation: i64,
    /// Floor under the threat budget, so a wave is never empty.
    pub base_threat: i64,
    /// Provocation ceiling. Everything scales against this.
    pub provocation_max: i64,
    /// Raised per 100 items stripped from the terrain.
    pub provocation_per_100_harvested: i64,
    /// Raised per burn of the burner. Smoke announces you.
    pub provocation_per_burn: i64,
    /// Raised per 100 units stripped out of a ruin. Sits close to
    /// `provocation_per_100_harvested` on purpose: the wardens are the
    /// price of a ruin, and a second, much louder price on top would
    /// make salvage a thing nobody does twice (`SYSTEMS.md` §3.4).
    pub provocation_per_100_salvaged: i64,
    /// Bled off per 100 ticks of walking quietly.
    pub provocation_decay_per_100_ticks: i64,
    /// Crew time and poles to put one hit point back.
    pub repair_ticks_per_hp: u32,
    pub repair_poles_per_10_hp: i64,
    /// Hit points one shift of repair work puts back. Bigger shifts
    /// mean fewer, chunkier interruptions to hauling.
    pub repair_hp_per_shift: i64,
    /// Below this fraction of full health (per-mille), a room stops
    /// working entirely rather than merely running slower.
    pub wrecked_permille: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockBalance {
    pub ticks_per_day: u32,
    /// `(permille_of_day, sun_pct)` anchors, linearly interpolated in
    /// integers. A curve rather than a staircase: a step change in
    /// charge income at a daypart boundary reads as a bug.
    pub sun_curve: Vec<(i64, i64)>,
    /// Below this exposure the tower needs lamps lit.
    pub night_light_threshold: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerBalance {
    /// What the Heartseed itself makes, per 100 ticks, for nothing.
    ///
    /// **The floor under the whole charge economy, and the reason a
    /// stalled tower is not a dead one.**
    ///
    /// M6 cut the sails, which left the burner as the only income —
    /// and a burner runs on bamboo, and bamboo is harvested from
    /// *ground covered*. So a tower that runs out of charge stops
    /// walking, and a tower that has stopped walking harvests nothing,
    /// and nothing in the game could break the loop. Measured on
    /// `journey.rs`'s seed 1: charge 4/2300, fuel 0, and the tower sat
    /// at the same pace for 160,000 ticks.
    ///
    /// It is deliberately far below what anything costs. Striding is
    /// 20 per 100 ticks against this 6, so a tower living on the
    /// Heartseed alone walks in bursts of one block in four and runs
    /// no rooms at all. That is a limp, not an income — the shape it
    /// has to have is "you always crawl out, slowly", so that running
    /// dry is a setback rather than a save file to abandon.
    ///
    /// The priority order does the rest for free: lamps outrank legs,
    /// so a limping tower spends its trickle on light after dark and
    /// on walking by day.
    pub heartseed_charge_per_100_ticks: i64,
    pub starting_charge: i64,
    /// Charge the legs draw per 100 ticks of walking.
    pub stride_charge_per_100_ticks: i64,
    /// Lamps, per floor, per 100 ticks, after dark.
    pub light_charge_per_100_ticks_per_floor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportBalance {
    /// Ticks a stop costs before anyone has moved.
    pub dwell_base_ticks: u32,
    /// Extra ticks per unit boarding or alighting.
    pub dwell_per_unit_ticks: u32,
    /// An idle car waits for this many callers before departing.
    pub dispatch_threshold: u8,
    /// …unless someone has been waiting this long, whichever is first.
    pub dispatch_max_wait_ticks: u32,
    /// Ticks added to a stairs estimate when the flight is full, so
    /// crew route around a congested staircase.
    pub queue_penalty_ticks: u32,
    /// Ticks a crew member is assumed to wait for a car, before the
    /// car's actual position is taken into account.
    pub elevator_base_wait_ticks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldBalance {
    /// How far the tower walks per 100 ticks, in whole paces.
    pub stride_paces_per_100_ticks: i64,
    pub band_min_paces: i64,
    pub band_max_paces: i64,
    /// Terrain kept generated ahead of the tower.
    pub stream_ahead_paces: i64,
    /// Terrain kept behind the tower before it is pruned.
    pub stream_behind_paces: i64,
}

/// One pre-placed room in the opening tower.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartingRoomDef {
    pub room: String,
    pub floor: u8,
    pub slot: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TowerBalance {
    pub starting_floors: u8,
    pub max_floors: u8,
    pub floor_slots: u8,
    pub floor_cost: Vec<CostEntryDef>,
    pub stairs_capacity: u8,
    /// Items placed on the starting storeroom's shelves so the first
    /// floor is buildable before the mill has ever run.
    pub starting_stock: Vec<CostEntryDef>,
    /// The rooms the tower sets out with, and where they sit.
    ///
    /// **Data rather than a `const` in `state.rs`**, because M6 turned
    /// the opening tower into a tuning question rather than a fixture:
    /// how much a new player is handed is the first thing anybody will
    /// want to move, and it should not need a rebuild.
    pub starting_rooms: Vec<StartingRoomDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrewBalance {
    pub starting_crew: u8,
    /// Ceiling on the crew an enclave may recruit the tower up to.
    pub crew_cap: u8,
    pub walk_ticks_per_slot: u32,
    pub climb_ticks_per_floor: u32,
    /// Added per item in a climber's arms, per floor.
    ///
    /// **The stairs are free for people and dear for goods**, which is
    /// the whole reason the other two shafts exist: a dumbwaiter carries
    /// items and no bodies, and an elevator counts a load as a seat.
    /// Before this, a crew member with three items on their back climbed
    /// exactly as fast as one going to eat, so the staircase was a
    /// perfectly good freight line and neither shaft had a job.
    ///
    /// Per item rather than a flat laden rate, so `carry_capacity` still
    /// means something — and bulk still wins: three items in one trip
    /// costs less than three trips of one, which it must, or crew would
    /// start ferrying single items to game the rate.
    ///
    /// Zero restores the old behaviour exactly.
    pub climb_ticks_per_item: u32,
    pub load_ticks: u32,
    pub unload_ticks: u32,
    /// Items a crew member carries in one trip.
    pub carry_capacity: i64,
    /// Ticks blocked before the cross-section tints a crew member red.
    pub stress_ticks: u32,
    /// How fast a room runs with somebody standing in it working it, in
    /// percent of normal.
    ///
    /// **The price is the person, not a resource.** A tower has three
    /// crew and one staircase, so posting somebody to a mill is a
    /// standing decision to take a porter off the stairs — which is the
    /// trade `DESIGN.md` insight 1 is about, made explicit rather than
    /// bolted on as a cost.
    pub manned_work_pct: i64,
    /// Ticks since a meal before a crew member goes to eat.
    pub hungry_ticks: u32,
    /// Ticks since a meal before they start working slowly. A working
    /// kitchen never reaches this, which is the point of it being a
    /// second threshold rather than the same one.
    pub starving_ticks: u32,
    /// Ticks of work a rested crew member has in them.
    pub rested_max_ticks: u32,
    /// Below this much rest left, they work slowly.
    pub tired_ticks: u32,
    /// Rest gained per tick asleep in a bunk.
    pub rest_gain_per_tick: u32,
    /// Rest gained per tick asleep on the deck, with no bed free.
    pub no_bunk_rest_gain: u32,
    /// How fast a starving crew member works, in percent of normal.
    pub hungry_work_pct: u32,
    /// How fast a tired one works. Deliberately the same figure, so a
    /// player learns the *look* of somebody working badly once and then
    /// asks why, rather than learning two separate symptoms.
    pub tired_work_pct: u32,
    /// How fast anybody works in the dark.
    pub dark_work_pct: u32,
}

/// The people aboard, in the order they come aboard.
///
/// A list rather than a roll, and the order is load-bearing for replays:
/// `add_crew` takes the next one by index, because a name drawn from a
/// stream would perturb that stream (`DECISIONS.md` §2). Reordering the
/// file is a simulation change.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrewNames {
    pub names: Vec<String>,
}

// ---------------------------------------------------------------------------
// The loaded registry
// ---------------------------------------------------------------------------

/// The interned content pack. Immutable for the life of a run.
#[derive(Debug, Clone)]
pub struct Content {
    pub balance: Balance,
    /// Names in authored order, taken by index as crew come aboard.
    pub crew_names: Vec<String>,
    /// Sorted by `id`; `ItemIdx` indexes this.
    pub items: Vec<ItemDef>,
    /// Sorted by `id`; `RoomIdx` indexes this.
    pub rooms: Vec<RoomDef>,
    /// Sorted by `id`; `ShaftIdx` indexes this.
    pub shafts: Vec<ShaftDef>,
    /// Sorted by `id`; `TerrainIdx` indexes this.
    pub terrain: Vec<TerrainDef>,
    /// Sorted by time of day, not by id; `DaypartIdx` indexes this.
    pub dayparts: Vec<DaypartDef>,
    /// Sorted by `id`; `EnemyIdx` indexes this.
    pub enemies: Vec<EnemyDef>,
    /// Sorted by `order`, not by id; `RegionIdx` indexes this, and the
    /// index *is* the position in the journey. See `RegionDef`.
    pub regions: Vec<RegionDef>,
    pub waypoints: Vec<WaypointDef>,
    pub waypoint_runtime: Vec<WaypointRuntime>,
    pub enemy_runtime: Vec<EnemyRuntime>,
    /// Every region's branches, flattened in region order and, within a
    /// region, by branch id; `BranchIdx` indexes this. Each region's
    /// slice is recorded in its `RegionRuntime`, so a fork draws from
    /// its own region without a second lookup.
    pub branches: Vec<BranchDef>,
    /// XXH3 of every pack byte, path-ordered. Stamped into replays.
    pub content_hash: u64,
    /// Pre-resolved room recipes and costs, so no system ever touches a
    /// string during a tick.
    pub room_runtime: Vec<RoomRuntime>,
    pub shaft_runtime: Vec<ShaftRuntime>,
    pub terrain_runtime: Vec<TerrainRuntime>,
    pub region_runtime: Vec<RegionRuntime>,
    pub branch_runtime: Vec<BranchRuntime>,
}

/// A room definition with every string already resolved to an index.
#[derive(Debug, Clone)]
pub struct RoomRuntime {
    pub build_cost: Vec<(ItemIdx, i64)>,
    pub recipe_inputs: Vec<(ItemIdx, i64, i64)>,
    pub recipe_outputs: Vec<(ItemIdx, i64, i64)>,
    pub craft_ticks: u32,
    pub intake_item: Option<ItemIdx>,
    /// Where the room draws from, verbatim from the definition. `None`
    /// for a room that harvests nothing.
    pub intake_source: Option<IntakeSource>,
    pub intake_buffer_max: i64,
    pub shelves: u8,
    pub per_shelf: i64,
    /// Beds in this room. Zero for everything that is not quarters.
    pub sleepers: u8,
    /// Fuel item and inbox size for a burner. Sized from the recipe
    /// rather than authored: six burns of runway is enough to ride out
    /// a delayed haul without hiding a persistent shortfall.
    pub burner_fuel: Option<(ItemIdx, i64)>,
    /// Ammo item and magazine size for an emplacement. An ordinary
    /// input stack, so the haul system feeds it with no special case.
    pub defence_ammo: Option<ItemIdx>,
    pub defence_buffer_max: i64,
    /// Interned `RoomDef::unlocked_by` — the room that has to be
    /// standing before this one can be built. `DECISIONS.md` §6: string
    /// IDs in data, dense indices in the simulation.
    pub unlocked_by: Option<RoomIdx>,
    pub crew_required: u8,
    pub front_only: bool,
    pub melee_damage: i64,
}

#[derive(Debug, Clone)]
pub struct TerrainRuntime {
    pub yield_pct: i64,
    pub sun_pct: i64,
    /// One flag per entry in `feature_kinds`: is a feature scattered
    /// with that kind a salvageable ruin? Resolved here so the
    /// generator never compares a feature-kind string.
    pub ruin_feature: Vec<bool>,
    pub salvage_min: i64,
    pub salvage_max: i64,
}

/// A region with its palette resolved to indices and its branches
/// located in the flat `Content.branches` list.
#[derive(Debug, Clone)]
pub struct RegionRuntime {
    /// `(terrain, weight)`, positive weights only, sorted by index.
    pub palette: Vec<(TerrainIdx, i64)>,
    /// Half-open range of `BranchIdx` belonging to this region.
    pub branch_first: u16,
    pub branch_end: u16,
    pub enclave: Option<EnclaveRuntime>,
}

#[derive(Debug, Clone)]
pub struct BranchRuntime {
    /// `(terrain, weight)`, positive weights only, sorted by index.
    pub palette: Vec<(TerrainIdx, i64)>,
}

#[derive(Debug, Clone)]
pub struct EnclaveRuntime {
    pub offers: Vec<OfferRuntime>,
    pub recruit_cost: Vec<(ItemIdx, i64)>,
    /// Cost and effect of one round of shell work.
    pub reinforce: Option<(Vec<(ItemIdx, i64)>, i64)>,
}

#[derive(Debug, Clone, Copy)]
pub struct OfferRuntime {
    pub give: (ItemIdx, i64),
    pub take: (ItemIdx, i64),
    pub stock: i64,
}

/// A shaft definition with its costs resolved to indices.
#[derive(Debug, Clone)]
pub struct ShaftRuntime {
    pub build_cost: Vec<(ItemIdx, i64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// Where pack bytes come from. Embedded in shipped builds; a directory
/// when a tool wants to load an edited pack.
pub trait DataSource {
    fn read(&self, path: &str) -> Result<Cow<'_, [u8]>, LoadError>;
    /// Every `.ron` path under `prefix`, sorted, so loads are ordered.
    fn list(&self, prefix: &str) -> Vec<String>;
}

/// A pack read off disk, for tuning without a rebuild.
///
/// The shipped game is always [`EmbeddedSource`] — `DECISIONS.md` §6
/// wants one pack, hashed into the replay, and a browser has no
/// filesystem to read a different one from. This exists for the
/// instruments in `examples/`, where the loop is *edit a number, see
/// what it did*: `include_dir!` does make cargo rebuild the crate when
/// a `.ron` changes, so the embedded path is correct rather than stale,
/// but it charges twenty seconds of compile for a one-character edit.
///
/// Not compiled for wasm at all: there is nothing there to read, and a
/// `std::fs` call in the bridge would be a link error rather than a
/// runtime one, which is the right time to find out.
#[cfg(not(target_arch = "wasm32"))]
pub struct DirSource {
    root: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl DirSource {
    #[must_use]
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl DataSource for DirSource {
    fn read(&self, path: &str) -> Result<Cow<'_, [u8]>, LoadError> {
        std::fs::read(self.root.join(path))
            .map(Cow::Owned)
            .map_err(|err| LoadError {
                path: path.into(),
                message: err.to_string(),
            })
    }

    fn list(&self, prefix: &str) -> Vec<String> {
        let mut files = Vec::new();
        collect_dir(&self.root, &self.root.join(prefix), &mut files);
        files.sort();
        files
    }
}

/// Paths relative to `root`, slash-separated, so a directory pack lists
/// identically to the embedded one on every platform. A missing or
/// unreadable directory yields nothing and lets `load` report the
/// specific file it wanted, rather than panicking here about a path.
#[cfg(not(target_arch = "wasm32"))]
fn collect_dir(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_dir(root, &path, out);
        } else if path.extension().is_some_and(|ext| ext == "ron")
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

pub struct EmbeddedSource;

impl DataSource for EmbeddedSource {
    fn read(&self, path: &str) -> Result<Cow<'_, [u8]>, LoadError> {
        let file = EMBEDDED_DATA.get_file(path).ok_or_else(|| LoadError {
            path: path.into(),
            message: "embedded file missing".into(),
        })?;
        Ok(Cow::Borrowed(file.contents()))
    }

    fn list(&self, prefix: &str) -> Vec<String> {
        let mut files = Vec::new();
        collect_embedded(&EMBEDDED_DATA, prefix, &mut files);
        files.sort();
        files
    }
}

fn collect_embedded(dir: &Dir<'_>, prefix: &str, out: &mut Vec<String>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(child) => collect_embedded(child, prefix, out),
            DirEntry::File(file) => {
                let path = file.path().to_string_lossy().replace('\\', "/");
                if path.starts_with(prefix) && path.ends_with(".ron") {
                    out.push(path);
                }
            }
        }
    }
}

impl Content {
    pub fn load_embedded() -> Result<Self, Vec<LoadError>> {
        Self::load(&EmbeddedSource)
    }

    pub fn load(source: &dyn DataSource) -> Result<Self, Vec<LoadError>> {
        let mut errors = Vec::new();
        let mut hasher = Xxh3::new();

        let balance = parse_one::<Balance>(source, "balance.ron", &mut errors, &mut hasher);
        let crew_names = parse_one::<CrewNames>(source, "crew/names.ron", &mut errors, &mut hasher);
        let mut items = parse_dir::<ItemDef>(source, "items", &mut errors, &mut hasher);
        let mut rooms = parse_dir::<RoomDef>(source, "rooms", &mut errors, &mut hasher);
        let mut shafts = parse_dir::<ShaftDef>(source, "shafts", &mut errors, &mut hasher);
        let mut terrain = parse_dir::<TerrainDef>(source, "terrain", &mut errors, &mut hasher);
        let mut dayparts = parse_dir::<DaypartDef>(source, "dayparts", &mut errors, &mut hasher);
        let mut enemies = parse_dir::<EnemyDef>(source, "enemies", &mut errors, &mut hasher);
        let mut regions = parse_dir::<RegionDef>(source, "regions", &mut errors, &mut hasher);
        let mut waypoints = parse_dir::<WaypointDef>(source, "waypoints", &mut errors, &mut hasher);

        if !errors.is_empty() {
            return Err(errors);
        }

        // Sort by id so indices are a pure function of the pack content
        // and never of directory-walk order.
        items.sort_by(|a, b| a.id.cmp(&b.id));
        rooms.sort_by(|a, b| a.id.cmp(&b.id));
        shafts.sort_by(|a, b| a.id.cmp(&b.id));
        terrain.sort_by(|a, b| a.id.cmp(&b.id));
        enemies.sort_by(|a, b| a.id.cmp(&b.id));
        waypoints.sort_by(|a, b| a.id.cmp(&b.id));
        // Dayparts and regions are the exceptions: both index a
        // sequence — the day, and the journey — so sorting either by id
        // would make the index lie about position. See `DECISIONS.md`
        // §6 and `SYSTEMS.md` §3.2.
        dayparts.sort_by_key(|part| part.start_permille);
        regions.sort_by_key(|region| region.order);
        // Branch order within a region is arbitrary to the journey, so
        // it falls back on the ordinary id rule. Region order then
        // branch id makes the flat list below a pure function of the
        // pack.
        for region in &mut regions {
            region.branches.sort_by(|a, b| a.id.cmp(&b.id));
        }
        let branches: Vec<BranchDef> = regions
            .iter()
            .flat_map(|region| region.branches.iter().cloned())
            .collect();

        let Some(balance) = balance else {
            return Err(vec![LoadError {
                path: "balance.ron".into(),
                message: "missing".into(),
            }]);
        };
        let Some(crew_names) = crew_names else {
            return Err(vec![LoadError {
                path: "crew/names.ron".into(),
                message: "missing".into(),
            }]);
        };

        let mut content = Self {
            balance,
            crew_names: crew_names.names,
            items,
            rooms,
            shafts,
            terrain,
            dayparts,
            enemies,
            regions,
            waypoints,
            branches,
            content_hash: hasher.digest(),
            waypoint_runtime: Vec::new(),
            enemy_runtime: Vec::new(),
            room_runtime: Vec::new(),
            shaft_runtime: Vec::new(),
            terrain_runtime: Vec::new(),
            region_runtime: Vec::new(),
            branch_runtime: Vec::new(),
        };

        content.resolve(&mut errors);
        validate(&content, &mut errors);

        if errors.is_empty() {
            Ok(content)
        } else {
            Err(errors)
        }
    }

    /// Tick of the day at which the day shift takes over.
    ///
    /// The handover, found rather than authored: the one daypart on the
    /// day shift whose predecessor round the clock is on the night one.
    /// `validate_rota` guarantees there is exactly one, so this is a
    /// lookup and not a search with a policy.
    #[must_use]
    pub fn day_shift_start_tick(&self) -> u32 {
        let ticks = i64::from(self.balance.clock.ticks_per_day.max(1));
        let count = self.dayparts.len();
        (0..count)
            .find(|&i| {
                let previous = (i + count - 1) % count;
                self.dayparts[i].shift == Shift::Day
                    && self.dayparts[previous].shift == Shift::Night
            })
            .map_or(0, |i| {
                (self.dayparts[i].start_permille * ticks / 1000) as u32
            })
    }

    /// Does anything in the pack cost this item to build?
    ///
    /// The other half of "wanted" in `haul::find_destination`. A material
    /// whose only consumer is a build cost has no inbox anywhere and
    /// would otherwise read as rubbish to a chute — which is how a tower
    /// with a chute in it managed to throw away all its rope and lose
    /// the ability to build an elevator.
    #[must_use]
    pub fn builds_with(&self, item: ItemIdx) -> bool {
        self.room_runtime
            .iter()
            .any(|room| room.build_cost.iter().any(|(cost, _)| *cost == item))
            || self
                .shaft_runtime
                .iter()
                .any(|shaft| shaft.build_cost.iter().any(|(cost, _)| *cost == item))
    }

    /// Whether any settlement in the pack will take this item — across a
    /// trade board, a recruit's price, or a round of shell work.
    ///
    /// **This exists because a chute was quietly eating salvage.** A
    /// spill is offered only for what nothing `wanted`, and `wanted`
    /// asked two questions: does a live room's inbox take it, and does
    /// anything cost it to build. Scrap answers no to both — its only
    /// room consumer is the sun forge, and a tower without one has no
    /// forge inbox to want it — and yet scrap is the whole point of
    /// berthing at a ruin (`SYSTEMS.md` §3.4). So a player who built a
    /// chute stopped at a ruin, woke its wardens, took the damage,
    /// collected the scrap, and then watched their crew carry it
    /// straight out of the tower. Nothing in the game said so.
    ///
    /// The missing question is this one: an enclave's `Trade`,
    /// `Recruit` and `Reinforce` are *commands*, so what they consume
    /// never appears in any room's inputs and is invisible to a check
    /// that only reads rooms.
    ///
    /// **It returns a quantity rather than a yes, and that is the whole
    /// of getting it right.** The first version answered "will anybody
    /// ever take this", and protecting scrap without limit broke the
    /// game in the other direction: a salvaging tower fills every shelf
    /// with scrap nothing can move, the mill's inbox never clears, and
    /// the cutter arm stops. Measured on a whole region, three seeds of
    /// twelve harvested **zero bamboo** while walking normally, and two
    /// browned out permanently — 385,000 of 400,000 ticks unable to
    /// afford to move. Losing your salvage is bad; losing the tower is
    /// worse.
    ///
    /// A board's stock is finite, so the amount worth keeping is finite
    /// too, and the pack already knows it: every offer that takes this
    /// item, times how many times it may be taken, plus what recruits
    /// and shell work cost across every settlement in the run. Below
    /// that, it is salvage and a chute may not touch it. Above it, it is
    /// more than anybody in the world will ever buy, and it is rubbish.
    ///
    /// Deliberately not conditional on a settlement being *in reach*:
    /// "there is no buyer within forty minutes" is not a reason to throw
    /// something away, and a chute that reasoned that way could not be
    /// planned around. It is the whole run's demand or nothing.
    #[must_use]
    pub fn settlements_take(&self, item: ItemIdx) -> i64 {
        self.region_runtime
            .iter()
            .filter_map(|region| region.enclave.as_ref())
            .map(|enclave| {
                let traded: i64 = enclave
                    .offers
                    .iter()
                    .filter(|offer| offer.give.0 == item)
                    .map(|offer| offer.give.1 * offer.stock)
                    .sum();
                let hired: i64 = enclave
                    .recruit_cost
                    .iter()
                    .filter(|(cost, _)| *cost == item)
                    .map(|(_, amount)| *amount)
                    .sum();
                let plated: i64 = enclave.reinforce.as_ref().map_or(0, |(cost, _)| {
                    cost.iter()
                        .filter(|(entry, _)| *entry == item)
                        .map(|(_, amount)| *amount)
                        .sum()
                });
                traded + hired + plated
            })
            .sum()
    }

    /// Interned index of an item by its authored string ID.
    #[must_use]
    pub fn item_idx(&self, id: &str) -> Option<ItemIdx> {
        self.items
            .binary_search_by(|probe| probe.id.as_str().cmp(id))
            .ok()
            .map(|i| ItemIdx(i as u16))
    }

    #[must_use]
    pub fn room_idx(&self, id: &str) -> Option<RoomIdx> {
        self.rooms
            .binary_search_by(|probe| probe.id.as_str().cmp(id))
            .ok()
            .map(|i| RoomIdx(i as u16))
    }

    #[must_use]
    pub fn shaft_idx(&self, id: &str) -> Option<ShaftIdx> {
        self.shafts
            .binary_search_by(|probe| probe.id.as_str().cmp(id))
            .ok()
            .map(|i| ShaftIdx(i as u16))
    }

    #[must_use]
    pub fn terrain_idx(&self, id: &str) -> Option<TerrainIdx> {
        self.terrain
            .binary_search_by(|probe| probe.id.as_str().cmp(id))
            .ok()
            .map(|i| TerrainIdx(i as u16))
    }

    #[must_use]
    pub fn shaft(&self, idx: ShaftIdx) -> &ShaftDef {
        &self.shafts[idx.get()]
    }

    #[must_use]
    pub fn shaft_rt(&self, idx: ShaftIdx) -> &ShaftRuntime {
        &self.shaft_runtime[idx.get()]
    }

    #[must_use]
    pub fn enemy_idx(&self, id: &str) -> Option<EnemyIdx> {
        self.enemies
            .binary_search_by(|probe| probe.id.as_str().cmp(id))
            .ok()
            .map(|i| EnemyIdx(i as u16))
    }

    #[must_use]
    pub fn enemy(&self, idx: EnemyIdx) -> &EnemyDef {
        &self.enemies[idx.get()]
    }

    /// Interned index of a region. A linear scan rather than a binary
    /// search, because the list is ordered by journey position, not by
    /// id — and there are a handful of regions, not a handful of
    /// thousands.
    #[must_use]
    pub fn region_idx(&self, id: &str) -> Option<RegionIdx> {
        self.regions
            .iter()
            .position(|region| region.id == id)
            .map(|i| RegionIdx(i as u16))
    }

    #[must_use]
    pub fn region(&self, idx: RegionIdx) -> &RegionDef {
        &self.regions[idx.get()]
    }

    #[must_use]
    pub fn region_rt(&self, idx: RegionIdx) -> &RegionRuntime {
        &self.region_runtime[idx.get()]
    }

    /// Interned index of a route branch. Linear for the same reason
    /// `region_idx` is: the flat list follows region order first.
    #[must_use]
    pub fn branch_idx(&self, id: &str) -> Option<BranchIdx> {
        self.branches
            .iter()
            .position(|branch| branch.id == id)
            .map(|i| BranchIdx(i as u16))
    }

    #[must_use]
    pub fn branch(&self, idx: BranchIdx) -> &BranchDef {
        &self.branches[idx.get()]
    }

    #[must_use]
    pub fn branch_rt(&self, idx: BranchIdx) -> &BranchRuntime {
        &self.branch_runtime[idx.get()]
    }

    #[must_use]
    pub fn daypart(&self, idx: DaypartIdx) -> &DaypartDef {
        &self.dayparts[idx.get()]
    }

    /// Which daypart a per-mille of the day falls in.
    #[must_use]
    pub fn daypart_at(&self, permille: i64) -> DaypartIdx {
        let mut found = 0u16;
        for (i, part) in self.dayparts.iter().enumerate() {
            if part.start_permille <= permille {
                found = i as u16;
            }
        }
        DaypartIdx(found)
    }

    /// Sunlight as a percentage, interpolated between the anchors in
    /// `balance.clock.sun_curve`. Integer maths throughout.
    #[must_use]
    pub fn sun_pct_at(&self, permille: i64) -> i64 {
        let curve = &self.balance.clock.sun_curve;
        let Some(first) = curve.first() else {
            return 0;
        };
        if permille <= first.0 {
            return first.1;
        }
        for pair in curve.windows(2) {
            let (x0, y0) = pair[0];
            let (x1, y1) = pair[1];
            if permille < x1 {
                let span = (x1 - x0).max(1);
                return y0 + (y1 - y0) * (permille - x0) / span;
            }
        }
        curve.last().map_or(0, |last| last.1)
    }

    #[must_use]
    pub fn item(&self, idx: ItemIdx) -> &ItemDef {
        &self.items[idx.get()]
    }

    #[must_use]
    pub fn room(&self, idx: RoomIdx) -> &RoomDef {
        &self.rooms[idx.get()]
    }

    #[must_use]
    pub fn room_rt(&self, idx: RoomIdx) -> &RoomRuntime {
        &self.room_runtime[idx.get()]
    }

    #[must_use]
    pub fn terrain(&self, idx: TerrainIdx) -> &TerrainDef {
        &self.terrain[idx.get()]
    }

    /// Resolve every authored string into an index once, so the tick
    /// loop never compares strings.
    fn resolve(&mut self, errors: &mut Vec<LoadError>) {
        let mut room_runtime = Vec::with_capacity(self.rooms.len());
        for room in &self.rooms {
            let mut lookup = |item: &str, field: &str| match self.item_idx(item) {
                Some(idx) => idx,
                None => {
                    errors.push(LoadError {
                        path: room.id.clone(),
                        message: format!("{field} references unknown item {item}"),
                    });
                    ItemIdx(0)
                }
            };

            let build_cost = room
                .build_cost
                .iter()
                .map(|c| (lookup(&c.item, "build_cost"), c.amount))
                .collect();

            let (recipe_inputs, recipe_outputs, craft_ticks) = match &room.recipe {
                Some(recipe) => (
                    recipe
                        .inputs
                        .iter()
                        .map(|io| (lookup(&io.item, "recipe input"), io.amount, io.buffer_max))
                        .collect(),
                    recipe
                        .outputs
                        .iter()
                        .map(|io| (lookup(&io.item, "recipe output"), io.amount, io.buffer_max))
                        .collect(),
                    recipe.craft_ticks,
                ),
                None => (Vec::new(), Vec::new(), 0),
            };

            let (intake_item, intake_source, intake_buffer_max) = match &room.intake {
                Some(intake) => (
                    Some(lookup(&intake.item, "intake")),
                    Some(intake.source),
                    intake.buffer_max,
                ),
                None => (None, None, 0),
            };

            let (shelves, per_shelf) = match &room.storage {
                Some(storage) => (storage.shelves, storage.per_shelf),
                None => (0, 0),
            };

            let burner_fuel = room.burner.as_ref().map(|burner| {
                (
                    lookup(&burner.fuel, "burner fuel"),
                    burner.fuel_per_burn * 6,
                )
            });

            let defence_ammo = room
                .defence
                .as_ref()
                .map(|defence| lookup(&defence.ammo, "defence ammo"));
            let defence_buffer_max = room.defence.as_ref().map_or(0, |d| d.buffer_max);

            room_runtime.push(RoomRuntime {
                build_cost,
                recipe_inputs,
                recipe_outputs,
                craft_ticks,
                intake_item,
                intake_source,
                intake_buffer_max,
                shelves,
                per_shelf,
                burner_fuel,
                defence_ammo,
                sleepers: room.quarters.as_ref().map_or(0, |q| q.sleepers),
                defence_buffer_max,
                unlocked_by: room.unlocked_by.as_deref().and_then(|id| self.room_idx(id)),
                crew_required: room.crew_required,
                front_only: room.front_only,
                melee_damage: room.melee_damage,
            });
        }
        self.room_runtime = room_runtime;

        let mut shaft_runtime = Vec::with_capacity(self.shafts.len());
        for shaft in &self.shafts {
            let build_cost = shaft
                .build_cost
                .iter()
                .map(|cost| match self.item_idx(&cost.item) {
                    Some(idx) => (idx, cost.amount),
                    None => {
                        errors.push(LoadError {
                            path: shaft.id.clone(),
                            message: format!("build_cost references unknown item {}", cost.item),
                        });
                        (ItemIdx(0), cost.amount)
                    }
                })
                .collect();
            shaft_runtime.push(ShaftRuntime { build_cost });
        }
        self.shaft_runtime = shaft_runtime;

        self.terrain_runtime = self
            .terrain
            .iter()
            .map(|band| TerrainRuntime {
                yield_pct: band.yield_pct,
                sun_pct: band.sun_pct,
                ruin_feature: band
                    .feature_kinds
                    .iter()
                    .map(|kind| band.ruin_kinds.contains(kind))
                    .collect(),
                salvage_min: band.ruin_salvage_min,
                salvage_max: band.ruin_salvage_max,
            })
            .collect();

        self.resolve_enemies(errors);
        self.resolve_journey(errors);
    }

    /// Regions, their branches, and their enclaves. Split out of
    /// `resolve` only because the journey is a chunk of its own; it runs
    /// as part of the same pass.
    fn resolve_enemies(&mut self, errors: &mut Vec<LoadError>) {
        let mut runtime = Vec::with_capacity(self.enemies.len());
        for enemy in &self.enemies {
            runtime.push(EnemyRuntime {
                drops: enemy
                    .drops
                    .iter()
                    .filter_map(|entry| match self.item_idx(&entry.item) {
                        Some(idx) => Some((idx, entry.amount)),
                        None => {
                            errors.push(LoadError {
                                path: format!("enemies/{}", enemy.id),
                                message: format!("unknown drop {}", entry.item),
                            });
                            None
                        }
                    })
                    .collect(),
            });
        }
        self.enemy_runtime = runtime;
    }

    fn resolve_journey(&mut self, errors: &mut Vec<LoadError>) {
        let mut waypoint_runtime = Vec::with_capacity(self.waypoints.len());
        for waypoint in &self.waypoints {
            let intern = |list: &Vec<CostEntryDef>, errors: &mut Vec<LoadError>| {
                list.iter()
                    .filter_map(|entry| match self.item_idx(&entry.item) {
                        Some(idx) => Some((idx, entry.amount)),
                        None => {
                            errors.push(LoadError {
                                path: format!("waypoints/{}", waypoint.id),
                                message: format!("unknown item {}", entry.item),
                            });
                            None
                        }
                    })
                    .collect()
            };
            waypoint_runtime.push(WaypointRuntime {
                costs: intern(&waypoint.costs, errors),
                gives: intern(&waypoint.gives, errors),
            });
        }
        self.waypoint_runtime = waypoint_runtime;

        let mut branch_runtime = Vec::with_capacity(self.branches.len());
        for branch in &self.branches {
            branch_runtime.push(BranchRuntime {
                palette: self.resolve_palette(&branch.id, &branch.palette, errors),
            });
        }
        self.branch_runtime = branch_runtime;

        let mut region_runtime = Vec::with_capacity(self.regions.len());
        let mut branch_first = 0u16;
        for region in &self.regions {
            let palette = self.resolve_palette(&region.id, &region.palette, errors);
            let branch_end = branch_first + region.branches.len() as u16;

            let enclave = region.enclave.as_ref().map(|enclave| {
                let mut lookup = |item: &str, field: &str| match self.item_idx(item) {
                    Some(idx) => idx,
                    None => {
                        errors.push(LoadError {
                            path: enclave.id.clone(),
                            message: format!("{field} references unknown item {item}"),
                        });
                        ItemIdx(0)
                    }
                };
                EnclaveRuntime {
                    offers: enclave
                        .offers
                        .iter()
                        .map(|offer| OfferRuntime {
                            give: (lookup(&offer.give.item, "offer give"), offer.give.amount),
                            take: (lookup(&offer.take.item, "offer take"), offer.take.amount),
                            stock: offer.stock,
                        })
                        .collect(),
                    recruit_cost: enclave
                        .recruit_cost
                        .iter()
                        .map(|cost| (lookup(&cost.item, "recruit_cost"), cost.amount))
                        .collect(),
                    reinforce: enclave.reinforce.as_ref().map(|work| {
                        (
                            work.cost
                                .iter()
                                .map(|cost| (lookup(&cost.item, "reinforce cost"), cost.amount))
                                .collect(),
                            work.panel_hp,
                        )
                    }),
                }
            });

            region_runtime.push(RegionRuntime {
                palette,
                branch_first,
                branch_end,
                enclave,
            });
            branch_first = branch_end;
        }
        self.region_runtime = region_runtime;
    }

    /// Authored `(terrain id, weight)` pairs to `(TerrainIdx, weight)`,
    /// keeping only positive weights and sorting by index so the
    /// generator's weighted pick walks the pool in a fixed order.
    fn resolve_palette(
        &self,
        owner: &str,
        palette: &[TerrainWeight],
        errors: &mut Vec<LoadError>,
    ) -> Vec<(TerrainIdx, i64)> {
        let mut resolved: Vec<(TerrainIdx, i64)> = palette
            .iter()
            .filter_map(|entry| match self.terrain_idx(&entry.terrain) {
                Some(idx) if entry.weight > 0 => Some((idx, entry.weight)),
                Some(_) => None,
                None => {
                    errors.push(LoadError {
                        path: owner.to_owned(),
                        message: format!("palette references unknown terrain {}", entry.terrain),
                    });
                    None
                }
            })
            .collect();
        resolved.sort_by_key(|(idx, _)| *idx);
        resolved
    }
}

/// Each shift has to be one contiguous run of dayparts modulo the day,
/// and both have to exist.
///
/// A pack with two separate night stretches is not describing a rota, it
/// is describing a bug: crew would wake and sleep twice a day, and
/// `rested_max_ticks` would be tuned against a shift length that never
/// happens. Same spirit as the contiguous-`order` check on regions.
fn validate_rota(dayparts: &[DaypartDef], errors: &mut Vec<LoadError>) {
    if dayparts.is_empty() {
        return;
    }
    let path = "dayparts".to_string();
    if !dayparts.iter().any(|d| d.shift == Shift::Day)
        || !dayparts.iter().any(|d| d.shift == Shift::Night)
    {
        errors.push(LoadError {
            path,
            message: "the rota needs both a day shift and a night shift".into(),
        });
        return;
    }
    // Two bands round the clock means exactly two handovers, one onto
    // each shift. More than that and a band is split in half.
    let handovers = dayparts
        .iter()
        .zip(dayparts.iter().cycle().skip(1))
        .take(dayparts.len())
        .filter(|(a, b)| a.shift != b.shift)
        .count();
    if handovers != 2 {
        errors.push(LoadError {
            path,
            message: format!(
                "the rota changes shift {handovers} times round the day; a rota has \
                 exactly two handovers, one onto each shift"
            ),
        });
    }
}

fn validate(content: &Content, errors: &mut Vec<LoadError>) {
    validate_rota(&content.dayparts, errors);
    if content.items.is_empty() {
        errors.push(LoadError {
            path: "items".into(),
            message: "pack defines no items".into(),
        });
    }
    if content.terrain.is_empty() {
        errors.push(LoadError {
            path: "terrain".into(),
            message: "pack defines no terrain bands".into(),
        });
    }
    // How often a band comes up is a region's business now, so the old
    // pack-wide "something must have a positive weight" check lives in
    // `validate_regions` against the palettes instead.
    for band in &content.terrain {
        for kind in &band.ruin_kinds {
            if !band.feature_kinds.contains(kind) {
                errors.push(LoadError {
                    path: band.id.clone(),
                    message: format!("ruin_kinds names {kind}, which is not a feature_kind"),
                });
            }
        }
        if !band.ruin_kinds.is_empty()
            && (band.ruin_salvage_min <= 0 || band.ruin_salvage_max < band.ruin_salvage_min)
        {
            errors.push(LoadError {
                path: band.id.clone(),
                message: format!(
                    "ruin-bearing terrain needs a positive salvage range, got {}..={}",
                    band.ruin_salvage_min, band.ruin_salvage_max
                ),
            });
        }
    }

    for (i, def) in content.rooms.iter().enumerate() {
        let path = def.id.clone();
        let rt = &content.room_runtime[i];

        if def.width == 0 {
            errors.push(LoadError {
                path: path.clone(),
                message: "width must be at least 1".into(),
            });
        }
        if def.width > content.balance.tower.floor_slots {
            errors.push(LoadError {
                path: path.clone(),
                message: format!(
                    "width {} exceeds floor width {}",
                    def.width, content.balance.tower.floor_slots
                ),
            });
        }
        // Legibility rule from v2-plan §6.1: at most two inputs, so a
        // chain stays readable in the cross-section.
        if rt.recipe_inputs.len() > 2 {
            errors.push(LoadError {
                path: path.clone(),
                message: format!(
                    "{} recipe inputs; the legibility cap is 2",
                    rt.recipe_inputs.len()
                ),
            });
        }
        if let Some(recipe) = &def.recipe {
            if recipe.craft_ticks == 0 {
                errors.push(LoadError {
                    path: path.clone(),
                    message: "craft_ticks must be positive".into(),
                });
            }
            if recipe.outputs.is_empty() {
                errors.push(LoadError {
                    path: path.clone(),
                    message: "recipe produces nothing".into(),
                });
            }
            for io in recipe.inputs.iter().chain(&recipe.outputs) {
                if io.amount <= 0 || io.buffer_max < io.amount {
                    errors.push(LoadError {
                        path: path.clone(),
                        message: format!(
                            "{} slot has amount {} against buffer {}",
                            io.item, io.amount, io.buffer_max
                        ),
                    });
                }
            }
        }
        if let Some(intake) = &def.intake {
            if intake.buffer_max <= 0 {
                errors.push(LoadError {
                    path: path.clone(),
                    message: "an intake room needs somewhere to put what it takes".into(),
                });
            }
            // A rate has to be positive, and it also has to survive
            // Q8.8. Intake accrues a fraction of an item per tick, and
            // a fraction finer than 1/256 truncates to nothing — so a
            // rate slow enough to round away is a room that harvests
            // literally never, silently, forever. That is the failure
            // `Power::buy_block` was written to avoid, and this is the
            // version of it a designer can walk into by typing a bigger
            // number. Catch it at load, where a broken pack is a build
            // error (`AGENTS.md` §IV), rather than in play.
            match intake.source {
                IntakeSource::Terrain { paces_per_item } if paces_per_item <= 0 => {
                    errors.push(LoadError {
                        path: path.clone(),
                        message: "intake paces_per_item must be positive".into(),
                    });
                }
                // The old cliff here was a rate so slow it rounded to
                // zero and harvested nothing, silently and forever.
                // Intake accumulates effort against a threshold now, so
                // a slow rate is just a large threshold and there is no
                // rounding to fall off. What remains is the far end of
                // the same axis: a threshold too large for Q8.8 clamps,
                // and the room would then work at whatever the clamp
                // happens to be rather than at what it was authored to.
                IntakeSource::Terrain { paces_per_item }
                    if intake::terrain_effort(paces_per_item, 100) >= Fx(i32::MAX) =>
                {
                    errors.push(LoadError {
                        path: path.clone(),
                        message: format!(
                            "{paces_per_item} paces an item is further than fixed point can \
                             carry, so the room would not harvest at the rate it asks for"
                        ),
                    });
                }
                IntakeSource::Ruin {
                    ticks_per_item,
                    range_paces,
                } if ticks_per_item == 0 || range_paces <= 0 => {
                    errors.push(LoadError {
                        path: path.clone(),
                        message: "a ruin intake needs a positive rate and a reach".into(),
                    });
                }
                IntakeSource::Ruin { ticks_per_item, .. }
                    if intake::ruin_effort(ticks_per_item) >= Fx(i32::MAX) =>
                {
                    errors.push(LoadError {
                        path: path.clone(),
                        message: format!(
                            "{ticks_per_item} ticks an item is longer than fixed point can \
                             carry, so the room would not extract at the rate it asks for"
                        ),
                    });
                }
                IntakeSource::Sun { ticks_per_item: 0 } => {
                    errors.push(LoadError {
                        path: path.clone(),
                        message: "a sun intake needs a positive rate".into(),
                    });
                }
                // Same far-end clamp as the other two. A sun source is
                // scaled *down* by exposure at runtime, so the threshold
                // it is checked against here is the best case; if the
                // best case does not fit, nothing will.
                IntakeSource::Sun { ticks_per_item }
                    if intake::sun_effort(ticks_per_item, 100) >= Fx(i32::MAX) =>
                {
                    errors.push(LoadError {
                        path: path.clone(),
                        message: format!(
                            "{ticks_per_item} ticks an item is longer than fixed point can \
                             carry, so the room would not grow at the rate it asks for"
                        ),
                    });
                }
                _ => {}
            }
        }
        if let Some(burner) = &def.burner {
            if content.item_idx(&burner.fuel).is_none() {
                errors.push(LoadError {
                    path: path.clone(),
                    message: format!("burner fuel references unknown item {}", burner.fuel),
                });
            }
            if burner.burn_ticks == 0 || burner.charge_per_burn <= 0 || burner.fuel_per_burn <= 0 {
                errors.push(LoadError {
                    path: path.clone(),
                    message: "a burner must consume fuel and produce charge over time".into(),
                });
            }
        }
        if let Some(bank) = &def.bank
            && bank.capacity <= 0
        {
            errors.push(LoadError {
                path: path.clone(),
                message: "a cell bank must hold something".into(),
            });
        }
        // Every category must actually be wired to a system, or it is
        // content with no consumer — the v1 failure mode.
        let wired = match def.category {
            RoomCategory::Intake => def.intake.is_some(),
            RoomCategory::Production => def.recipe.is_some(),
            RoomCategory::Storage => def.storage.is_some(),
            RoomCategory::Energy => def.burner.is_some() || def.bank.is_some(),
            RoomCategory::Defence => def.defence.is_some(),
            RoomCategory::Quarters => def.quarters.is_some(),
            RoomCategory::Heart => true,
        };
        if !wired {
            errors.push(LoadError {
                path,
                message: format!(
                    "category {:?} but no matching behaviour block",
                    def.category
                ),
            });
        }
    }

    // Exactly one Heart, and it must be placeable.
    let hearts = content
        .rooms
        .iter()
        .filter(|r| r.category == RoomCategory::Heart)
        .count();
    if hearts != 1 {
        errors.push(LoadError {
            path: "rooms".into(),
            message: format!("expected exactly 1 Heart room, found {hearts}"),
        });
    }

    for cost in &content.balance.tower.floor_cost {
        if content.item_idx(&cost.item).is_none() {
            errors.push(LoadError {
                path: "balance.ron".into(),
                message: format!("floor_cost references unknown item {}", cost.item),
            });
        }
    }
    for stock in &content.balance.tower.starting_stock {
        if content.item_idx(&stock.item).is_none() {
            errors.push(LoadError {
                path: "balance.ron".into(),
                message: format!("starting_stock references unknown item {}", stock.item),
            });
        }
    }
    if content.balance.tower.starting_floors > content.balance.tower.max_floors {
        errors.push(LoadError {
            path: "balance.ron".into(),
            message: "starting_floors exceeds max_floors".into(),
        });
    }
    // A floor range that cannot contain a floor is a room nobody can
    // ever build, and it would present as a build card that refuses
    // every slot rather than as a broken pack. `AGENTS.md` §IV: an
    // invalid shipped pack is a build error.
    let starting = content.balance.tower.starting_floors;
    let ceiling = content.balance.tower.max_floors;
    for room in &content.rooms {
        let Some(min_floor) = room.min_floor else {
            continue;
        };
        if let Some(max_floor) = room.max_floor
            && min_floor > max_floor
        {
            errors.push(LoadError {
                path: format!("rooms/{}", room.id),
                message: format!("min_floor {min_floor} is above max_floor {max_floor}"),
            });
        }
        if min_floor >= ceiling {
            errors.push(LoadError {
                path: format!("rooms/{}", room.id),
                message: format!("min_floor {min_floor} is at or above max_floors {ceiling}"),
            });
        } else if min_floor >= starting {
            // Not fatal — a room the tower has to grow into is a fair
            // design — but it is worth being deliberate about, because
            // a bunk nobody can build until they add a floor is a very
            // different game from one they start with.
            errors.push(LoadError {
                path: format!("rooms/{}", room.id),
                message: format!(
                    "min_floor {min_floor} is above the starting tower's top floor                      ({}), so this room cannot be built until the tower grows",
                    starting.saturating_sub(1)
                ),
            });
        }
    }
    if content.balance.crew.walk_ticks_per_slot == 0
        || content.balance.crew.climb_ticks_per_floor == 0
    {
        errors.push(LoadError {
            path: "balance.ron".into(),
            message: "crew movement rates must be positive".into(),
        });
    }
    if content.balance.crew.crew_cap < content.balance.crew.starting_crew {
        errors.push(LoadError {
            path: "balance.ron".into(),
            message: "crew_cap is below starting_crew".into(),
        });
    }

    validate_shafts(content, errors);
    validate_clock(content, errors);
    validate_buffers(content, errors);
    validate_journey(content, errors);
}

/// Regions, palettes, branches, and enclaves.
///
/// Two of the checks here are the whole reason the journey is content
/// rather than code: `order` has to describe a real sequence, and a
/// palette has to have enough in it for the generator's no-repeat rule
/// to produce a horizon rather than a stripe.
/// Every buffer a room has is big enough to hold something.
///
/// **The hauling economy is made of buffers** (`DESIGN.md` §2 insight
/// 1): a room takes deliveries into an inbox, stalls when its outbox
/// fills, and the crew route around both. A buffer of zero is a room
/// that can never accept a delivery and never hold a result — it does
/// not fail loudly, it simply never participates, and the tower reads
/// as mysteriously slow.
///
/// The schema already makes `buffer_max` mandatory wherever it means
/// anything, so this is the one remaining way to author a room that
/// cannot take part: write the field and put a nought in it.
#[cfg(test)]
pub(crate) fn validate_buffers_for_test(content: &Content, errors: &mut Vec<LoadError>) {
    validate_buffers(content, errors);
}

fn validate_buffers(content: &Content, errors: &mut Vec<LoadError>) {
    for room in &content.rooms {
        let mut complain = |what: &str, item: &str| {
            errors.push(LoadError {
                path: format!("rooms/{}", room.id),
                message: format!("{what} buffer for {item} is zero, so nothing can ever go in it"),
            });
        };
        if let Some(recipe) = room.recipe.as_ref() {
            for entry in &recipe.inputs {
                if entry.buffer_max <= 0 {
                    complain("input", &entry.item);
                }
            }
            for entry in &recipe.outputs {
                if entry.buffer_max <= 0 {
                    complain("output", &entry.item);
                }
            }
        }
        if let Some(intake) = room.intake.as_ref()
            && intake.buffer_max <= 0
        {
            complain("intake", &intake.item);
        }
        if let Some(defence) = room.defence.as_ref()
            && defence.buffer_max <= 0
        {
            complain("ammo", &defence.ammo);
        }
        // A burner's rack is derived rather than authored — six burns
        // of runway — so the way to get a zero here is a zero
        // `fuel_per_burn`, which would also mean a burner that eats
        // nothing.
        if let Some(burner) = room.burner.as_ref()
            && burner.fuel_per_burn <= 0
        {
            complain("fuel", &burner.fuel);
        }
    }
}

fn validate_journey(content: &Content, errors: &mut Vec<LoadError>) {
    let journey = &content.balance.journey;
    if journey.enclave_berth_paces <= 0 {
        errors.push(LoadError {
            path: "balance.ron".into(),
            message: "enclave_berth_paces must be positive or no berth is ever possible".into(),
        });
    }
    if journey.fork_edge_margin_paces < 0 || journey.warden_wake_paces < 0 {
        errors.push(LoadError {
            path: "balance.ron".into(),
            message: "journey distances cannot be negative".into(),
        });
    }
    if journey.warden_threat_per_100_salvage <= 0 {
        errors.push(LoadError {
            path: "balance.ron".into(),
            message: "warden_threat_per_100_salvage must be positive".into(),
        });
    }

    if content.regions.is_empty() {
        errors.push(LoadError {
            path: "regions".into(),
            message: "pack defines no regions; a run has nowhere to walk".into(),
        });
        return;
    }

    // The journey is a sequence, and `RegionIdx` claims to be a
    // position in it. A gap or a duplicate makes that claim false.
    for (i, region) in content.regions.iter().enumerate() {
        if usize::from(region.order) != i {
            errors.push(LoadError {
                path: "regions".into(),
                message: format!(
                    "region order must be a contiguous run from zero; {} has order {} at \
                     journey position {i}",
                    region.id, region.order
                ),
            });
        }
    }

    for (i, region) in content.regions.iter().enumerate() {
        let path = region.id.clone();
        if content.regions[..i]
            .iter()
            .any(|other| other.id == region.id)
        {
            errors.push(LoadError {
                path: path.clone(),
                message: "two regions share an id".into(),
            });
        }
        if region.length_min_paces <= 0 || region.length_max_paces < region.length_min_paces {
            errors.push(LoadError {
                path: path.clone(),
                message: format!(
                    "length range {}..={} is not a positive range",
                    region.length_min_paces, region.length_max_paces
                ),
            });
        }
        if region.ruin_richness_min_pct <= 0
            || region.ruin_richness_max_pct < region.ruin_richness_min_pct
        {
            errors.push(LoadError {
                path: path.clone(),
                message: format!(
                    "ruin richness range {}..={} is not a positive range",
                    region.ruin_richness_min_pct, region.ruin_richness_max_pct
                ),
            });
        }
        if region.threat_pct <= 0 {
            errors.push(LoadError {
                path: path.clone(),
                message: "threat_pct must be positive".into(),
            });
        }
        if region.fork_interval_paces < 0 {
            errors.push(LoadError {
                path: path.clone(),
                message: "fork_interval_paces cannot be negative".into(),
            });
        }
        // A fork offers two *distinct* archetypes, so a region that
        // forks at all has to have two to draw between.
        if region.fork_interval_paces > 0 && region.branches.len() < 2 {
            errors.push(LoadError {
                path: path.clone(),
                message: format!(
                    "a forking region needs at least 2 branch archetypes, found {}",
                    region.branches.len()
                ),
            });
        }

        let rt = &content.region_runtime[i];
        validate_palette(&path, &rt.palette, errors);

        // A region's branches are contiguous in the flat list, in the
        // order they appear on the region — so the runtime palette for
        // `region.branches[n]` is at `branch_first + n`.
        for (n, branch) in region.branches.iter().enumerate() {
            let idx = BranchIdx(rt.branch_first + n as u16);
            if content
                .branches
                .iter()
                .filter(|other| other.id == branch.id)
                .count()
                > 1
            {
                errors.push(LoadError {
                    path: branch.id.clone(),
                    message: "two branches share an id".into(),
                });
            }
            if branch.length_paces <= 0 {
                errors.push(LoadError {
                    path: branch.id.clone(),
                    message: "a branch that overrides nothing is not a choice".into(),
                });
            }
            if branch.threat_pct <= 0 {
                errors.push(LoadError {
                    path: branch.id.clone(),
                    message: "threat_pct must be positive".into(),
                });
            }
            validate_palette(&branch.id, &content.branch_rt(idx).palette, errors);
        }

        if let Some(enclave) = &region.enclave {
            // The enclave has to stand inside the region however its
            // length rolls, or a short draw would leave it unreachable.
            if enclave.at_paces <= 0 || enclave.at_paces >= region.length_min_paces {
                errors.push(LoadError {
                    path: enclave.id.clone(),
                    message: format!(
                        "at_paces {} falls outside the shortest this region can roll ({})",
                        enclave.at_paces, region.length_min_paces
                    ),
                });
            }
            if enclave.offers.is_empty() && enclave.recruits == 0 {
                errors.push(LoadError {
                    path: enclave.id.clone(),
                    message: "an enclave with nothing to offer is a berth with no reason".into(),
                });
            }
            for offer in &enclave.offers {
                if offer.give.amount <= 0 || offer.take.amount <= 0 || offer.stock <= 0 {
                    errors.push(LoadError {
                        path: enclave.id.clone(),
                        message: format!(
                            "offer {} for {} has a non-positive amount or stock",
                            offer.give.item, offer.take.item
                        ),
                    });
                }
            }
            if enclave.recruits > 0 && enclave.recruit_cost.is_empty() {
                errors.push(LoadError {
                    path: enclave.id.clone(),
                    message: "a recruit has to cost something".into(),
                });
            }
        }
    }

    // A terrain kind no palette can produce is content with no consumer
    // (`DECISIONS.md` §9.1) — the exact failure the weight-per-terrain
    // model used to hide, since a weight of zero looked like tuning.
    for (i, band) in content.terrain.iter().enumerate() {
        let idx = TerrainIdx(i as u16);
        let referenced = content
            .region_runtime
            .iter()
            .map(|rt| &rt.palette)
            .chain(content.branch_runtime.iter().map(|rt| &rt.palette))
            .any(|palette| palette.iter().any(|(kind, _)| *kind == idx));
        if !referenced {
            errors.push(LoadError {
                path: band.id.clone(),
                message: "no region or branch palette can produce this terrain".into(),
            });
        }
    }
}

/// A palette needs at least three kinds with positive weight.
///
/// `pick_band_kind` never repeats the previous band's kind, so with two
/// entries that rule degenerates into strict ABABAB alternation — a
/// perfectly regular horizon, which reads as a bug rather than as
/// terrain (`SYSTEMS.md` §3.2). Enforced at load, in the same spirit as
/// the anti-frustration constraints already living invisibly inside the
/// generator.
fn validate_palette(owner: &str, palette: &[(TerrainIdx, i64)], errors: &mut Vec<LoadError>) {
    if palette.len() < 3 {
        errors.push(LoadError {
            path: owner.to_owned(),
            message: format!(
                "a palette needs at least 3 terrain kinds with positive weight, found {}; \
                 fewer makes the no-repeat rule alternate",
                palette.len()
            ),
        });
    }
    if palette.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        errors.push(LoadError {
            path: owner.to_owned(),
            message: "a palette names the same terrain twice".into(),
        });
    }
}

fn validate_shafts(content: &Content, errors: &mut Vec<LoadError>) {
    if content.shafts.is_empty() {
        errors.push(LoadError {
            path: "shafts".into(),
            message: "pack defines no vertical transport".into(),
        });
    }
    // Stairs are the baseline everything else is measured against, and
    // the starting tower assumes one exists.
    if !content
        .shafts
        .iter()
        .any(|shaft| shaft.kind == ShaftKind::Stairs)
    {
        errors.push(LoadError {
            path: "shafts".into(),
            message: "pack defines no stairs".into(),
        });
    }

    for shaft in &content.shafts {
        let path = shaft.id.clone();
        if shaft.ticks_per_floor == 0 {
            errors.push(LoadError {
                path: path.clone(),
                message: "ticks_per_floor must be positive".into(),
            });
        }
        if shaft.min_span < 1 {
            errors.push(LoadError {
                path: path.clone(),
                message: "min_span must be at least 1".into(),
            });
        }
        if shaft.max_span != 0 && shaft.max_span < shaft.min_span {
            errors.push(LoadError {
                path: path.clone(),
                message: "max_span is below min_span".into(),
            });
        }
        if shaft.capacity == 0 {
            errors.push(LoadError {
                path: path.clone(),
                message: "capacity must be positive".into(),
            });
        }
        match shaft.kind {
            ShaftKind::Elevator if shaft.cars == 0 => errors.push(LoadError {
                path,
                message: "an elevator with no cars cannot carry anyone".into(),
            }),
            ShaftKind::Dumbwaiter if shaft.cars == 0 || shaft.batch <= 0 => {
                errors.push(LoadError {
                    path,
                    message: "a dumbwaiter needs a car and a batch size".into(),
                });
            }
            _ => {}
        }
    }
}

fn validate_clock(content: &Content, errors: &mut Vec<LoadError>) {
    let clock = &content.balance.clock;
    if clock.ticks_per_day == 0 {
        errors.push(LoadError {
            path: "balance.ron".into(),
            message: "ticks_per_day must be positive".into(),
        });
    }
    if clock.sun_curve.len() < 2 {
        errors.push(LoadError {
            path: "balance.ron".into(),
            message: "sun_curve needs at least two anchors to interpolate".into(),
        });
    }
    if clock
        .sun_curve
        .windows(2)
        .any(|pair| pair[0].0 >= pair[1].0)
    {
        errors.push(LoadError {
            path: "balance.ron".into(),
            message: "sun_curve anchors must be strictly increasing in time".into(),
        });
    }

    if content.dayparts.is_empty() {
        errors.push(LoadError {
            path: "dayparts".into(),
            message: "pack defines no dayparts".into(),
        });
    }
    // The first daypart has to start at the top of the day, or there is
    // a stretch of time that belongs to nothing.
    if content
        .dayparts
        .first()
        .is_some_and(|part| part.start_permille != 0)
    {
        errors.push(LoadError {
            path: "dayparts".into(),
            message: "the first daypart must start at permille 0".into(),
        });
    }
    if content
        .dayparts
        .windows(2)
        .any(|pair| pair[0].start_permille == pair[1].start_permille)
    {
        errors.push(LoadError {
            path: "dayparts".into(),
            message: "two dayparts start at the same moment".into(),
        });
    }
}

fn parse_one<T>(
    source: &dyn DataSource,
    path: &str,
    errors: &mut Vec<LoadError>,
    hasher: &mut Xxh3,
) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = match source.read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    hasher.update(path.as_bytes());
    hasher.update(&bytes);

    match from_bytes::<T>(&bytes) {
        Ok(value) => Some(value),
        Err(err) => {
            errors.push(LoadError {
                path: path.into(),
                message: err.to_string(),
            });
            None
        }
    }
}

fn parse_dir<T>(
    source: &dyn DataSource,
    prefix: &str,
    errors: &mut Vec<LoadError>,
    hasher: &mut Xxh3,
) -> Vec<T>
where
    T: for<'de> Deserialize<'de>,
{
    source
        .list(prefix)
        .iter()
        .filter_map(|path| parse_one::<T>(source, path, errors, hasher))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_pack_loads_and_validates() {
        let content = Content::load_embedded().expect("embedded pack must load");
        assert!(!content.items.is_empty());
        assert!(!content.rooms.is_empty());
        assert!(!content.terrain.is_empty());
        assert_ne!(content.content_hash, 0);
    }

    #[test]
    fn indices_are_stable_and_sorted() {
        let content = Content::load_embedded().unwrap();
        for (i, item) in content.items.iter().enumerate() {
            assert_eq!(content.item_idx(&item.id), Some(ItemIdx(i as u16)));
        }
        for (i, room) in content.rooms.iter().enumerate() {
            assert_eq!(content.room_idx(&room.id), Some(RoomIdx(i as u16)));
        }
        assert!(content.items.windows(2).all(|w| w[0].id < w[1].id));
        assert!(content.rooms.windows(2).all(|w| w[0].id < w[1].id));
    }

    #[test]
    fn content_hash_is_reproducible() {
        let a = Content::load_embedded().unwrap();
        let b = Content::load_embedded().unwrap();
        assert_eq!(a.content_hash, b.content_hash);
    }

    #[test]
    fn unknown_item_reference_is_a_load_error() {
        struct BrokenSource;
        impl DataSource for BrokenSource {
            fn read(&self, path: &str) -> Result<Cow<'_, [u8]>, LoadError> {
                match path {
                    "balance.ron" => EmbeddedSource.read(path),
                    "rooms/broken.ron" => Ok(Cow::Borrowed(
                        br#"RoomDef(
                            id: "room.broken", name: "Broken", short: "BRK",
                            category: Production, width: 2,
                            recipe: Some(RecipeDef(
                                craft_ticks: 10,
                                inputs: [IoEntryDef(item: "item.nonexistent", amount: 1, buffer_max: 4)],
                                outputs: [IoEntryDef(item: "item.nonexistent", amount: 1, buffer_max: 4)],
                            )),
                        )"# as &[u8],
                    )),
                    _ => EmbeddedSource.read(path),
                }
            }
            fn list(&self, prefix: &str) -> Vec<String> {
                if prefix == "rooms" {
                    let mut all = EmbeddedSource.list(prefix);
                    all.push("rooms/broken.ron".into());
                    all.sort();
                    return all;
                }
                EmbeddedSource.list(prefix)
            }
        }

        let errors = Content::load(&BrokenSource).expect_err("broken pack must fail");
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("unknown item item.nonexistent")),
            "expected an unknown-item error, got {errors:?}"
        );
    }
}
