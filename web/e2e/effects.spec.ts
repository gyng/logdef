import { test } from "@playwright/test";

/**
 * Not a test — a harness for looking at the two effects nothing else can
 * reach.
 *
 * `npx playwright test effects` writes `capture/fx-smoke.png`,
 * `capture/fx-water.png`, and a pair of `capture/fx-bank-*.png`.
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
 *
 * The bank probe is the same problem a third time: the cell rack draws
 * `power.fill_permille`, so a still of a rack that happens to be full
 * and a still of a renderer that ignores the field are the same picture.
 * It photographs one rack twice, at the top and the bottom of the
 * tower's own charge cycle, and prints both fills — two shots that
 * differ are the evidence.
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

test("bank", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/?seed=4242");
  await page.waitForFunction(() => window.__understory?.slotPoint !== undefined, null, {
    timeout: 20_000,
  });

  // **The rack is the heartseed's, and it has always been there.** Two
  // wrong turns got here. First this probe tried to place a cell bank at
  // minute zero — refused, because one costs two charge cells and those
  // are the end of the tier-two chain — and photographed a tower with no
  // rack on it, which looks exactly like a rack that does not draw.
  // Then it bought the whole chain to earn one, and arrived at 17,000
  // paces with more load than sail and a bank pinned at 4‰: a real
  // failure state, and a useless picture, because a flat bank has no
  // swing in it to photograph.
  //
  // So: sails and one mill, which is the smallest tower with both an
  // income and an appetite, and let the day do the rest.
  const built = await page.evaluate(() => {
    const h = window.__understory!;
    const cat = h.catalog();
    const racks = h
      .view()
      .tower.floors.flatMap((f) => f.rooms)
      .filter((r) => (cat.rooms[r.def]?.bank_capacity ?? 0) > 0).length;
    const list = ["room.canopy_sails", "room.mill"];
    for (let i = 0; i < 400 && list.length > 0; i += 1) {
      const fork = h.view().journey.fork;
      if (fork && fork.answer === null) h.send({ TakeFork: { branch: 0 } });
      h.send({ SetStriding: { walking: true } });
      for (let at = list.length - 1; at >= 0; at -= 1) {
        let done = false;
        for (let floor = 0; floor < 6 && !done; floor += 1) {
          for (let slot = 0; slot < 8; slot += 1) {
            if (h.send({ PlaceRoom: { room: list[at]!, floor, slot } }) === "Ok") {
              done = true;
              break;
            }
          }
        }
        if (done) list.splice(at, 1);
      }
      h.step(120);
    }
    return `${racks} rack(s) on the starting tower; unbought: ${list.join(" ") || "nothing"}`;
  });
  console.log(`bank: ${built}`);

  // Then let the day cycle do the work: the tower earns through the
  // afternoon and spends through the night, so the two ends of that
  // swing are the two pictures. Stop *on* the threshold rather than
  // tracking a best — an earlier version noted the extreme, kept
  // walking, and photographed whatever came after it.
  const seek = async (want: "high" | "low") =>
    page.evaluate((dir) => {
      const h = window.__understory!;
      let seen = dir === "high" ? -1 : 1001;
      for (let i = 0; i < 4000; i += 1) {
        const v = h.view();
        const fork = v.journey.fork;
        if (fork && fork.answer === null) h.send({ TakeFork: { branch: 0 } });
        h.send({ SetStriding: { walking: true } });
        const fill = v.power.fill_permille;
        seen = dir === "high" ? Math.max(seen, fill) : Math.min(seen, fill);
        if (dir === "high" ? fill >= 880 : fill <= 500) break;
        h.step(10);
      }
      return `${dir}: stopped at ${h.view().power.fill_permille}permille (best ${seen})`;
    }, want);

  for (const want of ["high", "low"] as const) {
    console.log(`bank ${await seek(want)}`);
    await page.waitForTimeout(400);
    await page.screenshot({ path: `capture/fx-bank-${want}.png` });
  }
});

/**
 * Does a jammed staircase look jammed?
 *
 * `climb_ticks_per_item` made freight up the stairs expensive on
 * purpose, and `examples/lift.rs` measures a fourteen-floor stairs-only
 * tower spending 46,642 crew-ticks queued at a shaft. That is a *felt*
 * problem only if the queue is visible: crew in `Boarding` stand at the
 * shaft column, so a backed-up staircase should read as a stack of
 * people against the tower's left edge with the red stress tint on the
 * ones who have been there longest.
 *
 * If it does not read, the measured problem is invisible and a player
 * has no reason to buy the cure — which is the whole chain of reasoning
 * behind the last few balance changes.
 *
 * **It has not answered the question yet, and the reason is the
 * finding.** A scripted tower given nine thousand iterations to grow to
 * eight floors reaches **five**, and queues nobody: it runs out of poles
 * long before it runs out of ambition. So the congestion `lift.rs`
 * measures at eight and eleven floors is real but sits past where a
 * tower can afford to go, and a still of a five-floor tower says nothing
 * about a jammed staircase.
 *
 * What the still does show is that the stairwell is drawn — a hatched
 * column at slot 0, clearly a stair — so there is somewhere for a queue
 * to appear. Whether a queue in it reads as a queue is still open, and
 * wants either a tower with more poles than this one or a player.
 */
test("stairs", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/?seed=4242");
  await page.waitForFunction(() => window.__understory?.slotPoint !== undefined, null, {
    timeout: 20_000,
  });

  const built = await page.evaluate(() => {
    const h = window.__understory!;
    // Tall, fed, and with no shaft but the stairs — the shape the
    // instrument says is worst.
    const list = [
      "room.canopy_sails",
      "room.storeroom",
      "room.mill",
      "room.canteen",
      "room.bunk",
      "room.storeroom",
      "room.mill",
    ];
    for (let i = 0; i < 9000 && (list.length > 0 || h.view().tower.floors.length < 8); i += 1) {
      const v = h.view();
      const fork = v.journey.fork;
      if (fork && fork.answer === null) h.send({ TakeFork: { branch: 0 } });
      h.send({ SetStriding: { walking: true } });
      if (v.tower.floors.length < 8) h.send("BuildFloor");
      for (let at = list.length - 1; at >= 0; at -= 1) {
        let done = false;
        for (let floor = 0; floor < 8 && !done; floor += 1) {
          for (let slot = 0; slot < 8; slot += 1) {
            if (h.send({ PlaceRoom: { room: list[at]!, floor, slot } }) === "Ok") {
              done = true;
              break;
            }
          }
        }
        if (done) list.splice(at, 1);
      }
      h.step(120);
    }
    // Walk on until somebody is actually queueing, then stop there.
    let queued = 0;
    for (let i = 0; i < 9000; i += 1) {
      const v = h.view();
      const fork = v.journey.fork;
      if (fork && fork.answer === null) h.send({ TakeFork: { branch: 0 } });
      h.send({ SetStriding: { walking: true } });
      queued = v.crew.filter((c) => c.state === "board").length;
      if (queued >= 1) break;
      h.step(10);
    }
    const v = h.view();
    return `${v.tower.floors.length} floors, ${v.crew.length} crew, ${queued} queued, ${
      v.crew.filter((c) => c.stressed).length
    } stressed, unbought: ${list.join(" ") || "nothing"}`;
  });
  console.log(`stairs: ${built}`);
  await page.waitForTimeout(400);
  await page.screenshot({ path: "capture/fx-stairs.png" });
});
