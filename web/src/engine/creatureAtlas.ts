import type { EnemyStateTag } from "../bridge/types";

/** Stable species columns shared by the standing and optional motion atlases. */
export const CREATURE_ART_IDS = [
  "enemy.skitter",
  "enemy.root_borer",
  "enemy.glean_crow",
  "enemy.canopy_leaper",
  "enemy.night_prowler",
  "enemy.mire_hulk",
  "enemy.thicket_mother",
  "enemy.feral_warden",
] as const;

const CREATURE_ART_CELLS = new Map<string, number>(
  CREATURE_ART_IDS.map((id, index) => [id, index] as const),
);

/** Unknown pack additions keep their procedural silhouette. */
export function creatureArtCell(id: string): number | undefined {
  return CREATURE_ART_CELLS.get(id);
}

/**
 * Fixed row order in the optional 8-by-8 motion atlas.
 *
 * `hurt` is a readable weakened posture, not an invented impact event: the
 * bridge exposes current health but intentionally has no per-shot contract.
 */
export const CREATURE_MOTION_FRAMES = [
  "idle-approach-a",
  "idle-approach-b",
  "attack-a",
  "attack-b",
  "hurt",
  "dying",
  "leaving-a",
  "leaving-b",
] as const;

const HURT_THRESHOLD_PERMILLE = 350;

/** Select a motion-atlas row using simulation-derived values only. */
export function creatureMotionRow(
  state: EnemyStateTag,
  hpPermille: number,
  tick: number,
  enemyId: number,
): number {
  const stagger = Math.imul(enemyId, 17) >>> 0;
  switch (state) {
    case "dying":
      return 5;
    case "leaving":
      return 6 + (Math.floor((tick + stagger) / 12) % 2);
    case "attack":
      if (hpPermille <= HURT_THRESHOLD_PERMILLE) return 4;
      return 2 + (Math.floor((tick + stagger) / 5) % 2);
    case "approach":
      if (hpPermille <= HURT_THRESHOLD_PERMILLE) return 4;
      return Math.floor((tick + stagger) / 9) % 2;
  }
}
