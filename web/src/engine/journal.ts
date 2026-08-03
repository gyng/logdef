/**
 * The journal, and the run log.
 *
 * Two things that look similar and are not. A **run log** is a record of
 * one run, written when it ends, for the player to read and hand over
 * and for the difficulty pass to read in bulk (`SYSTEMS.md` §5.8). The
 * **journal** is what carries across runs: a list of things this player's
 * towers have done, and the unlocks those things opened.
 *
 * **Neither is game state, and that is the whole design.** `v2-plan.md`
 * §6.6 promises that a shared seed reproduces a run, and any unlock that
 * changed what a seed generates would break that silently — the same
 * seed producing a different world for a veteran than for a newcomer, in
 * a way neither could see. So nothing here enters `GameState`, nothing
 * here enters the replay, and what an unlock changes is *which commands
 * the player may send*. A replay carries the commands, so a replay of a
 * veteran's run works perfectly for somebody who has unlocked nothing:
 * they watch a tower build a room they cannot build themselves, which is
 * exactly right (§5.7).
 *
 * **And unlocks widen the toolkit, never the numbers.** A first-run tower
 * and a fiftieth-run tower start identical. The veteran has more things
 * they *could* build, and not one of them is stronger than what the
 * newcomer starts with — `v2-plan.md` §3's structural call, and the line
 * this file exists to hold.
 */

import type { ViewSnapshot } from "../bridge/types";

const STORE_KEY = "understory.journal.v1";

/**
 * Something a tower did that a tower had not done before.
 *
 * Deliberately *deeds* rather than milestones: "ran a forge" is a thing
 * you did, and "reached region 2" is a progress bar. Both are recorded
 * from the same place, but the phrasing is what a player reads, and a
 * journal that reads as a checklist is a checklist.
 */
export interface Deed {
  id: string;
  /** What the journal says happened. Past tense, plain. */
  said: string;
  /** Room and shaft ids this deed opens. Never a number, ever. */
  opens: string[];
}

/**
 * Everything a run can teach this player, and what it teaches them.
 *
 * The unlock table is *small on purpose*. Most of the pack is available
 * from the first run — a newcomer gets a whole game, not a demo — and
 * what is gated is the tier-two half, which is the part that only makes
 * sense once you have felt the chain it sits on top of. Gating the
 * canteen or the bunk would be gating comfort, which would be cruel and
 * would also make the first run the worst one.
 */
export const DEEDS: Deed[] = [
  {
    id: "walked",
    said: "Walked a tower out of the deep jungle.",
    // Nothing. Reaching region 2 is worth writing down and it does not
    // need to be worth *unlocking* — a deed with no reward is a deed
    // the journal records because it happened, which is what a journal
    // is for.
    opens: [],
  },
  {
    id: "berthed",
    said: "Stopped at a ruin and took it apart.",
    opens: ["room.sun_forge"],
  },
  {
    id: "forged",
    said: "Ran a forge, and made metal out of somebody else's wreck.",
    opens: ["room.fitter", "room.cellwright"],
  },
  {
    id: "fed_eight",
    said: "Fed eight people on a walking tower.",
    opens: ["room.garden"],
  },
  {
    id: "coast",
    said: "Reached the coast, where the canopy runs out.",
    opens: ["room.bombary", "room.seed_thrower"],
  },
  {
    id: "arrived",
    said: "Reached the Refugia.",
    opens: [],
  },
];

/**
 * **What is deliberately *not* gated, and why it took a broken run to
 * notice.**
 *
 * The first version of the table above opened the fiber comb, the
 * ropery and the chute on reaching region 2. That reads fine and is
 * wrong: from M5 an elevator costs rope, rope comes from a ropery, and
 * a ropery the player cannot build means **M1's centrepiece is
 * unreachable on a first run.** A newcomer would have played a whole
 * game without the thing the whole game is about, and `v2-plan.md` §3's
 * structural call — a first-run tower and a fiftieth-run tower start
 * identical — would have been quietly false.
 *
 * The rule that came out of it: **anything an existing system depends on
 * is never gated.** What unlocks is the tier-two half — the ruin
 * economy, the second emplacement, the garden — which is optional by
 * construction: a run that never builds any of it still reaches the
 * Refugia, and that is the test a candidate for this table has to pass.
 */

/** What one finished run is worth recording. */
export interface RunLog {
  /** Decimal string, so a 64-bit seed survives being written down. */
  seed: string;
  /** Ticks the run lasted. */
  ticks: number;
  days: number;
  paces: number;
  /** Region names in the order they were walked. */
  route: string[];
  /** Room ids built, in the order they were built. */
  built: string[];
  harvested: number;
  crafted: number;
  hauled: number;
  mealsEaten: number;
  itemsStolen: number;
  repelled: number;
  hpRepaired: number;
  crew: string[];
  /** How it ended. Never graded. */
  ending: "arrived" | "lost" | "abandoned";
  deeds: string[];
}

