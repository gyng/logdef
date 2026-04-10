import { test, expect } from "@playwright/test";

test("full game loop: start → 3 encounters → victory", async ({ page }) => {
  page.on("pageerror", (err) => console.log("PAGE_ERROR:", err.message));

  await page.goto("/");

  // ── Game loads ────────────────────────────────────────
  await expect(page.locator(".map-page")).toBeVisible({ timeout: 10_000 });

  // ── Node 1: select, prep, march ───────────────────────
  await page.locator(".map-node.reachable").first().click();
  await expect(page.locator(".prep-controls")).toBeVisible({ timeout: 5_000 });
  await page.getByRole("button", { name: /Build Wood Floor/ }).click();
  await page.getByRole("button", { name: /Place Fletcher/ }).click();
  await page.getByRole("button", { name: "March!" }).click();

  // ── Combat 1 ──────────────────────────────────────────
  await expect(page.locator(".combat-page")).toBeVisible({ timeout: 5_000 });
  await fireUntilDone(page);
  await expect(page.locator(".post-combat-page")).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: "Continue" }).click();

  // ── Node 2 ────────────────────────────────────────────
  await expect(page.locator(".map-page")).toBeVisible({ timeout: 5_000 });
  await page.locator(".map-node.reachable").first().click();
  await expect(page.locator(".prep-controls")).toBeVisible({ timeout: 5_000 });
  await page.getByRole("button", { name: "March!" }).click();
  await expect(page.locator(".combat-page")).toBeVisible({ timeout: 5_000 });
  await fireUntilDone(page);
  await expect(page.locator(".post-combat-page")).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: "Continue" }).click();

  // ── Node 3 (boss) ─────────────────────────────────────
  await expect(page.locator(".map-page")).toBeVisible({ timeout: 5_000 });
  await page.locator(".map-node.reachable").first().click();
  await expect(page.locator(".prep-controls")).toBeVisible({ timeout: 5_000 });
  await page.getByRole("button", { name: "March!" }).click();
  await expect(page.locator(".combat-page")).toBeVisible({ timeout: 5_000 });
  await fireUntilDone(page);
  await expect(page.locator(".post-combat-page")).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: "Continue" }).click();

  // ── Victory ───────────────────────────────────────────
  await expect(page.locator(".end-screen")).toBeVisible({ timeout: 5_000 });
  await expect(page.locator("h1")).toContainText("Victory");
});

async function fireUntilDone(page: import("@playwright/test").Page) {
  const canvas = page.locator(".combat-canvas");
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    const done = await page
      .locator(".post-combat-page, .end-screen")
      .isVisible()
      .catch(() => false);
    if (done) return;
    // Click to fire (one shot at a time, let sim process between)
    await canvas.click({ position: { x: 400, y: 200 } }).catch(() => {});
    await page.waitForTimeout(150);
  }
}
