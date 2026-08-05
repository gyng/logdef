/**
 * TypeScript mirrors of the Rust snapshot types.
 *
 * These are a contract, not a convenience: every type here corresponds
 * to a struct in `crates/core/src/snapshot.rs`, and the two change in
 * the same commit or the renderer draws garbage. Field names match the
 * Rust field names exactly — serde emits them verbatim.
 */

export type SimSpeed = "Paused" | "X1" | "X2" | "X4";

export type RoomCategory =
  | "Intake"
  | "Production"
  | "Storage"
  | "Energy"
  | "Defence"
  /** Somewhere to sleep. No recipe, no stock, no reach. */
  | "Quarters"
  | "Heart";

export type CrewStateTag =
  | "idle"
  | "walk"
  | "board"
  | "climb"
  | "ride"
  | "mend"
  | "load"
  | "unload"
  /** Sat down to a meal. */
  | "eat"
  /** Standing in a room, working it. */
  | "man"
  /**
   * Standing in a room something is taking from, until it leaves.
   * Nobody fights — being there is the whole of it.
   */
  | "shoo"
  /** Off shift — in a hammock if a bed was free, on the deck if not. */
  | "sleep";

/**
 * Which half of the rota a crew member works. The player sets it.
 *
 * Capitalised, unlike the lowercase tags around it, because the same
 * value crosses the bridge in both directions — `SetShift` carries it
 * back — and one fact should not have two spellings depending on which
 * way it is going.
 */
export type ShiftTag = "Day" | "Night";

/**
 * `dying` was shot down; `leaving` lost its grip on a walking tower.
 * They fade out the same way — the difference is that only one of them
 * counts as having been seen off.
 */
export type EnemyStateTag = "approach" | "attack" | "dying" | "leaving";

export type EnemyApproach = "Ground" | "Canopy" | "Burrow";

export type ShaftKind =
  | "Stairs"
  | "Dumbwaiter"
  | "Elevator"
  /**
   * One way, down, and out. No cars, no capacity, no charge and no
   * riders — things fall. Whatever goes in leaves the tower, which is
   * the escape hatch for the shelf-typing deadlock `BALANCE.md`'s
   * `storeroom` row has described since M2.
   */
  | "Chute";

export type ShaftPriority = "Balanced" | "FreightFirst" | "CrewFirst";

export type CarDirTag = "idle" | "up" | "down";

export type CarStateTag = "idle" | "moving" | "dwelling";

/**
 * Why the tower is standing still.
 *
 * Four situations that share one silhouette — same tower, same still
 * legs — and mean completely different things, which is why the
 * simulation names the reason rather than leaving the renderer to infer
 * it. `fork` in particular has to read as *waiting for you* rather than
 * as a frozen game (`SYSTEMS.md` §3.3).
 */
export type HaltView =
  | "walking"
  | "stopped"
  /**
   * Stopped at a ruin with a rig that can reach it — **the one place a
   * wave cannot be walked away from.** Everything else on this list is
   * a tower waiting; this one is a tower committed.
   */
  | "berthed"
  | "brownout"
  | "fork"
  | "arrived";

// ---------------------------------------------------------------------------
// Per-frame view
// ---------------------------------------------------------------------------

export interface ViewSnapshot {
  tick: number;
  /**
   * The seed this run started from, as a decimal string.
   *
   * A string because it is 64 bits and JavaScript numbers are not: a
   * seed that does not round-trip is a seed that cannot be shared.
   */
  seed: string;
  speed: SimSpeed;
  /** Fraction of a tick elapsed. For render interpolation. */
  alpha: number;
  clock: ClockView;
  power: PowerView;
  world: WorldView;
  tower: TowerView;
  siege: SiegeView;
  journey: JourneyView;
  crew: CrewView[];
  stock: StoreView[];
  /**
   * Room indices the tower may build right now, in catalog order.
   *
   * The opening ladder (`SYSTEMS.md` §6.11). Sent rather than derived
   * here, so the menu and the command layer can never disagree about
   * what is buildable — a card that offers something the simulation
   * refuses is worse than no card.
   */
  unlocked: number[];
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
  /**
   * Sunlight after terrain.
   *
   * Since M6 cut the sails nothing is paid in charge for it: it sets
   * the garden's rate, decides whether the lamps come on, and lights
   * the cross-section.
   */
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
  /**
   * What the player ranked to keep running when charge is short, best
   * first. Always all four.
   */
  priority: PowerUse[];
  /** What each use wanted this tick, in `POWER_USES` order. */
  demand: number[];
}