export interface Journal {
  /** Deed ids this player has ever done. */
  done: string[];
  /** Every run, newest last. */
  runs: RunLog[];
}

const EMPTY: Journal = { done: [], runs: [] };

export function loadJournal(): Journal {
  try {
    const raw = localStorage.getItem(STORE_KEY);
    if (!raw) return { ...EMPTY };
    const parsed = JSON.parse(raw) as Partial<Journal>;
    return {
      done: Array.isArray(parsed.done) ? parsed.done : [],
      runs: Array.isArray(parsed.runs) ? parsed.runs : [],
    };
  } catch {
    // A corrupt journal costs a player their unlocks and must never
    // cost them the game. Presentation degrades; the simulation is
    // authoritative and does not know this file exists.
    return { ...EMPTY };
  }
}

function save(journal: Journal): void {
  try {
    // Keep the last fifty runs. A journal is a record, not an archive,
    // and localStorage is not somewhere to put an unbounded list.
    const trimmed: Journal = { done: journal.done, runs: journal.runs.slice(-50) };
    localStorage.setItem(STORE_KEY, JSON.stringify(trimmed));
  } catch {
    // Private browsing, a full quota, a locked-down origin. None of
    // these is worth a crash.
  }
}

/**
 * Which rooms and shafts this player may build.
 *
 * Everything not named in any deed's `opens` is available from the
 * first run — the gate is an allowlist of *gated* things rather than of
 * permitted ones, so adding content to the pack makes it available by
 * default and gating it is the deliberate act.
 */
export function unlocked(journal: Journal): Set<string> {
  const gated = new Set(DEEDS.flatMap((deed) => deed.opens));
  const open = new Set<string>();
  for (const deed of DEEDS) {
    if (journal.done.includes(deed.id)) {
      for (const id of deed.opens) open.add(id);
    }
  }
  return new Set([...gated].filter((id) => open.has(id)));
}

/** Every id any deed gates. Everything else is available from run one. */
export const GATED: Set<string> = new Set(DEEDS.flatMap((deed) => deed.opens));

export function isLocked(journal: Journal, id: string): boolean {
  const gated = new Set(DEEDS.flatMap((deed) => deed.opens));
  return gated.has(id) && !unlocked(journal).has(id);
}

/**
 * What this run did that is worth writing down.
 *
 * **Read off the snapshot, never off a counter kept for the purpose.**
 * A number that exists only to be logged is a number that will drift
 * from the thing it claims to count, and `RunStats` already carries
 * everything a log needs (§5.8).
 */
export function deedsFrom(view: ViewSnapshot, builtIds: string[]): string[] {
  const done: string[] = [];
  if (view.journey.region >= 1) done.push("walked");
  if (view.journey.region >= 2) done.push("coast");
  if (view.stats.items_harvested > 0 && builtIds.includes("room.salvage_rig")) {
    done.push("berthed");
  }
  if (builtIds.includes("room.sun_forge")) done.push("forged");
  if (view.crew.length >= 8 && view.stats.meals_eaten > 0) done.push("fed_eight");
  if (view.journey.arrived) done.push("arrived");
  return done;
}

/**
 * Write one run down, and fold what it taught into the journal.
 *
 * **Not a score.** No rating, no grade, nothing that reads as a mark out
 * of ten (`DECISIONS.md` §8). A run's ending is a description of what
 * happened, and the only judgement anywhere in this file is the player's.
 */
export function recordRun(
  view: ViewSnapshot,
  extra: { seed: string; route: string[]; built: string[]; crew: string[] },
): Journal {
  const journal = loadJournal();
  const deeds = deedsFrom(view, extra.built);
  const log: RunLog = {
    seed: extra.seed,
    ticks: view.tick,
    days: view.clock.day + 1,
    paces: Math.floor(view.world.distance),
    route: extra.route,
    built: extra.built,
    harvested: view.stats.items_harvested,
    crafted: view.stats.crafts_completed,
    hauled: view.stats.hauls_completed,
    mealsEaten: view.stats.meals_eaten,
    itemsStolen: view.stats.items_stolen,
    repelled: view.siege.repelled,
    hpRepaired: view.stats.hp_repaired,
    crew: extra.crew,
    ending: view.journey.arrived ? "arrived" : view.siege.lost ? "lost" : "abandoned",
    deeds,
  };

  const next: Journal = {
    done: [...new Set([...journal.done, ...deeds])],
    runs: [...journal.runs, log],
  };
  save(next);
  return next;
}

/** Deeds this run did that this player had never done before. */
export function newlyLearned(before: Journal, deeds: string[]): Deed[] {
  return DEEDS.filter((deed) => deeds.includes(deed.id) && !before.done.includes(deed.id));
}
