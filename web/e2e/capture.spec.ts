import { test } from "@playwright/test";

/**
 * Not a test — a screenshot harness for looking at the game.
 *
 * `npx playwright test capture` writes stills of the tower at a few
 * points in a run. M0's exit criterion is a question you answer by
 * looking ("does the walking tower feel alive on one screen?"), so
 * there needs to be a repeatable way to look.
 */
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

  await page.getByTestId("build-room.mill").click();
  await page.waitForTimeout(200);
  const point = await page.evaluate(() => {
    const view = window.__understory!.view();
    return window.__understory!.slotPoint(view.tower.floors.length - 1, 2);
  });
  if (point) await page.mouse.move(point.x, point.y);
  await page.waitForTimeout(300);
  await page.screenshot({ path: "capture/tower-placing.png" });
});
