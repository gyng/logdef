/**
 * Turns one `ViewSnapshot` into a list of quads, back to front.
 *
 * Pure function of (snapshot, catalog, layout, place mode, clock). It
 * reads nothing, mutates nothing, and holds no state between frames —
 * which means a rendering bug is always reproducible from a snapshot,
 * and the whole scene can be rebuilt from scratch every frame without
 * anyone having to track dirty regions.
 *
 * Draw order is painter's algorithm: sky, far terrain, mid terrain,
 * ground, near terrain, the tower, then overlays.
 */

import type { QuadBatch } from "./QuadBatch";
import type { Color } from "./QuadBatch";
import { atNight, creatureColor, fade, mix, palette, roomColor, terrainColors } from "./palette";
import { floorY, slotX, worldX, type Layout } from "./layout";
import type {
  CarView,
  CatalogSnapshot,
  CrewView,
  EnemyView,
  FeatureView,
  FloorView,
  RoomView,
  ShaftView,
  TerrainInfo,
  ViewSnapshot,
} from "../bridge/types";

/**
 * A pending placement the player is aiming at a slot.
 *
 * Rooms and shafts share this because they share the interaction: pick
 * a thing, hover the tower, see where it fits, click. The only real
 * difference is that a shaft is one slot wide and many floors tall.
 */
export interface PlaceMode {
  kind: "room" | "shaft";
  /** Content id of the room or shaft being placed. */
  id: string;
  width: number;
  maxFloor: number | null;
  /** Floors a shaft will span upward from the clicked floor. */
  span: number;
  /** Slot the cursor is currently over, if it is over the tower. */
  hover: { floor: number; slot: number } | null;
}

export interface SceneContext {
  view: ViewSnapshot;
  catalog: CatalogSnapshot;
  layout: Layout;
  placeMode: PlaceMode | null;
  /** Wall-clock seconds since load. Cosmetic wobble only. */
  clock: number;
}

/**
 * How dark it is outside, 0 to 1, from the sun before terrain.
 *
 * Deliberately the raw sun rather than exposure: standing under thick
 * canopy costs you charge, but it does not make it night. Conflating
 * the two would have the sky go black every time the tower walks into
 * shade.
 */
function darkness(view: ViewSnapshot): number {
  return Math.max(0, Math.min(1, 1 - view.clock.sun_pct / 100));
}

/** Clamp to 0..1. Per-mille from the snapshot goes through here. */
function unit(value: number): number {
  return Math.max(0, Math.min(1, value));
}

/**
 * A stable 0..1 draw from an integer id.
 *
 * Creatures need a place on the tower's face and shafts need a point
 * to break at, and both have to stay put between frames — the renderer
 * keeps no state, so the id is the only thing available to derive them
 * from. `Math.imul` rather than a plain multiply because ids run to
 * 32 bits and the float product would lose the low ones.
 */
function hash01(id: number): number {
  return ((Math.imul(id, 2654435761) >>> 0) % 1000) / 1000;
}

export function drawScene(batch: QuadBatch, ctx: SceneContext): void {
  drawSky(batch, ctx);
  drawTerrain(batch, ctx);
  drawJourneyEdge(batch, ctx);
  drawFork(batch, ctx);
  drawLegs(batch, ctx);
  drawTower(batch, ctx);
  drawShafts(batch, ctx);
  drawCrew(batch, ctx);
  drawSiege(batch, ctx);
  drawPlaceMode(batch, ctx);
  drawVignette(batch, ctx);
}

// ---------------------------------------------------------------------------
// Sky and terrain
// ---------------------------------------------------------------------------

function drawSky(batch: QuadBatch, ctx: SceneContext): void {
  const { layout, view } = ctx;
  const { width } = layout.viewport;
  const horizon = layout.horizonY;
  const dark = darkness(view);

  // Two stops rather than one: humid teal overhead falling to a warm
  // haze at the treeline. A single gradient across the whole frame
  // washes out, which is exactly what the first pass looked like.
  const high = mix(palette.skyHigh, palette.nightHigh, dark);
  const mid = mix(palette.skyMid, palette.nightMid, dark);
  const low = mix(palette.skyLow, palette.nightLow, dark);

  batch.push(0, 0, width, horizon * 0.62, high, { colorBottom: mid });
  batch.push(0, horizon * 0.62 - 1, width, horizon - horizon * 0.62 + 2, mid, {
    colorBottom: low,
  });

  // The sun tracks the actual time of day, so the light source and the
  // charge income are visibly the same fact.
  const dayFraction = view.clock.permille / 1000;
  const radius = Math.min(width, layout.viewport.height) * 0.075;
  const arc = Math.sin(Math.max(0, Math.min(1, (dayFraction - 0.08) / 0.78)) * Math.PI);
  const sunX = width * (0.12 + dayFraction * 0.78);
  const sunY = horizon * (1.05 - arc * 0.85);

  if (view.clock.sun_pct > 0) {
    const brightness = 0.25 + (view.clock.sun_pct / 100) * 0.4;
    batch.push(
      sunX - radius,
      sunY - radius,
      radius * 2,
      radius * 2,
      fade(palette.sunlight, brightness),
      { radius, softness: radius * 1.1 },
    );
  } else {
    // A moon on the opposite arc, so a night sky is not simply empty.
    const moonX = width * (0.9 - dayFraction * 0.78);
    batch.push(
      moonX - radius * 0.5,
      horizon * 0.3 - radius * 0.5,
      radius,
      radius,
      fade(palette.moonlight, 0.5),
      { radius: radius * 0.5, softness: radius * 0.6 },
    );
  }
}

function drawTerrain(batch: QuadBatch, ctx: SceneContext): void {
  const { view, catalog, layout } = ctx;
  const { width, height } = layout.viewport;
  const distance = view.world.distance;
  const horizon = layout.horizonY;
  const depth = layout.groundY - horizon;
  const dark = darkness(view);
  // Where the generated world stops, if it stops inside the frame. Past
  // it there is nothing to draw because there is nothing decided —
  // `drawJourneyEdge` takes over from here.
  const edgeX = edgeScreenX(ctx);

  // A base plate under the whole strip in the colour of the band
  // underfoot. The per-band quads paint over it; this is only here so a
  // gap between the last band's linear end and the edge — which opens
  // up the moment the tower halts at a fork — reads as ground rather
  // than as a hole onto the sky.
  const underfoot = terrainColors(catalog.terrain[view.world.band ?? -1]?.id ?? "");
  batch.push(0, horizon, edgeX, height - horizon, atNight(mix(underfoot.far, palette.haze, 0.45), dark), {
    colorBottom: atNight(mix(underfoot.near, palette.ground, 0.75), dark),
  });

  // The ground plane, running from the horizon to the bottom of the
  // frame, coloured by the band it belongs to. Perspective is faked
  // with a gradient: hazy where it meets the sky, saturated underfoot.
  for (const band of view.world.bands) {
    const terrain = catalog.terrain[band.kind];
    if (!terrain) continue;
    const colors = terrainColors(terrain.id);
    const x1 = worldX(layout, band.start, distance, 1);
    const x2 = Math.min(worldX(layout, band.end, distance, 1), edgeX);
    if (x2 < -60 || x1 > width + 60) continue;
    batch.push(
      x1,
      horizon,
      x2 - x1,
      height - horizon,
      atNight(mix(colors.far, palette.haze, 0.45), dark),
      { colorBottom: atNight(mix(colors.near, palette.ground, 0.75), dark) },
    );
  }

  // Mist pooling where the canopy meets the sky. Sells the distance
  // more cheaply than any amount of extra geometry.
  const mist = atNight(palette.haze, dark);
  batch.push(0, horizon - depth * 0.14, width, depth * 0.3, fade(mist, 0.55), {
    colorBottom: fade(mist, 0),
    softness: 2,
  });

  // Three parallax layers between the horizon and the tower's feet.
  // Far is small, hazy and slow; near is large, saturated, and moves
  // with the tower. The depth cue is what makes the stride read as
  // travel rather than as a scrolling backdrop.
  // `toward` pushes the layer's colour: distant growth washes out into
  // the haze, near growth sinks toward silhouette. Without the second
  // half of that, every layer lands on the same value and the frame
  // reads as one flat wash.
  // Scales stay under 1 so nothing in the middle distance out-tops the
  // tower — it is the largest thing in the frame, and it should read
  // that way even in dense canopy.
  const layers = [
    { parallax: 0.22, tint: palette.haze, blend: 0.55, scale: 0.5, base: horizon + depth * 0.05 },
    { parallax: 0.55, tint: palette.haze, blend: 0.2, scale: 0.72, base: horizon + depth * 0.55 },
    {
      parallax: 1.0,
      tint: palette.vignette,
      blend: 0.45,
      scale: 0.95,
      base: layout.groundY + depth * 0.3,
    },
  ];

  for (const [layerIndex, layer] of layers.entries()) {
    for (const feature of view.world.features) {
      if (feature.layer !== layerIndex) continue;
      const terrain = catalog.terrain[feature.band];
      if (!terrain) continue;
      const x = worldX(layout, feature.at, distance, layer.parallax);
      if (x < -140 || x > Math.min(width, edgeX) + 140) continue;
      const colors = terrainColors(terrain.id);
      const base = layerIndex === 0 ? colors.far : colors.near;
      const tint = atNight(mix(base, layer.tint, layer.blend), dark);
      const size = depth * layer.scale * (0.45 + (feature.scale / 255) * 0.7);
      drawFeature(batch, terrain, feature, x, layer.base, size, tint, ctx.clock, dark);
    }
  }

  // The strip of ground the tower actually stands on, so its feet have
  // somewhere to land rather than floating over the parallax.
  const ground = atNight(palette.ground, dark);
  batch.push(0, layout.groundY, edgeX, height - layout.groundY, fade(ground, 0.55), {
    colorBottom: ground,
  });
  batch.push(0, layout.groundY - 2, edgeX, 4, fade(atNight(palette.groundLip, dark), 0.7));
}