/**
 * The four things that spend charge.
 *
 * Charge priority used to *be* the tick order — lifts first because
 * transport runs first, legs last because striding runs last — so it was
 * a constant rather than a decision. This is that ranking, handed over.
 */
export type PowerUse = "Lifts" | "Works" | "Lamps" | "Legs";

/** In the order the tick spends, which is also the default ranking. */
export const POWER_USES: PowerUse[] = ["Lifts", "Works", "Lamps", "Legs"];

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
  /**
   * Scrap still in this ruin, and zero for anything that is not one.
   * Whether a feature is a ruin at all is `catalog.terrain[band]
   * .ruin_kinds[kind]` — a stripped ruin is still a ruin, and has to
   * draw like one that has nothing left rather than like scenery.
   */
  salvage: number;
}

/** Where the run has got to, and what it is being asked. */
export interface JourneyView {
  /** Indexes `catalog.regions`. */
  region: number;
  /** How far through the current region, in per-mille. */
  region_permille: number;
  /** Paces still to walk before the far edge of the journey. */
  remaining: number;
  /** The split ahead, if the route has one the tower has not crossed. */
  fork: ForkView | null;
  /** The beat alongside right now, if any. */
  waypoint: WaypointView | null;
  /** Paces to the next one still ahead. */
  waypoint_ahead: number | null;
  /** The branch being walked through, if any. Indexes `catalog.branches`. */
  branch: number | null;
  /** Why the tower is standing still, if it is. */
  halt: HaltView;
  /**
   * Paces to the settlement, once it is somewhere ahead. Null once the
   * tower has passed it — there is no going back down the axis, so a
   * settlement behind you is gone rather than distant.
   */
  enclave_ahead: number | null;
  /** Berthed at the enclave right now. */
  at_enclave: boolean;
  /** What the enclave has left, one entry per authored offer. */
  offers: number[];
  recruits: number;
  /** How many more times the settlement will plate the shell. */
  shell_work: number;
  /** Hit points already added to every panel by shell work. */
  shell_bonus: number;
  /** The far edge of the last region, reached. The run is over. */
  arrived: boolean;
}

export interface WaypointView {
  /** Indexes `catalog.waypoints`. */
  def: number;
  /** Whether the shelves can pay for it. */
  affordable: boolean;
}

export interface ForkView {
  /** Paces from the tower to the split. */
  ahead: number;
  /** The two archetypes on offer. Index `catalog.branches`. */
  branches: [number, number];
  /** Which one the player has picked, if they have. */
  answer: number | null;
}

export interface SiegeView {
  enemies: EnemyView[];
  /** How much attention the tower has drawn, 0 to `provocation_max`. */
  provocation: number;
  provocation_max: number;
  /** Panels, rooms and shafts averaged by hit points, in per-mille. */
  integrity_permille: number;
  /** Creatures seen off. Reported, never celebrated. */
  repelled: number;
  /** The Heartseed is gone. The run is over. */
  lost: boolean;
  /** Poles it would take to put everything right. */
  repair_cost: number;
  /**
   * The creature every emplacement has been asked to prefer.
   *
   * Drawn as a mark on that creature and never as a target reticle —
   * `DECISIONS.md` §8 keeps creatures as animals defending their
   * territory rather than a gallery to clear.
   */
  focus: number | null;
}

export interface EnemyView {
  id: number;
  /** Indexes `catalog.enemies`. */
  def: number;
  /** Whole paces, on the same axis as `world.distance`. */
  at: number;
  hp_permille: number;
  state: EnemyStateTag;
}

export interface TowerView {
  floors: FloorView[];
  shafts: ShaftView[];
}

export interface FloorView {
  index: number;
  slots: number;
  rooms: RoomView[];
  /** The outer wall, in per-mille. Zero is a hole in the tower's skin. */
  panel_permille: number;
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
  /**
   * A `top_floor_only` room with a floor above it — since M6 that
   * means a garden in the dark. The price of building higher.
   */
  shaded: boolean;
  health_permille: number;
  /** Damaged past the point of working at all. */
  wrecked: boolean;
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
  health_permille: number;
  /** Cut through. Nothing travels on it until it is repaired. */
  severed: boolean;
  /**
   * The schedule, one entry per daypart in pack order.
   *
   * A schedule you cannot see is one you cannot edit, which is most of
   * why the per-daypart programs went three milestones without a UI
   * despite existing in the data model, the command layer and the
   * replay format the whole time.
   */
  programs: ProgramView[];
}

