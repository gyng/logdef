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
// **0.72 rather than 0.86, and the extra 14% of frame is leg room.**
//
// Free, which is why it is here: `slotW` is bound by `byWidth` (80 at
// 1600×900) not by `byHeight` (95.6), so the tower can be given a
// quarter of the frame to stand in without getting any smaller. 0.72 is
// the exact floor — below it `byHeight` binds and the tower shrinks.
//
// What it buys is a leg that can reach. At 0.86 the leg spanned 91 px
// and could swing ±42, so a planted foot covered two paces of ground and
// the gait had to run at nearly nine steps a second to keep up with the
// scroll. At 0.72 the span is 181 px and the swing ±83. See
// `STRIDE_SLOTS` in `scene.ts` for the other half of that sum.
const GROUND_FRACTION = 0.72;
/** Screen height fraction where the ground plane meets the sky. */
// Lowered with the ground line, so the gap between them — where the far
// and mid parallax layers live — keeps its depth. Left at 0.52 it would
// have compressed by 40% and the distance would have gone flat.
const HORIZON_FRACTION = 0.4;
/** Fraction of the viewport width the tower's left edge sits at. */
const TOWER_LEFT_FRACTION = 0.28;

/**
 * How far in and out the player may zoom.
 *
 * **The fit is still the fit.** `computeLayout` sizes the tower to the
 * frame first and this multiplies the result, so the default view is
 * unchanged and zoom is a lens over it rather than a second layout.
 *
 * Out to 0.55 because a fourteen-floor tower with a shaft on the far
 * column is a lot of cross-section and being able to see all of it at
 * once is the point of the cozy cap; in to 2.4 because at two floors —
 * which is where every run now starts (`SYSTEMS.md` §6.11) — the
 * default fit leaves a very small tower on a very large screen, and the
 * first five minutes are the ones a player most needs to be able to
 * read.
 */
export const MIN_ZOOM = 0.55;
export const MAX_ZOOM = 2.4;

/** Clamp a zoom factor to what the layout will honour. */
export function clampZoom(zoom: number): number {
  if (!Number.isFinite(zoom)) return 1;
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
}

/**
 * Fit the tower to the viewport. Height is the binding constraint —
 * the whole point of the cozy 8–14 floor cap is that the tower always
 * fits on one screen, so the layout scales to keep that true rather
 * than letting the player scroll.
 */
export function computeLayout(viewport: Viewport, shape: TowerShape, zoom = 1): Layout {
  const usableHeight = viewport.height * GROUND_FRACTION;
  // Always reserve room for a couple of floors beyond the current top,
  // so building upward doesn't make the whole tower jump in scale.
  const plannedFloors = Math.max(shape.floors + 2, 6);

  const byHeight = usableHeight / (plannedFloors * FLOOR_ASPECT);
  const byWidth = (viewport.width * 0.4) / Math.max(shape.slots, 1);
  const fitted = Math.max(MIN_SLOT_W, Math.min(MAX_SLOT_W, Math.min(byHeight, byWidth)));
  // **Applied after the clamp, not inside it.** `MIN_SLOT_W` and
  // `MAX_SLOT_W` exist to keep the *fitted* tower legible on very small
  // and very large screens; folding zoom in before them would make the
  // control do nothing at either end of that range, which reads as a
  // broken button rather than as a considered limit.
  const slotW = fitted * clampZoom(zoom);

  return {
    slotW,
    floorH: slotW * FLOOR_ASPECT,
    originX: Math.round(viewport.width * TOWER_LEFT_FRACTION),
    groundY: Math.round(viewport.height * GROUND_FRACTION),
    horizonY: Math.round(viewport.height * HORIZON_FRACTION),
    // **The scroll rate, and it is the gait's other half.**
    //
    // A planted foot has to cover exactly the ground that goes past, so
    // this and the leg length together decide the cadence — there is no
    // third number to hide a mismatch in. At `slotW * 0.5` a tower with
    // 181 px legs still had to take 4.3 steps a second; at 0.2 it takes
    // 1.7, which is the ponderous stride something this size should
    // have. Terrain crosses the screen in about 5.6 s rather than 2.2.
    //
    // Raise this and the tower scuttles. Lower it and it wades.
    paceW: slotW * 0.2,
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