function drawFeature(
  batch: QuadBatch,
  terrain: TerrainInfo,
  feature: FeatureView,
  x: number,
  baseY: number,
  size: number,
  tint: Color,
  clock: number,
  dark: number,
): void {
  const kind = terrain.feature_kinds[feature.kind] ?? "tree";
  const seed = feature.at;
  // A slow sway, phase-shifted per feature so the canopy breathes
  // instead of pulsing in unison.
  const sway = Math.sin(clock * 0.6 + seed * 0.11) * 0.02;

  // Deterministic per-feature variation, so no two neighbours are
  // identical but a given feature never changes between frames.
  const wobble = (salt: number) => ((seed * 2654435761 + salt * 40503) % 1000) / 1000;

  switch (kind) {
    case "tree": {
      const lean = (wobble(1) - 0.5) * 0.18;
      const trunkH = size * (0.55 + wobble(2) * 0.35);
      const trunkW = Math.max(1.5, size * 0.055);
      const bark = mix(tint, palette.towerShell, 0.45);
      const topX = x + lean * size + sway * size * 2.2;

      batch.pushLine(x, baseY, topX, baseY - trunkH, trunkW, bark);
      // Two boughs, so the silhouette has structure rather than being
      // a lollipop on a stick.
      batch.pushLine(
        topX,
        baseY - trunkH * 0.72,
        topX - size * 0.2,
        baseY - trunkH * 0.95,
        trunkW * 0.6,
        bark,
      );
      batch.pushLine(
        topX,
        baseY - trunkH * 0.82,
        topX + size * 0.22,
        baseY - trunkH,
        trunkW * 0.6,
        bark,
      );

      // Canopy: overlapping ellipses, widest and darkest low down,
      // brighter toward the light at the crown.
      const clumps = [
        { dx: -0.26, dy: 0.02, w: 0.52, h: 0.3, light: 0 },
        { dx: 0.24, dy: 0.06, w: 0.5, h: 0.28, light: 0.05 },
        { dx: -0.04, dy: -0.16, w: 0.62, h: 0.36, light: 0.14 },
        { dx: 0.08, dy: -0.34, w: 0.4, h: 0.26, light: 0.24 },
      ];
      for (const clump of clumps) {
        const cw = size * clump.w;
        const ch = size * clump.h;
        batch.push(
          topX + clump.dx * size - cw / 2 + sway * size * 3,
          baseY - trunkH + clump.dy * size - ch / 2,
          cw,
          ch,
          mix(tint, palette.sunlight, clump.light * 0.35),
          { radius: ch * 0.5, softness: 0.6 },
        );
      }
      break;
    }
    case "fern": {
      // A fan of arcing fronds from a single crown at ground level.
      const fronds = 6;
      const bright = mix(tint, palette.sunlight, 0.1);
      for (let i = 0; i < fronds; i += 1) {
        const t = i / (fronds - 1);
        const angle = (t - 0.5) * 2.1;
        const reach = size * (0.34 + (1 - Math.abs(t - 0.5) * 2) * 0.24);
        const tipX = x + Math.sin(angle) * reach + sway * size * 1.5;
        const tipY = baseY - Math.cos(angle) * reach * 0.85;
        // Two segments give the frond a droop.
        const midX = x + Math.sin(angle) * reach * 0.55;
        const midY = baseY - Math.cos(angle) * reach * 0.62;
        batch.pushLine(x, baseY, midX, midY, Math.max(1.5, size * 0.045), tint);
        batch.pushLine(midX, midY, tipX, tipY, Math.max(1, size * 0.03), bright);
      }
      break;
    }
    case "vine": {
      // Hangs from above rather than growing up. Leaves along its
      // length, drifting with the sway.
      const top = baseY - size * 1.5;
      const drop = size * 1.2;
      const tipX = x + sway * size * 5;
      batch.pushLine(x, top, tipX, top + drop, Math.max(1.2, size * 0.035), tint);
      for (let i = 1; i <= 4; i += 1) {
        const t = i / 5;
        const lx = x + (tipX - x) * t;
        const ly = top + drop * t;
        const leaf = size * 0.1;
        batch.push(lx - (i % 2 === 0 ? leaf : 0), ly, leaf, leaf * 0.55, tint, {
          radius: leaf * 0.3,
        });
      }
      break;
    }
    case "rock": {
      // Two overlapping boulders, the smaller tucked in front.
      const w = size * (0.34 + wobble(3) * 0.2);
      const h = w * 0.62;
      const stone = mix(tint, palette.ruinNear, 0.35);
      batch.push(x - w / 2, baseY - h, w, h, stone, { radius: h * 0.4 });
      batch.push(
        x - w * 0.1,
        baseY - h * 0.55,
        w * 0.6,
        h * 0.55,
        mix(stone, palette.sunlight, 0.08),
        { radius: h * 0.25 },
      );
      break;
    }
    case "ruin": {
      // A broken wall of the old world, drowned in green: a tall
      // fragment, a shorter one beside it, and growth on every ledge.
      //
      // From M3 a ruin is also a decision. `salvage` is what it still
      // holds, and the difference between a ruin worth berthing at and
      // one already picked over is the whole basis of that decision —
      // so it is drawn as the thing itself, warm worked metal stacked
      // against the wall, and not as a badge (`SYSTEMS.md` §3.4).
      const isRuin = terrain.ruin_kinds[feature.kind] === true;
      const spent = isRuin && feature.salvage <= 0;
      const stone = mix(mix(tint, palette.ruinFar, 0.4), palette.stripped, spent ? 0.55 : 0);
      const shade = mix(stone, palette.vignette, 0.22);
      const unitW = size * 0.18;
      // A stripped ruin has been climbed over and pulled apart. It
      // slumps, so "nothing left here" is legible from the silhouette
      // before the colour has resolved.
      const slump = spent ? 0.62 : 1;
      const tallH = size * (0.55 + wobble(4) * 0.5) * slump;
      const shortH = tallH * (0.35 + wobble(5) * 0.3);

      batch.push(x - unitW * 1.3, baseY - tallH, unitW * 1.2, tallH, stone, {
        colorBottom: shade,
        radius: 1,
      });
      batch.push(x + unitW * 0.2, baseY - shortH, unitW * 1.5, shortH, shade, { radius: 1 });
      // A window the jungle now looks through.
      batch.push(
        x - unitW * 1.0,
        baseY - tallH * 0.72,
        unitW * 0.55,
        tallH * 0.24,
        mix(shade, palette.vignette, 0.5),
        { radius: 1 },
      );
      // Growth reclaiming the ledges — the reason it's called a ruin
      // field and not rubble. A stripped one gets more of it: what the
      // tower did not take, the green did.
      const moss = mix(palette.canopyFar, tint, spent ? 0.15 : 0.35);
      batch.push(x - unitW * 1.4, baseY - tallH - size * 0.03, unitW * 1.4, size * 0.05, moss, {
        radius: size * 0.025,
      });
      batch.push(x + unitW * 0.1, baseY - shortH - size * 0.025, unitW * 1.7, size * 0.045, moss, {
        radius: size * 0.02,
      });
      batch.pushLine(
        x - unitW * 0.8,
        baseY - tallH,
        x - unitW * 0.8 + sway * size * 3,
        baseY - tallH * 0.4,
        Math.max(1, size * 0.02),
        moss,
      );
      if (isRuin && feature.salvage > 0) {
        drawSalvage(batch, x, baseY, size, feature.salvage, tallH, clock, dark, wobble);
      }
      break;
    }
    default:
      batch.push(x - size * 0.1, baseY - size * 0.2, size * 0.2, size * 0.2, tint, { radius: 3 });
  }
}

/**
 * What a ruin still holds, stacked against it.
 *
 * The one thing on the terrain strip drawn from an economic quantity
 * rather than from scenery, and the whole basis of the decision to
 * stop: §3.2's ruin-richness roll only means anything if a lean city
 * and a generous one look different from across the frame. So the heap
 * grows with `salvage`, and the glint on it is deliberately *not* run
 * through the layer's haze tint — the same exception the creature eye
 * takes, for the same reason. Warm against cold is the only cue that
 * survives being washed toward the mist at the far parallax layer, and
 * a ruin the tower could berth at has to be legible at the distance the
 * player first sees it, not once it is underfoot.
 */
function drawSalvage(
  batch: QuadBatch,
  x: number,
  baseY: number,
  size: number,
  salvage: number,
  wallH: number,
  clock: number,
  dark: number,
  wobble: (salt: number) => number,
): void {
  // Against the authored ceiling: 60 units before a region's richness
  // roll, which tops out at 140% (`assets/data/terrain/*.ron`). A ruin
  // at the very top of both rolls fills the heap and no further.
  const load = unit(salvage / 84);
  const metal = atNight(palette.salvage, dark * 0.55);
  const lit = atNight(palette.salvageLit, dark * 0.4);

  // The heap itself: wider and taller the more is in it.
  const heapW = size * (0.16 + load * 0.34);
  const heapH = size * (0.05 + load * 0.19);
  batch.push(x - heapW * 0.35, baseY - heapH, heapW, heapH, lit, {
    colorBottom: metal,
    radius: heapH * 0.4,
  });

  // Plate and beam leaning on the wall. One piece per twenty units, so
  // counting them is a rough read of what is in there without a number
  // ever being printed.
  const pieces = Math.min(4, 1 + Math.floor(load * 4));
  for (let i = 0; i < pieces; i += 1) {
    const lean = 0.2 + wobble(20 + i) * 0.45;
    const reach = Math.min(wallH * 0.75, size * (0.16 + load * 0.3));
    const footX = x - heapW * 0.2 + i * heapW * 0.26;
    batch.pushLine(
      footX,
      baseY,
      footX - reach * lean,
      baseY - reach,
      Math.max(1.2, size * 0.035),
      i % 2 === 0 ? metal : lit,
    );
  }

  // And a glint off it, breathing slowly. Small, but it is the part
  // that carries across the frame.
  const shine = 0.45 + Math.sin(clock * 0.9 + x * 0.05) * 0.2;
  const glow = size * (0.09 + load * 0.14);
  batch.push(x - glow * 0.5, baseY - heapH - glow * 0.5, glow, glow, fade(lit, shine), {
    radius: glow * 0.5,
    softness: glow * 0.9,
  });
}

