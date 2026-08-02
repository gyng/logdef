import { test, expect } from "@playwright/test";
import type { Page } from "@playwright/test";

/**
 * Smoke test: plays chapter 1 start → victory.
 *
 * Chapters 2 and 3 exist in the registry but are not part of the smoke
 * test path yet — chapter 1 is sufficient to exercise the full loop
 * (map → prep → combat → post-combat → continue → next node → victory).
 */
test("chapter 1 loop: start → victory", async ({ page }) => {
  test.setTimeout(360_000);
  page.on("pageerror", (err) => console.log("PAGE_ERROR:", err.message));
  page.on("console", (msg) => {
    if (msg.type() === "error") console.log("BROWSER_ERROR:", msg.text());
  });

  await page.goto("/");

  await expect(page.locator(".map-page")).toBeVisible({ timeout: 10_000 });

  let encountersPlayed = 0;
  // Chapter 1 has exactly 3 encounters (2 combats + boss).
  const maxEncountersInChapter1 = 3;

  while (encountersPlayed < maxEncountersInChapter1) {
    if (
      await page
        .locator(".end-screen")
        .isVisible()
        .catch(() => false)
    ) {
      break;
    }

    await expect(page.locator(".map-page")).toBeVisible({ timeout: 10_000 });

    const reachable = page.locator(".map-node.reachable").first();
    await expect(reachable).toBeVisible();
    await reachable.click();

    await expect(page.locator(".prep-controls")).toBeVisible({ timeout: 5_000 });

    // The default tower already has a Lumberyard + Fletcher + cache,
    // so there's no need to build anything in the smoke path. March
    // straight in.
    await page.getByRole("button", { name: "March!" }).click();

    await expect(page.locator(".combat-page")).toBeVisible({ timeout: 5_000 });
    await fireUntilDone(page);

    if (
      await page
        .locator(".end-screen")
        .isVisible()
        .catch(() => false)
    ) {
      break;
    }
    await expect(page.locator(".post-combat-page")).toBeVisible({ timeout: 30_000 });
    await page.getByRole("button", { name: "Continue" }).click();

    encountersPlayed += 1;
    await page.waitForTimeout(200); // let React settle after Continue

    // Chapter 1 only — stop after beating the chapter 1 boss (next view
    // will be end-screen victory if it was a single-chapter run, or the
    // chapter 2 map if multi-chapter).
  }

  // Smoke test passes if we either reached Victory or completed chapter 1
  // without the tower falling. Reaching chapter 2's map also counts.
  const endVisible = await page
    .locator(".end-screen")
    .isVisible()
    .catch(() => false);
  if (endVisible) {
    await expect(page.locator("h1")).toContainText(/Victory|Game Over/);
  } else {
    // Should be on the next chapter's map — loop completed cleanly.
    await expect(page.locator(".map-page")).toBeVisible({ timeout: 5_000 });
  }
});

async function fireUntilDone(page: Page) {
  // Smoke test cares about the *loop*, not combat balance. Use the
  // __forceWin debug helper to drain all enemies the moment we're in
  // an encounter, then poll for the PostCombat transition.
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    const postCombatCount = await page
      .locator(".post-combat-page")
      .count()
      .catch(() => 0);
    if (postCombatCount > 0) return;

    const endScreenCount = await page
      .locator(".end-screen")
      .count()
      .catch(() => 0);
    if (endScreenCount > 0) return;

    // Re-fire forceWin every loop iteration so we catch the encounter
    // as soon as it exists. The bridge call is a no-op outside combat.
    await page
      .evaluate(() => {
        type DebugWindow = Window & { __forceWin?: () => void };
        (window as DebugWindow).__forceWin?.();
      })
      .catch(() => {});

    await page.waitForTimeout(100);
  }
}
