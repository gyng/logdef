/**
 * Owns the GL context, the batch, and the label overlay.
 *
 * Stateless between frames by design: `render` takes a snapshot and
 * draws it, start to finish. There is no scene graph to keep in sync
 * and no dirty tracking to get wrong, which at this entity count costs
 * nothing and removes an entire category of bug.
 */

import { LabelLayer, type Label } from "./LabelLayer";
import { QuadBatch, type Color } from "./QuadBatch";
import { TextureBatch, type TextureHandle } from "./TextureBatch";
import { createContext, resizeToDisplay } from "./gl";
import { ITEM_CELLS } from "./itemAtlas";
import { crewArtCell, crewMotionPose, crewMotionRow } from "./crewAtlas";
import { creatureArtCell, creatureMotionRow } from "./creatureAtlas";
import { clampZoom, computeLayout, hitSlot, slotX, floorY, type Layout } from "./layout";
import { atNight, fade, palette } from "./palette";
import {
  drawScene,
  drawSceneAfterGround,
  drawSceneBehindRooms,
  drawSceneBehindRoomsAfterGround,
  drawSceneGround,
  drawSceneInFrontOfRooms,
  edgeScreenX,
  crewPosition,
  enemyPosition,
  featurePoint,
  beatGeometry,
  enclaveGeometry,
  forkGeometry,
  towerShape,
  towerBodyLayout,
  walkerLegPose,
  shaftCarRect,
  shaftTreadYs,
  shaftVisualLayout,
  roomProfile,
  type PlaceMode,
} from "./scene";
import type {
  CatalogSnapshot,
  CombatEffect,
  CombatEvent,
  CrewView,
  FeatureView,
  RoomView,
  StallTag,
  ViewSnapshot,
} from "../bridge/types";
import { waypointArtCell, type WaypointAtlasRect } from "./waypointAtlas";
import { combatFxCell, weaponComponentCell, type CombatFxFamily } from "./combatAtlas";

export interface RenderInput {
  view: ViewSnapshot;
  catalog: CatalogSnapshot;
  placeMode: PlaceMode | null;
  /** Crew the player has picked out. Presentation only. */
  picked: readonly number[];
  clock: number;
  /** Transient spatial actions emitted by this simulation frame. */
  combat: readonly CombatEvent[];
}

interface CombatVisual {
  effect: CombatEffect;
  sourceRoom: number;
  started: number;
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

interface WaypointAftermath {
  def: number;
  started: number;
  x: number;
  y: number;
  size: number;
}

interface ArtTextures {
  skyDawn: TextureHandle;
  skyDay: TextureHandle;
  skyDusk: TextureHandle;
  skyNight: TextureHandle;
  paper: TextureHandle;
  wash: TextureHandle;
  vignette: TextureHandle;
  terrainDoodads: TextureHandle;
  parallaxCanopy: TextureHandle;
  parallaxClearing: TextureHandle;
  parallaxRuins: TextureHandle;
  parallaxDrowned: TextureHandle;
  parallaxCoast: TextureHandle;
  crew: TextureHandle;
  /** Optional two-frame pose atlas; the authored standing sheet remains the fallback. */
  crewMotion: TextureHandle | null;
  creatures: TextureHandle;
  /** Optional state poses; creature-atlas and procedural transitions remain the fallback. */
  creatureMotion: TextureHandle | null;
  /** Optional diegetic stock sprites; subtle procedural tracks remain the fallback. */
  items: TextureHandle | null;
  /** Optional so a missing atlas keeps every procedural room intact. */
  rooms: TextureHandle | null;
  /** Optional fixed collapsed variants for wrecked authored interiors. */
  roomWrecks: TextureHandle | null;
  /** Optional full-cell room shells and crown components. */
  roomArchitecture: TextureHandle | null;
  /** Optional per-room shells; category architecture remains the fallback. */
  roomShells: TextureHandle | null;
  /** Optional component dressing; procedural structure remains its fallback. */
  walkerFrame: TextureHandle | null;
  /** Optional roof-garden silhouettes; procedural planter tufts remain their fallback. */
  roofFlora: TextureHandle | null;
  /** Optional per-floor shaft machinery; procedural shafts remain its fallback. */
  shaftComponents: TextureHandle | null;
  /** Optional route-beat vignettes; procedural silhouettes remain their fallback. */
  waypoints: TextureHandle | null;
  /** Optional authored projectile and impact washes. */
  combatFx: TextureHandle | null;
  /** Optional room-specific exterior emplacement assemblies. */
  weapons: TextureHandle | null;
}

export class Renderer {
  private readonly canvas: HTMLCanvasElement;
  private readonly gl: WebGL2RenderingContext;
  private readonly batch: QuadBatch;
  private readonly textures: TextureBatch;
  private readonly labels: LabelLayer;
  private layout: Layout | null = null;
  /**
   * The last snapshot drawn, kept only so a click can be tested against
   * the creatures that were actually on screen when it happened.
   *
   * Picking a slot needs the layout alone, because a slot does not move.
   * A creature does, every frame, so hit-testing one against a *fresh*
   * snapshot would test a position the player never saw.
   */
  private view: ViewSnapshot | null = null;
  private shape: { slots: number; floors: number } | null = null;
  private art: ArtTextures | null = null;
  private disposed = false;
  /** Presentation-only trails. They never feed back into the simulation. */
  private readonly combatVisuals: CombatVisual[] = [];
  /** A taken beat leaves a brief physical trace instead of popping out. */
  private waypointAftermath: WaypointAftermath | null = null;
  private lastClock = 0;

  constructor(
    canvas: HTMLCanvasElement,
    labelRoot: HTMLElement,
    private readonly toggleRoomPower: (floor: number, slot: number, active: boolean) => void,
  ) {
    this.canvas = canvas;
    this.gl = createContext(canvas);
    this.batch = new QuadBatch(this.gl);
    this.textures = new TextureBatch(this.gl);
    this.labels = new LabelLayer(labelRoot);
    void this.loadArt();
  }

  /**
   * The player's lens over the fitted layout.
   *
   * Lives on the renderer rather than in `GameState`: what somebody is
   * looking at is not a fact about the tower, it does not belong in a
   * replay, and two people watching the same seed should be free to
   * look at different parts of it.
   */
  private zoom = 1;

  setZoom(zoom: number): number {
    this.zoom = clampZoom(zoom);
    return this.zoom;
  }

  getZoom(): number {
    return this.zoom;
  }

  markWaypointTaken(): void {
    if (!this.view || !this.layout) return;
    const beat = beatGeometry(this.view, this.layout);
    if (!beat?.here) return;
    this.waypointAftermath = {
      def: beat.def,
      started: this.lastClock,
      x: beat.x,
      y: beat.y,
      size: beat.size,
    };
  }

  /** Test-only presentation of combat events produced by deterministic stepping. */
  presentCombat(combat: readonly CombatEvent[], catalog: CatalogSnapshot): void {
    if (!this.view || !this.layout) return;
    this.rememberCombat(
      {
        view: this.view,
        catalog,
        placeMode: null,
        picked: [],
        clock: this.lastClock,
        combat,
      },
      this.layout,
    );
  }

  render(input: RenderInput): void {
    const { view, catalog } = input;
    resizeToDisplay(this.canvas);

    const viewport = { width: this.canvas.clientWidth, height: this.canvas.clientHeight };
    if (viewport.width === 0 || viewport.height === 0) return;

    const shape = towerShape(view);
    const layout = computeLayout(viewport, shape, this.zoom);
    this.layout = layout;
    this.shape = shape;
    this.view = view;
    this.lastClock = input.clock;
    this.rememberCombat(input, layout);

    const gl = this.gl;
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.clearColor(0, 0, 0, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);

    if (this.art) {
      this.textures.begin();
      this.textures.pushCover(
        paintedSky(this.art, catalog.dayparts[view.clock.daypart]?.id),
        0,
        0,
        viewport.width,
        layout.horizonY + 2,
        { focusY: 0.58 },
      );
      drawPaintedEnvironmentBackground(this.textures, this.art, input, layout);
      this.textures.flush(viewport.width, viewport.height);

      // Depth is painter order, not just alpha. Each feature plane gets its own
      // contact shadows and sprite pass, then atmosphere washes only that plane
      // and everything behind it. The next, nearer layer stays crisp.
      for (let layer = 0; layer < 3; layer += 1) {
        this.batch.begin();
        drawPaintedFeatureShadows(this.batch, input, layout, layer);
        this.batch.flush(viewport.width, viewport.height);

        this.textures.begin();
        if (layer === 0) {
          drawLandmarkResidentForeshadow(this.textures, this.art, input, layout);
        }
        drawPaintedFeatureLayer(this.textures, this.art, input, layout, layer);
        this.textures.flush(viewport.width, viewport.height);

        if (layer < 2) {
          this.batch.begin();
          drawPaintedDepthFog(this.batch, input, layout, layer);
          this.batch.flush(viewport.width, viewport.height);
        }
      }
    }

    const beat = beatGeometry(view, layout);
    const waypointCell = waypointArtCell(
      beat === null ? undefined : catalog.waypoints[beat.def]?.id,
    );
    const paintedWaypoint = Boolean(this.art?.waypoints && waypointCell && beat);
    const scene = {
      view,
      catalog,
      layout,
      placeMode: input.placeMode,
      picked: input.picked,
      clock: input.clock,
      paintedSky: this.art !== null,
      paintedEnvironment: this.art !== null,
      paintedWalkerFrame: this.art?.walkerFrame !== null && this.art?.walkerFrame !== undefined,
      paintedRoofFlora: this.art?.roofFlora !== null && this.art?.roofFlora !== undefined,
      paintedWaypoint,
      paintedShafts: this.art?.shaftComponents !== null && this.art?.shaftComponents !== undefined,
      paintedCreatures: this.art !== null,
      paintedCrewNeeds: this.art?.crewMotion !== null && this.art?.crewMotion !== undefined,
      paintedCreatureTransitions: this.art?.creatureMotion !== null,
      paintedRoomArchitecture:
        this.art?.roomArchitecture !== null && this.art?.roomArchitecture !== undefined,
      paintedVignette: this.art !== null,
    };

    if (
      this.art &&
      (this.art.rooms ||
        this.art.roomArchitecture ||
        this.art.roomShells ||
        this.art.walkerFrame ||
        this.art.roofFlora ||
        this.art.shaftComponents)
    ) {
      const paintedRoomIds = this.art.roomArchitecture
        ? new Set(catalog.rooms.map((room) => room.id))
        : this.art.roomShells
          ? new Set(catalog.rooms.filter((room) => ROOM_CELLS[room.id]).map((room) => room.id))
          : this.art.rooms
            ? PAINTED_ROOM_IDS
            : EMPTY_ROOM_IDS;
      // The transparent room paintings sit between the tower shell and
      // every live fact drawn over a room: stalls, stock, damage,
      // progress, hearths, people, smoke, and placement feedback.
      if (paintedWaypoint && this.art.waypoints && waypointCell && beat) {
        this.batch.begin();
        drawSceneGround(this.batch, scene);
        this.batch.flush(viewport.width, viewport.height);

        this.textures.begin();
        drawPaintedWaypoint(this.textures, this.art.waypoints, input, beat, waypointCell);
        this.textures.flush(viewport.width, viewport.height);

        if (this.art.walkerFrame) {
          this.textures.begin();
          drawPaintedWalkerLegs(this.textures, this.art.walkerFrame, input, layout);
          this.textures.flush(viewport.width, viewport.height);
        }

        this.batch.begin();
        drawSceneBehindRoomsAfterGround(this.batch, scene, paintedRoomIds);
        this.batch.flush(viewport.width, viewport.height);
      } else if (this.art.walkerFrame) {
        // The articulated kit belongs behind the procedural hull but in
        // front of the terrain. Splitting the pass preserves that painter
        // order whether or not waypoint art loaded.
        this.batch.begin();
        drawSceneGround(this.batch, scene);
        this.batch.flush(viewport.width, viewport.height);

        this.textures.begin();
        drawPaintedWalkerLegs(this.textures, this.art.walkerFrame, input, layout);
        this.textures.flush(viewport.width, viewport.height);

        this.batch.begin();
        drawSceneBehindRoomsAfterGround(this.batch, scene, paintedRoomIds);
        this.batch.flush(viewport.width, viewport.height);
      } else {
        this.batch.begin();
        drawSceneBehindRooms(this.batch, scene, paintedRoomIds);
        this.batch.flush(viewport.width, viewport.height);
      }

      this.textures.begin();
      if (this.art.walkerFrame) {
        drawPaintedWalkerStructure(this.textures, this.art.walkerFrame, input, layout);
      }
      if (this.art.roofFlora) {
        drawPaintedRoofFlora(this.textures, this.art.roofFlora, input, layout);
      }
      if (this.art.shaftComponents) {
        drawPaintedShafts(this.textures, this.art.shaftComponents, input, layout);
      }
      if (this.art.roomArchitecture || this.art.roomShells) {
        drawPaintedRoomShells(
          this.textures,
          this.art.roomArchitecture,
          this.art.roomShells,
          input,
          layout,
        );
      }
      if (this.art.rooms)
        drawPaintedRooms(this.textures, this.art.rooms, this.art.roomWrecks, input, layout);
      if (this.art.roomArchitecture) {
        drawPaintedRoomCrowns(
          this.textures,
          this.art.roomArchitecture,
          input,
          layout,
          Boolean(this.art.weapons),
        );
      }
      if (this.art.weapons) {
        drawPaintedWeapons(this.textures, this.art.weapons, input, layout, this.combatVisuals);
      }
      if (this.art.items) drawPaintedRoomStock(this.textures, this.art.items, input, layout);
      this.textures.flush(viewport.width, viewport.height);

      this.batch.begin();
      drawSceneInFrontOfRooms(this.batch, scene, paintedRoomIds);
      this.batch.flush(viewport.width, viewport.height);
    } else if (paintedWaypoint && this.art?.waypoints && waypointCell && beat) {
      this.batch.begin();
      drawSceneGround(this.batch, scene);
      this.batch.flush(viewport.width, viewport.height);

      this.textures.begin();
      drawPaintedWaypoint(this.textures, this.art.waypoints, input, beat, waypointCell);
      this.textures.flush(viewport.width, viewport.height);

      this.batch.begin();
      drawSceneAfterGround(this.batch, scene);
      this.batch.flush(viewport.width, viewport.height);
    } else {
      this.batch.begin();
      drawScene(this.batch, scene);
      this.batch.flush(viewport.width, viewport.height);
    }

    if (this.art) {
      this.textures.begin();
      drawPaintedArt(this.textures, this.art, input, layout);
      if (this.art.waypoints && this.waypointAftermath) {
        drawWaypointAftermath(this.textures, this.art.waypoints, this.waypointAftermath, input);
        if (input.clock - this.waypointAftermath.started > 1.5) this.waypointAftermath = null;
      }
      if (this.art.combatFx) {
        drawCombatVisuals(
          this.textures,
          this.art.combatFx,
          this.combatVisuals,
          input.clock,
          layout,
        );
      }
      this.textures.flush(viewport.width, viewport.height);
    }

    this.labels.sync(
      buildLabels(view, catalog, layout, input.clock, input.picked, this.toggleRoomPower),
    );
  }

