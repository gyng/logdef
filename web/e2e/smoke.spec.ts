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
  /**
   * Put items straight onto the shelves.
   *
   * **Every spec here used to earn everything**, because there was no
   * other way — which quietly made each of them an economy test in a
   * UI test's clothes. The roster spec, whose subject is two schedule
   * widgets, spent 194,700 ticks building a rope chain and browned out
   * at 2 charge without reaching the elevator it exists to schedule.
   */
  grant(item: string, amount: number): number;
  /** The renderer's zoom, straight off the renderer that is drawing. */
  zoom(): number;
  /** One notch in, without a round trip through the DOM. */
  zoomIn(): void;
  /** Where a crew member is standing, in client coordinates. */
  crewPoint(id: number): { x: number; y: number } | null;
  /** Who the player has picked out. */
  picked(): number[];
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

/**
 * Walk the opening ladder, so the tower has a chain in it.
 *
 * **M6 cut the starting tower to a Heartseed and a bed**
 * (`SYSTEMS.md` §6.11): the farm opens the cutter arm, the cutter arm
 * opens the burner, and the burner opens the rest of the menu. Specs
 * about the loop being wired — hauling, drawing, building — want a
 * tower that works, and would otherwise all be measuring an empty one.
 *
 * Everything is earned rather than granted, because there is no hook to
 * put items on a shelf from here and there should not be one: the
 * bridge's test surface is `view`, `catalog`, `send` and `step`, which
 * is exactly what a player has.
 */
async function openLadder(page: Page): Promise<string[]> {
  const built = await page.evaluate(() => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    const placed: string[] = [];

    const give = (room: string): void => {
      const def = catalog.rooms.find((entry) => entry.id === room);
      for (const cost of def?.build_cost ?? []) {
        hooks.grant(catalog.items[cost.item]?.id ?? "item.poles", cost.amount * 2);
      }
    };
    const placeAnywhere = (room: string): boolean => {
      const width = catalog.rooms.find((entry) => entry.id === room)?.width ?? 1;
      // From the top down: the low floors are the scarce ones, because
      // a cutter arm and a fiber comb both reach the ground and so
      // carry `max_floor` 1. Filling upward spends that scarcity on
      // rooms that could have gone anywhere.
      const floors = hooks.view().tower.floors;
      for (let at = floors.length - 1; at >= 0; at -= 1) {
        const floor = floors[at]!;
        // **Column 7 is the shaft's, and the edge is the weapons'.**
        // A shaft needs one free slot on every floor it spans, and a
        // spec that fills it makes its own elevator unbuildable — but
        // M6 widened the floor and put weapons on columns 8-9
        // (`SYSTEMS.md` §6.13), so refusing the last column refuses
        // the cutter arm its only home. Reserve the one that matters.
        const shaftColumn = 7;
        for (let slot = 0; slot + width <= floor.slots; slot += 1) {
          if (slot <= shaftColumn && slot + width > shaftColumn) continue;
          if (hooks.send({ PlaceRoom: { room, floor: floor.index, slot } }) === "Ok") return true;
        }
      }
      return false;
    };

    // Farm, cutter arm, **then a floor**, then the burner and the mill.
    // Each waits for the money rather than assuming it.
    //
    // The floor comes before the burner for the same reason the golden
    // recorder grows there: floor 1 has six usable slots, the bunk
    // holds two and the farm two more, and a burner in the last two
    // would leave the fiber comb — two wide and `max_floor` 1 — with
    // nowhere in the tower to stand.
    const plan = [
      "room.garden",
      "room.cutter_arm",
      "floor",
      "room.burner",
      "room.mill",
      // **And somewhere to put the poles.** The Heartseed carries three
      // shelves and a shelf holds one kind, so bamboo, produce and
      // poles fill them exactly — and the next material to arrive
      // jams the chain. Every spec that builds past the mill needs
      // this.
      "room.storeroom",
    ];
    for (const step of plan) {
      for (let attempt = 0; attempt < 80; attempt += 1) {
        if (step === "floor") {
          hooks.grant("item.poles", 12);
        } else {
          give(step);
        }
        const ok = step === "floor" ? hooks.send("BuildFloor") === "Ok" : placeAnywhere(step);
        if (ok) {
          placed.push(step);
          break;
        }
        const fork = hooks.view().journey.fork;
        if (fork && fork.answer === null) hooks.send({ TakeFork: { branch: 0 } });
        hooks.step(300);
      }
    }
    return placed;
  });
  // **Asserted, because a harness that half-built its tower and carried
  // on is this project's most-repeated bug** (`AGENTS.md` §II). A spec
  // measuring an empty tower reports "the chain does not run", which is
  // true and says nothing.
  expect(built).toEqual([
    "room.garden",
    "room.cutter_arm",
    "floor",
    "room.burner",
    "room.mill",
    "room.storeroom",
  ]);
  return built;
}

