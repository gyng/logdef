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
    pub world: WorldView,
    pub tower: TowerView,
    pub crew: Vec<CrewView>,
    /// Summed across every storeroom shelf — what construction spends.
    pub stock: Vec<StockView>,
    pub stats: RunStats,
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
    pub kind: String,
    pub low: u8,
    pub high: u8,
    pub slot: u8,
    pub capacity: u8,
    pub riders: u8,
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
    /// Cosmetic-stream draw: animation phase offset.
    pub fidget: u16,
}

/// What a crew member is doing, flattened for the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CrewStateTag {
    Idle,
    Walk,
    /// Queued at a shaft column, waiting for capacity.
    Board,
    Climb,
    Load,
    Unload,
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
    pub terrain: Vec<TerrainInfo>,
    pub floor_cost: Vec<CostInfo>,
    pub max_floors: u8,
    pub floor_slots: u8,
    pub stress_ticks: u32,
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
        world: build_world(state, content),
        tower: build_tower(state, content),
        crew: build_crew(state, content),
        stock: build_stock(state),
        stats: state.stats.clone(),
    }
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
    let floors = state
        .tower
        .floors
        .iter()
        .map(|floor| FloorView {
            index: floor.index,
            slots: floor.slots,
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
                        stalled: has_work && (starved || backed_up),
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
            kind: format!("{:?}", shaft.kind),
            low: shaft.low,
            high: shaft.high,
            slot: shaft.slot,
            capacity: shaft.capacity,
            riders: shaft.riders,
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
                crate::state::CrewState::Loading { .. } => CrewStateTag::Load,
                crate::state::CrewState::Unloading { .. } => CrewStateTag::Unload,
            },
            carrying: member.carrying.map(|(item, count)| StockView {
                item: item.0,
                count,
            }),
            wait_ticks: member.wait_ticks,
            stressed: member.wait_ticks >= stress,
            fidget: member.fidget,
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
                }
            })
            .collect(),
        terrain: content
            .terrain
            .iter()
            .map(|band| TerrainInfo {
                id: band.id.clone(),
                name: band.name.clone(),
                yield_pct: band.yield_pct,
                feature_kinds: band.feature_kinds.clone(),
            })
            .collect(),
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
    }
}
