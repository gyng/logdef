import { expect, test, type Locator, type Page } from "@playwright/test";

const VIEWPORTS = [
  { width: 1600, height: 900, name: "1600x900" },
  { width: 1200, height: 760, name: "1200x760" },
] as const;

type Rect = { bottom: number; left: number; right: number; top: number };

function overlaps(a: Rect | null, b: Rect | null): boolean {
  return (
    a !== null &&
    b !== null &&
    a.left < b.right &&
    a.right > b.left &&
    a.top < b.bottom &&
    a.bottom > b.top
  );
}

async function boot(page: Page, viewport: (typeof VIEWPORTS)[number]): Promise<void> {
  await page.setViewportSize(viewport);
  await page.goto("/?seed=4242");
  await page.waitForFunction(() => window.__understory?.slotPoint !== undefined, null, {
    timeout: 20_000,
  });
  await page.getByTestId("speed-Paused").click();
}

async function rect(locator: Locator): Promise<Rect | null> {
  return locator.evaluate((element) => {
    const bounds = element.getBoundingClientRect();
    return {
      bottom: bounds.bottom,
      left: bounds.left,
      right: bounds.right,
      top: bounds.top,
    };
  });
}

async function expectInViewport(page: Page, locator: Locator, label: string): Promise<void> {
  const bounds = await rect(locator);
  expect(bounds, `${label} has no bounds`).not.toBeNull();
  expect(bounds!.left, `${label} escapes the left edge`).toBeGreaterThanOrEqual(0);
  expect(bounds!.top, `${label} escapes the top edge`).toBeGreaterThanOrEqual(0);
  expect(bounds!.right, `${label} escapes the right edge`).toBeLessThanOrEqual(
    await page.evaluate(() => window.innerWidth),
  );
  expect(bounds!.bottom, `${label} escapes the bottom edge`).toBeLessThanOrEqual(
    await page.evaluate(() => window.innerHeight),
  );
}

async function expectPhysicalSurface(locator: Locator, label: string): Promise<void> {
  const surface = await locator.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      backgroundImage: style.backgroundImage,
      borderWidth: style.borderTopWidth,
      borderStyle: style.borderTopStyle,
      shadow: style.boxShadow,
    };
  });
  expect.soft(surface.backgroundImage, `${label} has no material texture`).not.toBe("none");
  expect.soft(Number.parseFloat(surface.borderWidth), `${label} has no bezel`).toBeGreaterThan(0);
  expect.soft(surface.borderStyle, `${label} bezel is not drawn`).not.toBe("none");
  expect.soft(surface.shadow, `${label} has no physical depth`).not.toBe("none");
}

async function expectInstrumentWell(locator: Locator, label: string): Promise<void> {
  const well = await locator.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      background: style.backgroundColor,
      borderWidth: style.borderTopWidth,
      shadow: style.boxShadow,
    };
  });
  expect.soft(well.background, `${label} has no instrument well`).not.toBe("rgba(0, 0, 0, 0)");
  expect.soft(Number.parseFloat(well.borderWidth), `${label} has no rim`).toBeGreaterThan(0);
  expect.soft(well.shadow, `${label} is flat`).toContain("inset");
}

async function selectFirstRoom(page: Page): Promise<void> {
  const roomPoint = await page.evaluate(() => {
    const hooks = window.__understory!;
    for (const floor of hooks.view().tower.floors) {
      const room = floor.rooms[0];
      if (room) return hooks.slotPoint(floor.index, room.slot + room.width / 2);
    }
    return null;
  });
  expect(roomPoint).not.toBeNull();
  await page.mouse.click(roomPoint!.x, roomPoint!.y);
  await expect(page.getByTestId("selection")).toBeVisible();
}

async function advanceToWaypoint(page: Page): Promise<void> {
  const reached = await page.evaluate(() => {
    const hooks = window.__understory!;
    hooks.send({ SetStriding: { walking: true } });
    for (let attempt = 0; attempt < 200; attempt += 1) {
      if (hooks.view().journey.waypoint !== null) return true;
      hooks.step(300);
    }
    return false;
  });
  expect(reached).toBe(true);
  await expect(page.getByTestId("waypoint")).toBeVisible();
}

/**
 * Paired visual checkpoints for every high-frequency console surface. These
 * deliberately assert physical cues and collisions rather than pixels: the
 * watercolor beneath the console is allowed to move, but a bezel becoming a
 * flat translucent card or a field board covering the roster is not.
 */
