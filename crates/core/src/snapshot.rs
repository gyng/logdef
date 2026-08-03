//! Snapshots — the presentation boundary.
//!
//! The simulation never crosses the bridge. What crosses is a compact,
//! read-only description of what to draw, built fresh each frame.
//!
//! This is also the **only** place fixed point becomes floating point.
//! `Fx::to_f32` may be called here and nowhere else; the values it
//! produces are for pixels, never for decisions.
//!
//! One accessor per frame, not one per subsystem. v1 made a dozen
//! separate bridge calls at 30hz; the shape below is already grouped
//! the way a worker-and-shared-buffer bridge would want it, so moving
//! to one later is a transport change, not a redesign.

use serde::{Deserialize, Serialize};

use crate::content::{Content, RoomCategory};
use crate::fx::{FX_ONE, Paces};
use crate::state::{GameState, RunStats, SimSpeed};

// ---------------------------------------------------------------------------
// Per-frame view
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewSnapshot {
    pub tick: u64,
    pub speed: SimSpeed,
    /// Fraction of a tick elapsed, for render interpolation.
    pub alpha: f32,
    pub clock: ClockView,
    pub power: PowerView,
    pub world: WorldView,
    pub tower: TowerView,
    pub siege: SiegeView,
    pub journey: JourneyView,
    pub crew: Vec<CrewView>,
    /// Summed across every storeroom shelf — what construction spends.
    pub stock: Vec<StockView>,
    pub stats: RunStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockView {
    pub day: u32,
    /// How far through the day, in per-mille. Drives the sky.
    pub permille: i64,
    /// Index into `catalog.dayparts`.
    pub daypart: u16,
    /// Sunlight before terrain.
    pub sun_pct: i64,
    /// Sunlight after terrain — what the sails actually receive.
    pub exposure_pct: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerView {
    pub charge: i64,
    pub capacity: i64,
    /// Stored fraction in per-mille, so the gauge needs no division.
    pub fill_permille: i64,
    pub income_last: i64,
    pub spent_last: i64,
    /// Something went unpowered this tick. The tower dims.
    pub brownout: bool,
    /// Lamps are on — either it is daylight, or the tower can afford them.
    pub lit: bool,
    /// The legs are running.
    pub walking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiegeView {
    pub enemies: Vec<EnemyView>,
    /// How much attention the tower has drawn. The only difficulty dial
    /// in the game, and the player turns it by playing.
    pub provocation: i64,
    pub provocation_max: i64,
    /// Panels, rooms and shafts averaged by hit points, in per-mille.
    pub integrity_permille: i64,
    /// Creatures seen off. Reported, never celebrated.
    pub repelled: u64,
    /// The Heartseed is gone. The run is over.
    pub lost: bool,
    /// Poles it would take to put everything right, so the player can
    /// see the bill before deciding what to triage.
    pub repair_cost: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyView {
    pub id: u32,
    /// Indexes `catalog.enemies`.
    pub def: u16,
    /// Whole paces, on the same axis as `world.distance`.
    pub at: f32,
    pub hp_permille: i64,
    pub state: EnemyStateTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnemyStateTag {
    Approach,
    Attack,
    Dying,
    Leaving,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldView {
    /// Whole paces walked, with fraction, for smooth parallax.
    pub distance: f32,
    /// Terrain index the tower is standing in, if any.
    pub band: Option<u16>,
    pub yield_pct: i64,
    pub bands: Vec<BandView>,
    pub features: Vec<FeatureView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandView {
    pub start: f32,
    pub end: f32,
    pub kind: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureView {
    pub at: f32,
    /// Terrain kind of the band this feature stands in — look the
    /// glyph up in `catalog.terrain[band].feature_kinds[kind]`.
    pub band: u16,
    pub kind: u8,
    pub scale: u8,
    pub layer: u8,
    /// Scrap still in this ruin, and zero for anything that is not one.
    /// A ruin worth stopping at and one already stripped are drawn
    /// differently, so this has to cross the bridge.
    pub salvage: i64,
}

/// Where the run has got to, and what it is being asked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyView {
    /// Indexes `catalog.regions`.
    pub region: u16,
    /// How far through the current region, in per-mille.
    pub region_permille: i64,
    /// Paces still to walk before the far edge of the journey.
    pub remaining: f32,
    /// The split ahead, if the route has one the tower has not crossed.
    pub fork: Option<ForkView>,
    /// The branch the tower is walking through, if any. Indexes
    /// `catalog.branches`.
    pub branch: Option<u16>,
    /// Why the tower is standing still, if it is. The three reasons
    /// look identical in the cross-section and mean entirely different
    /// things, so the renderer is told which one it is drawing.
    pub halt: HaltView,
    /// Paces to the settlement, once it is somewhere ahead. `None`
    /// once the tower has passed it — there is no going back down the
    /// axis, so a settlement behind you is gone rather than distant.
    ///
    /// The renderer needs this to draw the place coming, which is the
    /// only warning a player gets that stopping is about to be worth
    /// something.
    pub enclave_ahead: Option<f32>,
    /// Berthed at the enclave right now.
    pub at_enclave: bool,
    /// What the enclave has left, one entry per authored offer.
    pub offers: Vec<i64>,
    pub recruits: u8,
    /// How many more times the settlement will plate the shell.
    pub shell_work: u8,
    /// Hit points already added to every panel by shell work.
    pub shell_bonus: i64,
    /// The far edge of the last region, reached. The run is over.
    pub arrived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkView {
    /// Paces from the tower to the split.
    pub ahead: f32,
    /// The two archetypes on offer. Index `catalog.branches`.
    pub branches: [u16; 2],
    /// Which one the player has picked, if they have.
    pub answer: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HaltView {
    /// The legs are running.
    Walking,
    /// The player stopped it.
    Stopped,
    /// Out of charge, though the player asked for the legs.
    Brownout,
    /// Standing at a fork with no answer. Waiting for the player, and
    /// it must not read as a freeze.
    Fork,
    /// The far edge of the journey.
    Arrived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerView {
    pub floors: Vec<FloorView>,
    pub shafts: Vec<ShaftView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorView {
    pub index: u8,
    pub slots: u8,
    pub rooms: Vec<RoomView>,
    /// The outer wall, in per-mille. Zero is a hole in the tower's skin.
    pub panel_permille: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomView {
    pub id: u32,
    pub def: u16,
    pub slot: u8,
    pub width: u8,
    /// Ticks into the current craft. Zero for rooms without a recipe.
    pub progress: u32,
    pub inputs: Vec<StackView>,
    pub outputs: Vec<StackView>,
    pub shelves: Vec<ShelfView>,
    /// Waiting on an input, or backed up on its output. A stalled room
    /// is drawn quiet — that silence is the warning.
    pub stalled: bool,
    /// Switched on by the player.
    pub active: bool,
    /// A sail that is no longer on the top floor. Shaded rooms draw
    /// stalled, and this is why.
    pub shaded: bool,
    pub health_permille: i64,
    /// Damaged past the point of working at all.
    pub wrecked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackView {
    pub item: u16,
    pub count: i64,
    pub max: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShelfView {
    pub item: Option<u16>,
    pub count: i64,
    pub max: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaftView {
    pub id: u32,
    /// Indexes `catalog.shafts`.
    pub def: u16,
    pub kind: String,
    pub low: u8,
    pub high: u8,
    pub slot: u8,
    pub capacity: u8,
    /// Crew on the stairs. Zero for shafts with cars.
    pub riders: u8,
    pub cars: Vec<CarView>,
    /// Crew queued at this shaft right now, across all its floors.
    pub queued: u8,
    pub health_permille: i64,
    /// Cut through. Nothing travels on it until it is repaired.
    pub severed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarView {
    /// Fractional floor position, for smooth travel.
    pub floor: f32,
    /// One of: idle, up, down.
    pub dir: String,
    /// One of: idle, moving, dwelling.
    pub state: String,
    /// Units aboard, against the shaft's capacity.
    pub load: u8,
    pub stops: Vec<u8>,
    /// Items aboard. Dumbwaiters only.
    pub freight: Vec<StockView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewView {
    pub id: u32,
    pub name: String,
    /// Fractional floor coordinate — smooth while climbing.
    pub floor: f32,
    /// Fractional slot coordinate — smooth while walking.
    pub slot: f32,
    pub state: CrewStateTag,
    pub carrying: Option<StockView>,
    pub wait_ticks: u32,
    /// Blocked long enough to tint red in the cross-section.
    pub stressed: bool,
    /// Cosmetic-stream draw: animation phase offset, and — frontend
    /// side — which face and which of several equivalent bark lines
    /// belong to this person. `portrait = fidget % faces` is the whole
    /// mechanism, and it costs no new state and no new draw.
    pub fidget: u16,
    /// Ticks since their last meal.
    pub hunger: u32,
    /// Ticks of work left in them.
    pub rested: u32,
    /// Which half of the rota they are on.
    pub shift: ShiftTag,
    /// Actually asleep, as against merely off shift and walking to bed.
    pub asleep: bool,
}

/// Which half of the rota, flattened for the renderer.
///
/// Deliberately **not** `rename_all = "lowercase"`, unlike the tags
/// around it. `SetShift` carries a `content::Shift`, which serialises as
/// `"Day"`/`"Night"`; spelling the same fact two ways depending on which
/// direction it is crossing the bridge is the kind of contract detail
/// that costs somebody an afternoon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShiftTag {
    Day,
    Night,
}

/// What a crew member is doing, flattened for the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CrewStateTag {
    Idle,
    Walk,
    /// Queued at a shaft column: waiting for room on the stairs, or for
    /// a car going their way.
    Board,
    Climb,
    /// Aboard a car.
    Ride,
    /// Working on damage.
    Mend,
    Load,
    Unload,
    /// Sat down to a meal.
    Eat,
    /// Off shift — in a hammock if a bed was free, on the deck if not.
    Sleep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockView {
    pub item: u16,
    pub count: i64,
}

// ---------------------------------------------------------------------------
// Static catalog
// ---------------------------------------------------------------------------

/// Everything the UI needs about the content pack. Fetched once at
/// startup — it cannot change while a run is live.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogSnapshot {
    pub content_hash: String,
    pub items: Vec<ItemInfo>,
    pub rooms: Vec<RoomInfo>,
    pub shafts: Vec<ShaftInfo>,
    pub terrain: Vec<TerrainInfo>,
    pub dayparts: Vec<DaypartInfo>,
    pub enemies: Vec<EnemyInfo>,
    pub regions: Vec<RegionInfo>,
    pub branches: Vec<BranchInfo>,
    pub floor_cost: Vec<CostInfo>,
    pub max_floors: u8,
    pub floor_slots: u8,
    pub stress_ticks: u32,
    pub ticks_per_day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaftInfo {
    pub id: String,
    pub name: String,
    pub short: String,
    /// One of: Stairs, Dumbwaiter, Elevator.
    pub kind: String,
    pub build_cost: Vec<CostInfo>,
    pub min_span: u8,
    /// Zero means "as tall as the tower".
    pub max_span: u8,
    pub capacity: u8,
    pub ticks_per_floor: u32,
    pub charge_per_floor: i64,
    pub cars: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyInfo {
    pub id: String,
    pub name: String,
    pub glyph: String,
    /// One of: Ground, Canopy, Burrow.
    pub approach: String,
    pub night_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaypartInfo {
    pub id: String,
    pub name: String,
    pub start_permille: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemInfo {
    pub id: String,
    pub name: String,
    pub glyph: String,
    pub order: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    pub short: String,
    pub category: RoomCategory,
    pub width: u8,
    pub build_cost: Vec<CostInfo>,
    pub max_floor: Option<u8>,
    pub unique: bool,
    pub craft_ticks: u32,
    pub inputs: Vec<CostInfo>,
    pub outputs: Vec<CostInfo>,
    pub intake_item: Option<u16>,
    pub shelves: u8,
    /// Only works on the roof. Growing taller shades it.
    pub top_floor_only: bool,
    /// Charge drawn per tick while working.
    pub power_draw: i64,
    /// Makes charge from sunlight.
    pub solar: bool,
    /// Burns an item for charge, and can be switched off.
    pub burner: bool,
    /// Charge capacity this room adds.
    pub bank_capacity: i64,
    /// Shoots back, and eats ammo off the same shelves as everything else.
    pub defence: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostInfo {
    pub item: u16,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainInfo {
    pub id: String,
    pub name: String,
    pub yield_pct: i64,
    pub feature_kinds: Vec<String>,
    /// Which of those kinds are ruins — a place the tower can berth at
    /// and work, rather than scenery.
    pub ruin_kinds: Vec<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionInfo {
    pub id: String,
    pub name: String,
    /// The settlement in this region, if it has one.
    pub enclave: Option<EnclaveInfo>,
}

/// What a settlement will do for you, so a trade board can say what it
/// is offering rather than that it is offering something.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnclaveInfo {
    pub name: String,
    /// How far into its region it stands.
    pub at_paces: i64,
    pub offers: Vec<OfferInfo>,
    pub recruits: u8,
    pub recruit_cost: Vec<CostInfo>,
    /// What one round of shell work costs and adds, if they do it.
    pub reinforce: Option<ReinforceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinforceInfo {
    pub cost: Vec<CostInfo>,
    /// Added to every panel, present and future.
    pub panel_hp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferInfo {
    pub give: CostInfo,
    pub take: CostInfo,
    /// How many times it could ever be taken. What is *left* is in
    /// `journey.offers`, which is state rather than content.
    pub stock: i64,
}

/// One side of a fork.
///
/// The two heaviest terrain kinds and a word for the threat are what
/// the fork card shows, and both are *derived* from the branch's own
/// data rather than authored alongside it — a hand-written line
/// describing a branch drifts out of step with its palette during
/// tuning, and a game that misdescribes the only informed choice it
/// asks the player to make is worse than one that describes it drily.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub id: String,
    pub name: String,
    /// Heaviest first. Indexes `catalog.terrain`.
    pub terrain: Vec<u16>,
    /// Against the region it interrupts: 100 is as usual.
    pub threat_pct: i64,
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

#[allow(clippy::cast_precision_loss)]
fn paces_to_f32(value: Paces) -> f32 {
    value as f32 / FX_ONE as f32
}

#[must_use]
pub fn build_view(state: &GameState, content: &Content, alpha: f32) -> ViewSnapshot {
    ViewSnapshot {
        tick: state.tick,
        speed: state.speed,
        alpha,
        clock: build_clock(state, content),
        power: build_power(state),
        world: build_world(state, content),
        tower: build_tower(state, content),
        siege: build_siege(state, content),
        journey: build_journey(state, content),
        crew: build_crew(state, content),
        stock: build_stock(state),
        stats: state.stats.clone(),
    }
}

fn build_siege(state: &GameState, content: &Content) -> SiegeView {
    SiegeView {
        enemies: state
            .siege
            .enemies
            .iter()
            .map(|enemy| EnemyView {
                id: enemy.id.0,
                def: enemy.def.0,
                at: paces_to_f32(enemy.at),
                hp_permille: {
                    let max = content.enemy(enemy.def).hp.max(1);
                    (enemy.hp.max(0) * 1000 / max).min(1000)
                },
                state: match enemy.state {
                    crate::state::EnemyState::Approaching => EnemyStateTag::Approach,
                    crate::state::EnemyState::Attacking { .. } => EnemyStateTag::Attack,
                    crate::state::EnemyState::Dying => EnemyStateTag::Dying,
                    crate::state::EnemyState::Leaving => EnemyStateTag::Leaving,
                },
            })
            .collect(),
        provocation: state.siege.provocation,
        provocation_max: content.balance.siege.provocation_max,
        integrity_permille: crate::systems::siege::tower_integrity_permille(state),
        repelled: state.siege.repelled,
        lost: state.siege.lost,
        repair_cost: crate::systems::repair::outstanding_repair_cost(state, content),
    }
}

fn build_clock(state: &GameState, content: &Content) -> ClockView {
    ClockView {
        day: state.clock.day,
        permille: state.clock.permille(content),
        daypart: state.clock.daypart(content).0,
        sun_pct: state.clock.sun_pct(content),
        exposure_pct: crate::systems::power::exposure_pct(state, content),
    }
}

fn build_power(state: &GameState) -> PowerView {
    PowerView {
        charge: state.power.charge,
        capacity: state.power.capacity,
        fill_permille: state.power.fill_permille(),
        income_last: state.power.income_last,
        spent_last: state.power.spent_last,
        brownout: state.power.brownout,
        lit: state.power.lit,
        walking: state.walking,
    }
}

fn build_regions(content: &Content) -> Vec<RegionInfo> {
    content
        .regions
        .iter()
        .enumerate()
        .map(|(i, region)| RegionInfo {
            id: region.id.clone(),
            name: region.name.clone(),
            enclave: region.enclave.as_ref().map(|def| {
                let rt = content
                    .region_rt(crate::ids::RegionIdx(i as u16))
                    .enclave
                    .as_ref()
                    .expect("a region with an enclave has one at runtime");
                EnclaveInfo {
                    name: def.name.clone(),
                    at_paces: def.at_paces,
                    offers: rt
                        .offers
                        .iter()
                        .map(|offer| OfferInfo {
                            give: CostInfo {
                                item: offer.give.0.0,
                                amount: offer.give.1,
                            },
                            take: CostInfo {
                                item: offer.take.0.0,
                                amount: offer.take.1,
                            },
                            stock: offer.stock,
                        })
                        .collect(),
                    recruits: def.recruits,
                    reinforce: rt.reinforce.as_ref().map(|(cost, panel_hp)| ReinforceInfo {
                        cost: cost
                            .iter()
                            .map(|(item, amount)| CostInfo {
                                item: item.0,
                                amount: *amount,
                            })
                            .collect(),
                        panel_hp: *panel_hp,
                    }),
                    recruit_cost: rt
                        .recruit_cost
                        .iter()
                        .map(|(item, amount)| CostInfo {
                            item: item.0,
                            amount: *amount,
                        })
                        .collect(),
                }
            }),
        })
        .collect()
}

fn build_branches(content: &Content) -> Vec<BranchInfo> {
    content
        .branches
        .iter()
        .enumerate()
        .map(|(i, branch)| {
            let mut palette = content
                .branch_rt(crate::ids::BranchIdx(i as u16))
                .palette
                .clone();
            // Heaviest first, ties by index so the order is a pure
            // function of the pack.
            palette.sort_by_key(|(idx, weight)| (-weight, idx.0));
            BranchInfo {
                id: branch.id.clone(),
                name: branch.name.clone(),
                terrain: palette.iter().map(|(idx, _)| idx.0).collect(),
                threat_pct: branch.threat_pct,
            }
        })
        .collect()
}

fn build_journey(state: &GameState, content: &Content) -> JourneyView {
    let world = &state.world;
    let region = world.region_at(world.distance);
    let start = world.region_start_of(region);
    let end = world
        .journey
        .get(region.get())
        .map_or(start, |roll| roll.end);
    let span = (end - start).max(1);

    JourneyView {
        region: region.0,
        region_permille: ((world.distance - start) * 1000 / span).clamp(0, 1000),
        remaining: paces_to_f32((world.journey_end() - world.distance).max(0)),
        fork: world.fork.map(|fork| ForkView {
            ahead: paces_to_f32((fork.at - world.distance).max(0)),
            branches: [fork.branches[0].0, fork.branches[1].0],
            answer: fork.answer,
        }),
        branch: world.branch.map(|branch| branch.def.0),
        halt: halt_reason(state),
        enclave_ahead: world
            .enclave_at(content)
            .filter(|at| *at >= world.distance)
            .map(|at| paces_to_f32(at - world.distance)),
        at_enclave: world.at_enclave(content, state.strode),
        offers: state.enclave_stock.clone(),
        recruits: state.enclave_recruits,
        shell_work: state.shell_work_left,
        shell_bonus: state.tower.shell_bonus,
        arrived: state.arrived,
    }
}

/// Why the tower is standing still.
///
/// All four reasons look identical in the cross-section — same
/// silhouette, same still legs — and mean completely different things.
/// A tower waiting at a fork in particular has to read as *waiting for
/// you* rather than as a frozen game, so the renderer is told which one
/// it is drawing instead of having to infer it.
fn halt_reason(state: &GameState) -> HaltView {
    if state.strode {
        return HaltView::Walking;
    }
    if state.arrived {
        return HaltView::Arrived;
    }
    if state
        .world
        .fork
        .is_some_and(|fork| fork.answer.is_none() && state.world.distance >= fork.at)
    {
        return HaltView::Fork;
    }
    if state.walking {
        // The player asked for the legs and did not get them.
        return HaltView::Brownout;
    }
    HaltView::Stopped
}

fn build_world(state: &GameState, content: &Content) -> WorldView {
    let world = &state.world;

    let bands: Vec<BandView> = world
        .bands
        .iter()
        .map(|band| BandView {
            start: paces_to_f32(band.start),
            end: paces_to_f32(band.end()),
            kind: band.kind.0,
        })
        .collect();

    // Bands and features are both sorted, so a single walk attaches
    // each feature to its band without a search per feature.
    let mut features = Vec::with_capacity(world.features.len());
    let mut cursor = 0usize;
    for feature in &world.features {
        while cursor + 1 < world.bands.len() && world.bands[cursor].end() <= feature.at {
            cursor += 1;
        }
        let band = world.bands.get(cursor);
        if band.is_none_or(|b| !b.contains(feature.at)) {
            continue;
        }
        features.push(FeatureView {
            at: paces_to_f32(feature.at),
            band: band.map_or(0, |b| b.kind.0),
            kind: feature.kind,
            scale: feature.scale,
            layer: feature.layer,
            salvage: feature.salvage,
        });
    }

    WorldView {
        distance: paces_to_f32(world.distance),
        band: world.band_at(world.distance).map(|b| b.kind.0),
        yield_pct: world.current_yield_pct(content),
        bands,
        features,
    }
}

fn build_tower(state: &GameState, content: &Content) -> TowerView {
    let top = state.tower.top_floor();
    let floors = state
        .tower
        .floors
        .iter()
        .map(|floor| FloorView {
            index: floor.index,
            slots: floor.slots,
            panel_permille: floor.panel.permille(),
            rooms: floor
                .rooms
                .iter()
                .map(|room| {
                    let rt = content.room_rt(room.def);
                    let starved = rt
                        .recipe_inputs
                        .iter()
                        .enumerate()
                        .any(|(i, (_, amount, _))| {
                            room.inputs.get(i).is_none_or(|s| s.count < *amount)
                        });
                    let backed_up = room.outputs.iter().any(crate::state::Stack::is_full);
                    let wrecked = room.health.permille() < content.balance.siege.wrecked_permille;
                    let def = content.room(room.def);
                    // A sail below the roof is in the tower's own
                    // shadow. Growing taller has a cost, and this is
                    // where the player is shown it.
                    let shaded = def.top_floor_only && floor.index != top;
                    let burner_dry = def.burner.as_ref().is_some_and(|burner| {
                        room.inputs
                            .last()
                            .is_none_or(|fuel| fuel.count < burner.fuel_per_burn)
                    });
                    // An emplacement with an empty rack is quiet for
                    // exactly the reason a starved mill is, and has to
                    // read that way — the whole point of feeding a
                    // battery off the ordinary shelves is that it fails
                    // like everything else. Without this it kept its
                    // working colours while sitting silent, which is
                    // the one thing the cross-section must not do.
                    let magazine_dry = def.defence.as_ref().is_some_and(|defence| {
                        rt.defence_ammo.is_some_and(|ammo| {
                            room.inputs
                                .iter()
                                .find(|stack| stack.item == ammo)
                                .is_none_or(|rack| rack.count < defence.ammo_per_shot)
                        })
                    });
                    let has_work = rt.craft_ticks > 0 || rt.intake_item.is_some();

                    RoomView {
                        id: room.id.0,
                        def: room.def.0,
                        slot: room.slot,
                        width: room.width,
                        progress: room.progress,
                        inputs: room.inputs.iter().map(stack_view).collect(),
                        outputs: room.outputs.iter().map(stack_view).collect(),
                        shelves: room
                            .shelves
                            .iter()
                            .map(|shelf| ShelfView {
                                item: shelf.item.map(|i| i.0),
                                count: shelf.count,
                                max: shelf.max,
                            })
                            .collect(),
                        stalled: (has_work && (starved || backed_up))
                            || shaded
                            || burner_dry
                            || magazine_dry
                            || wrecked
                            || !room.active,
                        active: room.active,
                        shaded,
                        health_permille: room.health.permille(),
                        wrecked,
                    }
                })
                .collect(),
        })
        .collect();

    let shafts = state
        .tower
        .shafts
        .iter()
        .map(|shaft| ShaftView {
            id: shaft.id.0,
            def: shaft.def.0,
            kind: format!("{:?}", shaft.kind),
            low: shaft.low,
            high: shaft.high,
            slot: shaft.slot,
            capacity: shaft.capacity,
            riders: shaft.riders,
            cars: shaft
                .cars
                .iter()
                .enumerate()
                .map(|(index, car)| CarView {
                    floor: car.pos.to_f32(),
                    dir: match car.dir {
                        crate::state::CarDir::Idle => "idle",
                        crate::state::CarDir::Up => "up",
                        crate::state::CarDir::Down => "down",
                    }
                    .into(),
                    state: match car.state {
                        crate::state::CarState::Idle => "idle",
                        crate::state::CarState::Moving => "moving",
                        crate::state::CarState::Dwelling { .. } => "dwelling",
                    }
                    .into(),
                    load: shaft.car_load(index, &state.crew),
                    stops: car.stops.clone(),
                    freight: car
                        .freight
                        .iter()
                        .map(|stack| StockView {
                            item: stack.item.0,
                            count: stack.count,
                        })
                        .collect(),
                })
                .collect(),
            queued: state
                .crew
                .iter()
                .filter(|member| {
                    matches!(member.state, crate::state::CrewState::Boarding { shaft: at, .. }
                        if at == shaft.id)
                })
                .count()
                .min(255) as u8,
            health_permille: shaft.health.permille(),
            severed: shaft.is_severed(),
        })
        .collect();

    TowerView { floors, shafts }
}

fn stack_view(stack: &crate::state::Stack) -> StackView {
    StackView {
        item: stack.item.0,
        count: stack.count,
        max: stack.max,
    }
}

fn build_crew(state: &GameState, content: &Content) -> Vec<CrewView> {
    let stress = content.balance.crew.stress_ticks;
    state
        .crew
        .iter()
        .map(|member| CrewView {
            id: member.id.0,
            name: member.name.clone(),
            floor: member.floor_fx.to_f32(),
            slot: member.slot_fx.to_f32(),
            state: match member.state {
                crate::state::CrewState::Idle => CrewStateTag::Idle,
                crate::state::CrewState::Walking { .. } => CrewStateTag::Walk,
                crate::state::CrewState::Boarding { .. } => CrewStateTag::Board,
                crate::state::CrewState::Climbing { .. } => CrewStateTag::Climb,
                crate::state::CrewState::Riding { .. } => CrewStateTag::Ride,
                crate::state::CrewState::Repairing { .. } => CrewStateTag::Mend,
                crate::state::CrewState::Loading { .. } => CrewStateTag::Load,
                crate::state::CrewState::Unloading { .. } => CrewStateTag::Unload,
                crate::state::CrewState::Eating { .. } => CrewStateTag::Eat,
                crate::state::CrewState::Sleeping => CrewStateTag::Sleep,
            },
            carrying: member.carrying.map(|(item, count)| StockView {
                item: item.0,
                count,
            }),
            wait_ticks: member.wait_ticks,
            stressed: member.wait_ticks >= stress,
            fidget: member.fidget,
            hunger: member.hunger,
            rested: member.rested,
            shift: match member.shift {
                crate::content::Shift::Day => ShiftTag::Day,
                crate::content::Shift::Night => ShiftTag::Night,
            },
            asleep: member.is_asleep(),
        })
        .collect()
}

fn build_stock(state: &GameState) -> Vec<StockView> {
    let mut totals: Vec<StockView> = Vec::new();
    for shelf in state
        .tower
        .floors
        .iter()
        .flat_map(|f| f.rooms.iter())
        .flat_map(|r| r.shelves.iter())
    {
        let Some(item) = shelf.item else {
            continue;
        };
        match totals.iter_mut().find(|s| s.item == item.0) {
            Some(entry) => entry.count += shelf.count,
            None => totals.push(StockView {
                item: item.0,
                count: shelf.count,
            }),
        }
    }
    totals.sort_by_key(|s| s.item);
    totals
}

#[must_use]
pub fn build_catalog(content: &Content) -> CatalogSnapshot {
    let cost = |entries: &[(crate::ids::ItemIdx, i64)]| -> Vec<CostInfo> {
        entries
            .iter()
            .map(|(item, amount)| CostInfo {
                item: item.0,
                amount: *amount,
            })
            .collect()
    };
    let io = |entries: &[(crate::ids::ItemIdx, i64, i64)]| -> Vec<CostInfo> {
        entries
            .iter()
            .map(|(item, amount, _)| CostInfo {
                item: item.0,
                amount: *amount,
            })
            .collect()
    };

    CatalogSnapshot {
        // As a string: u64 loses precision through JSON's number type.
        content_hash: format!("{:016x}", content.content_hash),
        items: content
            .items
            .iter()
            .map(|item| ItemInfo {
                id: item.id.clone(),
                name: item.name.clone(),
                glyph: item.glyph.clone(),
                order: item.order,
            })
            .collect(),
        rooms: content
            .rooms
            .iter()
            .enumerate()
            .map(|(i, room)| {
                let rt = &content.room_runtime[i];
                RoomInfo {
                    id: room.id.clone(),
                    name: room.name.clone(),
                    short: room.short.clone(),
                    category: room.category,
                    width: room.width,
                    build_cost: cost(&rt.build_cost),
                    max_floor: room.max_floor,
                    unique: room.unique,
                    craft_ticks: rt.craft_ticks,
                    inputs: io(&rt.recipe_inputs),
                    outputs: io(&rt.recipe_outputs),
                    intake_item: rt.intake_item.map(|i| i.0),
                    shelves: rt.shelves,
                    top_floor_only: room.top_floor_only,
                    power_draw: room.power_draw,
                    solar: room.solar.is_some(),
                    burner: room.burner.is_some(),
                    bank_capacity: room.bank.as_ref().map_or(0, |bank| bank.capacity),
                    defence: room.defence.is_some(),
                }
            })
            .collect(),
        shafts: content
            .shafts
            .iter()
            .enumerate()
            .map(|(i, shaft)| ShaftInfo {
                id: shaft.id.clone(),
                name: shaft.name.clone(),
                short: shaft.short.clone(),
                kind: format!("{:?}", shaft.kind),
                build_cost: cost(&content.shaft_runtime[i].build_cost),
                min_span: shaft.min_span,
                max_span: shaft.max_span,
                capacity: shaft.capacity,
                ticks_per_floor: shaft.ticks_per_floor,
                charge_per_floor: shaft.charge_per_floor,
                cars: shaft.cars,
            })
            .collect(),
        enemies: content
            .enemies
            .iter()
            .map(|enemy| EnemyInfo {
                id: enemy.id.clone(),
                name: enemy.name.clone(),
                glyph: enemy.glyph.clone(),
                approach: format!("{:?}", enemy.approach),
                night_only: enemy.night_only,
            })
            .collect(),
        dayparts: content
            .dayparts
            .iter()
            .map(|part| DaypartInfo {
                id: part.id.clone(),
                name: part.name.clone(),
                start_permille: part.start_permille,
            })
            .collect(),
        terrain: content
            .terrain
            .iter()
            .enumerate()
            .map(|(i, band)| TerrainInfo {
                id: band.id.clone(),
                name: band.name.clone(),
                yield_pct: band.yield_pct,
                feature_kinds: band.feature_kinds.clone(),
                ruin_kinds: content.terrain_runtime[i].ruin_feature.clone(),
            })
            .collect(),
        regions: build_regions(content),
        branches: build_branches(content),
        floor_cost: content
            .balance
            .tower
            .floor_cost
            .iter()
            .filter_map(|entry| {
                content.item_idx(&entry.item).map(|item| CostInfo {
                    item: item.0,
                    amount: entry.amount,
                })
            })
            .collect(),
        max_floors: content.balance.tower.max_floors,
        floor_slots: content.balance.tower.floor_slots,
        stress_ticks: content.balance.crew.stress_ticks,
        ticks_per_day: content.balance.clock.ticks_per_day,
    }
}