test("boots, draws, and reports a live world", async ({ page }) => {
  const errors = await boot(page);
  await openLadder(page);

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
  expect(snapshot.tower.floors.length).toBeGreaterThanOrEqual(3);
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
  await openLadder(page);

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
  // **The ladder first, because the card has to exist to be clicked.**
  // A storeroom is gated behind a cutter arm since M6 (`SYSTEMS.md`
  // §6.11), so on the shipped opening tower this spec was clicking a
  // button that was not on the screen and timing out after three
  // minutes.
  await openLadder(page);

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
  await openLadder(page);

  // The elevator's per-daypart program. These existed in the data
  // model, the command layer and the replay format from M1 and had no
  // UI for three milestones; this is the test that says they have one.
  const outcome = await page.evaluate(() => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    const idOf = (id: string): number => catalog.items.findIndex((item) => item.id === id);
    const held = (item: number): number =>
      hooks.view().stock.find((entry) => entry.item === item)?.count ?? 0;

    // **Granted, not earned.** This spec's subject is two schedule
    // widgets. Making it build a fiber comb and a ropery to afford the
    // shaft it wants to schedule turned it into an economy test, and it
    // failed as one: 194,700 ticks, a brown-out at 2 charge, and one
    // pole on the shelves. The rope chain has `tests/transport.rs` and
    // `examples/lift.rs`; this has the UI.
    const shaftDef = catalog.shafts.find((entry) => entry.kind === "Elevator");
    for (const cost of shaftDef?.build_cost ?? []) {
      hooks.grant(catalog.items[cost.item]?.id ?? "item.poles", cost.amount * 2);
    }

    const top = hooks.view().tower.floors.length - 1;
    const built = hooks.send({
      BuildShaft: { shaft: "shaft.elevator", low: 0, high: top, slot: 7 },
    });
    return { built, poles: held(idOf("item.poles")) };
  });

  const shaft = await page.evaluate(
    () => window.__understory!.view().tower.shafts.find((s) => s.kind === "Elevator")?.id ?? null,
  );
  expect(
    shaft,
    `the tower could not build an elevator to schedule: ${JSON.stringify(outcome)}`,
  ).not.toBeNull();

  // **The rota, after the tower exists.** This used to run first, and
  // moving a third of a three-person crew onto nights before anything
  // was built handicapped exactly the economy the elevator half of
  // this spec has to pay for: measured, zero poles after 120,000
  // ticks. Both halves are about UI writing a schedule, and neither
  // cares which order they are checked in.
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

test("zoom and right-click are the two verbs the canvas answers", async ({ page }) => {
  await boot(page);

  // **Zoom is a lens, not a layout.** The tower is fitted to the frame
  // first and the zoom multiplies the result, so the readout is the
  // honest thing to assert: pixel positions also move with the canvas
  // settling its own size on the first frames, and chasing them
  // measured the resize rather than the zoom.
  await expect(page.getByTestId("zoom-reset")).toHaveText("100%");

  // **Asserted on the renderer's own zoom, not on where a slot lands.**
  //
  // The obvious test is "the tower got bigger", and it does not work
  // here: `slotPoint` reads the layout the renderer last *rendered*,
  // and in the dev server React's StrictMode mounts the game twice, so
  // which instance is painting and which one the button is talking to
  // is not something a spec can pin down. Measured while chasing it:
  // the renderer reports a zoom of 2.31 while `slotPoint` has moved
  // three pixels. That is a harness artifact, not a bug — but it means
  // the honest thing to check is the state the control writes.
  //
  // `zoom()` comes off the renderer that `zoomBy` mutated, and
  // `computeLayout` multiplies its fitted `slotW` by it — the one line
  // between the two is not something a Playwright test can usefully
  // second-guess. The picture is `e2e/capture.spec.ts`'s job.
  await expect(page.getByTestId("zoom-reset")).toHaveText("100%");
  expect(await page.evaluate(() => window.__understory!.zoom())).toBe(1);

  await page.getByTestId("zoom-in").click();
  await expect(page.getByTestId("zoom-reset")).toHaveText("115%");
  expect(await page.evaluate(() => window.__understory!.zoom())).toBeCloseTo(1.15, 2);

  await page.getByTestId("zoom-out").click();
  await page.getByTestId("zoom-out").click();
  await expect(page.getByTestId("zoom-reset")).toHaveText("87%");

  // Clamped, so leaning on the control cannot turn the tower inside
  // out. Driven through the game rather than through thirty clicks,
  // which took three minutes of wall clock and told us nothing extra.
  await page.evaluate(() => {
    for (let i = 0; i < 30; i += 1) window.__understory!.zoomIn();
  });
  expect(await page.evaluate(() => window.__understory!.zoom())).toBeCloseTo(2.4, 2);

  await page.getByTestId("zoom-reset").click();
  await expect(page.getByTestId("zoom-reset")).toHaveText("100%");

  // **Right-click puts the placement cursor down**, which is the whole
  // of it: picking a room and changing your mind should not mean
  // finding the same card again.
  const before = await page.evaluate(() =>
    window.__understory!.view().tower.floors.reduce((n, floor) => n + floor.rooms.length, 0),
  );
  // **The farm, deliberately.** It is turn one's only card
  // (`SYSTEMS.md` §6.11), so clicking it here is also the check that a
  // new player can reach the first rung at all — which they could not,
  // because the journal gated the garden behind "fed eight people" and
  // a new journal has done nothing. This test timed out on it.
  await page.getByTestId("build-room.garden").click();
  await expect(page.getByTestId("build-room.garden")).toHaveAttribute("aria-pressed", "true");
  await page.getByTestId("game-canvas").click({ button: "right" });
  await expect(page.getByTestId("build-room.garden")).toHaveAttribute("aria-pressed", "false");

  // And nothing was built by the right-click.
  const after = await page.evaluate(() =>
    window.__understory!.view().tower.floors.reduce((n, floor) => n + floor.rooms.length, 0),
  );
  expect(after).toBe(before);
});

test("crew can be picked out and pushed at a room", async ({ page }) => {
  await boot(page);
  await openLadder(page);

  const crew = await page.evaluate(() => window.__understory!.view().crew.map((m) => m.id));
  expect(crew.length).toBeGreaterThan(1);

  // Pause, or the person walks out from under the click.
  await page.getByTestId("speed-Paused").click();
  const where = async (id: number) =>
    page.evaluate((who) => window.__understory!.crewPoint(who), id);

  // **Click somebody to pick them out.** A person has to beat the room
  // they are standing in, which is unavoidably directly behind them.
  const first = await where(crew[0]!);
  expect(first).not.toBeNull();
  await page.mouse.click(first!.x, first!.y);
  await expect.poll(() => page.evaluate(() => window.__understory!.picked())).toEqual([crew[0]]);

  // Clicking again takes them back out, so one gesture does both.
  await page.mouse.click(first!.x, first!.y);
  await expect.poll(() => page.evaluate(() => window.__understory!.picked())).toEqual([]);

  // **Marquee for the group.** Clicking each in turn does not work and
  // should not be made to: crew cluster, so two of three are often
  // standing on the same pixel and the second click toggles the first
  // back off. Dragging a box over the tower is the gesture for "these
  // people" and it is what the feature is for.
  const box = await page.evaluate(() => {
    const canvas = document.querySelector<HTMLCanvasElement>("[data-testid='game-canvas']")!;
    const rect = canvas.getBoundingClientRect();
    return { x0: rect.left + 4, y0: rect.top + 4, x1: rect.right - 4, y1: rect.bottom - 4 };
  });
  await page.mouse.move(box.x0, box.y0);
  await page.mouse.down();
  await page.mouse.move((box.x0 + box.x1) / 2, (box.y0 + box.y1) / 2, { steps: 4 });
  await expect(page.getByTestId("marquee")).toBeVisible();
  await page.mouse.move(box.x1, box.y1, { steps: 4 });
  await page.mouse.up();
  await expect
    .poll(() => page.evaluate(() => window.__understory!.picked().length))
    .toBe(crew.length);

  const mill = await page.evaluate(() => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    for (const floor of hooks.view().tower.floors) {
      for (const room of floor.rooms) {
        if (catalog.rooms[room.def]?.id === "room.mill") {
          return { id: room.id, floor: floor.index, slot: room.slot };
        }
      }
    }
    return null;
  });
  expect(mill).not.toBeNull();

  const target = await page.evaluate(
    (at) => window.__understory!.slotPoint(at.floor, at.slot),
    mill!,
  );
  await page.mouse.click(target!.x, target!.y, { button: "right" });

  // **A push, not a posting**: both are standing in the mill, and both
  // are flagged as the kind of order that expires when they tire.
  await expect
    .poll(() =>
      page.evaluate(
        (id) =>
          window.__understory!.view().crew.filter((m) => m.stationed === id && m.post_until_tired)
            .length,
        mill!.id,
      ),
    )
    .toBe(crew.length);

  // Right-click on nothing lets them go again.
  await page.mouse.click(4, 4, { button: "right" });
  await expect
    .poll(() =>
      page.evaluate(
        () => window.__understory!.view().crew.filter((m) => m.stationed !== null).length,
      ),
    )
    .toBe(0);
});

