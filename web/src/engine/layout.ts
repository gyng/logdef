/**
 * Where everything sits on screen.
 *
 * One module owns the mapping from simulation coordinates — floor
 * index, slot index, paces walked — to pixels, because the renderer and
 * the click handler have to agree exactly. Any disagreement shows up as
 * "I clicked the slot and it placed the room one over", which is the
 * kind of bug that eats an afternoon.
 */

export interface Viewport {
  width: number;
  height: number;
}

export interface Layout {
  /** Pixels per horizontal slot. */
  slotW: number;
  /** Pixels per floor. */
  floorH: number;
  /** Left edge of slot 0. */
  originX: number;
  /** Screen y of the tower's ground line (the base of floor 0). */
  groundY: number;
  /**
   * Screen y where the ground plane meets the sky. Well above
   * `groundY`, because the tower is standing on ground that recedes
   * into the distance behind it — that gap is where the far and mid
   * parallax layers live.
   */
  horizonY: number;
  /** Pixels per pace of world travel. */
  paceW: number;
  viewport: Viewport;
}

/** Width of the tower in slots. Comes from the content pack. */
export interface TowerShape {
  slots: number;
  floors: number;
}

const MIN_SLOT_W = 26;
const MAX_SLOT_W = 86;
/** Floors are taller than slots are wide; the tower should read tall. */
const FLOOR_ASPECT = 1.35;
/** Screen height fraction where the tower's feet rest. */
const GROUND_FRACTION = 0.86;
/** Screen height fraction where the ground plane meets the sky. */
const HORIZON_FRACTION = 0.52;
/** Fraction of the viewport width the tower's left edge sits at. */
const TOWER_LEFT_FRACTION = 0.28;

/**
 * Fit the tower to the viewport. Height is the binding constraint —
 * the whole point of the cozy 8–14 floor cap is that the tower always
 * fits on one screen, so the layout scales to keep that true rather
 * than letting the player scroll.
 */
export function computeLayout(viewport: Viewport, shape: TowerShape): Layout {
  const usableHeight = viewport.height * GROUND_FRACTION;
  // Always reserve room for a couple of floors beyond the current top,
  // so building upward doesn't make the whole tower jump in scale.
  const plannedFloors = Math.max(shape.floors + 2, 6);

  const byHeight = usableHeight / (plannedFloors * FLOOR_ASPECT);
  const byWidth = (viewport.width * 0.4) / Math.max(shape.slots, 1);
  const slotW = Math.max(MIN_SLOT_W, Math.min(MAX_SLOT_W, Math.min(byHeight, byWidth)));

  return {
    slotW,
    floorH: slotW * FLOOR_ASPECT,
    originX: Math.round(viewport.width * TOWER_LEFT_FRACTION),
    groundY: Math.round(viewport.height * GROUND_FRACTION),
    horizonY: Math.round(viewport.height * HORIZON_FRACTION),
    // Terrain scrolls a touch faster than the tower is wide, so the
    // stride reads as real movement rather than a treadmill.
    paceW: slotW * 0.5,
    viewport,
  };
}

/** Left edge of a slot. */
export function slotX(layout: Layout, slot: number): number {
  return layout.originX + slot * layout.slotW;
}

/** Top edge of a floor. Floor 0 sits directly on the ground line. */
export function floorY(layout: Layout, floor: number): number {
  return layout.groundY - (floor + 1) * layout.floorH;
}

/** Screen x of a world position, given how far the tower has walked. */
export function worldX(layout: Layout, at: number, distance: number, parallax: number): number {
  return layout.originX + (at - distance) * layout.paceW * parallax;
}

/**
 * Which slot a screen point falls in, or null if it is outside the
 * tower. Floors are counted from the ground up, so the arithmetic is
 * inverted relative to the y axis.
 */
export function hitSlot(
  layout: Layout,
  shape: TowerShape,
  x: number,
  y: number,
): { floor: number; slot: number } | null {
  const slot = Math.floor((x - layout.originX) / layout.slotW);
  if (slot < 0 || slot >= shape.slots) return null;

  const floor = Math.floor((layout.groundY - y) / layout.floorH);
  if (floor < 0 || floor >= shape.floors) return null;

  return { floor, slot };
}
