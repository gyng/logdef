/**
 * TypeScript mirrors of the Rust snapshot types.
 *
 * These are a contract, not a convenience: every type here corresponds
 * to a struct in `crates/core/src/snapshot.rs`, and the two change in
 * the same commit or the renderer draws garbage. Field names match the
 * Rust field names exactly — serde emits them verbatim.
 */

export type SimSpeed = "Paused" | "X1" | "X2" | "X4";

export type RoomCategory = "Intake" | "Production" | "Storage" | "Heart";

export type CrewStateTag = "idle" | "walk" | "board" | "climb" | "load" | "unload";

// ---------------------------------------------------------------------------
// Per-frame view
// ---------------------------------------------------------------------------

export interface ViewSnapshot {
  tick: number;
  speed: SimSpeed;
  /** Fraction of a tick elapsed. For render interpolation. */
  alpha: number;
  world: WorldView;
  tower: TowerView;
  crew: CrewView[];
  stock: StockView[];
  stats: RunStats;
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
  /** Starved or backed up. Drawn quiet rather than flagged. */
  stalled: boolean;
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
  kind: string;
  low: number;
  high: number;
  slot: number;
  capacity: number;
  riders: number;
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
  terrain: TerrainInfo[];
  floor_cost: CostInfo[];
  max_floors: number;
  floor_slots: number;
  stress_ticks: number;
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
  | { RemoveRoom: { floor: number; slot: number } };

/** Rust's `CommandResult`: `"Ok"` or `{ Error: … }`. */
export type CommandResult = "Ok" | { Error: unknown };

export interface ReplayReport {
  ok: boolean;
  checked: number;
  final_tick: number;
  divergence: { tick: number; expected: string; actual: string } | null;
  message: string;
}

export type SoundEvent = "Harvest" | "Craft" | "Pickup" | "Deliver" | "BandChange";