// ---------------------------------------------------------------------------
// The journey ahead
// ---------------------------------------------------------------------------

/**
 * The furthest ahead the world exists, in paces.
 *
 * `stream_ahead_paces` in `assets/data/balance.ron` — the generator
 * produces terrain this far in front of the tower and no further, so it
 * is also how much warning a fork gives: fifty seconds at 1×
 * (`SYSTEMS.md` §3.3).
 */
const HORIZON_PACES = 900;

/**
 * Where the approach stops compressing, in paces. Larger than the
 * creature approach's knee because a fork is a decision made minutes
 * out rather than a fight decided in the last thirty paces — the useful
 * resolution is "it is coming" long before it is "it is here".
 */
const HORIZON_KNEE = 45;

/** Where the world stops, if it stops inside the frame. */
export interface JourneyEdge {
  /** `fork` is undecided ground; `end` is the far edge of the journey. */
  kind: "fork" | "end";
  ahead: number;
}

/**
 * Whether the generated world runs out in front of the tower.
 *
 * Two things stop the generator, and they are the same event as far as
 * the strip is concerned: an unanswered fork, past which the palette
 * depends on an answer that does not exist, and the far edge of the
 * last region, past which there is no more journey. Both have to be
 * drawn, because otherwise the frame simply ends in raw sky, and an
 * answered fork must not — generation resumes the moment a branch is
 * picked (`SYSTEMS.md` §3.3).
 */
export function journeyEdge(view: ViewSnapshot): JourneyEdge | null {
  const fork = view.journey.fork;
  if (fork && fork.answer === null) return { kind: "fork", ahead: fork.ahead };
  if (view.journey.remaining < HORIZON_PACES) {
    return { kind: "end", ahead: view.journey.remaining };
  }
  return null;
}

/**
 * A point on the ground plane `ahead` paces in front of the tower.
 *
 * The same logarithmic compression the creature approach uses, and for
 * the same reason: laid out at the scene's own `paceW` the ground in
 * front of the tower is under thirty paces wide, so a fork nine hundred
 * paces out would sit thirty-six thousand pixels off the right edge and
 * then cross the whole frame in a second and a half. Compressed, it
 * crests at the vanishing point and walks in — which is what "you can
 * see it coming, and you have the whole streaming window to answer" has
 * to look like.
 *
 * Exported because the label layer has to land on exactly the same spot
 * the quads do.
 */
export function aheadPoint(
  view: ViewSnapshot,
  layout: Layout,
  ahead: number,
): { x: number; y: number; far: number } {
  const spanX = towerShape(view).slots * layout.slotW;
  const far = unit(
    Math.log1p(Math.max(0, ahead) / HORIZON_KNEE) / Math.log1p(HORIZON_PACES / HORIZON_KNEE),
  );
  const flankX = layout.originX + spanX;
  const margin = layout.slotW * 0.4;
  const corridor = Math.max(0, layout.viewport.width - flankX - margin);
  const depth = layout.groundY - layout.horizonY;
  const farGroundY = layout.horizonY + depth * 0.06;
  const groundLine = layout.groundY + depth * 0.14;
  return { x: flankX + corridor * far, y: groundLine + (farGroundY - groundLine) * far, far };
}

/**
 * Screen x the drawn ground has to stop at.
 *
 * Deliberately the compressed position rather than the literal one.
 * `worldX` puts the tower's own distance at its *left* edge, so a tower
 * halted at a fork would have the void opening under its own feet; the
 * compressed edge keeps it at the leading flank, where the ground the
 * tower has not walked on yet actually is.
 */
function edgeScreenX(ctx: SceneContext): number {
  const edge = journeyEdge(ctx.view);
  if (!edge) return ctx.layout.viewport.width;
  return Math.min(ctx.layout.viewport.width, aheadPoint(ctx.view, ctx.layout, edge.ahead).x);
}

/**
 * What is past the end of the world.
 *
 * An unanswered fork dissolves into mist: the way ahead has not been
 * decided, and drawing it as undecided is the literal truth rather than
 * a metaphor. The far edge of the journey does the opposite — it opens
 * out into light, because standing still there is the run being over
 * and not a decision pending, and those two must never read the same
 * (`SYSTEMS.md` §3.3).
 */
function drawJourneyEdge(batch: QuadBatch, ctx: SceneContext): void {
  const edge = journeyEdge(ctx.view);
  if (!edge) return;

  const { layout, view } = ctx;
  const { width, height } = layout.viewport;
  const x = edgeScreenX(ctx);
  if (x >= width) return;

  const dark = darkness(view);
  const horizon = layout.horizonY;
  const wash = edge.kind === "fork" ? palette.undecided : palette.farEdge;
  const body = atNight(wash, dark);

  // Everything past the edge, thinning downward so the ground does not
  // simply stop at a hard line.
  batch.push(x, horizon, width - x, height - horizon, fade(body, 0.92), {
    colorBottom: fade(body, edge.kind === "fork" ? 0.72 : 0.85),
  });
  // A soft shoulder on the edge itself, so it reads as ground giving
  // way to distance rather than as a cut.
  batch.push(x - layout.slotW * 0.5, horizon, layout.slotW, height - horizon, fade(body, 0), {
    colorBottom: fade(body, 0),
    softness: layout.slotW * 0.5,
  });
  batch.push(x - layout.slotW * 0.6, horizon, layout.slotW * 1.2, height - horizon, fade(body, 0.5), {
    softness: layout.slotW * 0.6,
  });

  if (edge.kind === "end") {
    // The far edge gets a low band of light along the ground: nothing
    // beyond, and nothing threatening about that.
    batch.push(x, layout.groundY - height * 0.06, width - x, height * 0.12, fade(palette.sunlight, 0.3), {
      softness: height * 0.05,
    });
  }
}

/**
 * The split, and the two ways out of it.
 *
 * Drawn whether or not it has been answered, because the answer stays
 * changeable until the tower crosses. What changes with the answer is
 * which way is lit: the chosen track carries the tower's own lamplight
 * and the other one goes back to being scenery.
 *
 * The tracks are coloured from `catalog.branches[i].terrain[0]` — the
 * heaviest kind in that branch's own palette — so the two ways are
 * literally painted the colour of what they are made of, and the
 * picture cannot drift out of step with the data during tuning the way
 * an authored blurb would.
 */
function drawFork(batch: QuadBatch, ctx: SceneContext): void {
  const { view, catalog, layout, clock } = ctx;
  const fork = view.journey.fork;
  if (!fork) return;

  const { x, y, far } = aheadPoint(view, layout, fork.ahead);
  if (x > layout.viewport.width + 40) return;
  const dark = darkness(view);
  // Same curve the creature approach uses for the same job: a thing at
  // the vanishing point is small, and a thing at your feet is not.
  const scale = 0.3 + 0.7 * (1 - far);
  const post = layout.slotW * 0.9 * scale;
  const waiting = view.journey.halt === "fork";

  // The two tracks, fanning out toward the horizon. Drawn from the
  // fork rather than from the tower, so they read as the ways *out* of
  // the decision rather than as roads the tower is already on.
  const reach = Math.max(post * 2.2, (layout.groundY - layout.horizonY) * 0.55 * (0.4 + far * 0.9));
  for (const [side, branchIdx] of fork.branches.entries()) {
    const info = catalog.branches[branchIdx];
    const terrain = info ? catalog.terrain[info.terrain[0] ?? -1] : undefined;
    const colors = terrainColors(terrain?.id ?? "");
    const chosen = fork.answer === side;
    const lift = side === 0 ? -0.55 : 0.12;
    const tone = atNight(mix(colors.near, colors.far, chosen ? 0.55 : 0.2), dark);
    batch.pushLine(
      x,
      y,
      x + reach * (0.85 + side * 0.15),
      y + reach * lift,
      Math.max(2, post * (chosen ? 0.34 : 0.24)),
      fade(tone, chosen ? 0.95 : 0.5),
    );
    if (chosen) {
      // Lamplight running down the way the tower has been told to
      // take. The commitment is visible on the ground, not just on a
      // card in the corner.
      batch.pushLine(
        x,
        y,
        x + reach * (0.85 + side * 0.15),
        y + reach * lift,
        Math.max(1, post * 0.1),
        fade(palette.lamplight, 0.4),
      );
    }
  }

  // The waypost. Two boards on a leaning stake — the only man-made
  // thing on the strip that is not the tower.
  const timber = atNight(palette.wayPost, dark);
  const board = atNight(palette.wayBoard, dark * 0.6);
  batch.pushLine(x, y, x, y - post, Math.max(1.5, post * 0.12), timber);
  for (const side of [0, 1]) {
    const chosen = fork.answer === side;
    const armY = y - post * (side === 0 ? 0.92 : 0.66);
    batch.push(x, armY - post * 0.09, post * 0.62, post * 0.18, fade(board, chosen ? 1 : 0.6), {
      radius: post * 0.05,
      rotation: side === 0 ? -0.22 : 0.14,
    });
  }

  // Waiting for you, and unmistakably so. A tower halted at a fork and
  // a tower the player parked are the same silhouette with the same
  // still legs, so the difference has to be somewhere — and the right
  // somewhere is the thing that is actually waiting, lit like a lamp
  // left on for you rather than flashed like an alarm.
  if (waiting) {
    const pulse = 0.3 + Math.abs(Math.sin(clock * 1.5)) * 0.35;
    const halo = post * 2.4;
    batch.push(x - halo / 2, y - post * 1.1 - halo * 0.35, halo, halo, fade(palette.lamplight, pulse * 0.5), {
      radius: halo * 0.5,
      softness: halo * 0.6,
    });
    batch.push(x - post * 0.09, y - post * 1.12, post * 0.18, post * 0.18, fade(palette.sunlight, 0.85), {
      radius: post * 0.09,
      softness: post * 0.12,
    });
  }
}

// ---------------------------------------------------------------------------
// Creatures
// ---------------------------------------------------------------------------

/**
 * How much of the approach is spent fanning out onto the tower.
 *
 * Everything in contact reports the tower's own position, so without
 * this a whole wave would stack on one pixel at the tower's left edge.
 * Spreading them over the last few paces also makes the arrival read
 * as a wave settling in rather than as a pop.
 */