/** One daypart's worth of a shaft's schedule. */
export interface ProgramView {
  /** Indexed by floor. A floor the car will not stop at. */
  served: boolean[];
  priority: ShaftPriority;
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
  /**
   * Cosmetic-stream draw, and the only source of per-person variety the
   * renderer gets. Animation phase offset, which face is theirs
   * (`fidget % faces`), and which of several equivalent bark lines —
   * all derived from this one number, so a bark can never perturb the
   * simulation because there is nothing to perturb.
   */
  fidget: number;
  /** Ticks since their last meal. Hover-only; never a bar over a head. */
  hunger: number;
  /** Ticks of work left in them. Hover-only, for the same reason. */
  rested: number;
  /**
   * The room this person has been posted to, if any.
   *
   * A standing order, so it survives them going to eat and to bed. The
   * roster reads it to say where somebody belongs, not merely where they
   * are this second.
   */
  stationed: number | null;
  /**
   * Is that posting a *push* — one that ends when they tire?
   *
   * Drawn differently from a standing posting, because the two read as
   * the same thing on a cross-section and are not.
   */
  post_until_tired: boolean;
  /**
   * A kit this person is carrying, if the tower has lent them one.
   *
   * Held rather than consumed: it is off the shelves while they have it
   * and back on them when they hand it in.
   */
  kit: number | null;
  shift: ShiftTag;
  /** Actually asleep, as against merely off shift and walking to bed. */
  asleep: boolean;
}

export interface StockView {
  item: number;
  count: number;
}

/**
 * One item on the tower's shelves, and how much shelf it has.
 *
 * Separate from `StockView` — which also carries a dumbwaiter's freight
 * and what a crew member is holding, neither of which has a capacity.
 *
 * `space` is the shelf capacity currently committed to this item, so
 * `count === space` is exactly the condition that stalls the chain
 * feeding it. Until now that fact was legible only as a row of full
 * pips inside the cross-section.
 */
export interface StoreView {
  item: number;
  count: number;
  /** Never zero: an item is listed only because a shelf holds it. */
  space: number;
}

