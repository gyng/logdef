import { expect, test } from "@playwright/test";

/**
 * The release cut, actually opened.
 *
 * `npm run build` succeeding proves the bundler was happy and nothing
 * else. This opens the *built* files the way itch will — relative asset
 * paths, no dev server, no HMR — and plays a few seconds, because the
 * failure this catches is a bundle that 404s its own WASM and shows a
 * blank canvas, which every other check in the project passes.
 */
const PREVIEW = "http://localhost:4173";

test("the built bundle boots and runs", async ({ page }) => {
  // **Skips rather than fails when no preview is up.** This is the one
  // check that cannot use the dev server, because the thing under test
  // is the *built* output; but the ordinary `npm run e2e` has no reason
  // to have run `vite preview` first, and a suite that goes red because
  // somebody did not is a suite people learn to ignore. `make release`
  // is where this is meant to run.
  const reachable = await page.request
    .get(PREVIEW, { timeout: 2000 })
    .then((response) => response.ok())
    .catch(() => false);
  test.skip(!reachable, "no `vite preview` on :4173 — run `make release` to check the cut");

  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await page.goto(`${PREVIEW}/?seed=7`);
  await expect(page.getByTestId("game-canvas")).toBeVisible({ timeout: 20_000 });
  await page.waitForFunction(() => window.__understory !== undefined, null, { timeout: 20_000 });

  const before = await page.evaluate(() => window.__understory!.view().tick);
  await page.getByTestId("speed-X4").click();
  await page.waitForTimeout(1500);
  const after = await page.evaluate(() => window.__understory!.view().tick);
  expect(after, "the built bundle did not advance the simulation").toBeGreaterThan(before);
  expect(errors, `the built bundle logged errors: ${errors.join(" | ")}`).toEqual([]);
});
