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

use crate::ids::{DaypartIdx, ItemIdx, RoomIdx, ShaftIdx, TerrainIdx};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeDef {
    pub item: String,
    /// Ticks per item at 100% terrain yield.
    pub ticks_per_item: u32,
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

/// Canopy sails: charge from sunlight, scaled by how much of it reaches
/// this stretch of jungle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolarDef {
    /// Charge per 100 ticks at 100% exposure.
    pub charge_per_100_ticks: i64,
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
    #[serde(default)]
    pub storage: Option<StorageDef>,
    #[serde(default)]
    pub solar: Option<SolarDef>,
    #[serde(default)]
    pub burner: Option<BurnerDef>,
    #[serde(default)]
    pub bank: Option<BankDef>,
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
    /// Relative weight when the generator picks the next band.
    pub weight: i64,
    /// Decorative features scattered per 100 paces of this band.
    pub features_per_100_paces: i64,
    /// Feature glyphs this band may scatter. Presentation only.
    pub feature_kinds: Vec<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrewBalance {
    pub starting_crew: u8,
    pub walk_ticks_per_slot: u32,
    pub climb_ticks_per_floor: u32,
    pub load_ticks: u32,
    pub unload_ticks: u32,
    /// Items a crew member carries in one trip.
    pub carry_capacity: i64,
    /// Ticks blocked before the cross-section tints a crew member red.
    pub stress_ticks: u32,
}

// ---------------------------------------------------------------------------
// The loaded registry
// ---------------------------------------------------------------------------

/// The interned content pack. Immutable for the life of a run.
#[derive(Debug, Clone)]
pub struct Content {
    pub balance: Balance,
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
    /// XXH3 of every pack byte, path-ordered. Stamped into replays.
    pub content_hash: u64,
    /// Pre-resolved room recipes and costs, so no system ever touches a
    /// string during a tick.
    pub room_runtime: Vec<RoomRuntime>,
    pub shaft_runtime: Vec<ShaftRuntime>,
    pub terrain_runtime: Vec<TerrainRuntime>,
}

/// A room definition with every string already resolved to an index.
#[derive(Debug, Clone)]
pub struct RoomRuntime {
    pub build_cost: Vec<(ItemIdx, i64)>,
    pub recipe_inputs: Vec<(ItemIdx, i64, i64)>,
    pub recipe_outputs: Vec<(ItemIdx, i64, i64)>,
    pub craft_ticks: u32,
    pub intake_item: Option<ItemIdx>,
    pub intake_ticks_per_item: u32,
    pub intake_buffer_max: i64,
    pub shelves: u8,
    pub per_shelf: i64,
    /// Fuel item and inbox size for a burner. Sized from the recipe
    /// rather than authored: six burns of runway is enough to ride out
    /// a delayed haul without hiding a persistent shortfall.
    pub burner_fuel: Option<(ItemIdx, i64)>,
}

#[derive(Debug, Clone)]
pub struct TerrainRuntime {
    pub yield_pct: i64,
    pub sun_pct: i64,
    pub weight: i64,
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
        let mut items = parse_dir::<ItemDef>(source, "items", &mut errors, &mut hasher);
        let mut rooms = parse_dir::<RoomDef>(source, "rooms", &mut errors, &mut hasher);
        let mut shafts = parse_dir::<ShaftDef>(source, "shafts", &mut errors, &mut hasher);
        let mut terrain = parse_dir::<TerrainDef>(source, "terrain", &mut errors, &mut hasher);
        let mut dayparts = parse_dir::<DaypartDef>(source, "dayparts", &mut errors, &mut hasher);

        if !errors.is_empty() {
            return Err(errors);
        }

        // Sort by id so indices are a pure function of the pack content
        // and never of directory-walk order.
        items.sort_by(|a, b| a.id.cmp(&b.id));
        rooms.sort_by(|a, b| a.id.cmp(&b.id));
        shafts.sort_by(|a, b| a.id.cmp(&b.id));
        terrain.sort_by(|a, b| a.id.cmp(&b.id));
        // Dayparts are the exception: they index by time of day, not by
        // name, so the day would run out of order if sorted by id.
        dayparts.sort_by_key(|part| part.start_permille);

        let Some(balance) = balance else {
            return Err(vec![LoadError {
                path: "balance.ron".into(),
                message: "missing".into(),
            }]);
        };

        let mut content = Self {
            balance,
            items,
            rooms,
            shafts,
            terrain,
            dayparts,
            content_hash: hasher.digest(),
            room_runtime: Vec::new(),
            shaft_runtime: Vec::new(),
            terrain_runtime: Vec::new(),
        };

        content.resolve(&mut errors);
        validate(&content, &mut errors);

        if errors.is_empty() {
            Ok(content)
        } else {
            Err(errors)
        }
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

            let (intake_item, intake_ticks_per_item, intake_buffer_max) = match &room.intake {
                Some(intake) => (
                    Some(lookup(&intake.item, "intake")),
                    intake.ticks_per_item,
                    intake.buffer_max,
                ),
                None => (None, 0, 0),
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

            room_runtime.push(RoomRuntime {
                build_cost,
                recipe_inputs,
                recipe_outputs,
                craft_ticks,
                intake_item,
                intake_ticks_per_item,
                intake_buffer_max,
                shelves,
                per_shelf,
                burner_fuel,
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
                weight: band.weight,
            })
            .collect();
    }
}

fn validate(content: &Content, errors: &mut Vec<LoadError>) {
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
    if content.terrain_runtime.iter().all(|t| t.weight <= 0) {
        errors.push(LoadError {
            path: "terrain".into(),
            message: "no terrain band has a positive weight".into(),
        });
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
        if let Some(intake) = &def.intake
            && intake.ticks_per_item == 0
        {
            errors.push(LoadError {
                path: path.clone(),
                message: "intake ticks_per_item must be positive".into(),
            });
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
            RoomCategory::Energy => {
                def.solar.is_some() || def.burner.is_some() || def.bank.is_some()
            }
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
    if content.balance.crew.walk_ticks_per_slot == 0
        || content.balance.crew.climb_ticks_per_floor == 0
    {
        errors.push(LoadError {
            path: "balance.ron".into(),
            message: "crew movement rates must be positive".into(),
        });
    }

    validate_shafts(content, errors);
    validate_clock(content, errors);
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
