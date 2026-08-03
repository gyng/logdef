import { test, type Page } from "@playwright/test";

/**
 * Not a test — a screenshot harness for looking at the game.
 *
 * `npx playwright test capture` writes stills of the tower at a few
 * points in a run. M0's exit criterion is a question you answer by
 * looking ("does the walking tower feel alive on one screen?"), so
 * there needs to be a repeatable way to look.
 */

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

  await page.evaluate(() => {
    window.__understory!.step(900);
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/tower-early.png" });

  await page.evaluate(() => {
    window.__understory!.step(5400);
  });
  await page.getByTestId("speed-X4").click();
  await page.waitForTimeout(600);
  await page.screenshot({ path: "capture/tower-running.png" });

  await page.getByTestId("build-shaft.elevator").click();
  await page.waitForTimeout(200);
  const point = await page.evaluate(() => window.__understory!.slotPoint(0, 6));
  if (point) {
    await page.mouse.move(point.x, point.y);
    await page.waitForTimeout(200);
    await page.screenshot({ path: "capture/tower-placing.png" });
    await page.mouse.click(point.x, point.y);
  }

  // Run on until the elevator has crew in it and the sun has moved.
  await page.evaluate(() => {
    window.__understory!.step(5400);
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
    window.__understory!.step(wait);
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
  const armPoint = await page.evaluate(() => window.__understory!.slotPoint(0, 6));
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
    await page.evaluate(() => window.__understory!.step(3000));
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
      hooks.step(stride);
      budget -= stride;
    }
    return hooks.view().siege.integrity_permille;
  });
  await page.waitForTimeout(400);
  await page.screenshot({ path: "capture/tower-siege.png" });
  console.log(`sighting still: ${sighted}`);
  console.log(`closing still: ${closing}`);
  console.log(`siege still captured at ${standing} per-mille standing`);
});