  private rememberCombat(input: RenderInput, layout: Layout): void {
    const body = towerBodyLayout(input.view, layout, input.clock);
    for (const event of input.combat) {
      const info = input.catalog.rooms[event.source.room_def];
      const enemy = input.view.siege.enemies.find(
        (candidate) => candidate.id === event.target.enemy_id,
      );
      const enemyInfo = input.catalog.enemies[event.target.enemy_def];
      if (!info || !enemy || !enemyInfo) continue;
      const target = enemyPosition(input.view, layout, enemyInfo.approach, enemy);
      const roomX = slotX(body, event.source.slot);
      const roomY = floorY(body, event.source.floor);
      const downward = event.effect === "root_ward";
      this.combatVisuals.push({
        effect: event.effect,
        sourceRoom: event.source.room_id,
        started: input.clock,
        x0: roomX + info.width * body.slotW * (downward ? 0.55 : 0.92),
        y0: roomY + body.floorH * (downward ? 0.82 : 0.3),
        x1: target.x,
        y1: target.y - body.slotW * (enemyInfo.approach === "Canopy" ? 0.25 : 0.08),
      });
    }
    const cutoff = input.clock - 0.72;
    while (this.combatVisuals[0] && this.combatVisuals[0].started < cutoff) {
      this.combatVisuals.shift();
    }
  }

  /**
   * Which crew member is under the pointer, if any.
   *
   * The same shape as `pickEnemy`, and the same 26px forgiveness: a
   * person on a cross-section is a small thing and asking for pixel
   * accuracy would make selecting one a chore rather than a gesture.
   */
  pickCrew(clientX: number, clientY: number): number | null {
    if (!this.layout || !this.view) return null;
    const rect = this.canvas.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    let best: { id: number; d: number } | null = null;
    for (const member of this.view.crew) {
      const at = crewPosition(this.layout, member);
      const d = Math.hypot(at.x - x, at.y - (y + 8));
      if (d < 26 && (!best || d < best.d)) best = { id: member.id, d };
    }
    return best?.id ?? null;
  }

  /** Where a crew member's feet are, in client coordinates. */
  crewPointOf(member: CrewView): { x: number; y: number } | null {
    if (!this.layout) return null;
    const rect = this.canvas.getBoundingClientRect();
    const at = crewPosition(this.layout, member);
    return { x: rect.left + at.x, y: rect.top + at.y - 8 };
  }

  /** Everybody whose feet fall inside a client-space rectangle. */
  crewInBox(box: { x0: number; y0: number; x1: number; y1: number }): number[] {
    if (!this.layout || !this.view) return [];
    const rect = this.canvas.getBoundingClientRect();
    const left = Math.min(box.x0, box.x1) - rect.left;
    const right = Math.max(box.x0, box.x1) - rect.left;
    const top = Math.min(box.y0, box.y1) - rect.top;
    const bottom = Math.max(box.y0, box.y1) - rect.top;
    const found: number[] = [];
    for (const member of this.view.crew) {
      const at = crewPosition(this.layout, member);
      // A person is drawn upward from their feet, so the box catches
      // anyone whose body overlaps it rather than only their soles.
      if (at.x >= left && at.x <= right && at.y >= top - 24 && at.y <= bottom + 4) {
        found.push(member.id);
      }
    }
    return found;
  }

  pick(clientX: number, clientY: number): { floor: number; slot: number } | null {
    if (!this.layout || !this.shape) return null;
    const rect = this.canvas.getBoundingClientRect();
    return hitSlot(this.layout, this.shape, clientX - rect.left, clientY - rect.top);
  }

  /**
   * Centre of a slot in client coordinates — the inverse of `pick`.
   *
   * Exists so the smoke test can click where a player would click,
   * rather than at a pixel guessed from the layout constants. A test
   * that hardcodes coordinates passes until someone changes the tower's
   * scale, then fails for a reason that has nothing to do with the bug.
   */
  slotCenter(floor: number, slot: number): { x: number; y: number } | null {
    if (!this.layout || !this.shape) return null;
    if (floor < 0 || floor >= this.shape.floors) return null;
    if (slot < 0 || slot >= this.shape.slots) return null;
    const rect = this.canvas.getBoundingClientRect();
    return {
      x: rect.left + slotX(this.layout, slot) + this.layout.slotW / 2,
      y: rect.top + floorY(this.layout, floor) + this.layout.floorH / 2,
    };
  }

  /**
   * Where a scattered feature is being drawn, in client coordinates.
   *
   * The same job `slotCenter` does for the tower, for the terrain: the
   * screenshot harness has to be able to frame a ruin it wants a
   * picture of, and the alternative is guessing at `paceW` and the
   * parallax table from outside the renderer — which is exactly the
   * kind of hardcoded coordinate that passes until someone changes the
   * scale and then fails for an unrelated-looking reason.
   */
  featureCenter(distance: number, feature: FeatureView): { x: number; y: number } | null {
    if (!this.layout) return null;
    const rect = this.canvas.getBoundingClientRect();
    const { x, y } = featurePoint(distance, this.layout, feature);
    return { x: rect.left + x, y: rect.top + y };
  }

  get quadCount(): number {
    return this.batch.queued + this.textures.queued;
  }

  dispose(): void {
    this.disposed = true;
    this.batch.dispose();
    this.textures.dispose();
    this.labels.dispose();
  }

