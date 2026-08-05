/**
 * Owns the GL context, the batch, and the label overlay.
 *
 * Stateless between frames by design: `render` takes a snapshot and
 * draws it, start to finish. There is no scene graph to keep in sync
 * and no dirty tracking to get wrong, which at this entity count costs
 * nothing and removes an entire category of bug.
 */

import { LabelLayer, type Label } from "./LabelLayer";
import { QuadBatch } from "./QuadBatch";
import { createContext, resizeToDisplay } from "./gl";
import { clampZoom, computeLayout, hitSlot, slotX, floorY, type Layout } from "./layout";
import {
  drawScene,
  edgeScreenX,
  enemyPosition,
  featurePoint,
  forkGeometry,
  towerShape,
  type PlaceMode,
} from "./scene";
import type { CatalogSnapshot, FeatureView, ViewSnapshot } from "../bridge/types";

export interface RenderInput {
  view: ViewSnapshot;
  catalog: CatalogSnapshot;
  placeMode: PlaceMode | null;
  clock: number;
}

export class Renderer {
  private readonly canvas: HTMLCanvasElement;
  private readonly gl: WebGL2RenderingContext;
  private readonly batch: QuadBatch;
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

  constructor(canvas: HTMLCanvasElement, labelRoot: HTMLElement) {
    this.canvas = canvas;
    this.gl = createContext(canvas);
    this.batch = new QuadBatch(this.gl);
    this.labels = new LabelLayer(labelRoot);
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

    const gl = this.gl;
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.clearColor(0, 0, 0, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);

    this.batch.begin();
    drawScene(this.batch, {
      view,
      catalog,
      layout,
      placeMode: input.placeMode,
      clock: input.clock,
    });
    this.batch.flush(viewport.width, viewport.height);

    this.labels.sync(buildLabels(view, catalog, layout));
  }

  /**
   * Slot under a client-space point, or null if outside the tower.
   * Uses the layout from the last rendered frame, which is what the
   * player was actually looking at when they clicked.
   */
  /**
   * The creature under this point, if any.
   *
   * Generous by design — twenty-odd pixels of slack around a small,
   * moving thing. Missing is worse than being approximate here: the
   * player is clicking during a wave, and a mis-click that clears the
   * focus costs them the order they just gave.
   */
  pickEnemy(clientX: number, clientY: number, catalog: CatalogSnapshot): number | null {
    if (!this.layout || !this.view) return null;
    const rect = this.canvas.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    let best: { id: number; d: number } | null = null;
    for (const enemy of this.view.siege.enemies) {
      if (enemy.state === "dying" || enemy.state === "leaving") continue;
      const info = catalog.enemies[enemy.def];
      if (!info) continue;
      const at = enemyPosition(this.view, this.layout, info.approach, enemy);
      const d = Math.hypot(at.x - x, at.y - y);
      if (d < 26 && (!best || d < best.d)) best = { id: enemy.id, d };
    }
    return best?.id ?? null;
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
    return this.batch.queued;
  }

  dispose(): void {
    this.batch.dispose();
    this.labels.dispose();
  }
}

/**
 * The text layer. Kept deliberately sparse: the cross-section carries
 * state through fill levels and colour, and numbers are a precision
 * layer on top, not the primary read (`DECISIONS.md` §8).
 */
function buildLabels(view: ViewSnapshot, catalog: CatalogSnapshot, layout: Layout): Label[] {
  const labels: Label[] = [];

  labels.push(...forkLabels(view, catalog, layout, edgeScreenX(view, layout)));
  labels.push(...salvageLabels(view, catalog, layout));

  for (const floor of view.tower.floors) {
    const y = floorY(layout, floor.index);
    labels.push({
      key: `floor-${floor.index}`,
      text: `F${floor.index}`,
      x: layout.originX - 14,
      y: y + layout.floorH - 4,
      variant: "label-floor",
    });

    for (const room of floor.rooms) {
      const info = catalog.rooms[room.def];
      if (!info) continue;
      const cx = slotX(layout, room.slot) + (room.width * layout.slotW) / 2;
      // A wreck outranks a stall: one of them will start again on its
      // own and the other needs poles and somebody's time.
      const wear = room.wrecked ? " label-wrecked" : room.stalled ? " label-stalled" : "";
      labels.push({
        key: `room-${room.id}`,
        text: info.short,
        x: cx,
        y: y + layout.floorH * 0.52,
        variant: `label-room${wear}`,
      });

      // One number per room: whatever it is accumulating. More than
      // that turns the cross-section into a spreadsheet.
      const headline = room.outputs[0] ?? room.inputs[0];
      if (headline) {
        const item = catalog.items[headline.item];
        labels.push({
          key: `room-count-${room.id}`,
          text: `${item?.glyph ?? ""}${headline.count}`,
          x: cx,
          y: y + layout.floorH * 0.78,
          variant: "label-count",
        });
      } else if (room.shelves.length > 0) {
        const total = room.shelves.reduce((sum, shelf) => sum + shelf.count, 0);
        labels.push({
          key: `room-count-${room.id}`,
          text: String(total),
          x: cx,
          y: y + layout.floorH * 0.78,
          variant: "label-count",
        });
      }
    }
  }

  for (const member of view.crew) {
    labels.push({
      key: `crew-${member.id}`,
      text: member.name,
      x: layout.originX + (member.slot + 0.5) * layout.slotW,
      y: layout.groundY - member.floor * layout.floorH - layout.floorH * 0.42,
      variant: member.stressed ? "label-crew label-stressed" : "label-crew",
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
