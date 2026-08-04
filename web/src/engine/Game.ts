/**
 * The frame driver — the "custom engine" half of the frontend.
 *
 * React never sees a frame. This class owns the animation loop, calls
 * into wasm, hands the snapshot to the renderer, and publishes a small
 * digest to the UI a few times a second. React re-renders the chrome
 * (build menu, readouts) at that rate; the game view runs at display
 * rate and never touches the reconciler.
 *
 * That split is why there is no state library here and does not need to
 * be one: the authoritative state lives in Rust, the render loop reads
 * it directly, and React only holds what a human is looking at.
 */

import { AudioManager } from "./AudioManager";
import { GATED, type Journal, loadJournal, newlyLearned, recordRun, unlocked } from "./journal";
import { Renderer } from "./Renderer";
import type { PlaceMode } from "./scene";
import { slotRangeFree, towerShape } from "./scene";
import type { Bridge } from "../bridge";
import { commandFailed } from "../bridge";
import type {
  CatalogSnapshot,
  CrewView,
  EnclaveInfo,
  FeatureView,
  ForkView,
  GameCommand,
  HaltView,
  RoomInfo,
  ShaftInfo,
  ShaftPriority,
  ShaftView,
  ShiftTag,
  SimSpeed,
  StoreView,
  ViewSnapshot,
} from "../bridge/types";

/** What the React chrome needs. Deliberately small. */
export interface UiState {
  tick: number;
  speed: SimSpeed;
  distance: number;
  terrain: string;
  yieldPct: number;
  floors: number;
  stock: StoreView[];
  harvested: number;
  crafted: number;
  hauled: number;
  /** Crew currently blocked at a shaft — the bottleneck at a glance. */
  waiting: number;
  /**
   * Everybody aboard, verbatim from the snapshot.
   *
   * The roster is the one panel M4 adds, and it is defensible under
   * `DECISIONS.md` §8 because a rota is a *schedule the player writes*
   * rather than a readout of state — the same category as the
   * elevator's per-daypart programs. What is not defensible is the
   * roster becoming the primary place hunger and tiredness are read; if
   * the tower can only be understood through this list, the art pass
   * failed.
   */
  /**
   * Room and shaft ids this player has *not* unlocked yet.
   *
   * The build menu greys these rather than hiding them, so a newcomer
   * can see the shape of what the game becomes without being able to
   * reach for it — and so an unlock is a thing that opens rather than a
   * thing that appears from nowhere.
   */
  /** This run's seed, so it can be read off and handed over. */
  seed: string;
  locked: string[];
  /** Deeds this run has done that this player had never done before. */
  learned: { said: string; opens: string[] }[];

  crew: CrewView[];
  /** Every shaft, so the roster can carry their schedules. */
  shafts: ShaftView[];
  selected: SelectedRoom | null;
  /** Whether the selected room is switched on. */
  selectedActive: boolean;
  placing: string | null;
  day: number;
  daypart: string;
  /** Which daypart, as an index into the catalog. */
  daypartIndex: number;
  sunPct: number;
  /** Sun after terrain — what the sails actually get. */
  exposurePct: number;
  charge: number;
  chargeCapacity: number;
  /** Stored fraction in per-mille. */
  chargeFill: number;
  chargeIncome: number;
  chargeSpend: number;
  brownout: boolean;
  walking: boolean;
  /** How much attention the tower has drawn, against its ceiling. */
  provocation: number;
  provocationMax: number;
  /** Panels, rooms and shafts averaged by hit points, in per-mille. */
  integrity: number;
  /** Poles it would take to put everything right. */
  repairCost: number;
  /** Creatures seen off. Reported, never celebrated. */
  repelled: number;
  /** The Heartseed is gone. The run is over. */
  lost: boolean;
  /** Where the run has got to. */
  region: string;
  /** How far through that region, in per-mille. */
  regionPermille: number;
  /** The branch archetype being walked through, if any. */
  branch: string | null;
  /** Why the tower is standing still, if it is. */
  halt: HaltView;
  /** The split ahead, if there is one the tower has not crossed. */
  fork: ForkView | null;
  /** Berthed at the enclave. */
  atEnclave: boolean;
  /** What the settlement in this region is called, if it has one. */
  enclave: EnclaveInfo | null;
  /** Remaining stock, one entry per posted offer. */
  enclaveAhead: number | null;
  offers: number[];
  recruits: number;
  shellWork: number;
  shellBonus: number;
  /** The far edge of the journey, reached. */
  arrived: boolean;
  fps: number;
  quads: number;
  lastError: string | null;
}

