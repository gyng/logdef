import { expect, test, type Page } from "@playwright/test";

import type { ViewSnapshot } from "../src/bridge/types";

/**
 * Not a test — a screenshot harness for looking at the game.
 *
 * `npx playwright test capture` writes stills of the tower at a few
 * points in a run. M0's exit criterion is a question you answer by
 * looking ("does the walking tower feel alive on one screen?"), and
 * M3's are the same shape: whether a ruin worth stopping at is
 * tellable from a stripped one across the frame, and whether a tower
 * halted at a fork reads as waiting rather than as hung.
 *
 * **Everything here that steps the engine must answer forks.** The
 * tower halts at a split it has not been told about, so a harness that
 * ignores them photographs a parked tower with total confidence —
 * `SYSTEMS.md` §3.3 names this as the failure mode to design against,
 * and this file has hit it twice. `arm()` installs the stepping the
 * rest of the harness uses; raw `hooks.step` is only correct inside a
 * loop that answers on its own.
 *
 * **Known flaky: the two approach stills, and it is this file's fault
 * rather than the game's.** `tower-approach.png` and
 * `tower-closing.png` need a creature closing on the tower, and on some
 * runs they come out empty. Three things were ruled out while chasing
 * it, and they are written down so nobody chases them again:
 *
 * - *Not the budget.* Raising the search from 25,000 ticks to 90,000
 *   made it worse — the tower reaches the far edge and photographs
 *   that, which is what the budget was lowered to prevent.
 * - *Not a missed window.* Instrumented, the search reports **zero
 *   ticks with anything alive anywhere in the world**, so there was no
 *   window to miss.
 * - *Not something to provoke away.* Walking 40,000 ticks with both
 *   arms running took provocation from 0 to **15**, against a
 *   glean-crow's `min_provocation` of 250. This tower's storerooms fill
 *   and its arms jam, so it harvests almost nothing and never draws
 *   attention; the creatures that do appear are left over from waves
 *   much earlier in the run.
 *
 * The actual cause is that **this harness is not deterministic.** It
 * interleaves `hooks.step()` with real-time `waitForTimeout`s while the
 * renderer's own frame loop is also ticking, so the same seed lands in
 * different places on different runs — two consecutive runs berthed at
 * the drowned city and at the coast respectively. Fixing it means
 * freezing the frame loop for the duration, which is a bigger change
 * than a screenshot harness has earned. Until then: if a still is
 * empty, re-run before believing it.
 *
 * A chute would keep the arms clear and was tried. It does not fit —
 * slot 7 is the only column free on every floor of this tower, and the
 * dart battery is already there. That is the design working (a shaft is
 * a tax on every floor it passes) rather than a bug to route around.
 */

declare global {
  interface Window {
    /** Harness-only stepping. Installed by `arm`, never shipped. */
    __capture?: {
      /** Step `ticks`, answering any fork encountered on the way. */
      walk(ticks: number, stride?: number): void;
      /** Answer the pending fork, if any. Returns the way taken. */
      answer(): string | null;
      /** Screen point of the first free slot on a floor, if any. */
      freeSlot(floor: number, width: number): { x: number; y: number } | null;
      /** The first gap `width` slots wide anywhere in the tower. */
      freeSlotAny(width: number): { floor: number; slot: number } | null;
      /** Screen point of a slot clear on every floor of a span. */
      freeColumn(low: number, high: number): { x: number; y: number } | null;
    };
  }
}

async function arm(page: Page): Promise<void> {
  await page.evaluate(() => {
    const hooks = window.__understory!;

    const answer = (): string | null => {
      const fork = hooks.view().journey.fork;
      if (!fork || fork.answer !== null) return null;
      const catalog = hooks.catalog();
      const [left, right] = fork.branches;
      // Take the quieter of the two. An unattended harness that always
      // picked the loud way would measure a harder game than anybody
      // plays, and the choice has to be made from the branch's own data
      // for the same reason the card is.
      const side =
        (catalog.branches[left]?.threat_pct ?? 100) <= (catalog.branches[right]?.threat_pct ?? 100)
          ? 0
          : 1;
      hooks.send({ TakeFork: { branch: side } });
      return catalog.branches[fork.branches[side]]?.name ?? null;
    };

    // A fork is visible 900 paces out — 1,500 ticks — so a 300-tick
    // stride always sees one well before the tower reaches the line.
    const walk = (ticks: number, stride = 300): void => {
      for (let left = ticks; left > 0; left -= stride) {
        answer();
        hooks.step(Math.min(stride, left));
      }
      answer();
    };

    const freeSlot = (floor: number, width: number): { x: number; y: number } | null => {
      const view = hooks.view();
      const deck = view.tower.floors[floor];
      if (!deck) return null;
      for (let slot = 0; slot + width <= deck.slots; slot += 1) {
        const end = slot + width;
        const takenByRoom = deck.rooms.some(
          (room) => slot < room.slot + room.width && room.slot < end,
        );
        const takenByShaft = view.tower.shafts.some(
          (shaft) =>
            shaft.low <= floor && floor <= shaft.high && shaft.slot >= slot && shaft.slot < end,
        );
        if (!takenByRoom && !takenByShaft) return hooks.slotPoint(floor, slot);
      }
      return null;
    };

    const freeSlotAny = (width: number): { floor: number; slot: number } | null => {
      const view = hooks.view();
      for (const deck of view.tower.floors) {
        for (let slot = 0; slot + width <= deck.slots; slot += 1) {
          const end = slot + width;
          const takenByRoom = deck.rooms.some(
            (room) => slot < room.slot + room.width && room.slot < end,
          );
          const takenByShaft = view.tower.shafts.some(
            (shaft) =>
              shaft.low <= deck.index &&
              deck.index <= shaft.high &&
              shaft.slot >= slot &&
              shaft.slot < end,
          );
          if (!takenByRoom && !takenByShaft) return { floor: deck.index, slot };
        }
      }
      return null;
    };

    /**
     * A slot free on *every* floor of a span, which is what a shaft
     * needs and `freeSlot` does not check.
     *
     * `freeSlot(0, 1)` asks "is this clear on the ground floor", and a
     * shaft claims a column on every deck it passes through — so a
     * point it returns is very often a placement the engine then
     * rejects with `SlotOccupied` naming a floor two decks up. That is
     * a silent no-op in a harness: the button was enabled, the click
     * landed, and nothing was built. Measured on seed 4242 by asking
     * the bridge directly, seven of eight columns were blocked and only
     * slot 7 would take a chute, while `freeSlot` cheerfully pointed at
     * slot 4.
     */
    const freeColumn = (low: number, high: number): { x: number; y: number } | null => {
      const view = hooks.view();
      const decks = view.tower.floors.filter((deck) => deck.index >= low && deck.index <= high);
      if (decks.length === 0) return null;
      const slots = Math.min(...decks.map((deck) => deck.slots));
      for (let slot = 0; slot < slots; slot += 1) {
        const clear = decks.every(
          (deck) =>
            !deck.rooms.some((room) => room.slot <= slot && slot < room.slot + room.width) &&
            !view.tower.shafts.some(
              (shaft) => shaft.low <= deck.index && deck.index <= shaft.high && shaft.slot === slot,
            ),
        );
        if (clear) return hooks.slotPoint(low, slot);
      }
      return null;
    };

    window.__capture = { walk, answer, freeSlot, freeSlotAny, freeColumn };
  });
}

