import { expect, test } from "@playwright/test";

const EXPECTED = new Set([
  "waypoint.broken_funicular",
  "waypoint.cloud_cistern",
  "waypoint.fallen_carrier",
  "waypoint.field_kitchen",
  "waypoint.lantern_post",
  "waypoint.relay_orchard",
  "waypoint.seep_pool",
  "waypoint.signal_bridge",
  "waypoint.snare_thicket",
  "waypoint.tool_cradle",
  "waypoint.windfall_rig",
  "waypoint.wire_tangle",
]);

/**
 * A live contact sheet for route art. It deliberately discovers waypoints
 * through the deterministic world stream instead of mocking the snapshot, so
 * every capture proves content loading, bridge identity and atlas placement at
 * once. Run directly while iterating; it is intentionally broader than smoke.
 */
test("all authored waypoints have readable world art", async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize({ width: 1600, height: 900 });
  const seen = new Set<string>();

  for (let seed = 1; seed <= 24 && seen.size < EXPECTED.size; seed += 1) {
    await page.goto(`/?seed=${seed}`);
    await page.waitForFunction(() => window.__understory?.view !== undefined, null, {
      timeout: 20_000,
    });
    await page.evaluate(() => window.__understory!.send({ SetSpeed: { speed: "Paused" } }));

    for (let step = 0; step < 150 && seen.size < EXPECTED.size; step += 1) {
      const found = await page.evaluate(() => {
        const hooks = window.__understory!;
        const fork = hooks.view().journey.fork;
        if (fork && fork.answer === null) hooks.send({ TakeFork: { branch: 0 } });
        hooks.step(300);
        const view = hooks.view();
        const beat = view.journey.waypoint;
        return beat === null ? null : (hooks.catalog().waypoints[beat.def]?.id ?? null);
      });
      if (!found || seen.has(found)) continue;
      seen.add(found);
      await page.evaluate(
        () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())),
      );
      await page.screenshot({ path: `capture/${found.replace("waypoint.", "waypoint-")}.png` });
    }
  }

  expect(seen.size).toBe(EXPECTED.size);
  for (const id of EXPECTED) expect(seen.has(id), `missing ${id}`).toBe(true);
});

test("the thicket resident is foreshadowed inside the distant environment", async ({ page }) => {
  test.setTimeout(90_000);
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/?seed=4242");
  await page.waitForFunction(() => window.__understory?.view !== undefined, null, {
    timeout: 20_000,
  });
  await page.evaluate(() => window.__understory!.send({ SetSpeed: { speed: "Paused" } }));

  const reached = await page.evaluate(() => {
    const hooks = window.__understory!;
    for (let step = 0; step < 180; step += 1) {
      const fork = hooks.view().journey.fork;
      if (fork && fork.answer === null) hooks.send({ TakeFork: { branch: 0 } });
      hooks.step(300);
      const ahead = hooks.view().journey.landmark_ahead;
      if (ahead !== null && ahead.ahead < 760 && ahead.ahead > 300) return ahead;
    }
    return null;
  });

  expect(reached, "the unique landmark never entered its approach window").not.toBeNull();
  await expect(page.getByText(/territory ahead/i)).toBeVisible();
  await page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())));
  await page.screenshot({ path: "capture/landmark-resident-foreshadow.png" });
});