export interface SelectedRoom {
  floor: number;
  slot: number;
  id: number;
  info: RoomInfo;
  removable: boolean;
}

/** How often the React chrome is refreshed, in milliseconds. */
const UI_INTERVAL_MS = 100;

export class Game {
  private readonly bridge: Bridge;
  private readonly renderer: Renderer;
  private readonly catalog: CatalogSnapshot;

  /** The content pack, for chrome that has to name things. */
  catalogInfo(): CatalogSnapshot {
    return this.catalog;
  }

  private readonly listeners = new Set<(ui: UiState) => void>();

  private frameHandle = 0;
  private lastFrameMs = 0;
  private lastUiMs = 0;
  private startedMs = 0;
  private fps = 0;

  private readonly audio = new AudioManager();

  /**
   * What this player has unlocked, and what this run has built.
   *
   * **Player-level, never run-level.** None of this enters `GameState`
   * or the replay: an unlock changes which commands the player may
   * send, and a replay carries the commands, so a veteran's recording
   * replays perfectly for somebody who has unlocked nothing
   * (`SYSTEMS.md` §5.7).
   */
  private journal: Journal = loadJournal();
  private readonly built: string[] = [];
  private readonly routeSeen: string[] = [];
  private recorded = false;
  private learned: { said: string; opens: string[] }[] = [];

  private placeMode: PlaceMode | null = null;
  private selected: SelectedRoom | null = null;
  private lastError: string | null = null;
  private latest: ViewSnapshot | null = null;

  constructor(canvas: HTMLCanvasElement, labelRoot: HTMLElement, bridge: Bridge) {
    this.bridge = bridge;
    this.catalog = bridge.catalog();
    this.renderer = new Renderer(canvas, labelRoot);
    this.exposeTestHooks();
  }

  /**
   * Extend the bridge's test-hook namespace with the one thing only the
   * renderer knows: where a given slot is on screen. Lets the smoke
   * test click where a player would click instead of at a pixel guessed
   * from the layout constants.
   */
  private exposeTestHooks(): void {
    if (typeof window === "undefined") return;
    const hooks = (window as unknown as Record<string, unknown>).__understory;
    if (hooks && typeof hooks === "object") {
      (hooks as Record<string, unknown>).slotPoint = (floor: number, slot: number) =>
        this.renderer.slotCenter(floor, slot);
      // The same thing for the terrain strip: the screenshot harness
      // has to be able to frame a ruin, and only the renderer knows
      // where a given parallax layer put it. The distance is passed in
      // rather than read off the last snapshot, because the harness
      // asks from inside a stepping loop that no frame has rendered
      // during — `this.latest` there is thousands of paces stale.
      (hooks as Record<string, unknown>).featurePoint = (feature: FeatureView, distance: number) =>
        this.renderer.featureCenter(distance, feature);
    }
  }

  start(): void {
    if (this.frameHandle !== 0) return;
    this.startedMs = performance.now();
    this.lastFrameMs = this.startedMs;
    this.lastUiMs = 0;
    this.frameHandle = requestAnimationFrame(this.tick);
  }

  stop(): void {
    if (this.frameHandle !== 0) cancelAnimationFrame(this.frameHandle);
    this.frameHandle = 0;
  }

  dispose(): void {
    this.stop();
    this.renderer.dispose();
    this.listeners.clear();
  }

  subscribe(listener: (ui: UiState) => void): () => void {
    this.listeners.add(listener);
    if (this.latest) listener(this.buildUiState());
    return () => this.listeners.delete(listener);
  }

  getCatalog(): CatalogSnapshot {
    return this.catalog;
  }

  // -------------------------------------------------------------------
  // Player input
  // -------------------------------------------------------------------

  /**
   * Put one crew member on a shift.
   *
   * One person per command rather than a bulk setter, matching the
   * simulation: a rejection then names the crew member it is about, and
   * the replay reads as a list of decisions about people.
   */
  setShift(crew: number, shift: ShiftTag): void {
    this.send({ SetShift: { crew, shift } });
  }

