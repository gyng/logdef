import { expect, test, type Page } from "@playwright/test";

import type {
  SoundEvent,
  CatalogSnapshot,
  CommandResult,
  FeatureView,
  GameCommand,
  ReplayReport,
  ViewSnapshot,
} from "../src/bridge/types";

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
  /**
   * Anything that drives the engine without a player has to be able to
   * answer a fork, or it walks into one and photographs a parked tower
   * (`SYSTEMS.md` §3.3).
   */
  send(cmd: GameCommand): CommandResult;
  stateHash(): string;
  /** Step, and hand back the sounds that tick would have made. */
  step(ticks: number): SoundEvent[];
  verifyGolden(): ReplayReport;
  exportReplay(): string;
  /** Centre of a slot in client coordinates, from the live layout. */
  slotPoint(floor: number, slot: number): { x: number; y: number } | null;
  /**
   * Where the renderer would put a scattered feature, in client
   * coordinates, for a tower standing at `distance`.
   */
  featurePoint(feature: FeatureView, distance: number): { x: number; y: number } | null;
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

  // The tower halts at a fork it has not been given an answer for, and
  // an unattended harness that never answers measures a parked tower
  // with total confidence — the exact failure mode `SYSTEMS.md` §3.3
  // records this project having had already. So the smoke path walks
  // into one deliberately, checks the halt is a fork rather than a
  // hang, answers it, and checks the legs start again.
  const halted = await page.evaluate(() => {
    const hooks = window.__understory!;
    let budget = 60_000;
    while (budget > 0 && hooks.view().journey.halt !== "fork") {
      hooks.step(120);
      budget -= 120;
    }
    return hooks.view();
  });
  expect(halted.journey.halt).toBe("fork");
  expect(halted.journey.fork?.answer ?? null).toBeNull();

  const walking = await page.evaluate(() => {
    const hooks = window.__understory!;
    hooks.send({ TakeFork: { branch: 0 } });
    const at = hooks.view().world.distance;
    hooks.step(300);
    const now = hooks.view();
    return { moved: now.world.distance - at, halt: now.journey.halt };
  });
  expect(walking.moved).toBeGreaterThan(0);
  expect(walking.halt).not.toBe("fork");
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

  // Bank enough poles for a floor plus a room — by waiting for the
  // money rather than by stepping a fixed number of ticks. A fixed
  // 3,600 was marginal once room widths and intake rates moved, and a
  // smoke test that is *usually* rich enough is a smoke test that fails
  // in CI and passes on your machine.
  await page.evaluate(() => {
    const catalog = window.__understory!.catalog();
    const poles = catalog.items.findIndex((item) => item.id === "item.poles");
    const need =
      (catalog.floor_cost.find((cost) => cost.item === poles)?.amount ?? 6) +
      (catalog.rooms
        .find((room) => room.id === "room.storeroom")
        ?.build_cost.find((cost) => cost.item === poles)?.amount ?? 3);
    for (let i = 0; i < 40; i += 1) {
      const held =
        window.__understory!.view().stock.find((entry) => entry.item === poles)?.count ?? 0;
      if (held >= need) return;
      window.__understory!.step(600);
    }
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

/**
 * The two schedules the player writes, both through the roster panel.
 *
 * A rota and a shaft program are the same category of thing — something
 * written against the daypart clock — which is why they share a panel
 * (`SYSTEMS.md` §4.5, §4.8) and why they share a test. Both are also the
 * kind of UI that can look right and be wired to nothing, so this drives
 * them the way a player does, through the DOM, and checks the
 * simulation actually moved.
 */
test("the roster writes both of the player's schedules", async ({ page }) => {
  await boot(page);

  // The rota. One click should move one named person onto the night
  // shift and leave everybody else alone.
  const crew = await page.evaluate(() => window.__understory!.view().crew.map((m) => m.id));
  expect(crew.length).toBeGreaterThan(0);
  const who = crew[0]!;
  await expect(page.getByTestId(`shift-${who}`)).toHaveAttribute("aria-pressed", "false");
  await page.getByTestId(`shift-${who}`).click();
  await expect
    .poll(() =>
      page.evaluate((id) => window.__understory!.view().crew.find((m) => m.id === id)?.shift, who),
    )
    .toBe("Night");
  const others = await page.evaluate(
    (id) =>
      window
        .__understory!.view()
        .crew.filter((m) => m.id !== id)
        .every((m) => m.shift === "Day"),
    who,
  );
  expect(others).toBe(true);

  // The elevator's per-daypart program. These existed in the data
  // model, the command layer and the replay format from M1 and had no
  // UI for three milestones; this is the test that says they have one.
  await page.evaluate(() => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    const poles = catalog.items.findIndex((item) => item.id === "item.poles");
    const cost =
      catalog.shafts
        .find((shaft) => shaft.id === "shaft.elevator")
        ?.build_cost.find((entry) => entry.item === poles)?.amount ?? 18;
    for (let i = 0; i < 80; i += 1) {
      const held = hooks.view().stock.find((entry) => entry.item === poles)?.count ?? 0;
      if (held >= cost) break;
      hooks.step(600);
    }
    const top = hooks.view().tower.floors.length - 1;
    hooks.send({ BuildShaft: { shaft: "shaft.elevator", low: 0, high: top, slot: 7 } });
  });

  const shaft = await page.evaluate(
    () => window.__understory!.view().tower.shafts.find((s) => s.kind === "Elevator")?.id ?? null,
  );
  expect(shaft, "the tower could not afford an elevator to schedule").not.toBeNull();

  // Skip a floor this daypart, and check the program says so.
  await expect(page.getByTestId(`stop-${shaft}-1`)).toHaveAttribute("aria-pressed", "true");
  await page.getByTestId(`stop-${shaft}-1`).click();
  await expect
    .poll(() =>
      page.evaluate((id) => {
        const view = window.__understory!.view();
        const found = view.tower.shafts.find((s) => s.id === id);
        return found?.programs[view.clock.daypart]?.served[1] ?? null;
      }, shaft),
    )
    .toBe(false);
  // And that it is *this* daypart only — a program editor that silently
  // wrote every daypart would be a different, worse feature.
  const elsewhere = await page.evaluate((id) => {
    const view = window.__understory!.view();
    const found = view.tower.shafts.find((s) => s.id === id);
    return found?.programs.filter((program) => program.served[1] === false).length ?? 0;
  }, shaft);
  expect(elsewhere).toBe(1);
});
