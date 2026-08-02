/**
 * The solarpunk-tropical palette.
 *
 * Verdigris and warm brass against deep jungle green. The tower reads
 * warm and lived-in; the world outside reads cool, wet, and enormous.
 * Nothing in here is grey-brown military — see `DECISIONS.md` §8.
 *
 * Colours are straight RGBA in 0..1 because that is what the shader
 * wants; the hex literals next to each one are what you would paste
 * into a colour picker.
 */

import type { Color } from "./QuadBatch";

function hex(value: string, alpha = 1): Color {
  const digits = value.replace("#", "");
  if (digits.length !== 6) {
    // An 8-digit literal would silently shift the channels and produce
    // a colour nobody chose. Alpha goes in the second argument.
    throw new Error(`palette colour "${value}" must be 6 hex digits; pass alpha separately`);
  }
  const n = Number.parseInt(digits, 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255, alpha];
}

/** Same colour at a different opacity. */
export function fade(color: Color, alpha: number): Color {
  return [color[0], color[1], color[2], color[3] * alpha];
}

/** Blend two colours; `t` of 0 gives `a`. */
export function mix(a: Color, b: Color, t: number): Color {
  return [
    a[0] + (b[0] - a[0]) * t,
    a[1] + (b[1] - a[1]) * t,
    a[2] + (b[2] - a[2]) * t,
    a[3] + (b[3] - a[3]) * t,
  ];
}

export const palette = {
  // Sky: deep humid teal overhead, warm haze down at the treeline.
  skyHigh: hex("#2b6f74"),
  skyMid: hex("#69aca4"),
  skyLow: hex("#cfd9a8"),
  /** Mist sitting in the canopy at the horizon. */
  haze: hex("#b7cbb0"),

  // Terrain, by band. `far` is the silhouette against the sky, `near`
  // is the ground plane the tower walks on.
  canopyFar: hex("#20503c"),
  canopyNear: hex("#12301f"),
  clearingFar: hex("#4c8248"),
  clearingNear: hex("#2b5230"),
  ruinFar: hex("#537066"),
  ruinNear: hex("#2f4442"),

  ground: hex("#11241a"),
  groundLip: hex("#3c6b43"),

  // The tower: warm timber and weathered brass against all that green.
  towerShell: hex("#3a2f26"),
  towerShellLip: hex("#7a6244"),
  floorPlate: hex("#2b231c"),
  floorLit: hex("#3d3225"),
  floorEdge: hex("#8a7048"),
  leg: hex("#2f2820"),
  legJoint: hex("#a8834d"),

  roomBody: hex("#6b5a44"),
  roomBodyLit: hex("#96805e"),
  roomStalled: hex("#3f3831"),
  roomIntake: hex("#6b8f45"),
  roomProduction: hex("#a37a45"),
  roomStorage: hex("#527a86"),
  roomHeart: hex("#a85878"),

  bufferWell: hex("#1a1510"),
  bufferFill: hex("#e0b464"),
  outputFill: hex("#a8d46a"),
  progress: hex("#f0d488"),

  shaft: hex("#1c1611"),
  shaftRail: hex("#8a7350"),
  shaftBusy: hex("#e0b464"),

  crew: hex("#f7ead0"),
  crewCarrying: hex("#ffd98a"),
  crewStressed: hex("#e0714f"),
  crewShadow: hex("#000000"),

  cargo: hex("#b4dc72"),

  ghostValid: hex("#9fe08a"),
  ghostBlocked: hex("#e0714f"),
  slotHint: hex("#ffffff"),

  sunlight: hex("#fff3c4"),
  lamplight: hex("#ffc978"),
  vignette: hex("#08150e"),

  // Night. The sky does not just darken, it shifts hue — a blue-black
  // that makes the tower's own warm light the only warmth on screen.
  nightHigh: hex("#0a1622"),
  nightMid: hex("#12283a"),
  nightLow: hex("#1d3a3c"),
  moonlight: hex("#cfe0f0"),

  // Charge.
  charge: hex("#7fe0c4"),
  chargeLow: hex("#e0b455"),
  chargeEmpty: hex("#e0714f"),
} as const;

/**
 * Blend a colour toward night by `darkness` (0 = full day, 1 = full
 * night). Everything outdoors goes through this, so dusk reads as one
 * coherent change rather than a set of independently fading elements.
 */
export function atNight(color: Color, darkness: number): Color {
  return mix(color, mix(palette.nightMid, color, 0.18), darkness);
}

/** Far and near colours for a terrain band, by catalog terrain id. */
export function terrainColors(terrainId: string): { far: Color; near: Color } {
  switch (terrainId) {
    case "terrain.canopy":
      return { far: palette.canopyFar, near: palette.canopyNear };
    case "terrain.ruin_field":
      return { far: palette.ruinFar, near: palette.ruinNear };
    default:
      return { far: palette.clearingFar, near: palette.clearingNear };
  }
}

/** Body colour for a room, by category. */
export function roomColor(category: string): Color {
  switch (category) {
    case "Intake":
      return palette.roomIntake;
    case "Production":
      return palette.roomProduction;
    case "Storage":
      return palette.roomStorage;
    case "Heart":
      return palette.roomHeart;
    default:
      return palette.roomBody;
  }
}
