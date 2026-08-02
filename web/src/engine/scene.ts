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
import { fade, mix, palette, roomColor, terrainColors } from "./palette";
import { floorY, slotX, worldX, type Layout } from "./layout";
import type { CatalogSnapshot, CrewView, RoomView, ViewSnapshot } from "../bridge/types";

/** A pending placement the player is aiming at a slot. */
export interface PlaceMode {
  roomId: string;
  width: number;
  maxFloor: number | null;
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

export function drawScene(batch: QuadBatch, ctx: SceneContext): void {
  drawSky(batch, ctx);
  drawTerrain(batch, ctx);
  drawLegs(batch, ctx);
  drawTower(batch, ctx);
  drawShafts(batch, ctx);
  drawCrew(batch, ctx);
  drawPlaceMode(batch, ctx);
  drawVignette(batch, ctx);
}

// ---------------------------------------------------------------------------
// Sky and terrain
// ---------------------------------------------------------------------------

function drawSky(batch: QuadBatch, { layout }: SceneContext): void {
  const { width } = layout.viewport;
  const horizon = layout.horizonY;

  // Two stops rather than one: humid teal overhead falling to a warm
  // haze at the treeline. A single gradient across the whole frame
  // washes out, which is exactly what the first pass looked like.
  batch.push(0, 0, width, horizon * 0.62, palette.skyHigh, { colorBottom: palette.skyMid });
  batch.push(0, horizon * 0.62 - 1, width, horizon - horizon * 0.62 + 2, palette.skyMid, {
    colorBottom: palette.skyLow,
  });

  // A soft sun high on the right. Atmospheric for now; M1's charge
  // system gives it a job.
  const sunX = width * 0.76;
  const sunY = horizon * 0.28;
  const radius = Math.min(width, layout.viewport.height) * 0.075;
  batch.push(sunX - radius, sunY - radius, radius * 2, radius * 2, fade(palette.sunlight, 0.55), {
    radius,
    softness: radius * 1.1,
  });
}

function drawTerrain(batch: QuadBatch, ctx: SceneContext): void {
  const { view, catalog, layout } = ctx;
  const { width, height } = layout.viewport;
  const distance = view.world.distance;
  const horizon = layout.horizonY;
  const depth = layout.groundY - horizon;

  // The ground plane, running from the horizon to the bottom of the
  // frame, coloured by the band it belongs to. Perspective is faked
  // with a gradient: hazy where it meets the sky, saturated underfoot.
  for (const band of view.world.bands) {
    const terrain = catalog.terrain[band.kind];
    if (!terrain) continue;
    const colors = terrainColors(terrain.id);
    const x1 = worldX(layout, band.start, distance, 1);
    const x2 = worldX(layout, band.end, distance, 1);
    if (x2 < -60 || x1 > width + 60) continue;
    batch.push(x1, horizon, x2 - x1, height - horizon, mix(colors.far, palette.haze, 0.45), {
      colorBottom: mix(colors.near, palette.ground, 0.75),
    });
  }

  // Mist pooling where the canopy meets the sky. Sells the distance
  // more cheaply than any amount of extra geometry.
  batch.push(0, horizon - depth * 0.14, width, depth * 0.3, fade(palette.haze, 0.55), {
    colorBottom: fade(palette.haze, 0),
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
      if (x < -140 || x > width + 140) continue;
      const kind = terrain.feature_kinds[feature.kind] ?? "tree";
      const colors = terrainColors(terrain.id);
      const base = layerIndex === 0 ? colors.far : colors.near;
      const tint = mix(base, layer.tint, layer.blend);
      const size = depth * layer.scale * (0.45 + (feature.scale / 255) * 0.7);
      drawFeature(batch, kind, x, layer.base, size, tint, ctx.clock, feature.at);
    }
  }

  // The strip of ground the tower actually stands on, so its feet have
  // somewhere to land rather than floating over the parallax.
  batch.push(0, layout.groundY, width, height - layout.groundY, fade(palette.ground, 0.55), {
    colorBottom: palette.ground,
  });
  batch.push(0, layout.groundY - 2, width, 4, fade(palette.groundLip, 0.7));
}

function drawFeature(
  batch: QuadBatch,
  kind: string,
  x: number,
  baseY: number,
  size: number,
  tint: Color,
  clock: number,
  seed: number,
): void {
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
      const stone = mix(tint, palette.ruinFar, 0.4);
      const shade = mix(stone, palette.vignette, 0.22);
      const unitW = size * 0.18;
      const tallH = size * (0.55 + wobble(4) * 0.5);
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
      // field and not rubble.
      const moss = mix(palette.canopyFar, tint, 0.35);
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
      break;
    }
    default:
      batch.push(x - size * 0.1, baseY - size * 0.2, size * 0.2, size * 0.2, tint, { radius: 3 });
  }
}