const ARRIVAL_PACES = 8;

/**
 * The furthest ahead this frame can see, in paces.
 *
 * Taken from the simulation rather than guessed at: `spawn_paces_ahead`
 * in `assets/data/balance.ron` is 540, and `siege.rs` scatters arrivals
 * over another quarter of that again, so 675 is the furthest out a
 * creature is ever placed. Anything beyond it parks at the vanishing
 * point instead of being dropped, so the number can drift in data
 * without creatures winking into existence in mid-air.
 */
const SIGHT_PACES = 675;

/**
 * Where the approach stops compressing and starts opening out, in
 * paces. Half a dart battery's reach (`range_paces` 60), which lands
 * that whole reach in the near 35% of the corridor: the stretch where
 * the fight is actually decided gets a third of the screen to itself,
 * and the four hundred paces behind it share the rest.
 */
const SIGHT_KNEE = 30;

/**
 * How small a creature gets at the limit of sight.
 *
 * Not the tenth or so true perspective would give it — a three-pixel
 * silhouette is not a creature, it is a dust mote, and the player is
 * meant to be able to count what is coming. The far parallax layer
 * makes the same trade (`drawTerrain` runs it at half scale rather than
 * at its true depth) for the same reason: the frame is a diagram of the
 * tower's situation before it is a photograph.
 */
const FAR_SCALE = 0.32;

/**
 * How far out a creature is, 0 at the tower and 1 at the limit of
 * sight. The single number the whole approach is drawn from.
 *
 * Laid out at the scene's own `paceW` the approach does not fit, and
 * not by a little. Measured on a 1600-wide frame: `paceW` is 40 pixels,
 * so of the 540 paces a creature spawns out at, the last 29 were the
 * only ones on screen — under a second at 1x, and half that of it clear
 * of the tower's own silhouette. A dart battery reaches sixty paces, so
 * it opened fire on something a screen and a half off camera and spent
 * the whole fight there. What the player saw was a counter going up.
 *
 * So distance ahead is compressed logarithmically, which is the one
 * curve where every doubling of the remaining distance is worth the
 * same amount of screen. Far out that folds hundreds of paces into a
 * narrow band under the horizon, where the eye could not have told 500
 * from 400 anyway; close in it opens out, so the last thirty paces are
 * worth as much travel as the sixty behind them, and those as much as
 * the hundred and twenty behind those. Closing therefore reads as
 * accelerating, which is what closing looks like.
 *
 * A linear mapping wide enough to hold 540 paces would do the opposite:
 * a creature pinned near the frame edge for half a minute and then
 * covering the part that matters in three frames.
 */
function approachDepth(gap: number): number {
  return unit(Math.log1p(Math.abs(gap) / SIGHT_KNEE) / Math.log1p(SIGHT_PACES / SIGHT_KNEE));
}

/** Everything one creature is drawn out of, resolved once per creature. */
interface CreatureSkin {
  body: Color;
  lit: Color;
  /** The part that differs by approach: seams, membranes, mandibles. */
  accent: Color;
  eye: Color;
  /** Earth turned over by a borer. Unused by the other two. */
  spoil: Color;
  /** Animation phase, so no two of them move in lockstep. */
  phase: number;
  /** -1 when the tower is to its left. */
  facing: number;
}

/**
 * Where a creature stands, how big it is at that distance, and how far
 * through its approach it is.
 *
 * Exported because the label layer has to land on exactly the same
 * spot the quads do — a glyph floating away from its own silhouette
 * would be worse than no glyph at all. Every kind arrives somewhere
 * different, and that difference is the whole reason the three
 * approaches exist as separate things.
 */
export function enemyPosition(
  view: ViewSnapshot,
  layout: Layout,
  approach: string,
  enemy: EnemyView,
): { x: number; y: number; arrived: number; scale: number } {
  const shape = towerShape(view);
  const spanX = shape.slots * layout.slotW;
  const depth = layout.groundY - layout.horizonY;
  const reach = layout.viewport.height - layout.groundY;
  const roofY = floorY(layout, shape.floors - 1);
  // A shade below the tower's feet, matching the near parallax layer
  // the biggest growth stands on.
  const groundLine = layout.groundY + depth * 0.14;

  const gap = enemy.at - view.world.distance;
  const arrived = unit(1 - Math.abs(gap) / ARRIVAL_PACES);
  const scatter = hash01(enemy.id);
  const far = approachDepth(gap);

  // Ahead is to the right, because that is the way the tower walks:
  // `worldX` puts a larger world position further right, so the terrain
  // slides leftward underneath it. So a creature closing from ahead
  // comes in off the leading flank, and one the tower has already
  // shaken off falls away past the trailing one — the same compression
  // either way, run out from whichever flank it is on into whatever
  // room the frame has left on that side.
  const ahead = gap >= 0;
  const flankX = ahead ? layout.originX + spanX : layout.originX;
  const margin = layout.slotW * 0.6;
  const corridor = Math.max(0, (ahead ? layout.viewport.width - flankX : flankX) - margin);
  const roamX = ahead ? flankX + corridor * far : flankX - corridor * far;

  // The ground recedes to the horizon, so anything walking in on it
  // travels a straight line to a vanishing point — x and y both come
  // off `far`, which is what keeps the approach reading as one plane
  // going away rather than as a creature sliding in sideways. The far
  // end sits just under the horizon, where the far parallax layer roots
  // its growth.
  const farGroundY = layout.horizonY + depth * 0.06;
  const moundY = groundLine + reach * 0.1;

  let stationX: number;
  let stationY: number;
  let roamY: number;
  switch (approach) {
    case "Canopy": {
      stationX = layout.originX + spanX * (0.1 + scatter * 0.8);
      stationY = roofY;
      // Coming in over the treetops and settling onto the roof. Not on
      // the ground plane, so not on the ground's straight line either:
      // a leaper crosses at height and then stoops, which is why the
      // descent is eased rather than linear in depth. The descent is
      // the mechanic — height is exposure — so it wants to happen where
      // it can be watched instead of being spread thin over the whole
      // crossing. Cruise altitude is a fraction of the sky above the
      // roof rather than a fixed drop, because a fourteen-floor tower
      // leaves very little of it and a leaper pinned off the top of the
      // frame is a leaper nobody sees coming.
      const cruiseY = roofY - Math.max(0, roofY - depth * 0.08) * 0.55;
      roamY = roofY + (cruiseY - roofY) * Math.sqrt(far);
      break;
    }
    case "Burrow":
      stationX = layout.originX + spanX * (0.16 + scatter * 0.7);
      stationY = layout.groundY + reach * 0.45;
      roamY = moundY + (farGroundY - moundY) * far;
      break;
    default:
      // Ground creatures work on the skin, so they gather on the
      // leading flank rather than milling about under the middle.
      stationX = layout.originX + spanX * (0.58 + scatter * 0.55);
      stationY = layout.groundY;
      roamY = groundLine + (farGroundY - groundLine) * far;
      break;
  }

  return {
    x: roamX + (stationX - roamX) * arrived,
    y: roamY + (stationY - roamY) * arrived,
    arrived,
    scale: FAR_SCALE + (1 - FAR_SCALE) * (1 - far),
  };
}

/**
 * Creatures, standing on the same ground the terrain features do.
 *
 * Drawn after the tower rather than woven into the parallax layers: a
 * creature working on the outboard panel has to be visible from the
 * front, and at this entity count the depth error costs far less than
 * losing the read would. Readability over beauty.
 */
function drawSiege(batch: QuadBatch, ctx: SceneContext): void {
  const { view, catalog, layout, clock } = ctx;
  if (view.siege.enemies.length === 0) return;

  const dark = darkness(view);
  const spanX = towerShape(view).slots * layout.slotW;
  const spoil = atNight(palette.spoil, dark);

  for (const enemy of view.siege.enemies) {
    const approach = catalog.enemies[enemy.def]?.approach ?? "Ground";
    const scatter = hash01(enemy.id);
    const { x: stood, y, arrived, scale } = enemyPosition(view, layout, approach, enemy);
    let x = stood;
    if (x < -160 || x > layout.viewport.width + 160) continue;

    const facing = x > layout.originX + spanX * 0.5 ? -1 : 1;
    // A bite is a short lunge toward whatever it is working on. Half a
    // sine, so it jabs rather than sways.
    if (enemy.state === "attack") {
      x -= facing * Math.max(0, Math.sin(clock * 6 + scatter * 7)) * layout.slotW * 0.07;
    }

    // Hit points read as substance draining out: a hurt creature
    // washes toward the colour of the mist and thins as it goes, so
    // "nearly seen off" is a glance rather than a number.
    //
    // Which is exactly why distance is carried by size and height alone
    // and never by haze, the way the terrain layers carry it. Washing a
    // far creature toward the mist would say it was nearly dead, and
    // the whole point of making the approach visible is that the player
    // can tell how the fight out there is going.
    const spent = 1 - unit(enemy.hp_permille / 1000);
    const dying = enemy.state === "dying" || enemy.state === "leaving";
    const alpha = dying ? 0.3 : 1 - spent * 0.4;
    const wash = spent * 0.85;
    const skin: CreatureSkin = {
      body: fade(atNight(mix(palette.creature, palette.creatureSpent, wash), dark), alpha),
      lit: fade(atNight(mix(palette.creatureLit, palette.creatureSpent, wash), dark), alpha),
      accent: fade(
        atNight(mix(creatureColor(approach), palette.creatureSpent, wash * 0.8), dark),
        alpha,
      ),
      // Deliberately not run through `atNight`: the eyes are the one
      // thing out there that gets brighter as the sky goes out.
      eye: fade(palette.creatureEye, dying ? 0 : (1 - spent) * (0.45 + dark * 0.55)),
      spoil: fade(spoil, alpha),
      phase: scatter * Math.PI * 2,
      facing,
    };

    const size = layout.slotW * (dying ? 0.34 : 0.42) * scale;
    if (dying) {
      // Going, and visibly so: a pale bloom where it stood, spreading
      // outward. It survives one tick, so it has to be unmissable in
      // the frame or two it gets.
      batch.push(
        x - size * 2,
        y - size * 2.4,
        size * 4,
        size * 3.2,
        fade(palette.creatureSpent, 0.22),
        {
          radius: size * 1.6,
          softness: size * 1.4,
        },
      );
    }
    const standY = dying ? y - size * 0.2 : y;

    switch (approach) {
      case "Canopy":
        drawLeaper(batch, x, standY, size, skin, clock);
        break;
      case "Burrow":
        drawBorer(batch, x, standY, size, skin, clock, arrived > 0.5);
        break;
      default:
        drawScuttler(batch, x, standY, size, skin, clock);
        break;
    }
  }
}

