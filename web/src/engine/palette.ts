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
  // The drowned city's own band. Sun-struck bleached concrete over
  // standing water — paler and colder than a ruin field, so region 2
  // reads as somewhere else rather than as region 1 with more rubble
  // (`SYSTEMS.md` §3.2).
  drownedFar: hex("#8aa79b"),
  drownedNear: hex("#23403e"),

  ground: hex("#11241a"),
  groundLip: hex("#3c6b43"),

  // Salvage. Warm worked metal against cold wet stone: a ruin with
  // something left in it has to be tellable from a stripped one at the
  // distance the player first sees it, and warm-against-cold is the
  // only cue that survives the haze the far parallax layer is washed
  // with (`SYSTEMS.md` §3.4).
  salvage: hex("#d9a25a"),
  salvageLit: hex("#f5d79a"),
  /** A ruin with nothing left: colder, lower, and taken by the green. */
  stripped: hex("#33453e"),

  // The route splitting. Warm timber and brass, because a waypost is
  // something people put there — the one man-made thing on the strip
  // that is not the tower.
  wayPost: hex("#8a6b41"),
  wayBoard: hex("#e0c07a"),
  /** Ground the tower has not been told how to draw yet. */
  undecided: hex("#cbd9ca"),
  /** The far edge of the journey: open, bright, and nothing past it. */
  farEdge: hex("#ecdfba"),

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
  roomEnergy: hex("#4f8f83"),
  roomDefence: hex("#7b6f9a"),
  roomHeart: hex("#a85878"),
  /** Quarters. Dyed cloth and slung rope — the one soft room. */
  roomQuarters: hex("#9a6a72"),

  // Aged copper and worked brass, named rather than borrowed.
  //
  // Shell plating and shaft rails were both drawn in `roomStorage`,
  // which is a cool blue-green doing double duty; naming the two metals
  // is most of what makes them read as *aged copper on a walking house*
  // rather than as paint. Verdigris is the oxide, brass is what is
  // still polished by hands.
  verdigris: hex("#5f8f80"),
  verdigrisDeep: hex("#3a5b52"),
  brass: hex("#c69a54"),

  // The canteen's hearth: the warmest thing in the tower, and the
  // kitchen chain's own stall signal. A cold hearth is *why* people are
  // going hungry, in the same place you notice that they are.
  hearth: hex("#ffb057"),
  hearthCore: hex("#fff0c4"),
  steam: hex("#e8e4d6"),
  /** Slung canvas, and the blanket over a sleeper. */
  hammock: hex("#c88f7a"),
  blanket: hex("#7d5b6a"),

  // Damage. The tower is timber and living matter, so it splits and
  // slumps rather than denting: a bruised body, a black split, raw
  // pale wood where something broke through.
  hurt: hex("#4a3a2e"),
  wreck: hex("#241c17"),
  crack: hex("#120c08"),
  splinter: hex("#c9a877"),
  /** What shows through a hole in the tower's skin. */
  breach: hex("#0b1712"),

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
  // **Working clothes, and they are this light on purpose.**
  //
  // The first pass used a muted forest green (#6d7f6a) and a muted
  // brown, which is what people in a jungle would sensibly wear and is
  // almost exactly the colour of the tower's unlit interior. Looked at
  // in `home-evening.png` the crew were technically in frame and
  // effectively invisible — and "a frame with no people in it cannot
  // pass" (`SYSTEMS.md` §4.9) makes crew legibility the load-bearing
  // thing in the whole art pass, not a detail. Sun-bleached linen reads
  // against a dark deck at any hour.
  crewCloth: hex("#c9d3b4"),
  crewClothWarm: hex("#e0c39a"),
  /** Skin, for the head, kept warm against all the green. */
  crewSkin: hex("#f0d4ad"),

  // Overgrowth beyond the planters: vines between floors, moss at the
  // shell lip, growth thickening on the leeward side. All of it is
  // stable per floor from `hash01`, so it sits still frame to frame
  // rather than crawling.
  vine: hex("#3f6b3a"),
  vineDeep: hex("#27441f"),
  moss: hex("#4e7a44"),
  /** Dappling through the canopy, and fireflies after dark. */
  dapple: hex("#fff3c4"),
  firefly: hex("#d8ff9a"),

  cargo: hex("#b4dc72"),
  // **A load's own colour, so you can see what is moving and not only
  // that something is.**
  //
  // `DESIGN.md` insight 1 is that transport is contested — every chain
  // competes for the same stairs — and a player can only reason about
  // that if they can tell a stalk of bamboo from a coil of rope going up
  // them. Until now every load was the same green crate, so the
  // cross-section reported *that* the tower was busy and never *what
  // with*.
  //
  // Keyed by item id, in the renderer rather than in the content pack:
  // this is a presentation choice, and `ItemInfo` carries a glyph and an
  // order because those are the only two the simulation needed. Anything
  // missing falls back to `cargo`, so a new item is drab rather than
  // invisible.
  cargoOf: {
    "item.bamboo": hex("#9ecb63"),
    "item.poles": hex("#c39a5e"),
    "item.fiber": hex("#d8d2a6"),
    "item.rope": hex("#b08d5c"),
    "item.scrap": hex("#8d9aa0"),
    "item.alloy": hex("#c8d4dc"),
    "item.darts": hex("#8fb9a4"),
    "item.meals": hex("#e0a866"),
    "item.resin_feedstock": hex("#d4737b"),
    "item.mechanisms": hex("#a6a2b8"),
    "item.charge_cells": hex("#7fd4c8"),
    "item.resonator_drums": hex("#b7a05e"),
  } as Record<string, Color>,

  // Creatures. Wet, dark, forest-coloured things with their own light
  // in them — not a target gallery, and nothing here is gunmetal. The
  // accent is the one part that differs by approach, so the three
  // silhouettes stay legible at a glance.
  creature: hex("#3d322f"),
  creatureLit: hex("#6b564a"),
  /** Where a weakened creature fades to: the mist, not a corpse. */
  creatureSpent: hex("#8b9a8c"),
  creatureEye: hex("#f2b45e"),
  creatureGround: hex("#8a7a3e"),
  creatureCanopy: hex("#7d6a92"),
  creatureBurrow: hex("#bcac90"),
  /** Earth turned over by something coming up through it. */
  spoil: hex("#4a3b2a"),

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

  // Charge. Two colours, not three: these were once a green/amber/red
  // gauge ramp that nothing ever drew, and an amber "getting low" step
  // is a badge — `DECISIONS.md` §8 wants the roof rack emptying to be
  // the signal instead. What is left is the stored charge and the cold
  // casing it sits in.
  charge: hex("#7fe0c4"),
  /** A spent cell in the roof rack. Cold, not black — it is still a cell. */
  chargeEmpty: hex("#2f3f42"),
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
    case "terrain.drowned_street":
      return { far: palette.drownedFar, near: palette.drownedNear };
    default:
      return { far: palette.clearingFar, near: palette.clearingNear };
  }
}

/** The accent that tells one creature's approach from another. */
export function creatureColor(approach: string): Color {
  switch (approach) {
    case "Canopy":
      return palette.creatureCanopy;
    case "Burrow":
      return palette.creatureBurrow;
    default:
      return palette.creatureGround;
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
    case "Energy":
      return palette.roomEnergy;
    case "Defence":
      return palette.roomDefence;
    case "Heart":
      return palette.roomHeart;
    case "Quarters":
      return palette.roomQuarters;
    default:
      return palette.roomBody;
  }
}