// ---------------------------------------------------------------------------
// The tower
// ---------------------------------------------------------------------------

/**
 * Chicken legs. The gait is driven by distance walked, so the tower
 * strides when it is moving and stands still when it is paused — the
 * single clearest signal that the world is running.
 */
function drawLegs(batch: QuadBatch, { view, layout }: SceneContext): void {
  const shape = towerShape(view);
  const spanX = shape.slots * layout.slotW;
  const reach = layout.viewport.height - layout.groundY;
  const hipY = layout.groundY - reach * 0.1;
  const footY = layout.groundY + reach * 0.62;
  const phase = view.world.distance * 0.09;
  const thickness = layout.slotW * 0.24;

  for (let i = 0; i < 2; i += 1) {
    const hipX = layout.originX + spanX * (i === 0 ? 0.26 : 0.74);
    const step = Math.sin(phase + i * Math.PI);
    const lift = Math.max(0, Math.cos(phase + i * Math.PI)) * reach * 0.22;
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
    // Lamplight pooling along the deck the crew walk on.
    batch.push(
      layout.originX,
      y + layout.floorH * 0.62,
      spanX,
      layout.floorH * 0.38,
      fade(palette.lamplight, 0),
      { colorBottom: fade(palette.lamplight, 0.09) },
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
  // A stalled room is drawn dim rather than badged. The tower going
  // quiet is the warning; see DECISIONS.md §8.
  const body = room.stalled ? mix(base, palette.roomStalled, 0.6) : base;
  batch.push(x, y, w, h, mix(body, palette.roomBodyLit, 0.25), {
    colorBottom: body,
    radius: 4,
  });

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

  // The Heartseed glows, gently, always.
  if (info?.category === "Heart") {
    const pulse = 0.55 + Math.sin(ctx.clock * 1.2) * 0.12;
    batch.push(x - 4, y - 4, w + 8, h + 8, fade(palette.roomHeart, pulse * 0.5), {
      radius: 10,
      softness: 8,
    });
  }
}

function drawShafts(batch: QuadBatch, { view, layout }: SceneContext): void {
  for (const shaft of view.tower.shafts) {
    const x = slotX(layout, shaft.slot);
    const top = floorY(layout, shaft.high);
    const bottom = floorY(layout, shaft.low) + layout.floorH;
    batch.push(x + 2, top, layout.slotW - 4, bottom - top, palette.shaft, { radius: 3 });

    // Treads, so stairs read as stairs at a glance.
    const treads = Math.max(1, Math.round((bottom - top) / 9));
    const busy = shaft.riders >= shaft.capacity;
    const rail = busy ? palette.shaftBusy : palette.shaftRail;
    for (let i = 0; i < treads; i += 1) {
      const ty = top + ((i + 0.5) * (bottom - top)) / treads;
      batch.push(x + 4, ty, layout.slotW - 8, 1.5, fade(rail, busy ? 0.75 : 0.4));
    }
    batch.push(x + 2, top, 2, bottom - top, fade(rail, 0.8));
    batch.push(x + layout.slotW - 4, top, 2, bottom - top, fade(rail, 0.8));
  }
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
  const shape = towerShape(view);

  // Every slot that would accept the room, so the player can see their
  // options before committing rather than probing for a rejection.
  for (const floor of view.tower.floors) {
    if (placeMode.maxFloor !== null && floor.index > placeMode.maxFloor) continue;
    for (let slot = 0; slot + placeMode.width <= floor.slots; slot += 1) {
      if (!slotRangeFree(view, floor.index, slot, placeMode.width)) continue;
      const x = slotX(layout, slot);
      const y = floorY(layout, floor.index);
      batch.push(
        x + 2,
        y + 3,
        placeMode.width * layout.slotW - 4,
        layout.floorH - 9,
        fade(palette.slotHint, 0.07),
        { radius: 4 },
      );
    }
  }

  const hover = placeMode.hover;
  if (!hover) return;
  const free = slotRangeFree(view, hover.floor, hover.slot, placeMode.width);
  const allowed =
    free &&
    hover.slot + placeMode.width <= shape.slots &&
    (placeMode.maxFloor === null || hover.floor <= placeMode.maxFloor);

  const x = slotX(layout, hover.slot);
  const y = floorY(layout, hover.floor);
  const color = allowed ? palette.ghostValid : palette.ghostBlocked;
  batch.push(
    x + 2,
    y + 3,
    placeMode.width * layout.slotW - 4,
    layout.floorH - 9,
    fade(color, 0.4),
    { radius: 4, softness: 2 },
  );
}

function drawVignette(batch: QuadBatch, { layout }: SceneContext): void {
  const { width, height } = layout.viewport;
  const band = height * 0.18;
  batch.push(0, height - band, width, band, fade(palette.vignette, 0), {
    colorBottom: fade(palette.vignette, 0.55),
  });
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