/**
 * Low, wide and many-legged. The lesson it teaches is ammo economics,
 * so there are always several of them and each one has to stay legible
 * at a size where it is mostly silhouette.
 */
function drawScuttler(
  batch: QuadBatch,
  x: number,
  baseY: number,
  size: number,
  skin: CreatureSkin,
  clock: number,
): void {
  const w = size * 1.55;
  const h = size * 0.66;
  const shade = mix(skin.body, palette.crack, 0.45);

  for (let i = 0; i < 6; i += 1) {
    const hipX = x + (i / 5 - 0.5) * w * 0.82;
    const swing = Math.sin(clock * 7 + skin.phase + i * 2.1) * size * 0.2;
    batch.pushLine(hipX, baseY - h * 0.5, hipX + swing, baseY, Math.max(1, size * 0.09), shade);
  }

  batch.push(x - w / 2, baseY - h, w, h, skin.lit, { colorBottom: skin.body, radius: h * 0.5 });
  for (let i = 1; i <= 3; i += 1) {
    batch.push(
      x - w * 0.4 + (i * w * 0.8) / 4,
      baseY - h * 0.92,
      Math.max(1, w * 0.04),
      h * 0.76,
      fade(skin.accent, 0.5),
      { radius: 1 },
    );
  }

  const headR = size * 0.27;
  const headX = x + skin.facing * w * 0.5;
  const headY = baseY - h * 0.72;
  batch.push(headX - headR, headY - headR, headR * 2, headR * 2, skin.body, { radius: headR });
  drawEyes(batch, headX + skin.facing * headR * 0.35, headY, size * 0.11, skin.eye);
}

/**
 * Slim, high, and hanging off a pair of membranes. It arrives on the
 * roof, so it is drawn to be recognised from below and at distance —
 * the wings are most of the silhouette for exactly that reason.
 */
function drawLeaper(
  batch: QuadBatch,
  x: number,
  baseY: number,
  size: number,
  skin: CreatureSkin,
  clock: number,
): void {
  const w = size * 0.78;
  const h = size * 1.4;
  const beat = Math.sin(clock * 3.4 + skin.phase);

  for (const side of [-1, 1]) {
    batch.pushLine(
      x + side * w * 0.3,
      baseY - h * 0.78,
      x + side * w * 1.75,
      baseY - h * (1.02 + beat * side * 0.16),
      h * 0.3,
      fade(skin.accent, 0.42),
    );
  }

  batch.push(x - w / 2, baseY - h, w, h, skin.lit, { colorBottom: skin.body, radius: w * 0.5 });
  for (const side of [-1, 1]) {
    batch.pushLine(
      x + side * w * 0.22,
      baseY - h * 0.26,
      x + side * w * 0.52,
      baseY,
      Math.max(1, size * 0.08),
      mix(skin.body, palette.crack, 0.4),
    );
  }

  const headR = size * 0.25;
  const headX = x + skin.facing * w * 0.18;
  const headY = baseY - h - headR * 0.6;
  batch.push(headX - headR, headY - headR, headR * 2, headR * 2, skin.body, { radius: headR });
  drawEyes(batch, headX + skin.facing * headR * 0.4, headY, size * 0.1, skin.eye);
}

/**
 * A mound of turned earth with something coming up out of it.
 *
 * While it is still travelling the mound is all there is, which is the
 * read: something is coming, and it is coming under you. It has no
 * eyes — it works blind, and that absence is what tells it apart from
 * the other two before you can make out anything else.
 */
function drawBorer(
  batch: QuadBatch,
  x: number,
  baseY: number,
  size: number,
  skin: CreatureSkin,
  clock: number,
  erupted: boolean,
): void {
  const moundW = size * 1.9;
  batch.push(x - moundW / 2, baseY - size * 0.28, moundW, size * 0.32, skin.spoil, {
    colorBottom: mix(skin.spoil, palette.crack, 0.4),
    radius: size * 0.16,
  });

  if (!erupted) {
    for (let i = 0; i < 3; i += 1) {
      const throwUp = (clock * 1.6 + i * 0.37) % 1;
      batch.push(
        x + (i - 1) * size * 0.42,
        baseY - size * 0.32 - Math.sin(throwUp * Math.PI) * size * 0.5,
        size * 0.11,
        size * 0.11,
        skin.spoil,
        { radius: size * 0.055 },
      );
    }
    return;
  }

  const segments = 5;
  const heave = 0.92 + Math.sin(clock * 2.4 + skin.phase) * 0.08;
  let tipX = x;
  let tipY = baseY;
  for (let i = 0; i < segments; i += 1) {
    const t = i / (segments - 1);
    const sx = x + skin.facing * t * size * 1.05;
    const sy = baseY - size * 0.2 - Math.sin(t * Math.PI * 0.8) * size * heave;
    const r = size * (0.34 - t * 0.12);
    batch.push(sx - r, sy - r, r * 2, r * 2, mix(skin.body, skin.accent, 0.25 + t * 0.5), {
      radius: r,
    });
    tipX = sx;
    tipY = sy;
  }

  for (const side of [-1, 1]) {
    batch.pushLine(
      tipX,
      tipY,
      tipX + skin.facing * size * 0.3,
      tipY + side * size * 0.24,
      Math.max(1, size * 0.07),
      mix(skin.accent, palette.splinter, 0.4),
    );
  }
}

/** Two ocelli, stacked. Warm, and the last thing to go out. */
function drawEyes(batch: QuadBatch, x: number, y: number, r: number, eye: Color): void {
  for (let i = 0; i < 2; i += 1) {
    batch.push(x - r / 2, y - r * 0.9 + i * r * 1.4, r, r, eye, {
      radius: r / 2,
      softness: r * 0.6,
    });
  }
}

// ---------------------------------------------------------------------------
// The tower
// ---------------------------------------------------------------------------

/**
 * Chicken legs. The gait is driven by distance walked, so the tower
 * strides when it is moving and stands still when it is paused — the
 * single clearest signal that the world is running.
 *
 * Standing still is four different situations, though, and they used to
 * be one picture: a leg frozen halfway through a step, which is what a
 * hung game looks like. So the legs answer `journey.halt`. A tower the
 * player stopped plants both feet and settles onto them. A tower that
 * cannot afford the step keeps trying — a foot lifts, stutters, and
 * comes back down, which is the whole of a brown-out in one gesture.
 * A tower at a fork stands square, weight even, facing the decision
 * (the waypost carries the rest of that read). And a tower at the far
 * edge has arrived: feet together, nothing left to walk to.
 */
function drawLegs(batch: QuadBatch, { view, layout, clock }: SceneContext): void {
  const shape = towerShape(view);
  const spanX = shape.slots * layout.slotW;
  const reach = layout.viewport.height - layout.groundY;
  const halt = view.journey.halt;
  // Straighten out of mid-stride when the legs stop: a foot left in the
  // air is the single strongest "this is frozen" cue there is.
  const gait = halt === "walking" ? 1 : 0;
  // Arriving settles the stance narrow and low; a fork stands square
  // and open, ready to go either way.
  const stanceScale = halt === "arrived" ? 0.34 : halt === "stopped" ? 0.62 : 1;
  const settle = halt === "walking" ? 0 : halt === "arrived" ? reach * 0.06 : reach * 0.03;
  const hipY = layout.groundY - reach * 0.1 + settle;
  const footY = layout.groundY + reach * 0.62;
  const phase = view.world.distance * 0.09;
  const thickness = layout.slotW * 0.24;

  for (let i = 0; i < 2; i += 1) {
    const hipX = layout.originX + spanX * (i === 0 ? 0.26 : 0.74);
    // Off the gait while walking; a fixed open stance once halted, so
    // the feet land somewhere deliberate rather than wherever the last
    // pace happened to leave them.
    const step = gait
      ? Math.sin(phase + i * Math.PI)
      : (i === 0 ? -0.5 : 0.5) * stanceScale;
    // A brown-out is the legs asking and not being answered: a small
    // stuttering lift that never becomes a step.
    const strain =
      halt === "brownout" ? Math.max(0, Math.sin(clock * 5.5 + i * 2.3)) ** 6 * reach * 0.05 : 0;
    const lift = gait * Math.max(0, Math.cos(phase + i * Math.PI)) * reach * 0.22 + strain;
    const footX = hipX + step * layout.slotW * 0.9;
    const kneeX = hipX + step * layout.slotW * 0.36;
    const kneeY = (hipY + footY) / 2 - lift * 0.35;

    // A shadow that tightens as the foot lands. Cheap, and it does most
    // of the work of making the tower feel heavy.
    batch.push(
      footX - layout.slotW * 0.5,
      layout.groundY + reach * 0.6,
      layout.slotW,
      reach * 0.1,
      fade(palette.vignette, 0.35 - (lift / (reach * 0.22)) * 0.2),
      { radius: reach * 0.05, softness: 4 },
    );

    batch.pushLine(hipX, hipY, kneeX, kneeY, thickness, palette.leg);
    batch.pushLine(kneeX, kneeY, footX, footY - lift, thickness * 0.8, palette.leg);
    batch.push(
      kneeX - thickness * 0.55,
      kneeY - thickness * 0.55,
      thickness * 1.1,
      thickness * 1.1,
      palette.legJoint,
      { radius: thickness * 0.55 },
    );
    // A splayed foot, so it reads as planted rather than as a stick.
    batch.push(
      footX - layout.slotW * 0.32,
      footY - lift - thickness * 0.3,
      layout.slotW * 0.64,
      thickness * 0.6,
      palette.legJoint,
      { radius: thickness * 0.3 },
    );
  }
}

