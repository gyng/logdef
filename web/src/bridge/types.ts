/**
 * TypeScript mirrors of the Rust snapshot types.
 *
 * These are a contract, not a convenience: every type here corresponds
 * to a struct in `crates/core/src/snapshot.rs`, and the two change in
 * the same commit or the renderer draws garbage. Field names match the
 * Rust field names exactly — serde emits them verbatim.
 */

export type SimSpeed = "Paused" | "X1" | "X2" | "X4";

export type RoomCategory = "Intake" | "Production" | "Storage" | "Energy" | "Heart";

export type CrewStateTag = "idle" | "walk" | "board" | "climb" | "ride" | "load" | "unload";

export type ShaftKind = "Stairs" | "Dumbwaiter" | "Elevator";

export type ShaftPriority = "Balanced" | "FreightFirst" | "CrewFirst";

export type CarDirTag = "idle" | "up" | "down";

export type CarStateTag = "idle" | "moving" | "dwelling";

// ---------------------------------------------------------------------------
// Per-frame view
// ---------------------------------------------------------------------------

export interface ViewSnapshot {
  tick: number;
  speed: SimSpeed;
  /** Fraction of a tick elapsed. For render interpolation. */
  alpha: number;
  clock: ClockView;
  power: PowerView;
  world: WorldView;
  tower: TowerView;
  crew: CrewView[];
  stock: StockView[];
  stats: RunStats;
}

export interface ClockView {
  day: number;
  /** How far through the day, in per-mille. Drives the sky. */
  permille: number;
  /** Indexes `catalog.dayparts`. */
  daypart: number;
  /** Sunlight before terrain. */
  sun_pct: number;
  /** Sunlight after terrain — what the sails actually receive. */
  exposure_pct: number;
}

export interface PowerView {
  charge: number;
  capacity: number;
  /** Stored fraction in per-mille, so the gauge needs no division. */
  fill_permille: number;
  income_last: number;
  spent_last: number;
  /** Something went unpowered this tick. The tower dims. */
  brownout: boolean;
  /** Lamps are on — daylight, or the tower can afford them. */
  lit: boolean;
  /** The legs are running. */
  walking: boolean;
}

export interface WorldView {
  /** Whole paces walked, with fraction. */
  distance: number;
  /** Terrain index under the tower, or null between bands. */
  band: number | null;
  yield_pct: number;
  bands: BandView[];
  features: FeatureView[];
}

export interface BandView {
  start: number;
  end: number;
  kind: number;
}

export interface FeatureView {
  at: number;
  /** Terrain kind of the band this stands in. Indexes `catalog.terrain`. */
  band: number;
  /** Indexes that terrain's `feature_kinds`. */
  kind: number;
  scale: number;
  /** Parallax depth: 0 far, 1 mid, 2 near. */
  layer: number;
}

export interface TowerView {
  floors: FloorView[];
  shafts: ShaftView[];
}

export interface FloorView {
  index: number;
  slots: number;
  rooms: RoomView[];
}

export interface RoomView {
  id: number;
  /** Indexes `catalog.rooms`. */
  def: number;
  slot: number;
  width: number;
  progress: number;
  inputs: StackView[];
  outputs: StackView[];
  shelves: ShelfView[];
  /** Starved, backed up, shaded, dry, or switched off. Drawn quiet. */
  stalled: boolean;
  /** Switched on by the player. */
  active: boolean;
  /** A sail no longer on the roof. The price of building higher. */
  shaded: boolean;
}

export interface StackView {
  item: number;
  count: number;
  max: number;
}

export interface ShelfView {
  item: number | null;
  count: number;
  max: number;
}

export interface ShaftView {
  id: number;
  /** Indexes `catalog.shafts`. */
  def: number;
  kind: ShaftKind;
  low: number;
  high: number;
  slot: number;
  capacity: number;
  /** Crew on the stairs. Zero for shafts with cars. */
  riders: number;
  cars: CarView[];
  /** Crew queued at this shaft right now, across all its floors. */
  queued: number;
}