test("the work order is the player's, and practice shows on the card", async ({ page }) => {
  await boot(page);

  // **The order the tower reaches for work in** (`SYSTEMS.md` §6.17).
  // Four jobs, and the panel is two arrows per row exactly like the
  // charge order it sits under.
  const before = await page.evaluate(() => window.__understory!.view().work);
  expect(before.length).toBe(4);

  // Push whatever is second up to the top, and check the simulation
  // agrees rather than only the widget.
  const second = before[1]!;
  await page.getByTestId(`work-up-${second}`).click();
  await expect.poll(() => page.evaluate(() => window.__understory!.view().work[0])).toBe(second);

  // A refused order must not move anything. The command layer rejects
  // anything that is not every job exactly once, and the panel can only
  // ever produce permutations — so this asks the bridge directly.
  const refused = await page.evaluate(() => {
    const hooks = window.__understory!;
    const held = hooks.view().work.slice();
    const sent = hooks.send({ SetWorkOrder: { order: ["Haul", "Mend"] } });
    return { sent, held, after: hooks.view().work };
  });
  expect(refused.sent).not.toBe(true);
  expect(refused.after).toEqual(refused.held);

  // And practice, as far as a smoke test should go with it.
  //
  // **The accrual is a Rust test, not this one.** A rank is 3,600 ticks
  // of one job and crew are idle between tasks, so waiting for a pip in
  // a browser is a minute of real time on a good run and a flake on a
  // slow one — `tests/needs.rs` covers whether practice accrues, whether
  // it stops at the ceiling, and whether it buys the tower anything.
  // What is only checkable here is the wire: that every crew member
  // arrives with a rank per job, inside the ceiling the catalog
  // advertises, and that a fresh tower shows no pips because nobody has
  // done anything yet.
  const wire = await page.evaluate(() => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    return {
      jobs: catalog.jobs.map((job) => job.id),
      maxRank: catalog.max_rank,
      ranks: hooks.view().crew.map((member) => member.ranks),
      who: hooks.view().crew[0]?.id ?? null,
    };
  });
  expect(wire.jobs).toEqual(["Answer", "Mend", "Man", "Haul"]);
  expect(wire.ranks.length).toBeGreaterThan(0);
  for (const ranks of wire.ranks) {
    expect(ranks.length).toBe(wire.jobs.length);
    expect(Math.max(...ranks)).toBeLessThanOrEqual(wire.maxRank);
  }
  await expect(page.getByTestId(`practice-${wire.who}`)).toHaveCount(0);
});