  /**
   * Let the sound start.
   *
   * Browser autoplay policy suspends an `AudioContext` until the player
   * clicks, so this is called from the first interaction rather than
   * from the constructor. The opening frames being silent is a rule of
   * the platform, not a bug, and the smoke test never hears anything
   * for the same reason.
   */
  startAudio(): void {
    this.audio.start();
  }

  setAudioEnabled(on: boolean): void {
    this.audio.setEnabled(on);
    if (on) this.audio.start();
  }

  audioEnabled(): boolean {
    return this.audio.isEnabled();
  }

  /**
   * Rewrite one shaft's schedule for one daypart.
   *
   * The whole program is sent rather than a delta, because the command
   * is the record: a replay that says "floor 2 stopped being served at
   * dusk" is harder to read back than one that says what the schedule
   * *became*.
   */
  setShaftProgram(id: number, daypart: number, served: boolean[], priority: ShaftPriority): void {
    this.send({ SetShaftProgram: { id, daypart, served, priority } });
  }

  /**
   * Follow the run, and write it down once when it ends.
   *
   * Once: `recorded` is the guard, because the frame loop keeps running
   * after a run is over — the tower still stands there and the renderer
   * still draws it — and a journal that gained an entry every frame
   * after the Heartseed went would be a journal of one bad afternoon.
   */
  private noteRun(view: ViewSnapshot): void {
    const region = this.catalog.regions[view.journey.region]?.name;
    if (region && this.routeSeen.at(-1) !== region) this.routeSeen.push(region);

    if (this.recorded || (!view.journey.arrived && !view.siege.lost)) return;
    this.recorded = true;
    const before = this.journal;
    this.learned = newlyLearned(
      before,
      // Deeds are derived inside `recordRun` from the same snapshot, so
      // asking twice would be asking the same question twice; this is
      // the *new* ones, which is what the arrival has to say out loud.
      recordRun(view, {
        // From the snapshot rather than held here: the bridge already
        // knows it, and a second copy is a second thing to keep in step.
        seed: view.seed,
        route: [...this.routeSeen],
        built: [...this.built],
        crew: view.crew.map((member) => member.name),
      }).done,
    ).map((deed) => ({ said: deed.said, opens: deed.opens }));
    this.journal = loadJournal();
  }

  /** The whole journal, for the arrival and the elegy to read. */
  journalNow(): Journal {
    return this.journal;
  }

  /** Ids this player cannot build yet. */
  lockedIds(): Set<string> {
    const open = unlocked(this.journal);
    const gated = new Set<string>();
    for (const info of this.catalog.rooms) {
      if (!open.has(info.id) && GATED.has(info.id)) gated.add(info.id);
    }
    for (const info of this.catalog.shafts) {
      if (!open.has(info.id) && GATED.has(info.id)) gated.add(info.id);
    }
    return gated;
  }

  send(cmd: GameCommand): void {
    // **The unlock gate lives here and nowhere else.** Rust validates
    // legality — enough stock, a free slot, a real floor — and knows
    // nothing about who is playing. Whether this *player* has earned
    // the right to ask is a question about the player, so it is
    // answered on this side and never crosses the bridge.
    const wants =
      typeof cmd === "object" && "PlaceRoom" in cmd
        ? cmd.PlaceRoom.room
        : typeof cmd === "object" && "BuildShaft" in cmd
          ? cmd.BuildShaft.shaft
          : null;
    if (wants !== null && this.lockedIds().has(wants)) {
      this.lastError = "not yet — the journal has not learnt that one";
      return;
    }
    const result = this.bridge.send(cmd);
    // What this run built, for the log and for the deeds. Recorded from
    // accepted commands rather than by scanning the tower, because a
    // room built and later demolished still happened.
    if (!commandFailed(result) && wants !== null) this.built.push(wants);
    this.lastError = commandFailed(result) ? describeError(result.Error) : null;
    this.publish(true);
  }

  setSpeed(speed: SimSpeed): void {
    this.send({ SetSpeed: { speed } });
  }

  /** Enter placement for a room, or leave it when passed null. */
  beginPlacing(roomId: string | null): void {
    if (roomId === null) {
      this.placeMode = null;
      this.publish(true);
      return;
    }
    const info = this.catalog.rooms.find((room) => room.id === roomId);
    if (!info) return;
    this.placeMode = {
      kind: "room",
      id: roomId,
      width: info.width,
      maxFloor: info.max_floor,
      minFloor: info.min_floor,
      span: 1,
      hover: null,
    };
    this.selected = null;
    this.lastError = null;
    this.publish(true);
  }