/**
 * Where the run has got to, on the console.
 *
 * A screenshot harness that has silently walked off the end of the
 * world takes perfectly good pictures of nothing in particular, so
 * every phase reports the tick, the distance, the region and the halt
 * it finished at. Cheap, and it is what turns "the still is wrong" into
 * "the still is of tick 190,000".
 */
async function where(page: Page, label: string): Promise<void> {
  const at = await page.evaluate(() => {
    const hooks = window.__understory!;
    const view = hooks.view();
    const region = hooks.catalog().regions[view.journey.region]?.name ?? "?";
    return `tick ${view.tick}, ${Math.round(view.world.distance)} paces, ${region} ${Math.round(
      view.journey.region_permille / 10,
    )}%, ${view.journey.halt}, standing ${Math.round(
      view.siege.integrity_permille / 10,
    )}%, provocation ${view.siege.provocation}/${view.siege.provocation_max}, seen off ${
      view.siege.repelled
    }`;
  });
  console.log(`${label}: ${at}`);
}

/**
 * Step until a wave is `within` paces of the tower in daylight, and
 * stop it there. Reports what was in frame when it stopped.
 *
 * The reason this is fine-grained: the harness used to advance the
 * siege 600 ticks at a stride, which is longer than an entire approach
 * (a skitter closes 540 paces in about 540 ticks), so every still it
 * ever took was of a wave already standing on the tower. "Is the
 * approach watchable?" is a question about the half-minute before that,
 * and it cannot be answered by a harness that jumps over it.
 */
async function stepUntilWithin(page: Page, within: number): Promise<string> {
  return page.evaluate((limit) => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    // A window rather than a ceiling: by the time the batteries are up
    // there is usually already a wave halfway in, and "nearest is under
    // 380" would be satisfied by one at 20. Anything nearer than the
    // window means waiting out the current wave for the next one.
    const floor = Math.max(0, limit - 80);
    // Bounded well under a journey. This loop used to have two hundred
    // thousand ticks to play with, which is more than the whole walk —
    // so when a wave did not turn up it did not give up, it walked to
    // the far edge of the world and photographed that instead, and
    // every still after it was of a tower that had already arrived.
    let budget = 25_000;
    let seen = 0;
    let closest = Infinity;
    while (budget > 0) {
      const view = hooks.view();
      if (view.journey.arrived) return "the journey ended first";
      const gaps = view.siege.enemies
        .filter((enemy) => enemy.state === "approach")
        .map((enemy) => enemy.at - view.world.distance)
        .filter((gap) => gap >= 0);
      const nearest = gaps.length > 0 ? Math.min(...gaps) : null;
      seen += view.siege.enemies.length > 0 ? 1 : 0;
      if (nearest !== null) closest = Math.min(closest, nearest);
      // Daylight, because a silhouette against a night sky answers a
      // different question than the one being asked here.
      if (view.clock.sun_pct > 40 && nearest !== null && nearest <= limit && nearest >= floor) {
        return view.siege.enemies
          .map(
            (enemy) =>
              `${catalog.enemies[enemy.def]?.approach ?? "?"} at ${Math.round(
                enemy.at - view.world.distance,
              )}p`,
          )
          .join(", ");
      }
      // Coarse while nothing is close, fine once something is, so a
      // stride never jumps clean over the window being looked for.
      const stride =
        nearest === null || nearest < floor ? 60 : Math.max(1, Math.round((nearest - limit) * 0.4));
      window.__capture!.answer();
      hooks.step(stride);
      budget -= stride;
    }
    return `nothing in frame (${seen} tick(s) with anything alive, closest approach ${
      closest === Infinity ? "none" : Math.round(closest)
    }p)`;
  }, within);
}