for (const viewport of VIEWPORTS) {
  test(`diegetic console checkpoints at ${viewport.name}`, async ({ page }) => {
    await boot(page, viewport);

    const topbar = page.locator(".topbar");
    const sidebar = page.locator(".sidebar");
    const roster = page.getByTestId("roster");
    await expectPhysicalSurface(topbar, "top instrument housing");
    await expectPhysicalSurface(sidebar, "build instrument housing");
    await expectPhysicalSurface(roster, "crew instrument housing");
    await expectInstrumentWell(page.locator(".journey-bar"), "journey track");
    await expectInstrumentWell(page.locator(".charge-bar"), "charge gauge");
    await expectInstrumentWell(page.locator(".weather-bar"), "attention gauge");
    await expectInViewport(page, topbar, "top instrument housing");
    await expectInViewport(page, sidebar, "build instrument housing");
    await expectInViewport(page, roster, "crew instrument housing");

    // The depth pass lives on the choices themselves: a card must name
    // what occupies the same scarce space or consumes the same economy,
    // without adding another planning screen.
    await expect(page.getByTestId("tradeoff-room.cutter_arm")).toContainText("low-deck");
    await expect(page.getByTestId("tradeoff-shaft.elevator")).toContainText("Busbar");
    await expect(page.getByTestId("tradeoff-shaft.busbar")).toContainText("carries no people");

    await expect(roster.locator(".roster-portrait.loaded")).toHaveCount(3, { timeout: 10_000 });
    for (const row of await roster.locator(".roster-row").all()) {
      await expectPhysicalSurface(row, "crew identity card");
    }
    await page.screenshot({ path: `capture/ui-diegetic-hud-${viewport.name}.png` });

    await page.getByTestId("chain-toggle").click();
    const chain = page.getByTestId("economy");
    await expect(chain).toBeVisible();
    await expectPhysicalSurface(chain, "chain instrument housing");
    await expectPhysicalSurface(page.locator(".economy-item").first(), "chain material cell");
    await expectInViewport(page, chain, "chain instrument housing");
    expect(overlaps(await rect(chain), await rect(roster)), "chain covers crew console").toBe(
      false,
    );
    await page.screenshot({ path: `capture/ui-diegetic-chain-${viewport.name}.png` });
    await page.getByTestId("chain-toggle").click();

    await selectFirstRoom(page);
    const selection = page.getByTestId("selection");
    await expectPhysicalSurface(selection, "room inspection plate");
    const roomAction = page.getByTestId("remove-room");
    await expectPhysicalSurface(selection, "room inspection plate");
    await expectPhysicalSurface(roomAction, "room action key");
    await expectInViewport(page, roomAction, "room action key");
    await sidebar.screenshot({ path: `capture/ui-diegetic-selection-${viewport.name}.png` });

    await advanceToWaypoint(page);
    const waypoint = page.getByTestId("waypoint");
    await expectPhysicalSurface(waypoint, "waypoint field board");
    await expectPhysicalSurface(page.getByTestId("waypoint-take"), "waypoint action key");
    await expectInViewport(page, waypoint, "waypoint field board");
    expect(overlaps(await rect(waypoint), await rect(roster)), "waypoint covers crew console").toBe(
      false,
    );
    await page.screenshot({ path: `capture/ui-diegetic-waypoint-${viewport.name}.png` });

    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
      viewport.width,
    );
  });
}

test("a room breaker controls that room without opening the sidebar inspector", async ({
  page,
}) => {
  await boot(page, VIEWPORTS[0]);
  const burnerId = await page.evaluate(() => {
    const hooks = window.__understory!;
    hooks.grant("item.poles", 100);
    hooks.grant("item.bamboo", 100);
    const place = (id: string): number | null => {
      const info = hooks.catalog().rooms.find((room) => room.id === id);
      if (!info) return null;
      for (const floor of hooks.view().tower.floors) {
        for (let slot = 0; slot + info.width <= floor.slots; slot += 1) {
          if (hooks.send({ PlaceRoom: { room: id, floor: floor.index, slot } }) !== "Ok") {
            continue;
          }
          return (
            hooks.view().tower.floors[floor.index]?.rooms.find((room) => room.slot === slot)?.id ??
            null
          );
        }
      }
      return null;
    };
    place("room.garden");
    place("room.cutter_arm");
    return place("room.burner");
  });
  expect(burnerId).not.toBeNull();

  const breaker = page.getByRole("button", { name: /switch off burner/i });
  await expect(breaker).toBeVisible();
  await expect(breaker).toHaveAttribute("aria-pressed", "true");
  await breaker.click();

  await expect(page.getByTestId("selection")).toHaveCount(0);
  await expect
    .poll(() =>
      page.evaluate((id) => {
        for (const floor of window.__understory!.view().tower.floors) {
          const room = floor.rooms.find((candidate) => candidate.id === id);
          if (room) return room.active;
        }
        return null;
      }, burnerId),
    )
    .toBe(false);
  await expect(page.getByRole("button", { name: /switch on burner/i })).toHaveAttribute(
    "aria-pressed",
    "false",
  );
});
