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
import { computeLayout, hitSlot, slotX, floorY, type Layout } from "./layout";
import { drawScene, towerShape, type PlaceMode } from "./scene";
import type { CatalogSnapshot, ViewSnapshot } from "../bridge/types";

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
  private shape: { slots: number; floors: number } | null = null;

  constructor(canvas: HTMLCanvasElement, labelRoot: HTMLElement) {
    this.canvas = canvas;
    this.gl = createContext(canvas);
    this.batch = new QuadBatch(this.gl);
    this.labels = new LabelLayer(labelRoot);
  }

  render(input: RenderInput): void {
    const { view, catalog } = input;
    resizeToDisplay(this.canvas);

    const viewport = { width: this.canvas.clientWidth, height: this.canvas.clientHeight };
    if (viewport.width === 0 || viewport.height === 0) return;

    const shape = towerShape(view);
    const layout = computeLayout(viewport, shape);
    this.layout = layout;
    this.shape = shape;

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
      labels.push({
        key: `room-${room.id}`,
        text: info.short,
        x: cx,
        y: y + layout.floorH * 0.52,
        variant: room.stalled ? "label-room label-stalled" : "label-room",
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

  return labels;
}
