import { expect, test, type Page } from "@playwright/test";

import type { CatalogSnapshot, ReplayReport, ViewSnapshot } from "../src/bridge/types";

/**
 * The M0 smoke path.
 *
 * Three things have to be true after every change: the tower boots and
 * draws, the chain runs without anyone touching it, and the simulation
 * agrees with itself across native and wasm. The last one is the whole
 * determinism guarantee, so it is checked here rather than trusted.
 *
 * The run is seeded from the URL so the assertions describe one
 * specific run rather than an average one.
 */

const SEED = 4242;

/**
 * The hooks the app installs on `window` for tests to drive it.
 *
 * The payload types are imported rather than restated: a hand-written
 * copy silently rots every time the snapshot gains a field, and then
 * the tests are asserting against a shape the game stopped having.
 */
interface TestHooks {
  view(): ViewSnapshot;
  catalog(): CatalogSnapshot;
  stateHash(): string;
  step(ticks: number): void;
  verifyGolden(): ReplayReport;
  exportReplay(): string;
  /** Centre of a slot in client coordinates, from the live layout. */
  slotPoint(floor: number, slot: number): { x: number; y: number } | null;
}

declare global {
  interface Window {
    __understory?: TestHooks;
  }
}

async function boot(page: Page, seed = SEED) {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await page.goto(`/?seed=${seed}`);
  await expect(page.getByTestId("game-canvas")).toBeVisible({ timeout: 20_000 });
  await page.waitForFunction(() => window.__understory !== undefined, null, { timeout: 20_000 });
  return errors;
}

test("boots, draws, and reports a live world", async ({ page }) => {
  const errors = await boot(page);

  // WebGL2 actually produced a surface, rather than silently failing to
  // a blank canvas.
  const drawing = await page.evaluate(() => {
    const canvas = document.querySelector<HTMLCanvasElement>("[data-testid='game-canvas']");
    return {
      width: canvas?.width ?? 0,
      height: canvas?.height ?? 0,
      hasContext: canvas?.getContext("webgl2") !== null,
    };
  });
  expect(drawing.width).toBeGreaterThan(0);
  expect(drawing.height).toBeGreaterThan(0);
  expect(drawing.hasContext).toBe(true);

  const snapshot = await page.evaluate(() => window.__understory!.view());
  expect(snapshot.tower.floors.length).toBeGreaterThanOrEqual(4);
  expect(snapshot.tower.shafts.length).toBeGreaterThanOrEqual(1);
  expect(snapshot.crew.length).toBeGreaterThanOrEqual(1);
  expect(snapshot.world.bands.length).toBeGreaterThan(0);
  expect(snapshot.world.features.length).toBeGreaterThan(0);

  // The catalog is present and the pack hash is a real u64.
  const catalog = await page.evaluate(() => window.__understory!.catalog());
  expect(catalog.content_hash).toMatch(/^[0-9a-f]{16}$/);
  expect(catalog.rooms.length).toBeGreaterThan(0);

  // Labels render over the canvas.
  await expect(page.locator(".stage-labels .label-room").first()).toBeVisible();

  expect(errors).toEqual([]);
});

test("the chain runs unattended and the tower walks", async ({ page }) => {
  await boot(page);

  // Drive the simulation directly rather than waiting out real time:
  // the smoke test is about the loop being wired, not about pacing.
  const before = await page.evaluate(() => window.__understory!.view());
  await page.evaluate(() => {
    window.__understory!.step(1800);
  });
  const after = await page.evaluate(() => window.__understory!.view());

  expect(after.tick).toBeGreaterThan(before.tick);
  expect(after.world.distance).toBeGreaterThan(before.world.distance);
  expect(after.stats.items_harvested).toBeGreaterThan(0);
  expect(after.stats.hauls_completed).toBeGreaterThan(0);
  expect(after.stats.crafts_completed).toBeGreaterThan(0);
});

test("speed controls drive the clock", async ({ page }) => {
  await boot(page);

  await page.getByTestId("speed-X4").click();
  await expect(page.getByTestId("speed-X4")).toHaveAttribute("aria-pressed", "true");

  const running = await page.evaluate(() => window.__understory!.view().tick);
  await page.waitForTimeout(400);
  const later = await page.evaluate(() => window.__understory!.view().tick);
  expect(later).toBeGreaterThan(running);

  await page.getByTestId("speed-Paused").click();
  await expect(page.getByTestId("speed-Paused")).toHaveAttribute("aria-pressed", "true");
  const paused = await page.evaluate(() => window.__understory!.view().tick);
  await page.waitForTimeout(300);
  const stillPaused = await page.evaluate(() => window.__understory!.view().tick);
  expect(stillPaused).toBe(paused);
});

test("building a floor and placing a room round-trips through the bridge", async ({ page }) => {
  await boot(page);

  // Bank enough poles for a floor plus a room.
  await page.evaluate(() => {
    window.__understory!.step(3600);
  });

  const floorsBefore = await page.evaluate(() => window.__understory!.view().tower.floors.length);
  await page.getByTestId("build-floor").click();
  await expect
    .poll(() => page.evaluate(() => window.__understory!.view().tower.floors.length))
    .toBe(floorsBefore + 1);

  // Placement: pick the room, click a slot in the tower, see it appear.
  const roomsBefore = await page.evaluate(() =>
    window.__understory!.view().tower.floors.reduce((n, floor) => n + floor.rooms.length, 0),
  );
  await page.getByTestId("build-room.storeroom").click();
  await expect(page.getByTestId("build-room.storeroom")).toHaveAttribute("aria-pressed", "true");

  // Click where the player would click: the renderer reports the slot's
  // own screen position, so this survives any change to tower scale.
  const target = await page.evaluate(() => {
    const view = window.__understory!.view();
    const top = view.tower.floors.length - 1;
    // Slot 0 is the stairs column on every floor; the new top floor is
    // otherwise empty, so slot 2 leaves room for a two-wide storeroom.
    return window.__understory!.slotPoint(top, 2);
  });
  if (!target) throw new Error("the renderer could not locate the target slot");
  await page.mouse.click(target.x, target.y);

  await expect
    .poll(() =>
      page.evaluate(() =>
        window.__understory!.view().tower.floors.reduce((n, floor) => n + floor.rooms.length, 0),
      ),
    )
    .toBeGreaterThan(roomsBefore);
});

test("wasm and native agree on every state hash", async ({ page }) => {
  await boot(page);

  // The golden fixture is embedded in the wasm binary — the exact same
  // bytes the native `cargo test` suite verifies. If both pass, the two
  // platforms produced identical state hashes for every checkpoint of a
  // 1800-tick recorded session.
  const report = await page.evaluate(() => window.__understory!.verifyGolden());
  expect(report.ok, report.message).toBe(true);
  expect(report.checked).toBeGreaterThan(10);
  expect(report.final_tick).toBeGreaterThanOrEqual(1800);
});

test("the same seed produces the same run", async ({ page }) => {
  await boot(page, 777);
  await page.evaluate(() => {
    window.__understory!.step(900);
  });
  const first = await page.evaluate(() => window.__understory!.stateHash());

  await boot(page, 777);
  await page.evaluate(() => {
    window.__understory!.step(900);
  });
  const second = await page.evaluate(() => window.__understory!.stateHash());

  expect(second).toBe(first);
  expect(first).toMatch(/^[0-9a-f]{16}$/);
});