  /**
   * Enter placement for a shaft. The span defaults to as tall as the
   * definition and the tower both allow — a player who wants a short
   * elevator can build a short one, but the common case is "all the way
   * up", and making that the default saves a fiddly control.
   */
  beginPlacingShaft(shaftId: string | null): void {
    if (shaftId === null) {
      this.placeMode = null;
      this.publish(true);
      return;
    }
    const info = this.catalog.shafts.find((shaft) => shaft.id === shaftId);
    if (!info) return;
    const floors = this.latest?.tower.floors.length ?? 1;
    const ceiling = info.max_span === 0 ? floors : info.max_span;
    this.placeMode = {
      kind: "shaft",
      id: shaftId,
      width: 1,
      maxFloor: null,
      minFloor: null,
      span: Math.max(info.min_span, Math.min(ceiling, floors)),
      hover: null,
    };
    this.selected = null;
    this.lastError = null;
    this.publish(true);
  }

  setStriding(walking: boolean): void {
    this.send({ SetStriding: { walking } });
  }

  /**
   * Commit to one way at the pending fork.
   *
   * Legal from the moment the fork appears and re-sendable until the
   * tower crosses, so this is not a one-shot: the button stays live and
   * the last answer is the one that counts (`SYSTEMS.md` §3.3).
   */
  takeFork(branch: number): void {
    this.send({ TakeFork: { branch } });
  }

  /** Take one of the enclave's posted offers. */
  trade(offer: number): void {
    this.send({ Trade: { offer } });
  }

  /** Take somebody aboard, for poles. */
  recruit(): void {
    this.send("Recruit");
  }

  /** Have the settlement plate the tower's shell, for scrap. */
  reinforce(): void {
    this.send("Reinforce");
  }

  /** Switch the selected room off or on. */
  toggleSelectedRoom(): void {
    const selected = this.selected;
    if (!selected) return;
    const room = this.latest?.tower.floors[selected.floor]?.rooms.find(
      (candidate) => candidate.id === selected.id,
    );
    if (!room) return;
    this.send({
      SetRoomActive: { floor: selected.floor, slot: selected.slot, active: !room.active },
    });
  }

  handlePointerMove(clientX: number, clientY: number): void {
    if (!this.placeMode) return;
    this.placeMode.hover = this.renderer.pick(clientX, clientY);
  }

  handlePointerLeave(): void {
    if (this.placeMode) this.placeMode.hover = null;
  }

  handleClick(clientX: number, clientY: number): void {
    const hit = this.renderer.pick(clientX, clientY);
    if (!hit) {
      this.selected = null;
      this.publish(true);
      return;
    }

    // Stay in place mode either way: on success so a player can lay
    // down a row of rooms without re-picking from the menu, and on
    // failure so the rejection message lands next to another attempt.
    const placing = this.placeMode;
    if (placing) {
      if (placing.kind === "room") {
        this.send({ PlaceRoom: { room: placing.id, floor: hit.floor, slot: hit.slot } });
      } else {
        this.send({
          BuildShaft: {
            shaft: placing.id,
            low: hit.floor,
            high: hit.floor + placing.span - 1,
            slot: hit.slot,
          },
        });
      }
      return;
    }

    this.selected = this.findRoom(hit.floor, hit.slot);
    this.publish(true);
  }

  removeSelected(): void {
    if (!this.selected) return;
    const { floor, slot } = this.selected;
    this.send({ RemoveRoom: { floor, slot } });
    if (this.lastError === null) this.selected = null;
    this.publish(true);
  }

  /** Can the player afford this room right now? Drives the build menu. */
  canAfford(info: RoomInfo): boolean {
    return info.build_cost.every((cost) => this.stockOf(cost.item) >= cost.amount);
  }

  canAffordFloor(): boolean {
    return this.catalog.floor_cost.every((cost) => this.stockOf(cost.item) >= cost.amount);
  }

  /** Is there anywhere at all this room could go? */
  hasRoomFor(info: RoomInfo): boolean {
    const view = this.latest;
    if (!view) return false;
    return view.tower.floors.some((floor) => {
      if (info.max_floor !== null && floor.index > info.max_floor) return false;
      if (info.min_floor !== null && floor.index < info.min_floor) return false;
      for (let slot = 0; slot + info.width <= floor.slots; slot += 1) {
        if (slotRangeFree(view, floor.index, slot, info.width)) return true;
      }
      return false;
    });
  }

