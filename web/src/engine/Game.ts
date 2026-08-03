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

import { Renderer } from "./Renderer";
import type { PlaceMode } from "./scene";
import { slotRangeFree, towerShape } from "./scene";
import type { Bridge } from "../bridge";
import { commandFailed } from "../bridge";
import type {
  CatalogSnapshot,
  GameCommand,
  RoomInfo,
  ShaftInfo,
  SimSpeed,
  StockView,
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
  stock: StockView[];
  harvested: number;
  crafted: number;
  hauled: number;
  /** Crew currently blocked at a shaft — the bottleneck at a glance. */
  waiting: number;
  selected: SelectedRoom | null;
  /** Whether the selected room is switched on. */
  selectedActive: boolean;
  placing: string | null;
  day: number;
  daypart: string;
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
  private readonly listeners = new Set<(ui: UiState) => void>();

  private frameHandle = 0;
  private lastFrameMs = 0;
  private lastUiMs = 0;
  private startedMs = 0;
  private fps = 0;

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

  send(cmd: GameCommand): void {
    const result = this.bridge.send(cmd);
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

    // Sounds are produced and dropped until the audio pass in M4. They
    // are fire-and-forget by contract, so discarding them is correct
    // rather than a leak.
    this.bridge.frame(Math.round(deltaMs * 1000));

    const view = this.bridge.view();
    this.latest = view;

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
      selected: this.selected,
      selectedActive: this.selectedRoomActive(),
      placing: this.placeMode?.id ?? null,
      day: view?.clock.day ?? 0,
      daypart: view === null ? "—" : (this.catalog.dayparts[view.clock.daypart]?.name ?? "—"),
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
      fps: Math.round(this.fps),
      quads: this.renderer.quadCount,
      lastError: this.lastError,
    };
  }

  /** Can the player afford this shaft, and is there anywhere to put it? */
  canAffordShaft(info: ShaftInfo): boolean {
    return info.build_cost.every((cost) => this.stockOf(cost.item) >= cost.amount);
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

/** Turn a `CommandError` into one line a player can act on. */
function describeError(error: unknown): string {
  if (typeof error === "string") return humanise(error);
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
      case "AlreadyPlaced":
        return "There can only be one";
      case "FloorLimit":
        return `The legs will not carry more than ${String(detail.max_floors)} floors`;
      case "Undemolishable":
        return "That cannot be removed";
      case "NoRoomThere":
        return "Nothing there to remove";
      default:
        return humanise(kind);
    }
  }
  return "Command refused";
}

function humanise(kind: string): string {
  return kind.replace(/([a-z])([A-Z])/g, "$1 $2");
}