export interface RunStats {
  hauls_completed: number;
  crafts_completed: number;
  items_harvested: number;
  hp_repaired: number;
  /** Poles spent putting the tower back together. */
  repair_poles_spent: number;
  /** Meals eaten. The kitchen chain's own throughput figure. */
  meals_eaten: number;
  /** Loads taken out of an outbox by a thief — work you did not keep. */
  items_stolen: number;
  /** Crew-ticks spent asleep. What the rota actually costs. */
  crew_ticks_asleep: number;
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
  enemies: EnemyInfo[];
  regions: RegionInfo[];
  waypoints: WaypointInfo[];
  branches: BranchInfo[];
  floor_cost: CostInfo[];
  /** What one widening costs. Dearer than a floor. */
  widen_cost: CostInfo[];
  /** Slots one widening adds. */
  widen_slots: number;
  /** How wide the hull may get. */
  max_slots: number;
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

export interface EnemyInfo {
  id: string;
  name: string;
  glyph: string;
  approach: EnemyApproach;
  night_only: boolean;
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
  /** Whether a person can carry this. The roster offers these. */
  kit: boolean;
}

export interface RoomInfo {
  id: string;
  name: string;
  short: string;
  category: RoomCategory;
  width: number;
  build_cost: CostInfo[];
  max_floor: number | null;
  /** Lowest floor it may go on. The mirror of `max_floor`. */
  min_floor: number | null;
  unique: boolean;
  /** Catalog index of the room that must be standing first, if any. */
  unlocked_by: number | null;
  /** Crew who must be posted here for it to work at all. */
  crew_required: number;
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
  /** Burns an item for charge, and can be switched off. */
  burner: boolean;
  /** Charge capacity this room adds. */
  bank_capacity: number;
  /** Shoots back, and eats ammo off the same shelves as everything else. */
  defence: boolean;
  /**
   * Beds. Zero for everything that is not quarters. Drawn one hammock
   * apiece, which is what makes occupancy diegetic — you can see who is
   * asleep and whether a bed is spare, without a number.
   */
  sleepers: number;
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
  /**
   * Which of those kinds are ruins — a place the tower can berth at and
   * work, rather than scenery. Parallel to `feature_kinds`.
   */
  ruin_kinds: boolean[];
}

export interface EnclaveInfo {
  name: string;
  /** How far into its region it stands. */
  at_paces: number;
  offers: OfferInfo[];
  recruits: number;
  recruit_cost: CostInfo[];
  /** What one round of shell work costs and adds, if they do it. */
  reinforce: ReinforceInfo | null;
}

export interface ReinforceInfo {
  cost: CostInfo[];
  /** Added to every panel, present and future. */
  panel_hp: number;
}

export interface OfferInfo {
  give: CostInfo;
  take: CostInfo;
  /** How many times it could ever be taken. What is *left* is in
   * `journey.offers`, which is state rather than content. */
  stock: number;
}

/**
 * One beat the route can put in front of the tower.
 *
 * The prose is here rather than in the view because it is a fact about
 * the pack — the view only says which one is alongside.
 */
export interface WaypointInfo {
  id: string;
  name: string;
  said: string;
  take: string;
  costs: CostInfo[];
  gives: CostInfo[];
  /** Attention taking it draws. Negative sheds it. */
  provocation: number;
  /** Ground gained, or lost if negative. */
  paces: number;
}

export interface RegionInfo {
  id: string;
  name: string;
  /** The name of the settlement in this region, if it has one. */
  enclave: EnclaveInfo | null;
}

/**
 * One side of a fork.
 *
 * The two heaviest terrain kinds and a word for the threat are what the
 * fork card shows, and both are *derived* from the branch's own data
 * rather than authored alongside it — a hand-written line describing a
 * branch drifts out of step with its palette during tuning, and a game
 * that misdescribes the only informed choice it asks the player to make
 * is worse than one that describes it drily.
 */
export interface BranchInfo {
  id: string;
  name: string;
  /** Heaviest first. Indexes `catalog.terrain`. */
  terrain: number[];
  /** Against the region it interrupts: 100 is as usual. */
  threat_pct: number;
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
  | { SetStriding: { walking: boolean } }
  /** Take one of the enclave's posted offers, once. */
  | { Trade: { offer: number } }
  /** Take somebody aboard, for poles. */
  | "Recruit"
  | "TakeWaypoint"
  | "WidenTower"
  /** Have the settlement plate the tower's shell, for scrap. */
  | "Reinforce"
  /** Commit to branch 0 or 1 of the pending fork. Re-answerable. */
  | { TakeFork: { branch: number } }
  /**
   * Put one crew member on the day or the night shift. One person per
   * command, so a rejection names who it is about and the replay reads
   * as a list of decisions about people.
   */
  | { SetShift: { crew: number; shift: ShiftTag } }
  /**
   * Rank what keeps running when the bank runs short, best first.
   *
   * Sent whole rather than as a swap: the simulation rejects anything
   * that is not all four uses exactly once, because a partial order
   * would leave the rest ranked by an accident of list position.
   */
  | { SetPowerPriority: { order: PowerUse[] } }
  /** Ask every emplacement to prefer one creature. `null` clears it. */
  | { FocusEnemy: { enemy: number | null } }
  /**
   * Post somebody to a room, or call them back. `null` returns them to
   * hauling. A standing order about somebody's working day, the same
   * category as the shift rota.
   */
  | {
      StationCrew: {
        crew: number;
        room: number | null;
        /**
         * End the posting when they run out of energy — a *push*
         * rather than a job. Optional, and absent means a standing
         * posting, which is what M6's stationing has always been.
         */
        until_tired?: boolean;
      };
    }
  /**
   * Lend somebody a kit off the shelves, or take it back. `null` hands
   * in whatever they are carrying.
   */
  | { EquipCrew: { crew: number; kit: string | null } };

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
  | "BandChange"
  | "RegionChange"
  | "Arrived"
  | "WaveArrives"
  | "EnemyContact"
  | "Impact"
  | "Breach"
  | "Wrecked"
  | "Severed"
  | "Shot"
  | "EnemyDown"
  | "Repair"
  | "HeartseedLost"
  /** Somebody sat down to a meal. The warmest moment in the tower. */
  | "MealServed"
  /** The rota turned over — the only reliable way to *hear* the time. */
  | "ShiftChange"
  /**
   * A load went down a chute and out of the tower. Deliberately not
   * `Deliver`: something the chain worked for has just been thrown
   * away, and a tower that is spilling is telling you about itself.
   */
  | "Spill"
  /**
   * Something took a load out of an outbox and left with it. Not
   * `Impact`: nothing was hit and nothing needs mending, and it should
   * sound like a theft rather than like a blow.
   */
  | "Steal"
  /**
   * Something lost its grip and walked away. Distinct from `EnemyDown`
   * on purpose: only one of the two counts as having been seen off, and
   * the audio has to refuse to conflate them too. Never triumphant.
   */
  | "EnemyLeaves";