function drawTower(batch: QuadBatch, ctx: SceneContext): void {
  const { view, layout } = ctx;
  const shape = towerShape(view);
  const spanX = shape.slots * layout.slotW;
  const topY = floorY(layout, shape.floors - 1);

  // Outer shell, so gaps between rooms read as interior rather than as
  // holes onto the jungle.
  const shellPad = layout.slotW * 0.1;
  batch.push(
    layout.originX - shellPad,
    topY - shellPad,
    spanX + shellPad * 2,
    layout.groundY - topY + shellPad * 2,
    palette.towerShellLip,
    { colorBottom: palette.towerShell, radius: shellPad * 2 },
  );

  for (const floor of view.tower.floors) {
    const y = floorY(layout, floor.index);
    // Interior, lit from the deck upward — the tower is somebody's
    // home, and warm light from inside is most of what says so.
    batch.push(layout.originX, y, spanX, layout.floorH, palette.floorPlate, {
      colorBottom: palette.floorLit,
    });
    // Lamplight pooling along the deck the crew walk on. It gets
    // brighter as the sky darkens — and goes out entirely in a
    // brown-out, which is the whole point of tracking `lit`. There is
    // no warning banner; the tower simply goes dark.
    const lamp = ctx.view.power.lit ? 0.07 + darkness(ctx.view) * 0.3 : 0;
    batch.push(
      layout.originX,
      y + layout.floorH * 0.55,
      spanX,
      layout.floorH * 0.45,
      fade(palette.lamplight, 0),
      { colorBottom: fade(palette.lamplight, lamp) },
    );
    // Back-wall battens. Empty floors are common early on, and without
    // some interior texture the tower reads as an empty cabinet.
    const battens = Math.max(2, Math.round(shape.slots / 2));
    for (let i = 1; i < battens; i += 1) {
      batch.push(
        layout.originX + (spanX * i) / battens,
        y + layout.floorH * 0.12,
        1,
        layout.floorH * 0.74,
        fade(palette.floorEdge, 0.1),
      );
    }

    // The deck itself.
    batch.push(layout.originX, y + layout.floorH - 3, spanX, 3, palette.floorEdge);

    for (const room of floor.rooms) {
      drawRoom(batch, ctx, room, y);
    }

    // A planter on the outboard edge of every deck. Cheap, and it is
    // most of what says "garden-tower" rather than "shelving unit".
    drawPlanter(
      batch,
      layout.originX + spanX - layout.slotW * 0.55,
      y + layout.floorH - 3,
      layout.slotW * 0.5,
      ctx.clock + floor.index,
    );

    // Last, so a hole in the skin is drawn over the room behind it
    // rather than under it. The panel is the outermost thing there is.
    drawPanel(batch, ctx, floor, y, spanX, shellPad);
  }

  // Roof garden. The sails mount here in M1; until then it is where
  // the tower keeps its own greenery.
  const roofY = topY - shellPad * 0.4;
  batch.push(
    layout.originX,
    roofY - layout.floorH * 0.08,
    spanX,
    layout.floorH * 0.08,
    palette.canopyFar,
    {
      colorBottom: palette.towerShell,
      radius: 2,
    },
  );
  for (let i = 0; i < shape.slots; i += 1) {
    const px = layout.originX + (i + 0.5) * layout.slotW;
    drawPlanter(
      batch,
      px - layout.slotW * 0.22,
      roofY - layout.floorH * 0.07,
      layout.slotW * 0.44,
      ctx.clock + i * 1.7,
    );
  }

  // Roof lip — from M1 this is where the canopy sails mount, and
  // growing taller starts costing you your power deck.
  batch.push(
    layout.originX - shellPad * 2,
    topY - shellPad * 2.4,
    spanX + shellPad * 4,
    shellPad * 2,
    palette.towerShellLip,
    { radius: shellPad },
  );
}

/**
 * The state of a floor's outer skin, on both flanks.
 *
 * Sound skin draws nothing at all — the tower's shell already is the
 * panel, and the cheapest way to say "intact" is to leave it alone. So
 * everything below is damage: splits opening up as it takes hits, and
 * then a ragged hole with the tower's own lamplight coming out of it.
 * The light leaking is the point. It is a home, and you can see into
 * it now.
 */
function drawPanel(
  batch: QuadBatch,
  ctx: SceneContext,
  floor: FloorView,
  top: number,
  spanX: number,
  pad: number,
): void {
  const { layout, view } = ctx;
  const health = unit(floor.panel_permille / 1000);
  if (health >= 1) return;

  const hurt = 1 - health;
  const dark = darkness(view);
  const width = pad + layout.slotW * 0.14;
  const flanks = [layout.originX - pad, layout.originX + spanX + pad - width];

  for (const px of flanks) {
    if (floor.panel_permille > 0) {
      batch.push(px, top, width, layout.floorH, fade(palette.crack, hurt * 0.55));
      // Splits, widening with the damage. They run with the grain of
      // the timber, which is to say downward and slightly out.
      const splits = 1 + Math.floor(hurt * 3);
      for (let i = 0; i < splits; i += 1) {
        const y = top + layout.floorH * (0.14 + i * 0.26);
        batch.pushLine(
          px + width * 0.15,
          y,
          px + width * 0.85,
          y + layout.floorH * 0.14,
          1.3,
          fade(palette.crack, 0.75),
        );
      }
      continue;
    }

    // Breached: the skin is gone between the deck and the ceiling, and
    // the ragged ends of it are all that is left.
    const holeY = top + layout.floorH * 0.1;
    const holeH = layout.floorH * 0.78;
    const tear = layout.floorH * 0.07;
    const torn = fade(palette.splinter, 0.7);
    batch.push(px, holeY, width, holeH, atNight(palette.breach, dark), { radius: 2 });
    batch.pushLine(px, holeY, px + width, holeY + tear, 2, torn);
    batch.pushLine(px, holeY + holeH, px + width, holeY + holeH - tear, 2, torn);
    if (view.power.lit) {
      // Warm light getting out where the wall used to be. It is the
      // clearest possible statement that the tower's skin is open, and
      // after dark it is visible from across the frame.
      const spill = layout.slotW * 0.55;
      batch.push(
        px - spill,
        holeY - spill * 0.3,
        width + spill * 2,
        holeH + spill * 0.6,
        fade(palette.lamplight, 0.12 + dark * 0.2),
        { radius: spill * 0.8, softness: spill * 0.9 },
      );
    }
  }
}

/** A tuft of greenery on a ledge, swaying gently. */
function drawPlanter(
  batch: QuadBatch,
  x: number,
  baseY: number,
  width: number,
  phase: number,
): void {
  const sway = Math.sin(phase * 0.8) * width * 0.06;
  batch.push(x, baseY - width * 0.16, width, width * 0.16, palette.towerShell, { radius: 1 });
  for (let i = 0; i < 3; i += 1) {
    const t = (i - 1) * 0.32;
    batch.pushLine(
      x + width * (0.5 + t),
      baseY - width * 0.14,
      x + width * (0.5 + t * 1.5) + sway,
      baseY - width * (0.4 + (i === 1 ? 0.18 : 0)),
      Math.max(1, width * 0.08),
      mix(palette.cargo, palette.canopyFar, 0.35),
    );
  }
}

function drawRoom(batch: QuadBatch, ctx: SceneContext, room: RoomView, floorTop: number): void {
  const { catalog, layout } = ctx;
  const info = catalog.rooms[room.def];
  const inset = layout.slotW * 0.06;
  const x = slotX(layout, room.slot) + inset;
  const w = room.width * layout.slotW - inset * 2;
  // Rooms sit on the deck and stop short of the ceiling, so the floor
  // above reads as a separate storey rather than a stacked block.
  const y = floorTop + layout.floorH * 0.16;
  const h = layout.floorH * 0.84 - 3;

  const base = info ? roomColor(info.category) : palette.roomBody;
  if (room.wrecked) {
    drawWreck(batch, room, base, x, y, w, h);
    return;
  }

  // A stalled room is drawn dim rather than badged. The tower going
  // quiet is the warning; see DECISIONS.md §8.
  const stalled = room.stalled ? mix(base, palette.roomStalled, 0.6) : base;
  // Damage bruises on top of that: the colour goes out of it and the
  // panelling starts to split. Dim and hurt have to look different,
  // because one of them is a supply problem and the other needs poles.
  const hurt = 1 - unit(room.health_permille / 1000);
  const body = mix(stalled, palette.hurt, hurt * 0.7);
  batch.push(x, y, w, h, mix(body, palette.roomBodyLit, 0.25), {
    colorBottom: body,
    radius: 4,
  });
  if (hurt > 0.05) {
    drawSplits(batch, room.id, x, y, w, h, 1 + Math.floor(hurt * 4), fade(palette.crack, 0.7));
  }

  const wellH = Math.max(4, h * 0.2);
  const wellY = y + h - wellH - 3;

  // Inputs on the left, outputs on the right, mirroring the direction
  // things physically travel through the room.
  const gauges = [
    ...room.inputs.map((stack) => ({ stack, color: palette.bufferFill })),
    ...room.outputs.map((stack) => ({ stack, color: palette.outputFill })),
  ];
  const gaugeW = gauges.length > 0 ? (w - 10) / gauges.length : 0;
  gauges.forEach((gauge, i) => {
    const gx = x + 5 + i * gaugeW;
    const gw = Math.max(3, gaugeW - 3);
    batch.push(gx, wellY, gw, wellH, palette.bufferWell, { radius: 2 });
    const frac = gauge.stack.max > 0 ? gauge.stack.count / gauge.stack.max : 0;
    if (frac > 0) {
      const fillH = Math.max(2, wellH * frac);
      batch.push(gx, wellY + wellH - fillH, gw, fillH, gauge.color, { radius: 2 });
    }
  });

  // Shelves render as a row of pips, one per shelf, filled by how much
  // is on them. A storeroom's stock is readable without a number.
  if (room.shelves.length > 0) {
    const pipW = (w - 10) / room.shelves.length;
    room.shelves.forEach((shelf, i) => {
      const px = x + 5 + i * pipW;
      const pw = Math.max(3, pipW - 3);
      batch.push(px, wellY, pw, wellH, palette.bufferWell, { radius: 2 });
      const frac = shelf.max > 0 ? shelf.count / shelf.max : 0;
      if (frac > 0) {
        const fillH = Math.max(2, wellH * frac);
        batch.push(px, wellY + wellH - fillH, pw, fillH, palette.outputFill, { radius: 2 });
      }
    });
  }

  // Craft progress along the room's top edge.
  if (info && info.craft_ticks > 0) {
    const frac = Math.min(1, room.progress / info.craft_ticks);
    batch.push(x + 4, y + 3, w - 8, 3, palette.bufferWell, { radius: 1.5 });
    if (frac > 0) {
      batch.push(x + 4, y + 3, (w - 8) * frac, 3, palette.progress, { radius: 1.5 });
    }
  }

  // The Heartseed glows, gently, always — right up until it does not.
  if (info?.category === "Heart") {
    const pulse = 0.55 + Math.sin(ctx.clock * 1.2) * 0.12;
    batch.push(x - 4, y - 4, w + 8, h + 8, fade(palette.roomHeart, pulse * 0.5 * (1 - hurt)), {
      radius: 10,
      softness: 8,
    });
  }
}