test("capture stills", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/?seed=4242");
  await page.waitForFunction(() => window.__understory !== undefined, null, { timeout: 20_000 });
  await arm(page);

  await page.evaluate(() => {
    window.__capture!.walk(900);
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/tower-early.png" });

  await page.evaluate(() => {
    window.__capture!.walk(5400);
  });
  await page.getByTestId("speed-X4").click();
  await page.waitForTimeout(600);
  await page.screenshot({ path: "capture/tower-running.png" });

  // **The elevator is made of rope from M5**, so the tower has to run a
  // fiber comb and a ropery before the build button is even enabled —
  // and the ropery has to be switched off again once there is rope, or
  // it claims every shelf and the poles never arrive. Without this the
  // capture clicked a disabled button and the elevator stills were
  // stills of a tower with no elevator.
  await page.evaluate(() => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    const idOf = (id: string): number => catalog.items.findIndex((item) => item.id === id);
    const held = (item: number): number =>
      hooks.view().stock.find((entry) => entry.item === item)?.count ?? 0;
    const place = (room: string): void => {
      for (let attempt = 0; attempt < 60; attempt += 1) {
        for (const floor of hooks.view().tower.floors) {
          for (let slot = 0; slot + 2 < floor.slots; slot += 1) {
            if (hooks.send({ PlaceRoom: { room, floor: floor.index, slot } }) === "Ok") return;
          }
        }
        window.__capture!.walk(600);
      }
    };
    place("room.fiber_comb");
    place("room.ropery");

    const rope = idOf("item.rope");
    const poles = idOf("item.poles");
    let off = false;
    for (let i = 0; i < 200; i += 1) {
      if (!off && held(rope) >= 12) {
        off = true;
        for (const floor of hooks.view().tower.floors) {
          for (const room of floor.rooms) {
            const id = catalog.rooms[room.def]?.id ?? "";
            if (id === "room.ropery" || id === "room.fiber_comb") {
              hooks.send({
                SetRoomActive: { floor: floor.index, slot: room.slot, active: false },
              });
            }
          }
        }
      }
      if (held(poles) >= 18 && held(rope) >= 6) break;
      window.__capture!.walk(600);
    }
  });
  await page.waitForTimeout(200);

  await page.getByTestId("build-shaft.elevator").click();
  await page.waitForTimeout(200);
  const point = await page.evaluate(() => window.__capture!.freeSlot(0, 1));
  if (point) {
    await page.mouse.move(point.x, point.y);
    await page.waitForTimeout(200);
    await page.screenshot({ path: "capture/tower-placing.png" });
    await page.mouse.click(point.x, point.y);
  }

  // Run on until the elevator has crew in it and the sun has moved.
  await page.evaluate(() => {
    window.__capture!.walk(5400);
  });
  await page.waitForTimeout(400);
  await page.screenshot({ path: "capture/tower-elevator.png" });

  // And a night, which is when the charge economy has teeth.
  await page.evaluate(() => {
    const catalog = window.__understory!.catalog();
    const view = window.__understory!.view();
    // Step to just past dusk.
    const target = Math.round(catalog.ticks_per_day * 0.92);
    const now = Math.round((view.clock.permille / 1000) * catalog.ticks_per_day);
    const wait = (target - now + catalog.ticks_per_day) % catalog.ticks_per_day;
    window.__capture!.walk(wait, 120);
  });
  await page.waitForTimeout(400);
  await page.screenshot({ path: "capture/tower-night.png" });

  // And a wave. M2's exit criterion is the same kind of question M0's
  // was — whether a siege reads as drama on the cross-section without
  // an aimed weapon — so the harness has to be able to show one.
  //
  // Provoked the way a player provokes: a second cutter arm, and then
  // time. Nothing here reaches past the buttons the UI actually has.
  const armButton = page.getByTestId("build-room.cutter_arm");
  if (!(await armButton.isDisabled())) await armButton.click();
  const armPoint = await page.evaluate(() => window.__capture!.freeSlot(0, 1));
  if (armPoint) {
    await page.mouse.click(armPoint.x, armPoint.y);
  }
  // Somewhere to put what the second arm strips, and something to make
  // darts with. Without these the storerooms fill, the arms jam, and
  // the tower goes quiet again — which is correct behaviour and a
  // useless screenshot.
  //
  // Slots chosen against the current widths: floor 1 holds the stairs,
  // a one-wide cell bank at 1 and a two-wide storeroom at 2-3, so 4 is
  // the only three-wide gap in the tower and the rig is three wide.
  // These placements used to be silent no-ops after the widths changed
  // — the rig never got built, so the tower never had scrap, so the
  // enclave still photographed a board nobody could buy from.
  for (const [room, floor, slot] of [
    ["room.storeroom", 2, 5],
    ["room.storeroom", 3, 5],
    ["room.thornwright", 3, 1],
    ["room.dart_battery", 1, 7],

    // Built here rather than next to the ruin stills below, because by
    // then the tower is deep enough into the journey that the arrival
    // overlay can be up, and an overlay eats the click.
    ["room.salvage_rig", 1, 4],
  ] as const) {
    // **Clear the ground floors first.** Three rooms reach the ground
    // and therefore carry `max_floor: 1` — the cutter arm, the salvage
    // rig, and from M5 the fiber comb — and two floors do not hold all
    // three once the Heartseed, a cell bank and a storeroom have taken
    // theirs. The comb has done its job by now (there is rope banked
    // and the ropery is off), so it is what goes, which is also what a
    // player would do. Without this the rig's button is simply disabled
    // and the ruin stills are stills of a tower with no rig.
    if (room === "room.salvage_rig") {
      await page.evaluate(() => {
        const hooks = window.__understory!;
        const catalog = hooks.catalog();
        for (const deck of hooks.view().tower.floors) {
          for (const item of deck.rooms) {
            if (catalog.rooms[item.def]?.id === "room.fiber_comb") {
              hooks.send({ RemoveRoom: { floor: deck.index, slot: item.slot } });
            }
          }
        }
      });
      await page.waitForTimeout(150);
    }
    // **Skip what the tower cannot pay for rather than waiting on it.**
    // This clicked the button and let Playwright block until timeout,
    // which turns "the economy got tighter" into "the harness hung for
    // three minutes and produced nothing". A capture is a tool for
    // looking: it should take the stills it can and say what it missed.
    const button = page.getByTestId(`build-${room}`);
    if (await button.isDisabled()) {
      console.log(`capture: skipped ${room} — the tower could not build it`);
      continue;
    }
    await button.click();
    const spot = await page.evaluate(([f, s]) => window.__understory!.slotPoint(f, s), [
      floor,
      slot,
    ] as const);
    if (spot) await page.mouse.click(spot.x, spot.y);
    // Poles are scarce; give the mill time to make the next lot.
    await page.evaluate(() => {
      window.__capture!.walk(3000);
    });

    // And check it actually went up.
    //
    // This harness spent a while placing a three-wide rig at slot 6 of
    // an eight-slot floor and photographing the tower that resulted, and
    // it is the third script in this project to quietly measure a tower
    // it thought it had built — the others being `siege_run.rs` failing
    // to afford a thornwright and the golden recorder standing at a fork
    // for half its run. Every one of them sent a command and did not
    // look at what came back. So: look.
    const standing = await page.evaluate(
      ([f, s]) =>
        window
          .__understory!.view()
          .tower.floors[f]?.rooms.some((r) => r.slot <= s && s < r.slot + r.width) ?? false,
      [floor, slot] as const,
    );
    if (!standing) {
      throw new Error(
        `${room} was never built at floor ${String(floor)} slot ${String(slot)} — ` +
          "the still would show a tower missing the thing it is about",
      );
    }
  }

  // Everything below is paused, so each still lands exactly where it
  // was asked for: `step` ignores the speed setting, but the render
  // loop does not, and at 4x the couple of hundred milliseconds a
  // screenshot needs is another forty paces of closing.
  await page.getByTestId("speed-Paused").click();

  // ── M3 ────────────────────────────────────────────────────────────

  // Somewhere for scrap to go. The rig's outbox holds eight and the
  // chain has no consumer for scrap, so once every shelf is spoken for
  // by bamboo, poles and darts the rig stalls and the salvage stills
  // photograph a ruin nobody is ever going to finish.
  //
  // Two, and no more: growing taller to make room for four shades the
  // canopy sails off the roof, and a tower with no charge income in the
  // drowned city browns out, stops harvesting, stops making darts and
  // is taken apart by the wardens the berth itself roused. That run
  // ended at seven per cent standing and never reached region 2 — the
  // §3.6 spiral, arrived at by trying to buy shelf space.
  for (let i = 0; i < 2; i += 1) {
    const store = page.getByTestId("build-room.storeroom");
    // **Find the gap before selecting the card, and never leave place
    // mode dangling.** This used to click the card, then look for a
    // slot, and do nothing if there was not one — leaving the card
    // pressed and place mode open. Next time round it then waited three
    // minutes for a button place mode had disabled, and the run timed
    // out.
    //
    // It only started biting when the renderer's cost changed, because
    // this file drives the sim through real-time waits while the frame
    // loop is also ticking, so anything moving the frame rate moves the
    // tower's state at each step. The ordering below does not care.
    const gap = await page.evaluate(() => {
      const at = window.__capture!.freeSlotAny(2);
      return at === null ? null : window.__understory!.slotPoint(at.floor, at.slot);
    });
    if (!gap) {
      console.log("capture: skipped a storeroom — nowhere two slots wide left");
      break;
    }
    if (await store.isDisabled()) {
      console.log("capture: skipped a storeroom — the tower could not build it");
      break;
    }
    // `force` because affordability can flip between that check and this
    // click: the sim runs underneath and repair spends poles. A missed
    // storeroom is the worse outcome; a forced click on a card that has
    // just gone disabled is a no-op.
    await store.click({ force: true, timeout: 5_000 }).catch(() => {
      console.log("capture: the storeroom card went away mid-click");
    });
    await page.mouse.click(gap.x, gap.y);
    await page.evaluate(() => {
      window.__capture!.walk(2400);
    });
  }

  // Ruins, before anything has been taken out of them: the strip has to
  // say which ones are worth the stop.
  const berth = await page.evaluate(() => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    hooks.send({ SetStriding: { walking: true } });
    const isRuin = (f: { band: number; kind: number }) =>
      catalog.terrain[f.band]?.ruin_kinds[f.kind] === true;
    // Drawn clear of the tower's own body. `worldX` puts the tower's
    // distance at its *left* edge, so everything within about sixteen
    // paces ahead is behind the cross-section.
    const framed = (f: { at: number; layer: number; scale: number }, distance: number) => {
      const at = hooks.featurePoint(f as never, distance);
      return at !== null && at.x > 1120 && at.x < 1580;
    };

    let budget = 120_000;
    while (budget > 0) {
      const view = hooks.view();
      if (view.journey.arrived) break;
      const rich = view.world.features.filter((f) => isRuin(f) && f.salvage > 0);
      const gaps = rich.map((f) => Math.abs(f.at - view.world.distance));
      const closest = gaps.length > 0 ? Math.min(...gaps) : null;

      // The rig always works the *nearest* ruin it can reach, so the
      // only berth worth photographing is one where the nearest ruin is
      // also the one on screen. Berthing next to a visible ruin that
      // happened not to be the nearest — which is what this asked for
      // first — spends forty-five thousand ticks emptying something
      // hidden behind the tower and photographs no change at all.
      if (closest !== null && closest < 52 && view.clock.sun_pct > 50) {
        const target = rich.find((f) => Math.abs(f.at - view.world.distance) === closest);
        const others = rich.filter((f) => f !== target && framed(f, view.world.distance));
        if (target && framed(target, view.world.distance) && others.length > 0) {
          hooks.send({ SetStriding: { walking: false } });
          hooks.step(2);
          return `${target.salvage} nearest, ${others.map((f) => f.salvage).join("/")} beyond it`;
        }
      }
      window.__capture!.answer();
      const stride = closest !== null && closest < 80 ? 4 : 120;
      hooks.step(stride);
      budget -= stride;
    }
    return "none in reach";
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/ruin-rich.png" });
  // The same frame is a tower the player stopped: still legs, planted
  // feet, and nothing waiting on an answer. It has to be tellable from
  // the two fork stills below.
  await page.screenshot({ path: "capture/halt-stopped.png" });

  // Now let the rig work. It takes the nearest ruin first and moves on
  // when that one is empty, so a long berth leaves a trail of stripped
  // ones around the tower with the further ones still holding — which
  // is the comparison the whole mechanic rests on (`SYSTEMS.md` §3.4).
  const stripped = await page.evaluate(() => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    // Everything the rig can reach, and everything of that which is
    // drawn clear of the tower's own body — the second list is the one
    // the still has to show a difference across.
    const reachable = (view: ViewSnapshot) =>
      view.world.features.filter(
        (f) =>
          catalog.terrain[f.band]?.ruin_kinds[f.kind] === true &&
          Math.abs(f.at - view.world.distance) < 58,
      );
    const framed = (view: ViewSnapshot) =>
      reachable(view).filter((f) => {
        const at = hooks.featurePoint(f, view.world.distance);
        return at !== null && at.x > 1120 && at.x < 1580;
      });
    const before = framed(hooks.view())
      .map((f) => f.salvage)
      .join("/");

    // Shut the cutter arms for the duration — and only those. Measured,
    // the rig is crew-bound rather than rig-bound: its outbox holds
    // eight and it stalls there until somebody carries the scrap
    // upstairs, so against a chain also hauling bamboo it managed 28
    // units in 60,000 ticks against an authored rate of one every 60.
    // Freeing the porters is a thing a player berthing for salvage
    // would actually do. Shutting the *whole* chain, which is what this
    // did first, silences the thornwright too — and a berth rouses
    // wardens, so a tower that stops making darts while standing still
    // next to a ruin it is robbing gets taken apart. That run ended at
    // eight per cent standing and never walked again.
    const paused: { floor: number; slot: number }[] = [];
    for (const floor of hooks.view().tower.floors) {
      for (const room of floor.rooms) {
        const info = catalog.rooms[room.def];
        if (!info || info.category !== "Intake" || info.id === "room.salvage_rig") continue;
        paused.push({ floor: floor.index, slot: room.slot });
        hooks.send({ SetRoomActive: { floor: floor.index, slot: room.slot, active: false } });
      }
    }

    let budget = 130_000;
    while (budget > 0) {
      const view = hooks.view();
      // Done as soon as the frame can show the comparison: something
      // emptied standing next to something that still holds.
      const shown = framed(view);
      if (shown.some((f) => f.salvage <= 0) && shown.some((f) => f.salvage > 0)) break;
      if (reachable(view).every((f) => f.salvage <= 0)) break;
      // Or as soon as the wardens are winning. A berth is a commitment
      // and this harness has to be able to walk away from one.
      if (view.siege.integrity_permille < 780) break;
      hooks.step(120);
      budget -= 120;
    }

    for (const at of paused) {
      hooks.send({ SetRoomActive: { floor: at.floor, slot: at.slot, active: true } });
    }
    hooks.step(2);
    const end = hooks.view();
    const rig = end.tower.floors
      .flatMap((floor) => floor.rooms)
      .find((room) => catalog.rooms[room.def]?.id === "room.salvage_rig");
    const shelves = end.tower.floors
      .flatMap((floor) => floor.rooms)
      .flatMap((room) => room.shelves)
      .reduce((free, shelf) => free + (shelf.max - shelf.count), 0);
    return `in frame ${before} -> ${framed(end)
      .map((f) => f.salvage)
      .join("/")}; rig outbox ${rig?.outputs[0]?.count ?? "?"}/${
      rig?.outputs[0]?.max ?? "?"
    }, ${shelves} shelf space free`;
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/ruin-stripped.png" });
  await where(page, "after the ruin stills");

  // Three stills across one approach rather than one at contact.
  // Whether a wave reads is a question about the whole stretch a
  // creature spends closing — it should crest the horizon, be watched
  // in, and either be shot down or arrive — and only the last of those
  // three moments used to get photographed.
  await page.evaluate(() => {
    window.__understory!.send({ SetStriding: { walking: true } });
  });
  const sighted = await stepUntilWithin(page, 380);
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/tower-approach.png" });

  // Inside a dart battery's 60 paces — the stretch the whole mapping
  // exists to put on screen, because it is where the darts are.
  const closing = await stepUntilWithin(page, 55);
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/tower-closing.png" });

  // And the end of the same approach: something on the tower, and the
  // tower marked by it, because a still of an untouched tower with four
  // creatures walking past does not show whether damage reads.
  //
  // Stepped fine near the tower for the same reason the approach is. A
  // skitter's grip is 300 ticks and its bite lands every 25, so a
  // 600-tick stride steps clean over the whole of contact as easily as
  // it stepped over the approach — which is how this loop used to burn
  // seventy thousand ticks and photograph an empty jungle.
  const standing = await page.evaluate(() => {
    const hooks = window.__understory!;
    let budget = 14_000;
    while (budget > 0) {
      const view = hooks.view();
      if (view.journey.arrived) break;
      const daylight = view.clock.sun_pct > 40;
      const here = view.siege.enemies.some((enemy) => enemy.state === "attack");
      if (daylight && here && view.siege.integrity_permille < 1000) break;
      const nearest = Math.min(
        999,
        ...view.siege.enemies.map((enemy) => Math.abs(enemy.at - view.world.distance)),
      );
      const stride = nearest < 40 ? 10 : 60;
      window.__capture!.answer();
      hooks.step(stride);
      budget -= stride;
    }
    return hooks.view().siege.integrity_permille;
  });
  await page.waitForTimeout(400);
  await page.screenshot({ path: "capture/tower-siege.png" });
  await where(page, "after the siege stills");

  //
  // Ordered by distance rather than by subject, because a run is a walk
  // down one axis and every still after the far edge is a still of the
  // far edge. Every budget below is in ticks and every loop stops on
  // arrival: a journey is 86,000–114,000 paces end to end, comfortably
  // inside 200,000 ticks, so a loop with a 200,000-tick budget and no
  // arrival guard does not fail — it silently walks to the end of the
  // world and photographs that instead. That is exactly how the first
  // run of this file ended, with nine stills of a tower that had
  // already finished.

  // The enclave. Its position never crosses the bridge — `at_enclave`
  // is the only signal there is, and it is false while the legs are
  // running — so the only way to find it is to stop and ask. A player
  // has exactly the same problem, which is the gap this harness is
  // documenting as much as working around.
  const enclave = await page.evaluate(() => {
    const hooks = window.__understory!;
    hooks.send({ SetStriding: { walking: true } });
    // Region 2 is 34,000–46,000 paces long and the enclave sits 8,000
    // into it, so it is somewhere past 170 per-mille. Nothing to find
    // before then.
    let budget = 260_000;
    while (budget > 0) {
      const view = hooks.view();
      if (view.journey.arrived) return "walked past it";
      const close = view.journey.region >= 1 && view.journey.region_permille >= 150;
      if (close) {
        hooks.send({ SetStriding: { walking: false } });
        hooks.step(2);
        if (hooks.view().journey.at_enclave) {
          // In daylight. Berthing holds the tower in place, so waiting
          // out the dark costs nothing and does not drift out of range.
          let dawn = 20_000;
          while (dawn > 0 && hooks.view().clock.sun_pct < 70) {
            hooks.step(120);
            dawn -= 120;
          }
          return "berthed";
        }
        hooks.send({ SetStriding: { walking: true } });
      }
      window.__capture!.answer();
      // 36 paces a check inside the 160-pace berth window, so it cannot
      // be stepped over.
      const stride = close ? 60 : 600;
      hooks.step(stride);
      budget -= stride;
    }
    return "never got there";
  });
  await page.waitForTimeout(400);
  await page.screenshot({ path: "capture/enclave.png" });

  // And the two things the board can actually do.
  const traded = await page.evaluate(() => {
    const hooks = window.__understory!;
    const before = hooks.view();
    const stock = (item: number) => before.stock.find((entry) => entry.item === item)?.count ?? 0;
    // Plating first, and the order is a real decision rather than a
    // detail: the board's best offer also takes scrap, so trading
    // before plating can leave you two short of a hull you meant to
    // buy. Permanent beats convertible.
    const plate = hooks.send("Reinforce");
    const trade = hooks.send({ Trade: { offer: 0 } });
    const hire = hooks.send("Recruit");
    hooks.step(2);
    const after = hooks.view();
    const moved = after.stock
      .filter((entry) => entry.count !== stock(entry.item))
      .map(
        (entry) =>
          `${hooks.catalog().items[entry.item]?.id ?? "?"} ${entry.count - stock(entry.item)}`,
      )
      .join(", ");
    return `trade ${JSON.stringify(trade)}, recruit ${JSON.stringify(hire)}, plate ${JSON.stringify(plate)} (+${after.journey.shell_bonus} shell), crew ${after.crew.length}, shelves moved: ${moved || "nothing"}`;
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/enclave-after.png" });
  await where(page, "after the enclave");

  // The split, seen coming. §3.11's first open question is whether a
  // player notices it in time, and the answer to that is a picture.
  const sightedFork = await page.evaluate(() => {
    const hooks = window.__understory!;
    let budget = 80_000;
    while (budget > 0) {
      const view = hooks.view();
      if (view.journey.arrived) break;
      const fork = view.journey.fork;
      if (fork && fork.answer === null) {
        if (fork.ahead <= 260 && view.clock.sun_pct > 45) {
          return `${Math.round(fork.ahead)}p out`;
        }
        // Hold short of the split until morning rather than walking
        // into it in the dark. The first run of this photographed a
        // night approach, missed the window, and produced two stills of
        // the same frame; gating the *whole* wait on daylight instead
        // cost four fifths of the budget and left the tower a thousand
        // paces short of a fork. Only the approach has to be light.
        const night = view.clock.sun_pct <= 45;
        hooks.send({ SetStriding: { walking: !night || fork.ahead > 300 } });
        const stride = night ? 120 : 20;
        hooks.step(stride);
        budget -= stride;
        continue;
      }
      hooks.send({ SetStriding: { walking: true } });
      hooks.step(240);
      budget -= 240;
    }
    return "no fork in frame";
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/fork-ahead.png" });

  // Standing at it, with no answer. The one halt that must never read
  // as a frozen game.
  await page.evaluate(() => {
    const hooks = window.__understory!;
    // Striding, explicitly: the approach above holds the tower short of
    // the split overnight, and without this the halt loop steps a
    // parked tower for four thousand ticks and photographs it two
    // hundred and fifty paces from the thing it is meant to be at.
    hooks.send({ SetStriding: { walking: true } });
    let budget = 8_000;
    while (budget > 0 && hooks.view().journey.halt !== "fork") {
      hooks.step(20);
      budget -= 20;
    }
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/halt-fork.png" });

  // Answered, and still standing there for one more frame: the way the
  // tower has been told to take is lit on the ground.
  const taken = await page.evaluate(() => window.__capture!.answer());
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/fork-taken.png" });
  await where(page, "after the fork stills");

  // Wanting to walk and not being able to afford it. Forced rather than
  // waited for — shut the sails at dusk and keep the legs asking, which
  // is the same corner a player backs into by building one bank too few
  // (`SYSTEMS.md` §3.6).
  const brownout = await page.evaluate(() => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    const solar = catalog.rooms.findIndex((room) => room.solar);
    // In daylight, so the dimmed frame reads as the tower going out
    // rather than as the sun having gone down — those are two different
    // pictures and only one of them is a brown-out.
    let wait = 40_000;
    while (wait > 0 && hooks.view().clock.sun_pct < 80) {
      hooks.step(120);
      wait -= 120;
    }
    const view = hooks.view();
    for (const floor of view.tower.floors) {
      for (const room of floor.rooms) {
        if (room.def === solar) {
          hooks.send({
            SetRoomActive: { floor: floor.index, slot: room.slot, active: false },
          });
        }
      }
    }
    hooks.send({ SetStriding: { walking: true } });
    let budget = 40_000;
    while (budget > 0) {
      // Stepped before reading: `strode` is still false on the tick
      // `SetStriding` lands, so a tower that has just been told to walk
      // reports a brown-out it is not having.
      window.__capture!.answer();
      hooks.step(30);
      budget -= 30;
      const now = hooks.view();
      if (now.journey.halt === "brownout") return `charge ${now.power.charge}`;
      if (now.journey.arrived) break;
    }
    return "never ran dry";
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/halt-brownout.png" });

  // Sails back on, or the rest of the journey is spent in the dark.
  await page.evaluate(() => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    const solar = catalog.rooms.findIndex((room) => room.solar);
    const view = hooks.view();
    for (const floor of view.tower.floors) {
      for (const room of floor.rooms) {
        if (room.def === solar) {
          hooks.send({ SetRoomActive: { floor: floor.index, slot: room.slot, active: true } });
        }
      }
    }
  });
  await where(page, "after the halt stills");

  // The far edge. Standing still here is the run being over rather than
  // a decision pending, and it has to read that way.
  // First the edge coming: the last few hundred paces, where the world
  // visibly runs out ahead of a tower that is still walking. The
  // overlay covers the frame once it lands, so this is the only chance
  // to look at the treatment underneath it.
  await page.evaluate(() => {
    const hooks = window.__understory!;
    hooks.send({ SetStriding: { walking: true } });
    let budget = 160_000;
    while (budget > 0) {
      const view = hooks.view();
      if (view.journey.arrived) break;
      if (view.journey.remaining < 26) break;
      // Hold short of the last few hundred paces until morning, the
      // same way the fork approach does: the far edge is drawn as the
      // world opening out into light and a night crossing photographs
      // a dark frame with a caption on it.
      const night = view.clock.sun_pct <= 45;
      const close = view.journey.remaining < 500;
      hooks.send({ SetStriding: { walking: !(night && close) } });
      window.__capture!.answer();
      const stride = close ? (night ? 120 : 6) : 300;
      hooks.step(stride);
      budget -= stride;
    }
  });
  await where(page, "at the far edge");
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/journey-edge.png" });

  const arrival = await page.evaluate(() => {
    const hooks = window.__understory!;
    hooks.send({ SetStriding: { walking: true } });
    let budget = 60_000;
    while (budget > 0 && !hooks.view().journey.arrived) {
      window.__capture!.answer();
      hooks.step(30);
      budget -= 30;
    }
    // In daylight: the far edge is meant to open out into light, and a
    // night arrival photographs a dark frame with a caption on it.
    let wait = 20_000;
    while (wait > 0 && hooks.view().clock.sun_pct < 70) {
      hooks.step(120);
      wait -= 120;
    }
    const view = hooks.view();
    return view.journey.arrived
      ? `arrived at ${Math.round(view.world.distance)} paces on day ${view.clock.day + 1}`
      : "never got there";
  });
  // The overlay fades up over 2.6 seconds, so a 400ms wait photographs
  // it a third of the way in.
  await page.waitForTimeout(3000);
  await page.screenshot({ path: "capture/halt-arrived.png" });

  // **The run log, which is the input to the difficulty pass and had no
  // test at all.** `SYSTEMS.md` §5.8 asks for "a file the player can
  // read and hand over"; every run has been recorded to the journal
  // since M5 and until now there was no way to get one out, which is the
  // whole gap between somebody playing and `BALANCE.md`'s constants
  // being graded from run logs rather than from harnesses. Asserted here
  // rather than in its own test because this is the only place in the
  // suite that reaches an ending, and reaching one costs 375,000 ticks.
  const handed = await page.evaluate(() => {
    // The affordance and the data, checked separately. Reading the
    // clipboard back needs a permission grant Playwright does not have
    // by default, and a test that depends on one is a test that fails
    // for the wrong reason — so this asserts the button exists and that
    // what it would hand over is real, which is the contract.
    const button = document.querySelector<HTMLButtonElement>('[data-testid="copy-runs"]');
    if (!button) return "no copy button on the arrival";
    const raw = localStorage.getItem("understory.journal.v1");
    if (!raw) return "nothing written down";
    const journal = JSON.parse(raw) as { runs?: unknown };
    const runs = journal.runs;
    if (!Array.isArray(runs) || runs.length === 0) return "the journal holds no runs";
    const first = runs[0] as Record<string, unknown>;
    return typeof first.seed === "string" && typeof first.ending === "string"
      ? `${runs.length} run(s) to hand over, first ended "${String(first.ending)}"`
      : "a run log is missing its seed or its ending";
  });
  console.log(`run log: ${handed}`);
  expect(handed).toContain("to hand over");

  // **"Ending as an arrival rather than a score"** — the second half of
  // M5's first exit criterion, and the half a harness *can* check.
  // `DECISIONS.md` §8 forbids a rank, a grade or anything that reads as
  // a mark out of ten, and the arrival is the one screen where a game
  // like this would traditionally put one. Asserted rather than assumed,
  // because it is the sort of thing that gets added later by somebody
  // being helpful.
  const ending = await page.evaluate(() => {
    const card = document.querySelector('[data-testid="arrival"]');
    if (!card) return { text: "", crew: "" };
    // Crew are named individuals, not a headcount (`DESIGN.md` §2
    // structural call 4) — "Aboard: 3" would be a score with a friendly
    // face on it. Read the definition list by structure: the first
    // attempt anchored a regex to the end of the card's text, and the
    // card has the journal, the seed and a button after it.
    let crew = "";
    for (const row of card.querySelectorAll("dl div")) {
      if (row.querySelector("dt")?.textContent?.trim() === "Aboard") {
        crew = row.querySelector("dd")?.textContent?.trim() ?? "";
      }
    }
    return { text: (card.textContent ?? "").toLowerCase(), crew };
  });
  // Score *vocabulary*, not ordinary English. "out of" and "final" were
  // in this list for one run and both matched prose — a deed that reads
  // "Walked a tower out of the deep jungle" is not a mark out of ten.
  // A guard that cries wolf on the writing gets deleted by the next
  // person, so it checks the words a score actually uses.
  for (const word of ["score", "rank", "rating", "graded", "points", "percentile"]) {
    expect(ending.text, `the arrival reads as a score: found "${word}"`).not.toContain(word);
  }
  // And the shape of one, which is the part that would survive a
  // rename: a mark over a maximum, or a percentage.
  expect(ending.text, "the arrival shows a mark out of a maximum").not.toMatch(/\d+\s*\/\s*\d+/);
  expect(ending.text, "the arrival shows a percentage").not.toMatch(/\d+\s*%/);
  expect(ending.crew, "the arrival has no crew line at all").not.toBe("");
  expect(ending.crew, "the arrival counted the crew instead of naming them").toMatch(/[A-Za-z]{2}/);
  console.log("arrival reads as a description, not a score");

  console.log(`sighting still: ${sighted}`);
  console.log(`closing still: ${closing}`);
  console.log(`siege still captured at ${standing} per-mille standing`);
  console.log(`ruins in reach: ${berth}`);
  console.log(`salvage: ${stripped}`);
  console.log(`fork sighted: ${sightedFork}; took: ${taken ?? "already answered"}`);
  console.log(`brownout: ${brownout}`);
  console.log(`enclave: ${enclave} — ${traded}`);
  console.log(`arrival: ${arrival}`);
});

/**
 * Buy a room as soon as the tower can pay for it — **in the page**.
 *
 * Hoisted to module scope rather than closed over inside the test,
 * because everything here executes in the browser and nothing in it may
 * capture a Node-side variable; keeping it out here makes that a
 * property of where it is written rather than a thing to remember.
 *
 * Waiting for the money rather than hardcoding a tick is the rule the
 * Rust harnesses follow, and for the same reason: a script that assumes
 * "by now there will be five poles" breaks on every balance change, and
 * breaks by measuring the wrong tower rather than by failing.
 */
function buyInPage({ id, wide }: { id: string; wide: number }): boolean {
  const hooks = window.__understory!;
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const spot = window.__capture!.freeSlotAny(wide);
    // `send` answers "Ok" or `{ Error }` — it never answers null, and a
    // null check here quietly meant *every* attempt read as a failure,
    // so this loop bought the room forty times. Found by looking at the
    // still: a tower with four canteens in it.
    if (
      spot &&
      hooks.send({ PlaceRoom: { room: id, floor: spot.floor, slot: spot.slot } }) === "Ok"
    ) {
      return true;
    }
    window.__capture!.walk(600);
  }
  return false;
}

/**
 * Step until the frame has what it is a picture of: somebody eating,
 * and somebody asleep.
 *
 * Sleep is easy to catch — the night band is a third of the day — while
 * a meal is 300 ticks out of 4,800, so a loop that stops at the first
 * interesting state it sees *always* stops on a sleeper and the frame
 * never has anybody eating in it. Bounded to well under one night band,
 * so a search that finds no meal leaves the tower in the evening it was
 * walked to rather than wandering into the following dawn — a still
 * captioned "dusk" that is actually daybreak proves nothing.
 */
function settleOnAHomeInPage(): void {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    const states = window.__understory!.view().crew.map((member) => member.state);
    if (states.includes("eat") && states.includes("sleep")) break;
    window.__capture!.walk(20, 20);
  }
}

/**
 * M4's screenshot test: **does one frame say "solarpunk home, not war
 * machine"?**
 *
 * Two stills, to be shown to somebody who has never seen the game and
 * asked only "what is this place?".
 *
 * - `home-evening.png` — dusk, lamps on, the canteen's hearth lit and
 *   steaming, crew sitting to a meal, somebody asleep, planters full,
 *   the jungle going blue behind it. It has to come back as somebody's
 *   home, greenhouse, or ark. If it comes back as a factory, a rig, or
 *   a gun platform, the art pass failed.
 * - `home-siege.png` — the same tower mid-wave, as the control. It
 *   should read as a home under threat, not as a fortress that has
 *   finally found its purpose.
 *
 * **A frame with no people in it cannot pass**, which is why the
 * harness works so hard below to get crew visibly eating and asleep
 * before it takes the picture, rather than photographing whatever
 * happens to be on screen. If the tower is drawn beautifully and
 * nobody is home, the answer to "what is this place?" is "a machine".
 */
test("capture the home", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/?seed=909");
  await page.waitForFunction(() => window.__understory !== undefined, null, { timeout: 20_000 });
  await arm(page);

  // Build the two rooms M4 adds, as soon as the tower can pay for
  // them. Waiting for the money rather than hardcoding a tick keeps
  // this harness working across a balance change — the same rule the
  // Rust harnesses follow.
  const buy = async (id: string, wide: number): Promise<boolean> =>
    await page.evaluate(buyInPage, { id, wide });

  await page.evaluate(() => {
    window.__capture!.walk(1800);
  });
  await buy("room.canteen", 2);
  await buy("room.bunk", 2);

  // Put one crew member on nights, so the evening frame has somebody
  // up and about as well as somebody asleep. A dormitory with everyone
  // in it and nobody moving is a still life, not a home.
  await page.evaluate(() => {
    const crew = window.__understory!.view().crew;
    const last = crew[crew.length - 1];
    if (last) window.__understory!.send({ SetShift: { crew: last.id, shift: "Night" } });
  });

  // Walk to dusk, and then keep stepping in small increments until the
  // frame actually has what it is a picture of: somebody eating, and
  // somebody asleep. Photographing "roughly evening" and hoping is how
  // you end up with a still of three people standing in a corridor.
  await page.evaluate(() => {
    const catalog = window.__understory!.catalog();
    const view = window.__understory!.view();
    // 770 per-mille: a hair past the dusk boundary, so the search
    // below has the whole night band to spend and still lands in the
    // evening rather than at the following daybreak.
    const target = Math.round(catalog.ticks_per_day * 0.77);
    const now = Math.round((view.clock.permille / 1000) * catalog.ticks_per_day);
    window.__capture!.walk((target - now + catalog.ticks_per_day) % catalog.ticks_per_day, 120);
  });
  await page.evaluate(settleOnAHomeInPage);
  await page.waitForTimeout(500);
  await page.screenshot({ path: "capture/home-evening.png" });
  const inFrame = await page.evaluate(() =>
    window
      .__understory!.view()
      .crew.map((member) => member.state)
      .join("/"),
  );
  console.log(`home-evening: crew are ${inFrame}`);

  // The control: the same home, mid-wave. Earned rather than pinned —
  // there is no debug hook that writes provocation and there should not
  // be one, because a still of a tower in a state no run reaches is not
  // evidence about the game. A second cutter arm is what a player does
  // to get noticed, so it is what the harness does.
  await buy("room.cutter_arm", 2);
  const met = await page.evaluate(() => {
    const hooks = window.__understory!;
    let budget = 260_000;
    while (budget > 0) {
      const view = hooks.view();
      if (view.journey.arrived) break;
      // Daylight, so the frame is legible, and something actually on
      // the tower rather than merely on the horizon.
      if (view.clock.sun_pct > 40 && view.siege.enemies.some((e) => e.state === "attack")) {
        return true;
      }
      const nearest = Math.min(
        999,
        ...view.siege.enemies.map((enemy) => Math.abs(enemy.at - view.world.distance)),
      );
      const stride = nearest < 40 ? 10 : 240;
      window.__capture!.answer();
      hooks.step(stride);
      budget -= stride;
    }
    return false;
  });
  await page.waitForTimeout(500);
  await page.screenshot({ path: "capture/home-siege.png" });
  // Reported rather than asserted: this is a harness, and a run that
  // never drew a wave is a finding about the balance, not a test
  // failure. It still has to be said out loud, because a still of an
  // unbothered tower filed as `home-siege.png` is worse than no still.
  console.log(
    met
      ? "home-siege: a wave reached the tower"
      : "home-siege: NO WAVE — still is of a quiet tower",
  );
});