  private async loadArt(): Promise<void> {
    try {
      const [
        skyDawn,
        skyDay,
        skyDusk,
        skyNight,
        paper,
        wash,
        vignette,
        terrainDoodads,
        parallaxCanopy,
        parallaxClearing,
        parallaxRuins,
        parallaxDrowned,
        parallaxCoast,
        crew,
        creatures,
      ] = await Promise.all([
        this.textures.load(artUrl("sky-dawn.webp")),
        this.textures.load(artUrl("sky-day.webp")),
        this.textures.load(artUrl("sky-dusk.webp")),
        this.textures.load(artUrl("sky-night.webp")),
        this.textures.load(artUrl("paper-grain.png"), { mipmaps: true }),
        this.textures.load(artUrl("wash-overlay.webp")),
        this.textures.load(artUrl("vignette.png")),
        this.textures.load(artUrl("terrain-doodads-atlas.png")),
        this.textures.load(artUrl("parallax-canopy.webp")),
        this.textures.load(artUrl("parallax-clearing.webp")),
        this.textures.load(artUrl("parallax-ruins.webp")),
        this.textures.load(artUrl("parallax-drowned.webp")),
        this.textures.load(artUrl("parallax-coast.webp")),
        this.textures.load(artUrl("crew-atlas.png")),
        this.textures.load(artUrl("creature-atlas.png")),
      ]);
      if (this.disposed) return;
      this.art = {
        skyDawn,
        skyDay,
        skyDusk,
        skyNight,
        paper,
        wash,
        vignette,
        terrainDoodads,
        parallaxCanopy,
        parallaxClearing,
        parallaxRuins,
        parallaxDrowned,
        parallaxCoast,
        crew,
        crewMotion: null,
        creatures,
        creatureMotion: null,
        items: null,
        rooms: null,
        roomWrecks: null,
        roomArchitecture: null,
        roomShells: null,
        walkerFrame: null,
        roofFlora: null,
        shaftComponents: null,
        waypoints: null,
        combatFx: null,
        weapons: null,
      };
      const optional = await Promise.allSettled([
        this.textures.load(artUrl("rooms-atlas.png")),
        this.textures.load(artUrl("room-wrecks-atlas.png")),
        this.textures.load(artUrl("walker-frame-atlas.png")),
        this.textures.load(artUrl("shaft-components-atlas.png")),
        this.textures.load(artUrl("items-atlas.png")),
        this.textures.load(artUrl("room-architecture-atlas.png")),
        this.textures.load(artUrl("room-shells-atlas.png")),
        this.textures.load(artUrl("roof-flora-atlas.png")),
        this.textures.load(artUrl("waypoint-atlas.png")),
        this.textures.load(artUrl("crew-motion-atlas.png")),
        this.textures.load(artUrl("creature-motion-atlas.png")),
        this.textures.load(artUrl("combat-fx-atlas.png")),
        this.textures.load(artUrl("weapon-components-atlas.png")),
      ]);
      if (!this.disposed && this.art) {
        const [
          rooms,
          roomWrecks,
          walkerFrame,
          shaftComponents,
          items,
          roomArchitecture,
          roomShells,
          roofFlora,
          waypoints,
          crewMotion,
          creatureMotion,
          combatFx,
          weapons,
        ] = optional;
        this.art = {
          ...this.art,
          rooms: rooms.status === "fulfilled" ? rooms.value : null,
          roomWrecks: roomWrecks.status === "fulfilled" ? roomWrecks.value : null,
          walkerFrame: walkerFrame.status === "fulfilled" ? walkerFrame.value : null,
          shaftComponents: shaftComponents.status === "fulfilled" ? shaftComponents.value : null,
          items: items.status === "fulfilled" ? items.value : null,
          roomArchitecture: roomArchitecture.status === "fulfilled" ? roomArchitecture.value : null,
          roomShells: roomShells.status === "fulfilled" ? roomShells.value : null,
          roofFlora: roofFlora.status === "fulfilled" ? roofFlora.value : null,
          waypoints: waypoints.status === "fulfilled" ? waypoints.value : null,
          crewMotion: crewMotion.status === "fulfilled" ? crewMotion.value : null,
          creatureMotion: creatureMotion.status === "fulfilled" ? creatureMotion.value : null,
          combatFx: combatFx.status === "fulfilled" ? combatFx.value : null,
          weapons: weapons.status === "fulfilled" ? weapons.value : null,
        };
        for (const result of optional) {
          if (result.status === "rejected") {
            console.warn(
              "Optional painted structure did not load; keeping its procedural fallback",
              result.reason,
            );
          }
        }
      }
    } catch (error) {
      if (!this.disposed)
        console.warn("Understory art did not load; keeping procedural art", error);
    }
  }
}

function artUrl(file: string): string {
  return `${import.meta.env.BASE_URL}art/${file}`;
}

function paintedSky(art: ArtTextures, daypartId: string | undefined): TextureHandle {
  if (daypartId === "daypart.dawn" || daypartId === "daypart.predawn") return art.skyDawn;
  if (daypartId === "daypart.dusk") return art.skyDusk;
  if (daypartId === "daypart.night") return art.skyNight;
  return art.skyDay;
}

type AtlasRect = readonly [x: number, y: number, width: number, height: number];

const FRAME_CELLS = {
  roof: [0, 0, 128, 128],
  wall: [128, 0, 128, 128],
  deck: [256, 0, 128, 128],
  rib: [384, 0, 128, 128],
  underside: [0, 128, 128, 128],
  stairs: [128, 128, 128, 128],
  // The authored beam occupies only y54..74 inside its 128px source
  // cell. Sampling that opaque strip directly makes `thickness` mean
  // visible metal instead of mostly transparent padding.
  strut: [260, 182, 120, 20],
  joint: [384, 128, 128, 128],
  foot: [0, 256, 128, 128],
  planter: [128, 256, 128, 128],
  plating: [256, 256, 128, 128],
  moss: [384, 256, 128, 128],
} as const satisfies Record<string, AtlasRect>;

/** 4x2 authored roof-garden sheet; cell 3 is the trailing-vine silhouette. */
const ROOF_FLORA_CELLS = [
  [0, 0, 128, 128],
  [128, 0, 128, 128],
  [256, 0, 128, 128],
  [384, 0, 128, 128],
  [0, 128, 128, 128],
  [128, 128, 128, 128],
  [256, 128, 128, 128],
  [384, 128, 128, 128],
] as const satisfies readonly AtlasRect[];

// Alpha bounds in the packed 128 px cells. The packer centres each keyed
// silhouette, so the cell edge is not the plant's authored attachment point.
// Anchor upright plants by their opaque bottom and the trailing vine by its
// opaque top or they visibly hover above (or sink through) the planter.
const ROOF_FLORA_BOTTOM = [96, 105, 100, 110, 101, 109, 85, 124] as const;
const ROOF_FLORA_TOP = [32, 23, 28, 17, 26, 18, 43, 4] as const;

const SHAFT_CELLS = {
  stairsBay: [0, 0, 256, 256],
  stairTread: [256, 0, 256, 256],
  elevatorBay: [512, 0, 256, 256],
  elevatorCar: [0, 256, 256, 256],
  elevatorHead: [256, 256, 256, 256],
  counterweight: [512, 256, 256, 256],
} as const satisfies Record<string, AtlasRect>;

function pushFrameCell(
  batch: TextureBatch,
  texture: TextureHandle,
  cell: AtlasRect,
  x: number,
  y: number,
  width: number,
  height: number,
  tint: Color,
): void {
  batch.pushAtlas(texture, x, y, width, height, ...cell, tint);
}

/** Paint material onto the live hull dimensions and authoritative six-leg pose. */
function drawPaintedWalkerLegs(
  batch: TextureBatch,
  texture: TextureHandle,
  { view, clock }: RenderInput,
  layout: Layout,
): void {
  const body = towerBodyLayout(view, layout, clock);
  const daylight = Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  // Gunmetal rather than polished brass: the leg is the tower's dirty,
  // load-bearing undercarriage. Small warm bosses below provide the
  // steampunk punctuation without turning the whole limb gold.
  const material: Color = [
    0.46 + daylight * 0.18,
    0.48 + daylight * 0.18,
    0.45 + daylight * 0.16,
    0.98,
  ];

  // Rear legs paint first, muted and a little slimmer; the near-plane
  // tripod then crosses them cleanly. Both remain behind the hull.
  const poses = walkerLegPose(view, layout, clock);
  for (const rear of [true, false]) {
    for (const pose of poses) {
      if (pose.rear !== rear) continue;
      const tint: Color = rear
        ? [material[0] * 0.72, material[1] * 0.76, material[2] * 0.8, 0.86]
        : material;
      // Mechanical load path: a heavy upper actuator tapers into a
      // lighter lower link. Geometry still comes entirely from the
      // solved two-link pose; these values only control painted mass.
      const upperThickness = body.slotW * (rear ? 0.34 : 0.44);
      const lowerThickness = body.slotW * (rear ? 0.22 : 0.28);
      batch.pushAtlasSegment(
        texture,
        pose.hip.x,
        pose.hip.y,
        pose.joint.x,
        pose.joint.y,
        upperThickness,
        ...FRAME_CELLS.strut,
        tint,
      );
      batch.pushAtlasSegment(
        texture,
        pose.joint.x,
        pose.joint.y,
        pose.foot.x,
        pose.foot.y,
        lowerThickness,
        ...FRAME_CELLS.strut,
        tint,
      );

      // Hydraulic ram crossing the elbow: the barrel mounts partway
      // down the upper link and its rod meets the lower link below the
      // servo. Both endpoints derive from the solved pose, so the ram
      // visibly compresses and extends with the IK rather than skating.
      const pistonStart = {
        x: pose.hip.x + (pose.joint.x - pose.hip.x) * 0.18,
        y: pose.hip.y + (pose.joint.y - pose.hip.y) * 0.18,
      };
      const pistonEnd = {
        x: pose.joint.x + (pose.foot.x - pose.joint.x) * 0.3,
        y: pose.joint.y + (pose.foot.y - pose.joint.y) * 0.3,
      };
      const pistonCollar = {
        x: pistonStart.x + (pistonEnd.x - pistonStart.x) * 0.6,
        y: pistonStart.y + (pistonEnd.y - pistonStart.y) * 0.6,
      };
      batch.pushAtlasSegment(
        texture,
        pistonStart.x,
        pistonStart.y,
        pistonCollar.x,
        pistonCollar.y,
        body.slotW * (rear ? 0.085 : 0.11),
        ...FRAME_CELLS.strut,
        [tint[0] * 0.72, tint[1] * 0.74, tint[2] * 0.76, tint[3]],
      );
      batch.pushAtlasSegment(
        texture,
        pistonCollar.x,
        pistonCollar.y,
        pistonEnd.x,
        pistonEnd.y,
        body.slotW * (rear ? 0.038 : 0.052),
        ...FRAME_CELLS.strut,
        [0.74, 0.72, 0.66, rear ? 0.74 : 0.92],
      );

      // A second, shorter ram sits beside the main knee actuator. It
      // braces the upper beam against torsion and makes the machine read
      // as deliberately over-supported rather than as six elegant sticks.
      // Offset both mounts perpendicular to the upper link so the pair
      // remains visibly separate through the full IK cycle.
      const upperDx = pose.joint.x - pose.hip.x;
      const upperDy = pose.joint.y - pose.hip.y;
      const upperLength = Math.hypot(upperDx, upperDy) || 1;
      const braceSide = pose.rear ? -1 : 1;
      const braceOffset = upperThickness * 0.34 * braceSide;
      const braceNx = (-upperDy / upperLength) * braceOffset;
      const braceNy = (upperDx / upperLength) * braceOffset;
      const braceRailStart = { x: pose.hip.x + braceNx, y: pose.hip.y + braceNy };
      const braceRailEnd = { x: pose.joint.x + braceNx, y: pose.joint.y + braceNy };
      batch.pushAtlasSegment(
        texture,
        braceRailStart.x,
        braceRailStart.y,
        braceRailEnd.x,
        braceRailEnd.y,
        body.slotW * (rear ? 0.052 : 0.07),
        ...FRAME_CELLS.strut,
        [tint[0] * 0.68, tint[1] * 0.7, tint[2] * 0.71, tint[3]],
      );
      // Two short cross-ties turn the thigh into a visible box truss.
      for (const along of [0.3, 0.68]) {
        const mainX = pose.hip.x + upperDx * along;
        const mainY = pose.hip.y + upperDy * along;
        batch.pushAtlasSegment(
          texture,
          mainX,
          mainY,
          mainX + braceNx,
          mainY + braceNy,
          body.slotW * (rear ? 0.028 : 0.036),
          ...FRAME_CELLS.strut,
          [tint[0] * 0.72, tint[1] * 0.74, tint[2] * 0.74, tint[3]],
        );
      }
      const braceStart = {
        x: pose.hip.x + upperDx * 0.08 + braceNx,
        y: pose.hip.y + upperDy * 0.08 + braceNy,
      };
      const braceEnd = {
        x: pose.joint.x + (pose.foot.x - pose.joint.x) * 0.18 + braceNx * 0.35,
        y: pose.joint.y + (pose.foot.y - pose.joint.y) * 0.18 + braceNy * 0.35,
      };
      const braceCollar = {
        x: braceStart.x + (braceEnd.x - braceStart.x) * 0.64,
        y: braceStart.y + (braceEnd.y - braceStart.y) * 0.64,
      };
      batch.pushAtlasSegment(
        texture,
        braceStart.x,
        braceStart.y,
        braceCollar.x,
        braceCollar.y,
        body.slotW * (rear ? 0.062 : 0.082),
        ...FRAME_CELLS.strut,
        [tint[0] * 0.62, tint[1] * 0.64, tint[2] * 0.66, tint[3]],
      );
      batch.pushAtlasSegment(
        texture,
        braceCollar.x,
        braceCollar.y,
        braceEnd.x,
        braceEnd.y,
        body.slotW * (rear ? 0.03 : 0.04),
        ...FRAME_CELLS.strut,
        [0.72, 0.71, 0.66, rear ? 0.72 : 0.92],
      );
      pushFrameCell(
        batch,
        texture,
        FRAME_CELLS.joint,
        pose.hip.x - body.slotW * (rear ? 0.23 : 0.29),
        pose.hip.y - body.slotW * (rear ? 0.23 : 0.29),
        body.slotW * (rear ? 0.46 : 0.58),
        body.slotW * (rear ? 0.46 : 0.58),
        tint,
      );
      pushFrameCell(
        batch,
        texture,
        FRAME_CELLS.joint,
        pose.joint.x - body.slotW * (rear ? 0.21 : 0.25),
        pose.joint.y - body.slotW * (rear ? 0.21 : 0.25),
        body.slotW * (rear ? 0.42 : 0.5),
        body.slotW * (rear ? 0.42 : 0.5),
        tint,
      );
      // A smaller warm boss inside the large painted servo keeps the
      // pivot readable as a mechanism rather than a swollen bend.
      pushFrameCell(
        batch,
        texture,
        FRAME_CELLS.joint,
        pose.joint.x - body.slotW * 0.1,
        pose.joint.y - body.slotW * 0.1,
        body.slotW * 0.2,
        body.slotW * 0.2,
        [0.72, 0.62, 0.44, rear ? 0.7 : 0.88],
      );
      pushFrameCell(
        batch,
        texture,
        FRAME_CELLS.foot,
        pose.foot.x - body.slotW * 0.46,
        pose.foot.y - body.slotW * 0.25,
        body.slotW * 0.92,
        body.slotW * 0.5,
        tint,
      );
      // Short suspension link closes the old visual gap between hull and hip.
      batch.pushAtlasSegment(
        texture,
        pose.hip.x,
        body.groundY - body.slotW * 0.03,
        pose.hip.x,
        pose.hip.y,
        body.slotW * (rear ? 0.26 : 0.32),
        ...FRAME_CELLS.strut,
        tint,
      );
    }
  }
}

/** Paint material onto the live hull dimensions, above the articulated legs. */
function drawPaintedWalkerStructure(
  batch: TextureBatch,
  texture: TextureHandle,
  { view, clock }: RenderInput,
  layout: Layout,
): void {
  const body = towerBodyLayout(view, layout, clock);
  const shape = towerShape(view);
  const spanX = shape.slots * body.slotW;
  const topY = floorY(body, shape.floors - 1);
  const daylight = Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  const material: Color = [
    0.72 + daylight * 0.24,
    0.7 + daylight * 0.26,
    0.64 + daylight * 0.28,
    0.94,
  ];
  const backing: Color = [material[0] * 0.72, material[1] * 0.74, material[2] * 0.76, 0.66];

  // Repeating modules let the same machine grow from 10 to 16 slots
  // and from two to fourteen floors without stretching a fixed picture.
  for (const floor of view.tower.floors) {
    const y = floorY(body, floor.index);
    for (let slot = 0; slot < shape.slots; slot += 2) {
      const width = Math.min(2, shape.slots - slot) * body.slotW;
      const x = body.originX + slot * body.slotW;
      pushFrameCell(batch, texture, FRAME_CELLS.wall, x, y, width, body.floorH, backing);
      pushFrameCell(
        batch,
        texture,
        FRAME_CELLS.deck,
        x,
        y + body.floorH - body.slotW * 0.2,
        width,
        body.slotW * 0.24,
        material,
      );
    }
    for (let slot = 0; slot <= shape.slots; slot += 2) {
      pushFrameCell(
        batch,
        texture,
        FRAME_CELLS.rib,
        body.originX + slot * body.slotW - body.slotW * 0.16,
        y,
        body.slotW * 0.32,
        body.floorH,
        material,
      );
    }
  }

  for (let slot = 0; slot < shape.slots; slot += 2) {
    const width = Math.min(2, shape.slots - slot) * body.slotW;
    const x = body.originX + slot * body.slotW;
    pushFrameCell(
      batch,
      texture,
      FRAME_CELLS.underside,
      x,
      body.groundY - body.slotW * 0.12,
      width,
      body.slotW * 0.28,
      material,
    );
    pushFrameCell(
      batch,
      texture,
      FRAME_CELLS.roof,
      x,
      topY - body.slotW * 0.28,
      width,
      body.slotW * 0.25,
      material,
    );
  }
  for (let slot = 0; slot < shape.slots; slot += 1) {
    pushFrameCell(
      batch,
      texture,
      FRAME_CELLS.planter,
      body.originX + (slot + 0.25) * body.slotW,
      topY - body.slotW * 0.23,
      body.slotW * 0.5,
      body.slotW * 0.24,
      material,
    );
  }
  pushFrameCell(
    batch,
    texture,
    FRAME_CELLS.moss,
    body.originX - body.slotW * 0.08,
    topY - body.slotW * 0.25,
    spanX * 0.52,
    body.slotW * 0.28,
    [0.72, 0.84, 0.68, 0.78],
  );
}

/**
 * Repeat a small authored garden kit across any live hull width.
 *
 * Seven cells are upright silhouettes planted behind the roof lip; the vine
 * cell hangs over it. Selection is stable in world state, never the
 * render clock, so leaves do not flicker while the walker moves.
 */
function drawPaintedRoofFlora(
  batch: TextureBatch,
  texture: TextureHandle,
  { view, clock }: RenderInput,
  layout: Layout,
): void {
  const body = towerBodyLayout(view, layout, clock);
  const shape = towerShape(view);
  const topY = floorY(body, shape.floors - 1);
  const daylight = Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  const tint: Color = [0.72 + daylight * 0.28, 0.75 + daylight * 0.25, 0.7 + daylight * 0.3, 0.96];
  // The painted planter's visible rim sits about .15 slots above the floor
  // top (its own source cell also has transparent padding).
  const planterRimY = topY - body.slotW * 0.15;

  const uprightCells = [0, 1, 2, 4, 5, 6, 7] as const;
  const heightByCell = [0.78, 1.08, 0.8, 0, 0.9, 0.8, 0.68, 0.9] as const;
  const widthByCell = [1.12, 1.16, 1.08, 0, 1.08, 1.16, 1.14, 1.06] as const;
  for (let slot = 0; slot < shape.slots; slot += 1) {
    const variant = uprightCells[(slot * 5 + shape.floors * 3) % uprightCells.length]!;
    const height = body.slotW * heightByCell[variant]!;
    const width = body.slotW * widthByCell[variant]!;
    const x = body.originX + (slot + 0.5) * body.slotW - width / 2;
    pushFrameCell(
      batch,
      texture,
      ROOF_FLORA_CELLS[variant]!,
      x,
      planterRimY - height * (ROOF_FLORA_BOTTOM[variant]! / 128),
      width,
      height,
      tint,
    );

    if ((slot + shape.floors) % 3 === 0) {
      const spill = ROOF_FLORA_CELLS[3]!;
      const spillW = body.slotW * 0.82;
      const spillH = body.slotW * 0.82;
      pushFrameCell(
        batch,
        texture,
        spill,
        body.originX + (slot + 0.2) * body.slotW,
        planterRimY - spillH * (ROOF_FLORA_TOP[3]! / 128),
        spillW,
        spillH,
        tint,
      );
    }
  }
}

/** Repeated shaft machinery beneath live damage, doors, queues, cargo and crew. */
function drawPaintedShafts(
  batch: TextureBatch,
  texture: TextureHandle,
  { view, clock }: RenderInput,
  layout: Layout,
): void {
  const body = towerBodyLayout(view, layout, clock);
  const daylight = Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  const material: Color = [
    0.7 + daylight * 0.25,
    0.69 + daylight * 0.26,
    0.64 + daylight * 0.27,
    0.98,
  ];

  for (const shaft of view.tower.shafts) {
    if (shaft.kind === "Chute" || shaft.kind === "Busbar" || shaft.kind === "VentStack") continue;
    const visual = shaftVisualLayout(shaft, body);
    const tint: Color = shaft.severed
      ? [0.42, 0.4, 0.38, 0.82]
      : visual.busy
        ? [1, 0.9, 0.58, 1]
        : [
            material[0] * (1 - visual.hurt * 0.35),
            material[1] * (1 - visual.hurt * 0.4),
            material[2] * (1 - visual.hurt * 0.45),
            material[3],
          ];
    // Elevators are the deliberate transport upgrade, not another dark
    // service column. Cool enamel rails separate the bay from the warm
    // hull; polished brass stays on the moving machinery and cage.
    const elevatorTint: Color = shaft.severed
      ? tint
      : visual.busy
        ? [1.08, 0.98, 0.68, 1]
        : [0.86 + daylight * 0.12, 1, 0.96 + daylight * 0.06, 1];

    const bay = shaft.kind === "Stairs" ? SHAFT_CELLS.stairsBay : SHAFT_CELLS.elevatorBay;
    const bayTint = shaft.kind === "Stairs" ? tint : elevatorTint;
    for (let floor = shaft.high; floor >= shaft.low; floor -= 1) {
      pushFrameCell(
        batch,
        texture,
        bay,
        visual.x,
        floorY(body, floor),
        body.slotW,
        body.floorH,
        bayTint,
      );
    }

    if (shaft.kind === "Stairs") {
      for (const y of shaftTreadYs(shaft, body)) {
        pushFrameCell(
          batch,
          texture,
          SHAFT_CELLS.stairTread,
          visual.x + body.slotW * 0.08,
          y - Math.max(2, body.slotW * 0.045),
          body.slotW * 0.84,
          Math.max(4, body.slotW * 0.09),
          tint,
        );
      }
      continue;
    }

    pushFrameCell(
      batch,
      texture,
      SHAFT_CELLS.elevatorHead,
      visual.x + body.slotW * 0.05,
      visual.top - body.floorH * 0.12,
      body.slotW * 0.9,
      body.floorH * 0.42,
      shaft.severed ? tint : [1.08, 0.92, 0.7, 1],
    );
    const leadCar = shaft.cars[0];
    if (leadCar) {
      const inverseFloor = shaft.low + shaft.high - leadCar.floor;
      const counterY = body.groundY - inverseFloor * body.floorH - body.floorH * 0.55 - 4;
      pushFrameCell(
        batch,
        texture,
        SHAFT_CELLS.counterweight,
        visual.x + body.slotW * 0.67,
        counterY,
        body.slotW * 0.26,
        body.floorH * 0.55,
        [elevatorTint[0] * 0.7, elevatorTint[1] * 0.76, elevatorTint[2] * 0.8, 0.92],
      );
    }
    for (const car of shaft.cars) {
      const rect = shaftCarRect(car, body, visual.x);
      pushFrameCell(
        batch,
        texture,
        SHAFT_CELLS.elevatorCar,
        rect.x - 3,
        rect.y - body.floorH * 0.08,
        rect.width + 6,
        rect.height + body.floorH * 0.14,
        shaft.severed ? tint : [1.08, 0.98, 0.76, 1],
      );
    }
  }
}

/** Cells in the 4x4, 1024px room architecture atlas. */
const ROOM_SHELL_CELLS: Readonly<Record<string, AtlasRect>> = {
  Heart: [0, 0, 256, 256],
  Intake: [256, 0, 256, 256],
  Production: [512, 0, 256, 256],
  Storage: [768, 0, 256, 256],
  Energy: [0, 256, 256, 256],
  Defence: [256, 256, 256, 256],
  Quarters: [512, 256, 256, 256],
  fallback: [768, 256, 256, 256],
};

const ROOM_CROWN_CELLS: Readonly<Record<string, AtlasRect>> = {
  dome: [0, 512, 256, 256],
  trellis: [256, 512, 256, 256],
  cells: [512, 512, 256, 256],
  stack: [768, 512, 256, 256],
  vent: [0, 768, 256, 256],
  boom: [256, 768, 256, 256],
  barrel: [512, 768, 256, 256],
  fallback: [768, 768, 256, 256],
};

function roomArchitectureTint(view: ViewSnapshot, room: RoomView): Color {
  const daylight = Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  const ambient = view.power.lit ? 0.82 + daylight * 0.18 : 0.46 + daylight * 0.1;
  const state =
    !room.active || !room.powered ? 0.52 : room.stalled && room.stall !== "backedup" ? 0.68 : 1;
  const health = 0.72 + Math.max(0, Math.min(1, room.health_permille / 1000)) * 0.28;
  const level = ambient * state * health;
  return [level * 0.96, level, level * 0.98, 1];
}

/**
 * The room's permanent wall keeps its authored colour when machinery stops.
 * Activity belongs to the furnishing, lamps and motion above it; dimming the
 * whole backing erased exactly the paint, cloth and repair history this layer
 * exists to contribute.
 */
function roomShellTint(view: ViewSnapshot, room: RoomView): Color {
  const daylight = Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  const ambient = view.power.lit ? 0.8 + daylight * 0.2 : 0.5 + daylight * 0.12;
  const health = 0.78 + Math.max(0, Math.min(1, room.health_permille / 1000)) * 0.22;
  const level = ambient * health;
  return [level * 0.96, level, level * 0.98, 1];
}

/** Fill every occupied room with its authored shell or a category fallback. */
function drawPaintedRoomShells(
  batch: TextureBatch,
  categoryAtlas: TextureHandle | null,
  roomAtlas: TextureHandle | null,
  { view, catalog, clock }: RenderInput,
  layout: Layout,
): void {
  const body = towerBodyLayout(view, layout, clock);
  for (const floor of view.tower.floors) {
    const top = floorY(body, floor.index) + 1;
    for (const room of floor.rooms) {
      if (room.wrecked) continue;
      const info = catalog.rooms[room.def];
      if (!info) continue;
      const roomSource = info ? ROOM_CELLS[info.id] : undefined;
      if (roomAtlas && roomSource) {
        // Per-room backings use the furnishing atlas' exact one-, two-,
        // and three-slot cell contract. Drawing the source once avoids
        // the wallpaper repetition of a category bay while filling the
        // complete room footprint behind its separate machinery pass.
        batch.pushAtlas(
          roomAtlas,
          slotX(body, room.slot),
          top,
          room.width * body.slotW,
          body.floorH - 4,
          roomSource[0],
          roomSource[1],
          roomSource[2],
          roomSource[3],
          roomShellTint(view, room),
        );
        continue;
      }
      if (!categoryAtlas) continue;
      const tint = roomArchitectureTint(view, room);
      const source = ROOM_SHELL_CELLS[info?.category ?? ""] ?? ROOM_SHELL_CELLS.fallback!;
      // A tile is one structural bay. Repeating it preserves authored
      // scale in wide rooms and makes the room own all of its back wall.
      for (let slot = 0; slot < room.width; slot += 1) {
        batch.pushAtlas(
          categoryAtlas,
          slotX(body, room.slot + slot),
          top,
          body.slotW,
          body.floorH - 4,
          source[0],
          source[1],
          source[2],
          source[3],
          tint,
        );
      }
    }
  }
}

/** Painted silhouettes above the room furnishings, driven by live snapshot state. */
function drawPaintedRoomCrowns(
  batch: TextureBatch,
  atlas: TextureHandle,
  { view, catalog, clock }: RenderInput,
  layout: Layout,
  paintedWeapons: boolean,
): void {
  const body = towerBodyLayout(view, layout, clock);
  for (const floor of view.tower.floors) {
    const floorTop = floorY(body, floor.index);
    for (const room of floor.rooms) {
      if (room.wrecked) continue;
      const info = catalog.rooms[room.def];
      if (!info) continue;
      // The weapon sheet owns these complete exterior silhouettes. Keeping
      // the old generic lilac barrel underneath made the authored machines
      // look like stray UI bars.
      if (paintedWeapons && weaponComponentCell(info.id)) continue;
      const profile = roomProfile(info);
      if (profile.crown === "none") continue;
      const source = ROOM_CROWN_CELLS[profile.crown] ?? ROOM_CROWN_CELLS.fallback!;
      const inset = body.slotW * 0.06;
      const x = slotX(body, room.slot) + inset;
      const w = room.width * body.slotW - inset * 2;
      const usable = body.floorH - 3;
      const h = usable * profile.rise;
      const y = floorTop + usable - h;
      const head = usable - h;
      const tint = roomArchitectureTint(view, room);
      const working = room.active && room.powered && !room.stalled && !room.shaded;

      switch (profile.crown) {
        case "dome": {
          const crownW = Math.min(w * 0.42, head * 4.2);
          batch.pushAtlas(
            atlas,
            x + (w - crownW) * 0.5,
            floorTop,
            crownW,
            Math.max(head, 4),
            source[0],
            source[1],
            source[2],
            source[3],
            tint,
          );
          break;
        }
        case "trellis": {
          const exposure = room.shaded
            ? 0
            : Math.max(0, Math.min(1, view.clock.exposure_pct / 100));
          const crownH = Math.max(4, head * (0.55 + exposure * 0.45));
          const crownTint: Color = room.shaded
            ? [tint[0] * 0.62, tint[1] * 0.66, tint[2] * 0.62, tint[3]]
            : tint;
          batch.pushAtlas(
            atlas,
            x + w * 0.05,
            y - crownH,
            w * 0.9,
            crownH,
            source[0],
            source[1],
            source[2],
            source[3],
            crownTint,
          );
          break;
        }
        case "cells": {
          const crownX = x + w * 0.08;
          const crownW = w * 0.84;
          const crownH = Math.max(head, 5);
          const emptyTint: Color = [tint[0] * 0.42, tint[1] * 0.46, tint[2] * 0.5, tint[3]];
          batch.pushAtlas(
            atlas,
            crownX,
            y - crownH,
            crownW,
            crownH,
            source[0],
            source[1],
            source[2],
            source[3],
            emptyTint,
          );
          const charge = Math.max(0, Math.min(1, view.power.fill_permille / 1000));
          if (charge > 0.01) {
            const sourceH = source[3] * charge;
            const fillH = crownH * charge;
            const breath = working ? 0.9 + Math.sin(clock * 2.2) * 0.1 : 0.72;
            batch.pushAtlas(
              atlas,
              crownX,
              y - fillH,
              crownW,
              fillH,
              source[0],
              source[1] + source[3] - sourceH,
              source[2],
              sourceH,
              [tint[0] * breath, tint[1] * breath, tint[2] * breath, tint[3]],
            );
          }
          break;
        }
        case "boom": {
          const moving = working && view.journey.halt === "walking";
          const angle = 0.42 + (moving ? Math.sin(view.world.distance * 0.6) * 0.16 : 0);
          const pivotX = x + w * 0.55;
          const pivotY = y + h * 0.1;
          const reach = w * 0.75;
          batch.pushAtlasSegment(
            atlas,
            pivotX,
            pivotY,
            pivotX + Math.cos(angle) * reach,
            pivotY + Math.sin(angle) * reach,
            Math.max(6, h * 0.16),
            source[0],
            source[1],
            source[2],
            source[3],
            tint,
          );
          break;
        }
        case "barrel":
          batch.pushAtlas(
            atlas,
            x + w * 0.34,
            y - head * 0.58,
            w * 0.92,
            Math.max(6, head * 0.42),
            source[0],
            source[1],
            source[2],
            source[3],
            tint,
            -0.05,
          );
          break;
        case "stack":
          batch.pushAtlas(
            atlas,
            x + w * 0.48,
            floorTop,
            w * 0.46,
            Math.max(head, 5),
            source[0],
            source[1],
            source[2],
            source[3],
            tint,
          );
          break;
        case "vent":
        default:
          batch.pushAtlas(
            atlas,
            x + w * 0.54,
            floorTop,
            w * 0.34,
            Math.max(head, 5),
            source[0],
            source[1],
            source[2],
            source[3],
            tint,
          );
          break;
      }
    }
  }
}

/** Distinct authored exterior emplacements; live firing is a separate event layer. */
function drawPaintedWeapons(
  batch: TextureBatch,
  atlas: TextureHandle,
  { view, catalog, clock }: RenderInput,
  layout: Layout,
  combatVisuals: readonly CombatVisual[],
): void {
  const body = towerBodyLayout(view, layout, clock);
  for (const floor of view.tower.floors) {
    const floorTop = floorY(body, floor.index);
    for (const room of floor.rooms) {
      if (room.wrecked) continue;
      const info = catalog.rooms[room.def];
      if (!info) continue;
      const cell = weaponComponentCell(info.id);
      if (!cell) continue;
      let fired: CombatVisual | undefined;
      for (let index = combatVisuals.length - 1; index >= 0; index -= 1) {
        if (combatVisuals[index]?.sourceRoom === room.id) {
          fired = combatVisuals[index];
          break;
        }
      }
      const fireAge = fired ? Math.max(0, Math.min(1, (clock - fired.started) / 0.24)) : 1;
      const recoil = fired ? Math.sin(fireAge * Math.PI) * body.slotW * 0.08 : 0;
      const width = room.width * body.slotW * 1.12;
      const height = Math.max(body.floorH * 0.72, body.slotW * 0.72);
      const x = slotX(body, room.slot) + room.width * body.slotW - width * 0.78 - recoil;
      const y = floorTop - height * 0.2;
      const tint = roomArchitectureTint(view, room);
      batch.pushAtlas(atlas, x, y, width, height, ...cell, tint);
    }
  }
}

/** Pixel rectangles in the authored 1024x768 room atlas. */
const ROOM_CELLS: Readonly<Record<string, AtlasRect>> = {
  "room.heartseed": [0, 0, 384, 128],
  "room.salvage_rig": [384, 0, 384, 128],
  "room.bunk": [0, 128, 256, 128],
  "room.burner": [256, 128, 256, 128],
  "room.canteen": [512, 128, 256, 128],
  "room.cellwright": [768, 128, 256, 128],
  "room.cutter_arm": [0, 256, 256, 128],
  "room.fiber_comb": [256, 256, 256, 128],
  "room.fitter": [512, 256, 256, 128],
  "room.garden": [768, 256, 256, 128],
  "room.kitbench": [0, 384, 256, 128],
  "room.lantern_mast": [256, 384, 256, 128],
  "room.mill": [512, 384, 256, 128],
  "room.resonance_array": [768, 384, 256, 128],
  "room.resonator_works": [0, 512, 256, 128],
  "room.ropery": [256, 512, 256, 128],
  "room.storeroom": [512, 512, 256, 128],
  "room.sun_forge": [768, 512, 256, 128],
  "room.cell_bank": [0, 640, 128, 128],
  "room.dart_battery": [128, 640, 128, 128],
  "room.root_ward": [256, 640, 128, 128],
  "room.tanglenet": [384, 640, 128, 128],
  "room.thorn_gun": [512, 640, 128, 128],
  "room.thornwright": [640, 640, 128, 128],
};

const PAINTED_ROOM_IDS: ReadonlySet<string> = new Set(Object.keys(ROOM_CELLS));
const EMPTY_ROOM_IDS: ReadonlySet<string> = new Set();

function drawPaintedRooms(
  batch: TextureBatch,
  atlas: TextureHandle,
  wreckAtlas: TextureHandle | null,
  { view, catalog, clock }: RenderInput,
  layout: Layout,
): void {
  const body = towerBodyLayout(view, layout, clock);
  const daylight = Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  const ambient = view.power.lit ? 0.82 + daylight * 0.18 : 0.46 + daylight * 0.1;

  for (const floor of view.tower.floors) {
    const deck = floorY(body, floor.index) + body.floorH - 3;
    for (const room of floor.rooms) {
      const id = catalog.rooms[room.def]?.id;
      if (!id) continue;
      const source = ROOM_CELLS[id];
      if (!source) continue;

      // Every source pixel represents the same world scale: 128 px per
      // slot in both axes. This preserves painted proportions across
      // one-, two-, and three-slot rooms instead of stretching every
      // cell to the category's procedural body profile.
      const profile = roomProfile(catalog.rooms[room.def]);
      const inset = body.slotW * 0.06;
      const height = (body.floorH - 3) * profile.rise;
      const x = slotX(body, room.slot) + inset;
      const width = room.width * body.slotW - inset * 2;
      if (room.wrecked) {
        if (wreckAtlas) {
          batch.pushAtlas(
            wreckAtlas,
            x,
            deck - height,
            width,
            height,
            source[0],
            source[1],
            source[2],
            source[3],
            [0.68, 0.72, 0.69, 0.86],
          );
          continue;
        }
        // Identity survives destruction as pieces of the authored room,
        // underneath the procedural torn shell and splinters. One-slot
        // rooms split in two; wider rooms in one piece per slot.
        const pieces = Math.max(2, room.width);
        const sourcePiece = source[2] / pieces;
        const widthPiece = width / pieces;
        for (let piece = 0; piece < pieces; piece += 1) {
          const drift = (stableRoomDraw(room.id, piece * 977 + 17) - 0.5) * body.slotW * 0.14;
          const drop = body.slotW * (0.1 + stableRoomDraw(room.id, piece * 1597 + 31) * 0.2);
          batch.pushAtlas(
            atlas,
            x + piece * widthPiece + drift,
            deck - height + drop,
            widthPiece,
            height,
            source[0] + piece * sourcePiece,
            source[1],
            sourcePiece,
            source[3],
            [0.32, 0.34, 0.33, 0.62],
          );
        }
        continue;
      }

      // A switched-off machine must read before its label does: hold the
      // mechanism still in the procedural pass and take the light out of
      // its painted interior here. Starved is quieter but not as dead;
      // backed-up stays bright because its problem is visible abundance.
      // Preserve the authored silhouette even when a room is off or
      // starved. State is carried primarily by motion and the service
      // lamps; crushing the sprite to near-black made every idle room
      // the same unreadable rectangle.
      const state =
        !room.active || !room.powered ? 0.52 : room.stalled && room.stall !== "backedup" ? 0.68 : 1;
      const health = 0.72 + Math.max(0, Math.min(1, room.health_permille / 1000)) * 0.28;
      const level = ambient * state * health;
      batch.pushAtlas(
        atlas,
        x,
        deck - height,
        width,
        height,
        source[0],
        source[1],
        source[2],
        source[3],
        [level * 0.96, level, level * 0.98, 1],
      );
    }
  }
}

/** Put actual goods on the room's benches and shelves instead of filling bars. */
function drawPaintedRoomStock(
  batch: TextureBatch,
  atlas: TextureHandle,
  { view, catalog, clock }: RenderInput,
  layout: Layout,
): void {
  const body = towerBodyLayout(view, layout, clock);
  const daylight = Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  const ambient = view.power.lit ? 0.82 + daylight * 0.18 : 0.5 + daylight * 0.1;

  for (const floor of view.tower.floors) {
    const deck = floorY(body, floor.index) + body.floorH - 3;
    for (const room of floor.rooms) {
      if (room.wrecked) continue;
      const info = catalog.rooms[room.def];
      const profile = roomProfile(info);
      const inset = body.slotW * 0.06;
      const width = room.width * body.slotW - inset * 2;
      const height = (body.floorH - 3) * profile.rise;
      const x = slotX(body, room.slot) + inset;
      const stock = [
        ...room.inputs.map((stack) => ({ ...stack, item: stack.item })),
        ...room.outputs.map((stack) => ({ ...stack, item: stack.item })),
        ...room.shelves
          .filter((shelf) => shelf.item !== null)
          .map((shelf) => ({ ...shelf, item: shelf.item! })),
      ];
      if (stock.length === 0) continue;

      const laneW = width / stock.length;
      for (const [lane, stack] of stock.entries()) {
        if (stack.count <= 0 || stack.max <= 0) continue;
        const itemId = catalog.items[stack.item]?.id;
        const cell = itemId ? ITEM_CELLS[itemId] : undefined;
        if (cell === undefined) continue;

        const fraction = Math.max(0, Math.min(1, stack.count / stack.max));
        // One object says present; two and three make half/full shelves
        // readable as physical abundance without attempting a literal
        // one-sprite-per-unit inventory.
        const objects = Math.max(1, Math.min(3, Math.ceil(fraction * 3)));
        const icon = Math.min(body.slotW * 0.24, height * 0.24, laneW * 0.62);
        const step = icon * 0.42;
        const groupW = icon + step * (objects - 1);
        const left = x + lane * laneW + (laneW - groupW) * 0.5;
        const sourceX = (cell % 4) * 128;
        const sourceY = Math.floor(cell / 4) * 128;
        const state = room.active ? 1 : 0.66;
        const health = 0.78 + Math.max(0, Math.min(1, room.health_permille / 1000)) * 0.22;
        const level = ambient * state * health;

        for (let object = 0; object < objects; object += 1) {
          batch.pushAtlas(
            atlas,
            left + object * step,
            deck - icon - 4 - (object % 2) * icon * 0.08,
            icon,
            icon,
            sourceX,
            sourceY,
            128,
            128,
            [level, level, level * 0.98, 0.98],
          );
        }
      }
    }
  }
}

/** Stable 0..1 cosmetic draw from a room id and salt. */
function stableRoomDraw(id: number, salt: number): number {
  return ((Math.imul(id ^ salt, 2654435761) >>> 0) % 1000) / 1000;
}

function terrainAt(view: ViewSnapshot): number | null {
  for (const band of view.world.bands) {
    if (view.world.distance >= band.start && view.world.distance < band.end) return band.kind;
  }
  return view.world.bands.at(-1)?.kind ?? null;
}

function environmentTexture(art: ArtTextures, terrainId: string | undefined): TextureHandle {
  switch (terrainId) {
    case "terrain.clearing":
      return art.parallaxClearing;
    case "terrain.ruin_field":
      return art.parallaxRuins;
    case "terrain.drowned_street":
      return art.parallaxDrowned;
    case "terrain.salt_flat":
      return art.parallaxCoast;
    default:
      return art.parallaxCanopy;
  }
}

/** Region-specific painted distance, beneath all live terrain and journey signals. */
function drawPaintedEnvironmentBackground(
  batch: TextureBatch,
  art: ArtTextures,
  { view, catalog }: RenderInput,
  layout: Layout,
): void {
  const terrain = terrainAt(view);
  const terrainId = terrain === null ? undefined : catalog.terrain[terrain]?.id;
  const daylight = Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  const level = 0.5 + daylight * 0.5;
  const top = 0;
  const width = Math.max(0, edgeScreenX(view, layout));
  if (width <= 0) return;
  batch.pushCover(environmentTexture(art, terrainId), 0, top, width, layout.viewport.height - top, {
    focusY: terrainId === "terrain.salt_flat" ? 0.38 : 0.58,
    tint: [level * 0.94, level, level * 0.97, 0.92],
  });
}

const DOODAD_BASELINE = 250 / 256;

interface PaintedFeatureGeometry {
  feature: FeatureView;
  kind: string;
  cell: number;
  x: number;
  baseY: number;
  size: number;
}

function doodadScale(kind: string): number {
  switch (kind) {
    case "tree":
      return 1.05;
    case "fern":
      return 0.62;
    case "vine":
      return 0.92;
    case "rock":
      return 0.42;
    case "driftwood":
      return 0.4;
    case "salt_pan":
      return 0.26;
    case "ruin":
      return 0.8;
    case "wreck":
      return 0.55;
    default:
      return 0.7;
  }
}

function paintedFeatures(
  { view, catalog }: RenderInput,
  layout: Layout,
  layer: number,
): PaintedFeatureGeometry[] {
  const edgeX = edgeScreenX(view, layout);
  const found: PaintedFeatureGeometry[] = [];
  for (const feature of view.world.features) {
    if (feature.layer !== layer) continue;
    const terrain = catalog.terrain[feature.band];
    if (!terrain) continue;
    const kind = terrain.feature_kinds[feature.kind] ?? "tree";
    const isRuin = terrain.ruin_kinds[feature.kind] === true;
    // The authored plate already owns distant density. Keep all economic landmarks,
    // but thin decorative repeats so the world does not double-print itself.
    if (!isRuin && layer === 0 && Math.abs(feature.at) % 3 !== 0) continue;
    if (!isRuin && layer === 1 && Math.abs(feature.at) % 2 !== 0) continue;
    const at = featurePoint(view.world.distance, layout, feature);
    const size = at.size * doodadScale(kind);
    if (at.x + size / 2 < 0 || at.x - size / 2 > edgeX) continue;
    found.push({
      feature,
      kind,
      cell: sceneryCell(kind, feature),
      x: at.x,
      baseY: at.y,
      size,
    });
  }
  const ordered: PaintedFeatureGeometry[] = [];
  for (const item of found) {
    const before = ordered.findIndex((other) => {
      const order =
        item.baseY - other.baseY || item.size - other.size || item.feature.at - other.feature.at;
      return order < 0;
    });
    if (before < 0) ordered.push(item);
    else ordered.splice(before, 0, item);
  }
  return ordered;
}

function drawPaintedFeatureLayer(
  batch: TextureBatch,
  art: ArtTextures,
  input: RenderInput,
  layout: Layout,
  layer: number,
): void {
  const daylight = Math.max(0, Math.min(1, input.view.clock.sun_pct / 100));
  const level = 0.5 + daylight * 0.5;
  const depthAlpha = layer === 0 ? 0.48 : layer === 1 ? 0.74 : 0.94;
  for (const item of paintedFeatures(input, layout, layer)) {
    batch.pushAtlas(
      art.terrainDoodads,
      item.x - item.size / 2,
      item.baseY - item.size * DOODAD_BASELINE,
      item.size,
      item.size,
      (item.cell % 4) * 256,
      Math.floor(item.cell / 4) * 256,
      256,
      256,
      [level * 0.94, level, level * 0.97, depthAlpha],
    );
  }
}

/**
 * Let a unique resident haunt the authored landscape before it becomes an actor.
 *
 * It deliberately sits behind the far props and their fog wash. The snapshot only
 * exposes a landmark inside the streamed approach window, so this cannot spoil a
 * whole run or turn into a permanent objective marker.
 */
function drawLandmarkResidentForeshadow(
  batch: TextureBatch,
  art: ArtTextures,
  { view, catalog }: RenderInput,
  layout: Layout,
): void {
  const ahead = view.journey.landmark_ahead;
  if (!ahead || ahead.ahead <= 0) return;
  const waypoint = catalog.waypoints[ahead.def];
  if (!waypoint?.landmark || waypoint.resident === null) return;
  const resident = catalog.enemies[waypoint.resident];
  if (!resident) return;
  const cell = creatureArtCell(resident.id);
  if (cell === undefined) return;

  const warning = Math.max(1, catalog.landmark_warning_paces);
  const approach = Math.max(0, Math.min(1, 1 - ahead.ahead / warning));
  const depth = layout.groundY - layout.horizonY;
  const height = layout.floorH * (1.72 + approach * 0.42);
  const width = height / 2;
  const x = layout.viewport.width * (0.74 - approach * 0.11);
  const baseline = layout.horizonY + depth * (0.26 + approach * 0.08);
  const daylight = Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  const level = 0.29 + daylight * 0.2;
  const alpha = 0.07 + approach * 0.13;

  batch.pushAtlas(
    art.creatures,
    x - width / 2,
    baseline - height * (174 / 256),
    width,
    height,
    cell * 128,
    0,
    128,
    256,
    [level * 0.76, level * 0.9, level * 0.79, alpha],
  );
}

function drawPaintedFeatureShadows(
  batch: QuadBatch,
  input: RenderInput,
  layout: Layout,
  layer: number,
): void {
  if (layer === 0) return;
  const alpha = layer === 1 ? 0.16 : 0.28;
  for (const item of paintedFeatures(input, layout, layer)) {
    if (item.kind === "vine" || item.kind === "salt_pan") continue;
    const width = item.size * (layer === 1 ? 0.58 : 0.7);
    const height = Math.max(2, item.size * (layer === 1 ? 0.018 : 0.026));
    batch.push(
      item.x - width / 2,
      item.baseY - height * 0.55,
      width,
      height,
      [0.035, 0.065, 0.055, alpha],
      { radius: height * 0.5, softness: height * 0.8 },
    );
  }
}

function drawPaintedDepthFog(
  batch: QuadBatch,
  { view }: RenderInput,
  layout: Layout,
  layer: number,
): void {
  const dark = 1 - Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  const haze = atNight(palette.haze, dark);
  const depth = layout.groundY - layout.horizonY;
  const top = Math.max(0, layout.horizonY - depth * 0.1);
  const fadeIn = depth * 0.2;
  const height = depth * 0.72;
  const topAlpha = layer === 0 ? 0.3 : 0.13;
  const width = edgeScreenX(view, layout);
  batch.push(0, top, width, fadeIn, fade(haze, 0), {
    colorBottom: fade(haze, topAlpha),
  });
  batch.push(0, top + fadeIn, width, height, fade(haze, topAlpha), {
    colorBottom: fade(haze, layer === 0 ? 0.08 : 0.025),
  });
}

/** Paint one authored route beat while geometry and live state stay renderer-owned. */
function drawPaintedWaypoint(
  batch: TextureBatch,
  texture: TextureHandle,
  { view, clock }: RenderInput,
  beat: NonNullable<ReturnType<typeof beatGeometry>>,
  cell: WaypointAtlasRect,
): void {
  if (beat.near <= 0.04) return;
  const dark = Math.max(0, Math.min(1, 1 - view.clock.sun_pct / 100));
  const level = 0.56 + (1 - dark) * 0.44;
  const live = beat.here ? 1 + Math.sin(clock * 2) * 0.06 : 1;
  // A beat is scenery with a choice attached, not a foreground actor. Keep
  // the authored vignette comfortably below the walker's room-cell scale so
  // it announces the place without intruding into the hull silhouette.
  const width = beat.size * 2.05 * live;
  const height = beat.size * 1.54 * live;
  // Every packed source sits on y=184 in its 192px cell. Register that
  // shared baseline to the live ground rather than centring the vignette.
  batch.pushAtlas(
    texture,
    beat.x - width / 2,
    beat.y - height * (184 / 192),
    width,
    height,
    ...cell,
    [level * 0.93, level, level * 0.96, 0.96],
  );
}

/** A subdued, slumped echo bridges the simulation removing a taken beat. */
function drawWaypointAftermath(
  batch: TextureBatch,
  texture: TextureHandle,
  aftermath: WaypointAftermath,
  { catalog, clock }: RenderInput,
): void {
  const id = catalog.waypoints[aftermath.def]?.id;
  const cell = waypointArtCell(id);
  if (!cell) return;
  const age = Math.max(0, Math.min(1, (clock - aftermath.started) / 1.5));
  const width = aftermath.size * 2.05 * (1 - age * 0.06);
  const height = aftermath.size * 1.54 * (1 - age * 0.1);
  batch.pushAtlas(
    texture,
    aftermath.x - width / 2 + age * aftermath.size * 0.12,
    aftermath.y - height * (184 / 192) + age * aftermath.size * 0.14,
    width,
    height,
    ...cell,
    [0.48, 0.46, 0.4, (1 - age) * 0.78],
    age * 0.055,
  );
}

function drawPaintedArt(
  batch: TextureBatch,
  art: ArtTextures,
  { view, catalog, clock }: RenderInput,
  layout: Layout,
): void {
  const daylight = Math.max(0, Math.min(1, view.clock.sun_pct / 100));
  const level = 0.55 + daylight * 0.45;
  const lit: Color = [level * 0.94, level, level * 0.96, 0.94];
  for (const member of view.crew) {
    // The static standing fallback would erase these need-state signals. Their
    // authored-pixel derived rows exist only in the motion atlas, so retain the
    // stronger procedural poses if that optional texture did not load.
    if ((member.state === "eat" || member.state === "sleep") && !art.crewMotion) continue;
    const cell = crewArtCell(member.name, member.id);
    const at = crewPosition(layout, member);
    const height = layout.floorH * 0.82;
    const animated = art.crewMotion !== null;
    const width = animated ? height * (128 / 192) : height / 3;
    const pose = crewMotionPose(member.state, member.carrying !== null);
    const locomoting = member.state === "walk" || member.state === "climb";
    const row = crewMotionRow(
      pose,
      locomoting,
      view.tick,
      member.floor,
      member.slot,
      member.fidget,
    );
    batch.pushAtlas(
      art.crewMotion ?? art.crew,
      at.x - width / 2,
      at.y - height * (animated ? 184 / 192 : 0.92),
      width,
      height,
      cell * 128,
      animated ? row * 192 : 0,
      128,
      animated ? 192 : 384,
      lit,
    );
  }

  for (const enemy of view.siege.enemies) {
    const animated = art.creatureMotion !== null;
    // The standing atlas has no truthful transition poses; let the procedural
    // bloom and silhouette carry those states when motion art is absent.
    if (!animated && (enemy.state === "dying" || enemy.state === "leaving")) continue;
    const info = catalog.enemies[enemy.def];
    if (!info) continue;
    const cell = creatureArtCell(info.id);
    if (cell === undefined) continue;
    const at = enemyPosition(view, layout, info.approach, enemy);
    if (at.x < -80 || at.x > layout.viewport.width + 80) continue;
    const scatter = ((Math.imul(enemy.id, 2654435761) >>> 0) % 1000) / 1000;
    const spanX = view.tower.floors[0]?.slots ?? 10;
    const facing = at.x > layout.originX + spanX * layout.slotW * 0.5 ? -1 : 1;
    const lunge =
      !animated && enemy.state === "attack"
        ? -facing * Math.max(0, Math.sin(clock * 6 + scatter * 7)) * layout.slotW * 0.07
        : 0;
    const row = animated
      ? creatureMotionRow(enemy.state, enemy.hp_permille, view.tick, enemy.id)
      : 0;
    const height = layout.floorH * 1.55 * at.scale;
    const width = height / 2;
    batch.pushAtlas(
      art.creatureMotion ?? art.creatures,
      at.x + lunge - width / 2,
      at.y - height * 0.68,
      width,
      height,
      cell * 128,
      row * 256,
      128,
      256,
      [lit[0], lit[1], lit[2], Math.max(0.35, enemy.hp_permille / 1000)],
    );
  }

  // The two light touches that make procedural geometry and painted sprites belong
  // to one page. Kept quiet so room stalls, stress, and lamp state retain contrast.
  batch.pushFullscreenCover(art.wash, layout.viewport.width, layout.viewport.height, {
    tint: [1, 1, 1, 0.06],
  });
  batch.pushFullscreenRepeat(art.paper, layout.viewport.width, layout.viewport.height, {
    tileWidth: 512,
    tileHeight: 512,
    tint: [1, 1, 1, 0.085],
  });
  batch.pushFullscreenCover(art.vignette, layout.viewport.width, layout.viewport.height, {
    tint: [1, 1, 1, 0.58],
  });
}

function combatFamily(effect: CombatEffect, impact: boolean): CombatFxFamily {
  if (impact) return "creature_impact";
  switch (effect) {
    case "thorn":
      return "thorn_projectile";
    case "dart":
      return "dart_projectile";
    case "tanglenet":
      return "tanglenet_cast";
    case "lantern":
      return "lantern_pulse";
    case "root_ward":
      return "root_ward_pulse";
    case "resonance":
      return "resonance_wave";
    case "cutter":
    case "generic":
      return "mechanical_puff";
  }
}

/** Short event-owned trajectories. No wall-clock randomness and no simulated state. */
function drawCombatVisuals(
  batch: TextureBatch,
  atlas: TextureHandle,
  visuals: readonly CombatVisual[],
  clock: number,
  layout: Layout,
): void {
  for (const visual of visuals) {
    const age = Math.max(0, Math.min(1, (clock - visual.started) / 0.58));
    const travel = Math.min(1, age * 1.65);
    const x = visual.x0 + (visual.x1 - visual.x0) * travel;
    const y = visual.y0 + (visual.y1 - visual.y0) * travel;
    const dx = visual.x1 - visual.x0;
    const dy = visual.y1 - visual.y0;
    const angle = Math.atan2(dy, dx);
    const pulse = visual.effect === "lantern" || visual.effect === "resonance";
    const ground = visual.effect === "root_ward";
    const projectileSize = pulse
      ? layout.slotW * 0.78
      : ground
        ? layout.slotW * 0.62
        : layout.slotW * 0.42;
    const flightCell = combatFxCell(combatFamily(visual.effect, false));
    batch.pushAtlas(
      atlas,
      x - projectileSize * 0.5,
      y - projectileSize * 0.5,
      projectileSize,
      projectileSize,
      ...flightCell,
      [1, 1, 1, Math.max(0, 1 - age * 0.55)],
      angle,
    );
    if (age < 0.5) continue;
    const hitAge = (age - 0.5) * 2;
    const impactSize = layout.slotW * (0.36 + hitAge * 0.38);
    batch.pushAtlas(
      atlas,
      visual.x1 - impactSize * 0.5,
      visual.y1 - impactSize * 0.5,
      impactSize,
      impactSize,
      ...combatFxCell(combatFamily(visual.effect, true)),
      [1, 1, 1, 1 - hitAge],
    );
  }
}

function sceneryCell(kind: string, feature: FeatureView): number {
  const variant = Math.abs(feature.at) % 4;
  switch (kind) {
    case "tree":
      return variant % 2;
    case "fern":
      return 2;
    case "vine":
      return 3;
    case "rock":
      return variant % 2 === 0 ? 4 : 7;
    case "driftwood":
      return 5;
    case "salt_pan":
      return 6;
    case "ruin": {
      const family = Math.abs(feature.at) % 3;
      return 8 + family * 2 + (feature.salvage > 0 ? 0 : 1);
    }
    case "wreck":
      return feature.salvage > 0 ? 14 : 15;
    default:
      return 7;
  }
}

/**
 * The text layer. Kept deliberately sparse: the cross-section carries
 * state through fill levels and colour, and numbers are a precision
 * layer on top, not the primary read (`DECISIONS.md` §8).
 */
/**
 * Why a room is quiet, in the tower's own words.
 *
 * **Starved and backed up are the pair this exists for.** They looked
 * identical from outside — a quiet room — and they are answered by
 * opposite actions: feed the first, spend from the second. Four cutter
 * arms standing quiet because nothing wants more bamboo read exactly
 * like four arms on bare ground (`SYSTEMS.md` §6.26).
 */
const WHY_QUIET: Record<StallTag, string> = {
  wrecked: "wrecked, and needs mending before it works again",
  off: "switched off",
  unpowered: "waiting for its circuit",
  unvented: "waiting for the shared flue",
  shaded: "in the tower's own shadow",
  unarmed: "nothing on the rack",
  unfuelled: "nothing to burn",
  starved: "waiting on something nobody has brought",
  backedup: "nowhere to put what it makes",
};

function buildLabels(
  view: ViewSnapshot,
  catalog: CatalogSnapshot,
  layout: Layout,
  clock: number,
  picked: readonly number[],
  toggleRoomPower: (floor: number, slot: number, active: boolean) => void,
): Label[] {
  const labels: Label[] = [];
  // Labels are pieces of the walker now, not a stationary HUD laid over it.
  // Following the restrained cosmetic bob keeps every plate bolted to the
  // beam it names while simulation and picking remain on the stable layout.
  const bodyLayout = towerBodyLayout(view, layout, clock);

  labels.push(...forkLabels(view, catalog, layout, edgeScreenX(view, layout)));
  labels.push(...enclaveLabel(view, catalog, layout));
  labels.push(...beatLabel(view, catalog, layout));
  labels.push(...salvageLabels(view, catalog, layout));

  for (const floor of view.tower.floors) {
    const y = floorY(bodyLayout, floor.index);
    labels.push({
      key: `floor-${floor.index}`,
      text: `F${floor.index}`,
      x: bodyLayout.originX + 10,
      y: y + bodyLayout.floorH - 5,
      variant: "label-floor",
    });

    for (const room of floor.rooms) {
      const info = catalog.rooms[room.def];
      if (!info) continue;
      const cx = slotX(bodyLayout, room.slot) + (room.width * bodyLayout.slotW) / 2;
      // A wreck outranks a stall: one of them will start again on its
      // own and the other needs poles and somebody's time.
      const wear = room.wrecked
        ? " label-wrecked"
        : room.stall === "unpowered"
          ? " label-unpowered"
          : room.stall === "unvented"
            ? " label-unvented"
            : room.stall === "backedup"
              ? " label-backedup"
              : room.stalled
                ? " label-stalled"
                : !room.active
                  ? " label-inactive"
                  : " label-active";
      labels.push({
        key: `room-${room.id}`,
        text: info.short,
        x: cx,
        y: y + 14,
        variant: `label-room${wear}`,
        // **Which silence this is** (`SYSTEMS.md` §6.26). The room still
        // just goes quiet — that is the signal and it is unchanged. What
        // the hover adds is the difference between "nobody has brought
        // me anything" and "nobody wants what I make", which are
        // answered by opposite actions and looked identical.
        title: room.stall ? `${info.name} — ${WHY_QUIET[room.stall]}` : info.name,
      });
      if (info.burner || info.power_draw > 0) {
        const next = !room.active;
        labels.push({
          key: `room-breaker-${room.id}`,
          text: room.active ? "I" : "O",
          x: slotX(bodyLayout, room.slot) + room.width * bodyLayout.slotW - 10,
          y: y + 21,
          variant: `room-breaker ${room.active ? "breaker-on" : "breaker-off"}`,
          title: room.active ? `Open ${info.name}'s breaker` : `Close ${info.name}'s breaker`,
          control: {
            ariaLabel: `${room.active ? "Switch off" : "Switch on"} ${info.name}, floor ${floor.index}`,
            pressed: room.active,
            onActivate: () => toggleRoomPower(floor.index, room.slot, next),
          },
        });
      }
    }
  }

  for (const member of view.crew) {
    // The animated body, carried stock and work pose are the ordinary
    // activity read. A nameplate appears only when the player has picked
    // somebody out, or when stress makes finding them urgent.
    if (!picked.includes(member.id) && !member.stressed) continue;
    const feet = crewPosition(bodyLayout, member);
    labels.push({
      key: `crew-${member.id}`,
      text: member.name,
      x: feet.x,
      y: feet.y - bodyLayout.floorH * 0.48,
      variant: member.stressed
        ? "label-crew label-stressed label-crew-revealed"
        : "label-crew label-crew-revealed",
    });
  }

  // Creatures get their glyph and nothing else. The silhouette says
  // which of the three approaches it is; the glyph is what separates
  // two creatures that come the same way. A name over each one would
  // turn a wave into a list.
  for (const enemy of view.siege.enemies) {
    if (enemy.state === "dying" || enemy.state === "leaving") continue;
    const info = catalog.enemies[enemy.def];
    if (!info) continue;
    const { x, y, scale } = enemyPosition(view, layout, info.approach, enemy);
    if (x < 0 || x > layout.viewport.width) continue;
    // Nothing at all for the far half of the approach, fading in over
    // the near half. A 14px glyph over a ten-pixel silhouette is bigger
    // than the creature it names, and a row of them strung along the
    // horizon reads as a list of the wave rather than as a wave. By the
    // time it is fully in, the creature is inside a battery's reach and
    // telling it from its neighbour is worth something. Faded rather
    // than switched, because a glyph that pops on is a thing the eye
    // reports as an event.
    const near = Math.max(0, Math.min(1, (scale - 0.5) / 0.25));
    if (near <= 0) continue;
    labels.push({
      key: `enemy-${enemy.id}`,
      text: info.glyph,
      x,
      // Rides the silhouette rather than a fixed floor height, so it
      // stays on the creature as the approach scales that down.
      y: y - layout.floorH * 0.45 * scale,
      variant: "label-creature",
      alpha: Math.max(0.4, enemy.hp_permille / 1000) * near,
    });
  }

  return labels;
}

/**
 * The two ways, named on the ground.
 *
 * The fork card in the chrome is where the choice is made, but the
 * choice itself belongs to the strip: the tower is standing in front of
 * a split with two ways out of it, and both of them have names. Without
 * this the card is a dialogue box about something happening off screen
 * (`SYSTEMS.md` §3.3).
 */
/**
 * The settlement's name, once it is more than a speck.
 *
 * A place people live gets named the way a branch does — and for the
 * same reason the branch names are drawn from the catalog rather than
 * authored beside the geometry: a name that can drift out of step with
 * what it names is worse than no name.
 */
/**
 * The beat's name, once it is near enough to read.
 *
 * Later than the settlement's, because a beat is small and its name is
 * bigger than it is until it is close. Brighter alongside: the card is
 * open then, and the name is what ties the card to the thing.
 */
function beatLabel(view: ViewSnapshot, catalog: CatalogSnapshot, layout: Layout): Label[] {
  const beat = beatGeometry(view, layout);
  if (!beat || beat.near <= 0.35) return [];
  const info = catalog.waypoints[beat.def];
  if (!info) return [];
  return [
    {
      key: "beat-name",
      text: info.name,
      // **Beside it on the ground, not above it.** A beat alongside the
      // tower sits at the tower's own leading flank — that is what
      // "alongside" means — so a caption two sizes up lands inside the
      // cross-section, over the rooms. Under the object and out to the
      // right is the only place that is clear at every distance.
      x: beat.x + beat.size * 1.6,
      y: beat.y + beat.size * 0.7,
      variant: beat.here ? "label-beat label-beat-here" : "label-beat",
      alpha: beat.here ? 1 : (beat.near - 0.35) / 0.65,
    },
  ];
}

function enclaveLabel(view: ViewSnapshot, catalog: CatalogSnapshot, layout: Layout): Label[] {
  const place = enclaveGeometry(view, layout);
  if (!place || place.near <= 0.15) return [];
  const at = view.journey.enclave_at;
  const name = at === null ? undefined : catalog.regions[at]?.enclave?.name;
  if (name === undefined) return [];
  return [
    {
      key: "enclave-name",
      text: name,
      x: place.x,
      y: place.y - place.roof * 1.9,
      variant: place.berthed ? "label-place label-place-berthed" : "label-place",
      alpha: place.near,
    },
  ];
}

function forkLabels(
  view: ViewSnapshot,
  catalog: CatalogSnapshot,
  layout: Layout,
  edgeX: number,
): Label[] {
  const fork = view.journey.fork;
  const geometry = forkGeometry(view, layout, edgeX);
  if (!fork || !geometry) return [];
  // Nothing at all while it is a speck at the vanishing point, fading
  // in as it comes. Same treatment the creature glyphs get, for the
  // same reason: a name over a four-pixel post is bigger than the post.
  if (geometry.near <= 0) return [];

  return fork.branches.flatMap((branchIdx, side) => {
    const info = catalog.branches[branchIdx];
    const tip = geometry.tips[side];
    if (!info || !tip) return [];
    const chosen = fork.answer === side;
    // On the track once there is one, tucked beside the post before
    // then — the name has to be attached to something either way.
    const x = geometry.open ? tip.x : geometry.x + geometry.post * (0.8 + side * 0.1);
    const y = geometry.open
      ? tip.y - geometry.post * 0.3
      : geometry.y - geometry.post * (side === 0 ? 1.0 : 0.7);
    return [
      {
        key: `fork-${side}`,
        text: info.name,
        x,
        y,
        variant: chosen ? "label-way label-way-taken" : "label-way",
        alpha: geometry.near * (chosen ? 1 : 0.8),
      },
    ];
  });
}

/**
 * What a ruin holds, for the ones the tower could actually stop at.
 *
 * The heap on the strip is the primary read and carries across the
 * frame; this is the precision layer on top of it, and it is gated
 * hard — only ruins near enough to be a live decision get a figure, so
 * the horizon never turns into a list of numbers (`DECISIONS.md` §8).
 */
function salvageLabels(view: ViewSnapshot, catalog: CatalogSnapshot, layout: Layout): Label[] {
  const glyph = catalog.items.find((item) => item.id === "item.scrap")?.glyph ?? "";
  const shape = towerShape(view);
  const towerLeft = layout.originX;
  const towerRight = layout.originX + shape.slots * layout.slotW;
  const labels: Label[] = [];

  for (const feature of view.world.features) {
    if (feature.salvage <= 0) continue;
    const gap = feature.at - view.world.distance;
    // A berth is decided at the last moment, and this is that moment.
    if (Math.abs(gap) > 90) continue;
    const { x, y, size } = featurePoint(view.world.distance, layout, feature);
    // Nothing behind the tower's own body. The heap is hidden there, so
    // a figure floating over the cross-section would be a number with
    // nothing to attach to — which is exactly the dashboard reading
    // `DECISIONS.md` §8 rules out.
    if (x < 0 || x > layout.viewport.width) continue;
    if (x > towerLeft - 12 && x < towerRight + 12) continue;
    labels.push({
      key: `salvage-${feature.at}-${feature.layer}`,
      text: `${glyph}${feature.salvage}`,
      // Riding the ruin's own height rather than a fixed baseline, so
      // it stays on the thing it is describing at every parallax depth.
      x,
      y: y - size * 0.62,
      variant: "label-salvage",
      alpha: Math.max(0.4, 1 - Math.abs(gap) / 90),
    });
  }
  return labels;
}