  // -------------------------------------------------------------------
  // Loop
  // -------------------------------------------------------------------

  private readonly tick = (now: number): void => {
    this.frameHandle = requestAnimationFrame(this.tick);

    const deltaMs = Math.min(now - this.lastFrameMs, 250);
    this.lastFrameMs = now;
    // Smoothed, so the readout doesn't flicker on every hitch.
    if (deltaMs > 0) this.fps += (1000 / deltaMs - this.fps) * 0.1;

    // Sound is fire-and-forget: emitted during a tick, played or
    // dropped here, and never read back into the simulation. The list
    // is punctuation — things that *happened* — and the snapshot below
    // is what the continuous beds read, because a starved mill going
    // quiet is not an event at all, it is the absence of a loop.
    const sounds = this.bridge.frame(Math.round(deltaMs * 1000));

    const view = this.bridge.view();
    this.latest = view;
    this.audio.update(view, sounds, deltaMs);
    this.noteRun(view);

    // A demolished or newly built room can invalidate the selection.
    if (this.selected) {
      const current = this.findRoom(this.selected.floor, this.selected.slot);
      if (current?.id !== this.selected.id) this.selected = current;
    }

    this.renderer.render({
      view,
      catalog: this.catalog,
      placeMode: this.placeMode,
      clock: (now - this.startedMs) / 1000,
    });

    if (now - this.lastUiMs >= UI_INTERVAL_MS) {
      this.lastUiMs = now;
      this.publish(false);
    }
  };

  private publish(force: boolean): void {
    if (!this.latest && !force) return;
    const ui = this.buildUiState();
    for (const listener of this.listeners) listener(ui);
  }

  private buildUiState(): UiState {
    const view = this.latest;
    const terrainIndex = view?.world.band ?? null;
    return {
      tick: view?.tick ?? 0,
      speed: view?.speed ?? "Paused",
      distance: Math.floor(view?.world.distance ?? 0),
      terrain: terrainIndex === null ? "—" : (this.catalog.terrain[terrainIndex]?.name ?? "—"),
      yieldPct: view?.world.yield_pct ?? 100,
      floors: view?.tower.floors.length ?? 0,
      stock: view?.stock ?? [],
      harvested: view?.stats.items_harvested ?? 0,
      crafted: view?.stats.crafts_completed ?? 0,
      hauled: view?.stats.hauls_completed ?? 0,
      waiting: view?.crew.filter((member) => member.state === "board").length ?? 0,
      crew: view?.crew ?? [],
      seed: view?.seed ?? "",
      locked: [...this.lockedIds()],
      learned: this.learned,
      shafts: view?.tower.shafts ?? [],
      selected: this.selected,
      selectedActive: this.selectedRoomActive(),
      placing: this.placeMode?.id ?? null,
      day: view?.clock.day ?? 0,
      daypart: view === null ? "—" : (this.catalog.dayparts[view.clock.daypart]?.name ?? "—"),
      daypartIndex: view?.clock.daypart ?? 0,
      sunPct: view?.clock.sun_pct ?? 0,
      exposurePct: view?.clock.exposure_pct ?? 0,
      charge: view?.power.charge ?? 0,
      chargeCapacity: view?.power.capacity ?? 0,
      chargeFill: view?.power.fill_permille ?? 0,
      chargeIncome: view?.power.income_last ?? 0,
      chargeSpend: view?.power.spent_last ?? 0,
      brownout: view?.power.brownout ?? false,
      walking: view?.power.walking ?? true,
      provocation: view?.siege.provocation ?? 0,
      provocationMax: view?.siege.provocation_max ?? 0,
      integrity: view?.siege.integrity_permille ?? 1000,
      repairCost: view?.siege.repair_cost ?? 0,
      repelled: view?.siege.repelled ?? 0,
      lost: view?.siege.lost ?? false,
      region: view === null ? "—" : (this.catalog.regions[view.journey.region]?.name ?? "—"),
      regionPermille: view?.journey.region_permille ?? 0,
      branch: this.branchName(),
      halt: view?.journey.halt ?? "stopped",
      fork: view?.journey.fork ?? null,
      atEnclave: view?.journey.at_enclave ?? false,
      enclave: view === null ? null : (this.catalog.regions[view.journey.region]?.enclave ?? null),
      enclaveAhead: view?.journey.enclave_ahead ?? null,
      offers: view?.journey.offers ?? [],
      recruits: view?.journey.recruits ?? 0,
      shellWork: view?.journey.shell_work ?? 0,
      shellBonus: view?.journey.shell_bonus ?? 0,
      arrived: view?.journey.arrived ?? false,
      fps: Math.round(this.fps),
      quads: this.renderer.quadCount,
      lastError: this.lastError,
    };
  }

