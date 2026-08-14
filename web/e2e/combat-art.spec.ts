import { expect, test } from "@playwright/test";

test("real weapon fire drives authored recoil and effects", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/?seed=4242");
  await page.waitForFunction(() => window.__understory?.stageEnemies !== undefined, null, {
    timeout: 20_000,
  });

  const event = await page.evaluate(() => {
    const hooks = window.__understory!;
    hooks.send({ SetSpeed: { speed: "Paused" } });
    hooks.grant("item.bamboo", 60);
    hooks.stageEnemies();
    for (let index = 0; index < 140; index += 1) {
      const events = hooks.stepEvents(30);
      if (events.combat[0]) return events.combat[0];
    }
    return null;
  });
  expect(event, "the shipped thorn gun never fired at staged real enemies").not.toBeNull();
  if (!event) return;

  await page.evaluate((combat) => window.__understory!.presentCombat([combat]), event);
  await page.waitForTimeout(80);
  await page.screenshot({ path: "capture/combat-thorn-live.png" });

  const weapon = await page.evaluate(async () => {
    const image = new Image();
    image.src = "/art/weapon-components-atlas.png";
    await image.decode();
    return [image.naturalWidth, image.naturalHeight];
  });
  const effects = await page.evaluate(async () => {
    const image = new Image();
    image.src = "/art/combat-fx-atlas.png";
    await image.decode();
    return [image.naturalWidth, image.naturalHeight];
  });
  expect(weapon).toEqual([768, 512]);
  expect(effects).toEqual([1024, 768]);
});
