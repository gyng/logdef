import { expect, test } from "@playwright/test";

const VIEWPORTS = [
  { width: 1600, height: 900, name: "1600x900" },
  { width: 1200, height: 760, name: "1200x760" },
] as const;

/**
 * The chrome is deliberately dense, so its visual checkpoint also guards
 * against the two layout failures that a wide-only still cannot reveal:
 * controls continuing past the right edge, and passing-event cards covering
 * the roster at the compact desktop size.
 */
for (const viewport of VIEWPORTS) {
  test(`capture hybrid UI at ${viewport.name}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto("/?seed=4242");
    await page.waitForFunction(() => window.__understory !== undefined, null, {
      timeout: 20_000,
    });
    const waypointReached = await page.evaluate(() => {
      const hooks = window.__understory!;
      hooks.send({ SetStriding: { walking: true } });
      for (let attempt = 0; attempt < 200; attempt += 1) {
        if (hooks.view().journey.waypoint !== null) return true;
        hooks.step(300);
      }
      return false;
    });
    expect(waypointReached).toBe(true);
    await page.waitForTimeout(400);

    const layout = await page.evaluate(() => {
      const topbar = document.querySelector(".topbar")?.getBoundingClientRect() ?? null;
      const roster = document.querySelector(".roster")?.getBoundingClientRect() ?? null;
      const waypoint = document.querySelector(".waypoint")?.getBoundingClientRect() ?? null;
      const zoomIn =
        document.querySelector('[data-testid="zoom-in"]')?.getBoundingClientRect() ?? null;
      return {
        scrollWidth: document.documentElement.scrollWidth,
        viewportWidth: window.innerWidth,
        topbarRight: topbar?.right ?? Number.POSITIVE_INFINITY,
        zoomRight: zoomIn?.right ?? Number.POSITIVE_INFINITY,
        waypointOverRoster:
          waypoint !== null &&
          roster !== null &&
          waypoint.left < roster.right &&
          waypoint.right > roster.left &&
          waypoint.top < roster.bottom &&
          waypoint.bottom > roster.top,
      };
    });

    expect(layout.scrollWidth).toBeLessThanOrEqual(layout.viewportWidth);
    expect(layout.topbarRight).toBeLessThanOrEqual(layout.viewportWidth);
    expect(layout.zoomRight).toBeLessThanOrEqual(layout.viewportWidth);
    expect(layout.waypointOverRoster).toBe(false);

    await page.screenshot({ path: `capture/ui-hybrid-${viewport.name}.png` });

    const zoomIn = page.getByTestId("zoom-in");
    await zoomIn.focus();
    const focus = await zoomIn.evaluate((button) => {
      const style = getComputedStyle(button);
      return { width: style.outlineWidth, style: style.outlineStyle };
    });
    expect(focus.style).not.toBe("none");
    expect(Number.parseFloat(focus.width)).toBeGreaterThanOrEqual(2);

    const pressed = page.locator('[aria-label="Simulation speed"] [aria-pressed="true"]');
    const raised = page.locator('[aria-label="Simulation speed"] [aria-pressed="false"]').first();
    const keyStates = await Promise.all([
      pressed.evaluate((button) => {
        const style = getComputedStyle(button);
        return { background: style.backgroundImage, transform: style.transform };
      }),
      raised.evaluate((button) => {
        const style = getComputedStyle(button);
        return { background: style.backgroundImage, transform: style.transform };
      }),
    ]);
    expect(keyStates[0].background).not.toBe(keyStates[1].background);
    expect(keyStates[0].transform).not.toBe(keyStates[1].transform);
  });

  test(`capture crew portrait roster at ${viewport.name}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto("/?seed=4242");
    await page.waitForFunction(() => window.__understory !== undefined, null, {
      timeout: 20_000,
    });

    const roster = page.getByTestId("roster");
    await expect(roster).toBeVisible();
    await expect(roster.locator(".roster-row")).toHaveCount(3);
    await expect(roster.locator(".roster-portrait.loaded")).toHaveCount(3, { timeout: 10_000 });

    const cells = await roster.locator(".roster-face").evaluateAll((faces) =>
      faces.map((face) => ({
        name: face.parentElement?.querySelector(".roster-name")?.textContent,
        cell: face.getAttribute("data-crew-cell"),
      })),
    );
    expect(cells).toEqual([
      { name: "Wren", cell: "0" },
      { name: "Odile", cell: "1" },
      { name: "Bakri", cell: "2" },
    ]);

    await roster.screenshot({ path: `capture/crew-portraits-${viewport.name}.png` });
  });
}