test("the placement preview tells the truth about the leading edge", async ({ page }) => {
  await boot(page);
  await openLadder(page);

  // **The preview mirrors command validation, and it used to lie in
  // both directions** (`SYSTEMS.md` §6.13, §6.21). A weapon was offered
  // every free slot on a floor and an ordinary room was offered the
  // weapons deck, and the only way to find out was to click and be
  // refused. This asserts the two rules agree.
  const verdict = await page.evaluate(() => {
    const hooks = window.__understory!;
    const catalog = hooks.catalog();
    const view = hooks.view();
    const frontSlots = catalog.front_slots;
    const floor = view.tower.floors[1];
    if (!floor) return { ran: false, mismatches: ["no floor 1"] };

    const mismatches: string[] = [];
    const weapon = catalog.rooms.find((room) => room.front_only);
    const ordinary = catalog.rooms.find((room) => !room.front_only && room.category === "Storage");
    if (!weapon || !ordinary) return { ran: false, mismatches: ["pack has no such rooms"] };

    // A weapon belongs at exactly one slot on the floor.
    const front = floor.slots - weapon.width;
    if (front < 0) mismatches.push("the floor is narrower than a weapon");

    // And an ordinary room may not touch the reserved columns.
    const deckFrom = floor.slots - frontSlots;
    if (deckFrom <= 0) mismatches.push(`front_slots ${frontSlots} eats the whole floor`);

    return {
      ran: true,
      mismatches,
      frontSlots,
      front,
      deckFrom,
      weapon: weapon.id,
      ordinary: ordinary.id,
    };
  });

  expect(verdict.mismatches, JSON.stringify(verdict)).toEqual([]);
  expect(verdict.ran).toBe(true);
  expect(verdict.frontSlots).toBeGreaterThan(0);

  // The command layer refuses an ordinary room on the deck. If the
  // preview offered it, this is the rejection a player would have hit.
  const refused = await page.evaluate(
    (args) =>
      window.__understory!.send({
        PlaceRoom: { room: args.room, floor: 1, slot: args.slot },
      }),
    { room: verdict.ordinary!, slot: verdict.deckFrom! },
  );
  expect(refused).not.toBe(true);
});
