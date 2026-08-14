/** Stable row-major cells in the optional 3x2 exterior emplacement atlas. */
export const WEAPON_COMPONENT_IDS = [
  "room.thorn_gun",
  "room.dart_battery",
  "room.lantern_mast",
  "room.root_ward",
  "room.tanglenet",
  "room.resonance_array",
] as const;

export type CombatFxFamily =
  | "thorn_projectile"
  | "dart_projectile"
  | "tanglenet_cast"
  | "lantern_pulse"
  | "root_ward_pulse"
  | "resonance_wave"
  | "creature_impact"
  | "hull_impact"
  | "room_breach"
  | "shaft_sever"
  | "mechanical_puff"
  | "creature_fade";

/** Stable row-major cells in the optional 4x3 combat-effect atlas. */
export const COMBAT_FX_FAMILIES: readonly CombatFxFamily[] = [
  "thorn_projectile",
  "dart_projectile",
  "tanglenet_cast",
  "lantern_pulse",
  "root_ward_pulse",
  "resonance_wave",
  "creature_impact",
  "hull_impact",
  "room_breach",
  "shaft_sever",
  "mechanical_puff",
  "creature_fade",
];

/** Pixel UV rectangle in either atlas; every registered cell is 256px square. */
export type CombatAtlasRect = readonly [x: number, y: number, width: 256, height: 256];

export function weaponComponentCell(roomId: string): CombatAtlasRect | undefined {
  const index = WEAPON_COMPONENT_IDS.indexOf(roomId as (typeof WEAPON_COMPONENT_IDS)[number]);
  return index < 0 ? undefined : [(index % 3) * 256, Math.floor(index / 3) * 256, 256, 256];
}

export function combatFxCell(family: CombatFxFamily): CombatAtlasRect {
  const index = COMBAT_FX_FAMILIES.indexOf(family);
  return [(index % 4) * 256, Math.floor(index / 4) * 256, 256, 256];
}
