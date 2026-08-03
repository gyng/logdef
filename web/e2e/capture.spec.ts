import { test, type Page } from "@playwright/test";

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

    window.__capture = { walk, answer, freeSlot };
  });
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
    let budget = 200_000;
    while (budget > 0) {
      const view = hooks.view();
      const gaps = view.siege.enemies
        .filter((enemy) => enemy.state === "approach")
        .map((enemy) => enemy.at - view.world.distance)
        .filter((gap) => gap >= 0);
      const nearest = gaps.length > 0 ? Math.min(...gaps) : null;
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
    return "nothing in frame";
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
  await page.getByTestId("build-room.cutter_arm").click();
  const armPoint = await page.evaluate(() => window.__capture!.freeSlot(0, 1));
  if (armPoint) {
    await page.mouse.click(armPoint.x, armPoint.y);
  }
  // Somewhere to put what the second arm strips, and something to make
  // darts with. Without these the storerooms fill, the arms jam, and
  // the tower goes quiet again — which is correct behaviour and a
  // useless screenshot.
  for (const [room, floor, slot] of [
    ["room.storeroom", 2, 5],
    ["room.storeroom", 3, 5],
    ["room.thornwright", 3, 1],
    ["room.dart_battery", 1, 1],
  ] as const) {
    await page.getByTestId(`build-${room}`).click();
    const spot = await page.evaluate(([f, s]) => window.__understory!.slotPoint(f, s), [
      floor,
      slot,
    ] as const);
    if (spot) await page.mouse.click(spot.x, spot.y);
    // Poles are scarce; give the mill time to make the next lot.
    await page.evaluate(() => {
      window.__capture!.walk(3000);
    });
  }

  // Three stills across one approach rather than one at contact.
  // Whether a wave reads is a question about the whole stretch a
  // creature spends closing — it should crest the horizon, be watched
  // in, and either be shot down or arrive — and only the last of those
  // three moments used to get photographed.
  //
  // Paused first, so each still lands exactly where it was asked for:
  // `step` ignores the speed setting, but the render loop does not, and
  // at 4x the couple of hundred milliseconds a screenshot needs is
  // another forty paces of closing.
  await page.getByTestId("speed-Paused").click();

  // Well out: still most of the approach to run, and outside anything's
  // reach. If the creature is not plainly in frame here, the mapping is
  // wrong.
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
    let budget = 200_000;
    while (budget > 0) {
      const view = hooks.view();
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

  // ── M3 ────────────────────────────────────────────────────────────
  //
  // Left paused throughout: every one of these is a question about a
  // specific moment, and the render loop would have walked past it in
  // the time a screenshot takes.

  // The split, seen coming. §3.11's first open question is whether a
  // player notices it in time, and the answer to that is a picture, not
  // a paragraph.
  const sightedFork = await page.evaluate(() => {
    const hooks = window.__understory!;
    let budget = 200_000;
    while (budget > 0) {
      const view = hooks.view();
      const fork = view.journey.fork;
      if (fork && fork.answer === null && fork.ahead <= 300 && view.clock.sun_pct > 40) {
        return `${Math.round(fork.ahead)}p out`;
      }
      const stride = fork && fork.answer === null ? 30 : 240;
      hooks.step(stride);
      budget -= stride;
    }
    return "no fork in frame";
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/fork-ahead.png" });

  // Standing at it, with no answer. The one halt that must never read
  // as a frozen game.
  await page.evaluate(() => {
    const hooks = window.__understory!;
    let budget = 20_000;
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

  // A tower the player stopped. Same silhouette, same still legs, and
  // it has to be tellable from the two above.
  await page.evaluate(() => {
    const hooks = window.__understory!;
    window.__capture!.walk(1200, 120);
    hooks.send({ SetStriding: { walking: false } });
    hooks.step(30);
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/halt-stopped.png" });

  // Ruins, before anything has been taken out of them: the strip should
  // say which ones are worth the stop.
  const berth = await page.evaluate(() => {
    const hooks = window.__understory!;
    hooks.send({ SetStriding: { walking: true } });
    let budget = 200_000;
    while (budget > 0) {
      const view = hooks.view();
      const rich = view.world.features.filter((f) => f.salvage > 0);
      const near = rich.filter((f) => Math.abs(f.at - view.world.distance) <= 35);
      if (near.length > 0 && view.clock.sun_pct > 40) {
        hooks.send({ SetStriding: { walking: false } });
        return rich.map((f) => f.salvage).join("/");
      }
      const ahead = rich.map((f) => f.at - view.world.distance).filter((gap) => gap > 20);
      const nearest = ahead.length > 0 ? Math.min(...ahead) : null;
      const stride = nearest === null ? 200 : Math.max(4, Math.round(((nearest - 25) / 0.6) * 0.5));
      window.__capture!.answer();
      hooks.step(stride);
      budget -= stride;
    }
    return "none in reach";
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/ruin-rich.png" });

  // Now strip one, and photograph it next to the ones still holding
  // something. This is the whole read: a lean drowned city and a
  // generous one are the same ruins in the same places with different
  // amounts in them (`SYSTEMS.md` §3.4).
  await page.getByTestId("build-room.salvage_rig").click();
  const rigPoint = await page.evaluate(() => window.__capture!.freeSlot(0, 1));
  if (rigPoint) await page.mouse.click(rigPoint.x, rigPoint.y);

  const stripped = await page.evaluate(() => {
    const hooks = window.__understory!;
    const start = hooks.view();
    const gap = (f: { at: number }) => Math.abs(f.at - start.world.distance);
    const target = start.world.features
      .filter((f) => f.salvage > 0)
      .reduce<(typeof start.world.features)[number] | null>(
        (best, f) => (best === null || gap(f) < gap(best) ? f : best),
        null,
      );
    if (!target) return "nothing in reach";
    let budget = 40_000;
    while (budget > 0) {
      const here = hooks
        .view()
        .world.features.find((f) => Math.abs(f.at - target.at) < 0.01 && f.layer === target.layer);
      if (!here || here.salvage <= 0) break;
      hooks.step(60);
      budget -= 60;
    }
    const view = hooks.view();
    return `${target.salvage} taken; still standing: ${view.world.features
      .filter((f) => Math.abs(f.at - view.world.distance) < 120)
      .map((f) => f.salvage)
      .join("/")}`;
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/ruin-stripped.png" });

  console.log(`sighting still: ${sighted}`);
  console.log(`closing still: ${closing}`);
  console.log(`siege still captured at ${standing} per-mille standing`);
  console.log(`fork sighted: ${sightedFork}; took: ${taken ?? "already answered"}`);
  console.log(`ruins in reach: ${berth}`);
  console.log(`salvage: ${stripped}`);
});
