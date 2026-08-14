import type { CrewStateTag } from "../bridge/types";

/**
 * Authored crew use the same name-stable cell order everywhere their art
 * appears. Names, rather than runtime ids, survive recruitment order and make
 * a portrait stay attached to the same person across seeds.
 */
export const CREW_ART_NAMES = [
  "Wren",
  "Odile",
  "Bakri",
  "Sena",
  "Toma",
  "Ilay",
  "Rook",
  "Mira",
] as const;

const CREW_ART_CELLS = new Map<string, number>(
  CREW_ART_NAMES.map((name, index) => [name, index] as const),
);

/** Unknown recruits retain a deterministic cosmetic fallback. */
export function crewArtCell(name: string, fallback: number): number {
  return CREW_ART_CELLS.get(name) ?? Math.abs(fallback) % CREW_ART_NAMES.length;
}

/** 8 animation rows: two restrained frames for each readable posture. */
export type CrewMotionPose = "idle" | "walk" | "work" | "carry" | "eat" | "sleep";

const CREW_MOTION_FIRST_ROW: Record<CrewMotionPose, number> = {
  idle: 0,
  walk: 2,
  work: 4,
  carry: 6,
  eat: 8,
  sleep: 10,
};

export function crewMotionPose(state: CrewStateTag, carrying: boolean): CrewMotionPose {
  if (state === "sleep") return "sleep";
  if (state === "eat") return "eat";
  if (carrying || state === "load" || state === "unload") return "carry";
  if (state === "walk" || state === "climb") return "walk";
  if (state === "mend" || state === "man" || state === "shoo") return "work";
  return "idle";
}

/**
 * Choose a two-frame cycle from simulation-derived facts only. Locomotion is
 * tied to position so a tired person takes slower steps; restrained standing
 * poses advance with simulation ticks and stop while paused.
 */
export function crewMotionRow(
  pose: CrewMotionPose,
  locomoting: boolean,
  tick: number,
  floor: number,
  slot: number,
  fidget: number,
): number {
  const frame =
    pose === "walk" || locomoting
      ? Math.sin(slot * Math.PI * 2.6 + floor * 5 + (fidget / 65535) * Math.PI * 2) >= 0
        ? 0
        : 1
      : Math.floor(
          (tick + (fidget % 37)) /
            (pose === "work" || pose === "eat" ? 10 : pose === "sleep" ? 45 : 20),
        ) % 2;
  return CREW_MOTION_FIRST_ROW[pose] + frame;
}