/**
 * A room that has stopped being a room.
 *
 * Not the dim treatment a stalled room gets — a stalled mill is a
 * supply problem and will start again on its own, and the two must
 * never be confused. What is left here is the bottom third of the box
 * with its top torn off, a couple of struts still standing, and none
 * of the gauges: whatever was in it is not coming back out. The label
 * layer keeps the name and the count, so the precision read survives.
 */
function drawWreck(
  batch: QuadBatch,
  room: RoomView,
  base: Color,
  x: number,
  y: number,
  w: number,
  h: number,
): void {
  // A trace of the original colour, so you can still tell what you
  // lost from across the frame.
  const carcass = mix(palette.wreck, base, 0.16);
  const stubH = h * 0.38;
  batch.push(x, y + h - stubH, w, stubH, carcass, {
    colorBottom: palette.wreck,
    radius: 2,
  });

  // A torn top edge rather than a straight one. Five teeth is enough
  // to read as broken and few enough to stay cheap.
  const teeth = 5;
  for (let i = 0; i < teeth; i += 1) {
    const tw = w / teeth;
    const th = stubH * (0.12 + hash01(room.id + i * 977) * 0.55);
    batch.push(x + i * tw, y + h - stubH - th, tw * 0.92, th, carcass);
  }

  // Struts still standing where the walls were, leaning inward.
  batch.pushLine(x + w * 0.18, y + h, x + w * 0.28, y + h * 0.3, 2, fade(palette.splinter, 0.55));
  batch.pushLine(x + w * 0.8, y + h, x + w * 0.66, y + h * 0.42, 2, fade(palette.splinter, 0.45));
  batch.push(x, y + h - 3, w, 3, fade(palette.crack, 0.8));
}

/**
 * Splits opening across a surface as it takes damage.
 *
 * The colour is the caller's, because the two surfaces this runs on
 * are opposite values: a split in a room's pale timber reads as a dark
 * line, and the same line on a shaft's near-black column would be
 * invisible — there it has to be the raw wood showing through instead.
 */
function drawSplits(
  batch: QuadBatch,
  seed: number,
  x: number,
  y: number,
  w: number,
  h: number,
  count: number,
  color: Color,
): void {
  // Kept short whatever the surface: a shaft column is the height of
  // the whole tower, and splits scaled to that would be chasms.
  const reach = Math.min(h * 0.3, w * 0.6);
  for (let i = 0; i < count; i += 1) {
    const sx = x + w * (0.12 + hash01(seed + i * 7919) * 0.7);
    const sy = y + (h - reach) * (0.06 + hash01(seed + i * 104_729) * 0.88);
    batch.pushLine(sx, sy, sx + (hash01(seed + i * 31) - 0.5) * w * 0.3, sy + reach, 1.4, color);
  }
}

function drawShafts(batch: QuadBatch, ctx: SceneContext): void {
  const { view, layout, clock } = ctx;
  for (const shaft of view.tower.shafts) {
    const x = slotX(layout, shaft.slot);
    const top = floorY(layout, shaft.high);
    const bottom = floorY(layout, shaft.low) + layout.floorH;
    const hurt = 1 - unit(shaft.health_permille / 1000);

    // Where a severed column comes apart. Deterministic from the id,
    // because a break that wandered between frames would look like
    // damage happening over and over instead of damage that is there.
    const gapH = Math.min(layout.floorH * 0.5, (bottom - top) * 0.28);
    const breakY = top + (bottom - top - gapH) * (0.3 + hash01(shaft.id) * 0.4);
    // The two halves have slid past each other. Two pixels is enough:
    // the eye picks up a broken vertical line immediately.
    const shear = 2.5;

    if (shaft.severed) {
      batch.push(x + 2 + shear, top, layout.slotW - 4, breakY - top, palette.shaft, { radius: 3 });
      batch.push(
        x + 2 - shear,
        breakY + gapH,
        layout.slotW - 4,
        bottom - breakY - gapH,
        palette.shaft,
        { radius: 3 },
      );
    } else {
      batch.push(x + 2, top, layout.slotW - 4, bottom - top, palette.shaft, { radius: 3 });
    }

    // A shaft with people queueing on it glows. The queue is the
    // bottleneck instrument, so it has to be visible from the shaft as
    // well as from the crew standing at it.
    const busy = shaft.riders >= shaft.capacity || shaft.queued > 0;
    // A severed column is not busy and is not idle — it is dead, and
    // its rails go the colour of everything else that has broken.
    const rail = shaft.severed
      ? mix(palette.shaftRail, palette.crack, 0.65)
      : mix(busy ? palette.shaftBusy : palette.shaftRail, palette.crack, hurt * 0.5);
    const inGap = (y: number) => shaft.severed && y > breakY - 2 && y < breakY + gapH + 2;

    if (shaft.kind === "Stairs") {
      // Treads, so stairs read as stairs at a glance. Damage takes them
      // out one at a time, which is what a half-wrecked staircase
      // should look like before it goes entirely.
      const treads = Math.max(1, Math.round((bottom - top) / 9));
      for (let i = 0; i < treads; i += 1) {
        const ty = top + ((i + 0.5) * (bottom - top)) / treads;
        if (inGap(ty) || hash01(shaft.id + i * 6151) < hurt * 0.7) continue;
        batch.push(x + 4, ty, layout.slotW - 8, 1.5, fade(rail, busy ? 0.75 : 0.4));
      }
    } else {
      // Guide rails rather than treads, and a counterweight cable, so
      // a shaft with a car in it never gets mistaken for a staircase.
      batch.push(x + layout.slotW / 2 - 0.5, top, 1, bottom - top, fade(rail, 0.35));
    }

    if (shaft.severed) {
      // The uprights stop dead at the break rather than running past
      // it, and the ends they stop at are bent.
      for (const railX of [x + 2, x + layout.slotW - 4]) {
        batch.push(railX + shear, top, 2, breakY - top, fade(rail, 0.8));
        batch.push(railX - shear, breakY + gapH, 2, bottom - breakY - gapH, fade(rail, 0.8));
      }
      drawSeverance(batch, ctx, x, breakY, gapH, shear, clock);
    } else {
      batch.push(x + 2, top, 2, bottom - top, fade(rail, 0.8));
      batch.push(x + layout.slotW - 4, top, 2, bottom - top, fade(rail, 0.8));
      if (hurt > 0.05) {
        drawSplits(
          batch,
          shaft.id,
          x + 3,
          top,
          layout.slotW - 6,
          bottom - top,
          1 + Math.floor(hurt * 4),
          fade(palette.splinter, 0.35),
        );
      }
    }

    for (const car of shaft.cars) {
      drawCar(batch, ctx, shaft, car, x);
    }
  }
}

/**
 * The break in a severed column.
 *
 * This is the signature emergency of the siege — the tower's
 * circulation is cut in two and routing has already stopped going
 * through here — so it gets the loudest visual language in the game
 * short of the ending. Nothing about it is subtle on purpose: a gap
 * with the bare floor plates showing through, four snapped rails bent
 * away from one another, and dust still coming down out of it.
 */
function drawSeverance(
  batch: QuadBatch,
  ctx: SceneContext,
  x: number,
  breakY: number,
  gapH: number,
  shear: number,
  clock: number,
): void {
  const { layout } = ctx;
  const w = layout.slotW - 4;

  // Straight through: the floor plate behind the column, then nothing.
  batch.push(x + 2, breakY, w, gapH, palette.floorPlate, { colorBottom: palette.floorLit });
  batch.push(x + 2, breakY, w, gapH * 0.4, fade(palette.crack, 0.6), {
    colorBottom: fade(palette.crack, 0),
  });

  // Four snapped rail ends, each bent out of line with the half it
  // belongs to.
  const stubs: [number, number, number, number][] = [
    [x + 3 + shear, breakY - gapH * 0.22, x - 1 + shear, breakY + gapH * 0.1],
    [x + w - 1 + shear, breakY - gapH * 0.22, x + w + 3 + shear, breakY + gapH * 0.08],
    [x + 3 - shear, breakY + gapH * 1.22, x - 1 - shear, breakY + gapH * 0.9],
    [x + w - 1 - shear, breakY + gapH * 1.22, x + w + 3 - shear, breakY + gapH * 0.92],
  ];
  for (const [x1, y1, x2, y2] of stubs) {
    batch.pushLine(x1, y1, x2, y2, 2.2, palette.splinter);
  }

  // Dust still falling out of it, so the break reads as ongoing rather
  // than as something that was always drawn that way.
  for (let i = 0; i < 4; i += 1) {
    const drift = (clock * 0.55 + i * 0.27) % 1;
    batch.push(
      x + 5 + i * (w / 5),
      breakY + gapH * 0.2 + drift * gapH * 1.6,
      2,
      2,
      fade(palette.splinter, 0.5 * (1 - drift)),
      { radius: 1 },
    );
  }
}

