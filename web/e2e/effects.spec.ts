import { test } from "@playwright/test";

/**
 * Not a test — a harness for looking at the two effects nothing else can
 * reach.
 *
 * `npx playwright test effects` writes `capture/fx-smoke.png` and
 * `capture/fx-water.png`.
 *
 * `capture.spec.ts` cannot photograph either. It never builds a burner,
 * so it has never drawn smoke; and its drowned-city stills land wherever
 * a script full of real-time waits happens to stop, which on one run was
 * the coast. Both effects are conditional — smoke only from a burner
 * that is *burning*, water only where the band underfoot is drowned
 * street — so a still that misses the condition looks exactly like a
 * feature that does not work.
 *
 * The water probe walks until the band underfoot is drowned street
 * rather than until the region changes. An earlier version waited for
 * `journey.region > 0` first and never matched: the drowned street band
 * turns up inside region 1, at about 59,600 paces.
 */
test("smoke", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/?seed=4242");
  await page.waitForFunction(() => window.__understory?.slotPoint !== undefined, null, {
    timeout: 20_000,
  });
  const built = await page.evaluate(() => {
    const h = window.__understory!;
    h.send({ SetStriding: { walking: true } });
    for (let i = 0; i < 40; i += 1) h.step(600);
    let placed = "no";
    for (let floor = 0; floor < 4 && placed === "no"; floor += 1) {
      for (let slot = 0; slot < 8; slot += 1) {
        if (h.send({ PlaceRoom: { room: "room.burner", floor, slot } }) === "Ok") {
          placed = `floor ${floor} slot ${slot}`;
          break;
        }
      }
    }
    const burner = () => {
      const cat = h.catalog();
      for (const f of h.view().tower.floors) {
        for (const r of f.rooms) {
          if (cat.rooms[r.def]?.burner)
            return { fuel: r.inputs[0]?.count ?? 0, stalled: r.stalled };
        }
      }
      return { fuel: -1, stalled: true };
    };
    for (let i = 0; i < 200; i += 1) {
      const fork = h.view().journey.fork;
      if (fork && fork.answer === null) h.send({ TakeFork: { branch: 0 } });
      h.send({ SetStriding: { walking: true } });
      h.step(120);
      const b = burner();
      if (b.fuel > 0 && !b.stalled) break;
    }
    const b = burner();
    return `${placed}, fuel ${b.fuel}, stalled ${String(b.stalled)}`;
  });
  console.log(`burner: ${built}`);
  await page.waitForTimeout(400);
  await page.screenshot({ path: "capture/fx-smoke.png" });
});

test("water", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/?seed=4242");
  await page.waitForFunction(() => window.__understory?.slotPoint !== undefined, null, {
    timeout: 20_000,
  });
  const found = await page.evaluate(() => {
    const h = window.__understory!;
    const cat = h.catalog();
    const want = cat.terrain.findIndex((t) => t.id === "terrain.drowned_street");
    let last = "";
    for (let i = 0; i < 4000; i += 1) {
      const v = h.view();
      const fork = v.journey.fork;
      if (fork && fork.answer === null) h.send({ TakeFork: { branch: 0 } });
      h.send({ SetStriding: { walking: true } });
      if (v.world.band === want) {
        return `standing in drowned street at ${Math.round(v.world.distance)} paces, region ${v.journey.region}`;
      }
      last = `${Math.round(v.world.distance)} paces, region ${v.journey.region}, band ${String(v.world.band)}`;
      h.step(60);
    }
    return `never found it — last: ${last}`;
  });
  console.log(`water: ${found}`);
  await page.waitForTimeout(400);
  await page.screenshot({ path: "capture/fx-water.png" });
});