export interface CarView {
  /** Fractional floor position, for smooth travel. */
  floor: number;
  dir: CarDirTag;
  state: CarStateTag;
  /** Units aboard, against the shaft's capacity. */
  load: number;
  stops: number[];
  /** Items aboard. Dumbwaiters only. */
  freight: StockView[];
}

export interface CrewView {
  id: number;
  name: string;
  /** Fractional floor coordinate. */
  floor: number;
  /** Fractional slot coordinate. */
  slot: number;
  state: CrewStateTag;
  carrying: StockView | null;
  wait_ticks: number;
  stressed: boolean;
  /** Cosmetic-stream draw: animation phase offset. */
  fidget: number;
}

export interface StockView {
  item: number;
  count: number;
}

export interface RunStats {
  hauls_completed: number;
  crafts_completed: number;
  items_harvested: number;
}

// ---------------------------------------------------------------------------
// Static catalog
// ---------------------------------------------------------------------------

export interface CatalogSnapshot {
  content_hash: string;
  items: ItemInfo[];
  rooms: RoomInfo[];
  shafts: ShaftInfo[];
  terrain: TerrainInfo[];
  dayparts: DaypartInfo[];
  floor_cost: CostInfo[];
  max_floors: number;
  floor_slots: number;
  stress_ticks: number;
  ticks_per_day: number;
}

export interface ShaftInfo {
  id: string;
  name: string;
  short: string;
  kind: ShaftKind;
  build_cost: CostInfo[];
  min_span: number;
  /** Zero means "as tall as the tower". */
  max_span: number;
  capacity: number;
  ticks_per_floor: number;
  charge_per_floor: number;
  cars: number;
}

export interface DaypartInfo {
  id: string;
  name: string;
  start_permille: number;
}

export interface ItemInfo {
  id: string;
  name: string;
  glyph: string;
  order: number;
}

export interface RoomInfo {
  id: string;
  name: string;
  short: string;
  category: RoomCategory;
  width: number;
  build_cost: CostInfo[];
  max_floor: number | null;
  unique: boolean;
  craft_ticks: number;
  inputs: CostInfo[];
  outputs: CostInfo[];
  intake_item: number | null;
  shelves: number;
  /** Only works on the roof. Growing taller shades it. */
  top_floor_only: boolean;
  /** Charge drawn per tick while working. */
  power_draw: number;
  /** Makes charge from sunlight. */
  solar: boolean;
  /** Burns an item for charge, and can be switched off. */
  burner: boolean;
  /** Charge capacity this room adds. */
  bank_capacity: number;
}

export interface CostInfo {
  item: number;
  amount: number;
}

export interface TerrainInfo {
  id: string;
  name: string;
  yield_pct: number;
  feature_kinds: string[];
}

// ---------------------------------------------------------------------------
// Commands and results
// ---------------------------------------------------------------------------

export type GameCommand =
  | { SetSpeed: { speed: SimSpeed } }
  | "BuildFloor"
  | { PlaceRoom: { room: string; floor: number; slot: number } }
  | { RemoveRoom: { floor: number; slot: number } }
  | { SetRoomActive: { floor: number; slot: number; active: boolean } }
  | { BuildShaft: { shaft: string; low: number; high: number; slot: number } }
  | { RemoveShaft: { id: number } }
  | {
      SetShaftProgram: {
        id: number;
        daypart: number;
        served: boolean[];
        priority: ShaftPriority;
      };
    }
  | { SetStriding: { walking: boolean } };

/** Rust's `CommandResult`: `"Ok"` or `{ Error: … }`. */
export type CommandResult = "Ok" | { Error: unknown };

export interface ReplayReport {
  ok: boolean;
  checked: number;
  final_tick: number;
  divergence: { tick: number; expected: string; actual: string } | null;
  message: string;
}

export type SoundEvent =
  | "Harvest"
  | "Craft"
  | "Pickup"
  | "Deliver"
  | "CarStop"
  | "Burn"
  | "BandChange";