function drawCar(
  batch: QuadBatch,
  { layout, catalog }: SceneContext,
  shaft: ShaftView,
  car: CarView,
  x: number,
): void {
  const w = layout.slotW - 8;
  const h = layout.floorH * 0.6;
  const y = layout.groundY - car.floor * layout.floorH - h - 4;
  const dwelling = car.state === "dwelling";

  batch.push(x + 4, y, w, h, mix(palette.towerShellLip, palette.shaftRail, 0.4), {
    colorBottom: palette.towerShell,
    radius: 3,
  });
  // Doors: shut while travelling, open at a stop. The clearest possible
  // signal for what a car is doing right now.
  const gap = dwelling ? w * 0.34 : w * 0.04;
  batch.push(x + 4 + w * 0.06, y + h * 0.14, (w * 0.88 - gap) / 2, h * 0.72, palette.floorPlate, {
    radius: 2,
  });
  batch.push(
    x + 4 + w * 0.94 - (w * 0.88 - gap) / 2,
    y + h * 0.14,
    (w * 0.88 - gap) / 2,
    h * 0.72,
    palette.floorPlate,
    { radius: 2 },
  );

  // Load, as pips along the car's floor. Reading "how full is it"
  // should not need a number.
  if (shaft.capacity > 0 && car.load > 0) {
    const pip = Math.min(w / shaft.capacity - 1.5, 5);
    for (let i = 0; i < car.load; i += 1) {
      batch.push(x + 6 + i * (pip + 1.5), y + h - 5, pip, 3, palette.crewCarrying, { radius: 1.5 });
    }
  }

  // Dumbwaiter cargo rides visibly rather than invisibly.
  for (const [index, load] of car.freight.entries()) {
    const glyphless = Math.min(w * 0.3, 8);
    batch.push(x + 6 + index * (glyphless + 2), y + h * 0.3, glyphless, glyphless, palette.cargo, {
      radius: 2,
    });
    void load;
  }

  void catalog;
}

function drawCrew(batch: QuadBatch, { view, layout, clock }: SceneContext): void {
  for (const member of view.crew) {
    const { x, y } = crewPosition(layout, member);
    // Idle fidget, phase-offset per crew member from the cosmetic RNG
    // stream so nobody bobs in lockstep.
    const phase = (member.fidget / 65535) * Math.PI * 2;
    const bob = member.state === "idle" ? Math.sin(clock * 2.2 + phase) * 1.2 : 0;

    const bodyW = layout.slotW * 0.19;
    const bodyH = layout.floorH * 0.26;
    const color = member.stressed
      ? palette.crewStressed
      : member.carrying
        ? palette.crewCarrying
        : palette.crew;

    batch.push(x - bodyW * 0.6, y + 1, bodyW * 1.2, 3, fade(palette.crewShadow, 0.35), {
      radius: 2,
    });
    batch.push(x - bodyW / 2, y - bodyH + bob, bodyW, bodyH, color, {
      colorBottom: mix(color, palette.towerShell, 0.3),
      radius: bodyW * 0.45,
    });
    // Head.
    batch.push(x - bodyW * 0.36, y - bodyH - bodyW * 0.5 + bob, bodyW * 0.72, bodyW * 0.72, color, {
      radius: bodyW * 0.36,
    });

    if (member.carrying) {
      const crate = bodyW * 0.7;
      batch.push(x - crate / 2, y - bodyH - crate * 1.5 + bob, crate, crate, palette.cargo, {
        radius: 2,
      });
    }

    // Mending: a pole in hand and a small pool of worklight. Repair is
    // crew time spent somewhere other than the chain, so it has to be
    // visible as that and not mistaken for someone standing about.
    if (member.state === "mend") {
      batch.push(
        x - bodyW * 1.1,
        y - bodyH * 1.9,
        bodyW * 2.2,
        bodyH * 2.1,
        fade(palette.lamplight, 0.14 + Math.abs(Math.sin(clock * 3 + phase)) * 0.08),
        { radius: bodyW, softness: bodyW * 0.9 },
      );
      batch.pushLine(
        x - bodyW * 0.7,
        y - bodyH * 0.2,
        x + bodyW * 0.9,
        y - bodyH * 1.3,
        Math.max(1.2, bodyW * 0.18),
        palette.splinter,
      );
    }

    // Waiting reads as a swelling ring, not a number. The bottleneck
    // announces itself.
    if (member.state === "board") {
      const glow = 0.25 + Math.min(0.5, member.wait_ticks / 90);
      batch.push(
        x - bodyW,
        y - bodyH - bodyW * 0.7,
        bodyW * 2,
        bodyH + bodyW * 1.4,
        fade(member.stressed ? palette.crewStressed : palette.shaftBusy, glow * 0.5),
        { radius: bodyW, softness: bodyW * 0.9 },
      );
    }
  }
}

/** Screen position of a crew member's feet. */
export function crewPosition(layout: Layout, member: CrewView): { x: number; y: number } {
  return {
    x: layout.originX + (member.slot + 0.5) * layout.slotW,
    y: layout.groundY - member.floor * layout.floorH - 3,
  };
}

// ---------------------------------------------------------------------------
// Overlays
// ---------------------------------------------------------------------------

function drawPlaceMode(batch: QuadBatch, ctx: SceneContext): void {
  const { placeMode, view, layout } = ctx;
  if (!placeMode) return;

  // Every position that would accept it, so the player can see their
  // options before committing rather than probing for a rejection.
  for (const floor of view.tower.floors) {
    if (placeMode.maxFloor !== null && floor.index > placeMode.maxFloor) continue;
    for (let slot = 0; slot + placeMode.width <= floor.slots; slot += 1) {
      if (!placementFits(view, placeMode, floor.index, slot)) continue;
      const { x, y, w, h } = ghostRect(layout, placeMode, floor.index, slot);
      batch.push(x, y, w, h, fade(palette.slotHint, 0.07), { radius: 4 });
    }
  }

  const hover = placeMode.hover;
  if (!hover) return;
  const allowed = placementFits(view, placeMode, hover.floor, hover.slot);
  const { x, y, w, h } = ghostRect(layout, placeMode, hover.floor, hover.slot);
  const color = allowed ? palette.ghostValid : palette.ghostBlocked;
  batch.push(x, y, w, h, fade(color, 0.4), { radius: 4, softness: 2 });
}

/** The footprint a ghost would occupy: one floor for a room, several for a shaft. */
function ghostRect(
  layout: Layout,
  placeMode: PlaceMode,
  floor: number,
  slot: number,
): { x: number; y: number; w: number; h: number } {
  const x = slotX(layout, slot) + 2;
  const w = placeMode.width * layout.slotW - 4;
  if (placeMode.kind === "room") {
    return { x, y: floorY(layout, floor) + 3, w, h: layout.floorH - 9 };
  }
  const top = floorY(layout, floor + placeMode.span - 1);
  const bottom = floorY(layout, floor) + layout.floorH;
  return { x, y: top + 3, w, h: bottom - top - 6 };
}

/** Mirrors the command validation, so the highlight never lies. */
export function placementFits(
  view: ViewSnapshot,
  placeMode: PlaceMode,
  floor: number,
  slot: number,
): boolean {
  if (placeMode.maxFloor !== null && floor > placeMode.maxFloor) return false;
  if (placeMode.kind === "room") {
    return slotRangeFree(view, floor, slot, placeMode.width);
  }
  const top = floor + placeMode.span - 1;
  if (top >= view.tower.floors.length) return false;
  for (let f = floor; f <= top; f += 1) {
    if (!slotRangeFree(view, f, slot, 1)) return false;
  }
  return true;
}

function drawVignette(batch: QuadBatch, { layout, view }: SceneContext): void {
  const { width, height } = layout.viewport;
  const band = height * 0.18;
  batch.push(0, height - band, width, band, fade(palette.vignette, 0), {
    colorBottom: fade(palette.vignette, 0.55),
  });

  // A brown-out dims the whole frame, briefly and unmistakably. This is
  // the one place the renderer editorialises, and it earns it: losing
  // power is the emergency the charge economy exists to threaten.
  if (view.power.brownout) {
    const pulse = 0.1 + Math.abs(Math.sin(view.tick * 0.06)) * 0.12;
    batch.push(0, 0, width, height, fade(palette.vignette, pulse));
  }

  // The Heartseed is gone. The frame settles rather than flashing —
  // this is a home that did not make it, not a fail state, and nothing
  // about the ending should read as an alarm (DECISIONS.md §8).
  if (view.siege.lost) {
    batch.push(0, 0, width, height, fade(palette.vignette, 0.4), {
      colorBottom: fade(palette.vignette, 0.68),
    });
  }

  // The far edge of the journey. The other ending, and deliberately the
  // opposite treatment: the loss wash closes the frame down, this one
  // opens it out. Neither one grades the run — one reports a home that
  // did not make it and the other reports how far this one got.
  if (view.journey.arrived) {
    batch.push(0, 0, width, height * 0.7, fade(palette.farEdge, 0.3), {
      colorBottom: fade(palette.farEdge, 0.1),
    });
  }
}

// ---------------------------------------------------------------------------
// Shared queries
// ---------------------------------------------------------------------------

export function towerShape(view: ViewSnapshot): { slots: number; floors: number } {
  return {
    slots: view.tower.floors[0]?.slots ?? 8,
    floors: view.tower.floors.length,
  };
}

/**
 * Is `[slot, slot + width)` on `floor` clear of rooms and shaft
 * columns? Mirrors `Tower::slot_range_blocked` in the core so the
 * highlight matches what the command will actually accept.
 */
export function slotRangeFree(
  view: ViewSnapshot,
  floor: number,
  slot: number,
  width: number,
): boolean {
  const target = view.tower.floors[floor];
  if (!target) return false;
  const end = slot + width;
  if (slot < 0 || end > target.slots) return false;

  for (const room of target.rooms) {
    if (slot < room.slot + room.width && room.slot < end) return false;
  }
  for (const shaft of view.tower.shafts) {
    if (shaft.low <= floor && floor <= shaft.high && shaft.slot < end && slot <= shaft.slot) {
      return false;
    }
  }
  return true;
}