  /** Can the player afford this shaft, and is there anywhere to put it? */
  canAffordShaft(info: ShaftInfo): boolean {
    return info.build_cost.every((cost) => this.stockOf(cost.item) >= cost.amount);
  }

  /**
   * The branch archetype the tower is walking through, by name.
   *
   * The palette says the terrain has changed; only the name says which
   * of the two ways it was, and that is worth carrying because a player
   * comparing this stretch against the one they turned down has nothing
   * else to compare it by.
   */
  private branchName(): string | null {
    const branch = this.latest?.journey.branch;
    if (branch === null || branch === undefined) return null;
    return this.catalog.branches[branch]?.name ?? null;
  }

  private selectedRoomActive(): boolean {
    const selected = this.selected;
    if (!selected) return true;
    return (
      this.latest?.tower.floors[selected.floor]?.rooms.find((room) => room.id === selected.id)
        ?.active ?? true
    );
  }

  private stockOf(item: number): number {
    return this.latest?.stock.find((entry) => entry.item === item)?.count ?? 0;
  }

  private findRoom(floor: number, slot: number): SelectedRoom | null {
    const view = this.latest;
    const target = view?.tower.floors[floor];
    if (!target) return null;
    const room = target.rooms.find((r) => slot >= r.slot && slot < r.slot + r.width);
    if (!room) return null;
    const info = this.catalog.rooms[room.def];
    if (!info) return null;
    return {
      floor,
      slot: room.slot,
      id: room.id,
      info,
      removable: info.category !== "Heart",
    };
  }

  /** Tower dimensions, for anything outside that needs them. */
  shape(): { slots: number; floors: number } {
    return this.latest ? towerShape(this.latest) : { slots: 8, floors: 0 };
  }
}

/** Unit variants of `CommandError` cross as a bare string. */
const BARE_ERRORS: Record<string, string> = {
  NoForkPending: "The route does not split here",
  NotBerthedAtAnEnclave: "The tower is not stopped at the enclave",
  NobodyToRecruit: "Nobody else here wants to come",
};

/** Turn a `CommandError` into one line a player can act on. */
function describeError(error: unknown): string {
  if (typeof error === "string") return BARE_ERRORS[error] ?? humanise(error);
  if (error && typeof error === "object") {
    // A `CommandError` is externally tagged, so it has exactly one key.
    const entry = Object.entries(error as Record<string, unknown>)[0];
    if (!entry) return "Command refused";
    const [kind, payload] = entry;
    const detail = (payload ?? {}) as Record<string, unknown>;
    switch (kind) {
      case "InsufficientStock":
        return `Not enough ${String(detail.item).replace("item.", "")} — need ${String(
          detail.needed,
        )}, have ${String(detail.available)}`;
      case "SlotOccupied":
        return "Something is already there";
      case "SlotOutOfRange":
        return "That room does not fit on the floor";
      case "FloorTooHigh":
        return `That room only mounts up to floor ${String(detail.max_floor)}`;
      case "FloorTooLow":
        return `That room does not go below floor ${String(detail.min_floor)}`;
      case "AlreadyPlaced":
        return "There can only be one";
      case "FloorLimit":
        return `The legs will not carry more than ${String(detail.max_floors)} floors`;
      case "Undemolishable":
        return "That cannot be removed";
      case "NoRoomThere":
        return "Nothing there to remove";
      case "NoSuchBranch":
        return "The way does not go there";
      case "NoSuchOffer":
        return "Nothing like that is posted";
      case "OfferExhausted":
        return "That one is spoken for";
      case "CrewFull":
        return `There is no room aboard for more than ${String(detail.cap)}`;
      default:
        return humanise(kind);
    }
  }
  return "Command refused";
}

function humanise(kind: string): string {
  return kind.replace(/([a-z])([A-Z])/g, "$1 $2");
}
