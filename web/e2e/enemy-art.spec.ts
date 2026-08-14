import { expect, test } from "@playwright/test";

/**
 * The creature renderer uses a hand-authored id -> atlas-cell map. This
 * checkpoint keeps the art side of that contract visible and checks the
 * transparent cutouts at the same 1:2 aspect ratio and approximate
 * near-contact size used by `Renderer.drawPaintedArt`.
 *
 * This is deliberately not presented as an encounter test. The browser test
 * bridge cannot stage a chosen creature, and natural wave selection cannot
 * deterministically put all eight species in one frame. When a creature
 * staging hook exists, this gallery should be complemented by approach and
 * contact captures through the real scene renderer.
 */
test("capture every creature atlas cell at gameplay scale", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/?seed=4242");
  await page.waitForFunction(() => window.__understory?.catalog !== undefined, null, {
    timeout: 20_000,
  });
  await page.evaluate(() => window.__understory!.send({ SetSpeed: { speed: "Paused" } }));

  const expected = [
    "enemy.skitter",
    "enemy.root_borer",
    "enemy.glean_crow",
    "enemy.canopy_leaper",
    "enemy.night_prowler",
    "enemy.mire_hulk",
    "enemy.thicket_mother",
    "enemy.feral_warden",
  ];

  const audit = await page.evaluate(async (ids) => {
    const catalog = window.__understory!.catalog();
    const image = new Image();
    image.src = "/art/creature-atlas.png";
    await image.decode();

    const canvas = document.createElement("canvas");
    canvas.width = image.naturalWidth;
    canvas.height = image.naturalHeight;
    const context = canvas.getContext("2d", { willReadFrequently: true });
    if (!context) throw new Error("2D canvas unavailable while auditing creature art");
    context.drawImage(image, 0, 0);

    const cellWidth = image.naturalWidth / ids.length;
    const cells = ids.map((id, index) => {
      const info = catalog.enemies.find((enemy) => enemy.id === id);
      const pixels = context.getImageData(index * cellWidth, 0, cellWidth, image.naturalHeight);
      let opaque = 0;
      let minX = cellWidth;
      let maxX = -1;
      let minY = image.naturalHeight;
      let maxY = -1;
      for (let y = 0; y < image.naturalHeight; y += 1) {
        for (let x = 0; x < cellWidth; x += 1) {
          const alpha = pixels.data[(y * cellWidth + x) * 4 + 3] ?? 0;
          if (alpha <= 12) continue;
          opaque += 1;
          minX = Math.min(minX, x);
          maxX = Math.max(maxX, x);
          minY = Math.min(minY, y);
          maxY = Math.max(maxY, y);
        }
      }
      return {
        id,
        name: info?.name ?? "missing",
        approach: info?.approach ?? "missing",
        opaque,
        bounds: opaque === 0 ? null : { minX, maxX, minY, maxY },
      };
    });

    const gallery = document.createElement("section");
    gallery.setAttribute("data-testid", "creature-art-gallery");
    gallery.style.cssText = [
      "position:fixed",
      "inset:54px 28px 28px",
      "z-index:10000",
      "display:grid",
      "grid-template-columns:repeat(8,minmax(0,1fr))",
      "gap:10px",
      "padding:24px",
      "background:rgba(20,27,25,.94)",
      "border:2px solid #766e56",
      "box-shadow:0 12px 48px rgba(0,0,0,.65)",
      "font:12px/1.25 monospace",
      "color:#d8d1b3",
    ].join(";");

    for (const [index, cell] of cells.entries()) {
      const card = document.createElement("article");
      card.style.cssText = [
        "display:flex",
        "min-width:0",
        "flex-direction:column",
        "align-items:center",
        "justify-content:flex-end",
        "gap:8px",
        "padding:12px 6px",
        "background:rgba(185,190,157,.06)",
        "border:1px solid rgba(216,209,179,.24)",
      ].join(";");

      const native = document.createElement("div");
      native.style.cssText = [
        "width:128px",
        "height:256px",
        "background-image:url('/art/creature-atlas.png')",
        `background-position:-${String(index * 128)}px 0`,
        "background-repeat:no-repeat",
      ].join(";");

      // A representative near-contact draw. The renderer varies floor height
      // with viewport and zoom, then draws at 1.55 floors tall and 1:2 wide.
      const gameplay = document.createElement("div");
      gameplay.style.cssText = [
        "width:48px",
        "height:96px",
        "background-image:url('/art/creature-atlas.png')",
        "background-size:384px 96px",
        `background-position:-${String(index * 48)}px 0`,
        "background-repeat:no-repeat",
        "filter:drop-shadow(0 3px 2px rgba(0,0,0,.45))",
      ].join(";");

      const label = document.createElement("div");
      label.style.cssText = "min-height:45px;text-align:center";
      label.textContent = `${cell.name}\n${cell.approach}`;
      label.style.whiteSpace = "pre-line";
      card.append(native, gameplay, label);
      gallery.append(card);
    }
    document.body.append(gallery);

    return {
      width: image.naturalWidth,
      height: image.naturalHeight,
      catalogIds: catalog.enemies.map((enemy) => enemy.id),
      cells,
    };
  }, expected);

  expect({ width: audit.width, height: audit.height }).toEqual({ width: 1024, height: 256 });
  expect(audit.catalogIds).toEqual(expect.arrayContaining(expected));
  expect(new Set(audit.catalogIds).size).toBe(audit.catalogIds.length);
  for (const cell of audit.cells) {
    expect(cell.name, `${cell.id} is missing from the live catalog`).not.toBe("missing");
    expect(cell.opaque, `${cell.id} has no visible pixels in its atlas cell`).toBeGreaterThan(80);
    expect(cell.bounds, `${cell.id} has no alpha bounds`).not.toBeNull();
    if (cell.bounds) {
      expect(cell.bounds.minX, `${cell.id} bleeds in from the previous atlas cell`).toBeGreaterThan(
        2,
      );
      expect(cell.bounds.maxX, `${cell.id} is clipped by the next atlas cell`).toBeLessThan(125);
      expect(cell.bounds.maxY, `${cell.id} has no clean baseline gutter`).toBeLessThanOrEqual(250);
    }
  }

  console.log("creature atlas audit:", JSON.stringify(audit.cells));
  await expect(page.getByTestId("creature-art-gallery")).toBeVisible();
  await page.screenshot({ path: "capture/creature-gallery.png" });
});

test("capture every creature in the live cross-section", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/?seed=4242");
  await page.waitForFunction(() => window.__understory?.stageEnemies !== undefined, null, {
    timeout: 20_000,
  });
  await page.evaluate(() => {
    const hooks = window.__understory!;
    hooks.send({ SetSpeed: { speed: "Paused" } });
    if (hooks.stageEnemies() !== 8) throw new Error("did not stage all eight creatures");
    document.querySelector<HTMLElement>(".chrome")?.style.setProperty("display", "none");
    document.querySelector<HTMLElement>(".stage-labels")?.style.setProperty("display", "none");
  });
  await page.waitForFunction(() => window.__understory!.view().siege.enemies.length === 8);
  await page.waitForTimeout(300);
  expect(await page.evaluate(() => window.__understory!.view().siege.lost)).toBe(false);
  await page.screenshot({ path: "capture/creatures-live-day.png" });
});
