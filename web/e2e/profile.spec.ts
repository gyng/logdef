import { chromium, test } from "@playwright/test";

declare global {
  interface Window {
    /** Unparsed bridge output, for splitting a frame's cost. */
    __understoryRaw?: { view: () => string; frame: (elapsedUs: number) => string };
  }
}

/**
 * Not a test — the renderer's half of the profile.
 *
 * ```text
 * npx playwright test profile
 * ```
 *
 * `crates/core/examples/profile.rs` measures the sim and finds it costs
 * 1.8–2.6 µs a tick, which is two hundred times under budget. So if
 * anything is slow it is on this side, and nothing had ever looked.
 *
 * **The budget is 16.7 ms a frame at 60 fps**, shared with the sim: at
 * 4× the sim wants 8 ticks a frame, which even at a pessimistic 10×
 * WASM penalty is well under a millisecond. Everything else is drawing.
 *
 * Measured as a distribution rather than an average. A mean frame time
 * hides exactly the thing that matters — one 40 ms frame in sixty is
 * a visible hitch and moves the mean by half a millisecond.
 *
 * **Headless Chromium is not a graphics card**, and these numbers are
 * pessimistic by an unknown factor. They are useful for *comparing*
 * shapes — a big tower against a small one, a siege against quiet — and
 * for catching an order-of-magnitude problem. They are not a promise
 * about a player's machine.
 */
test("frame budget", async () => {
  // **Its own browser, with the GPU switched on.** Playwright's default
  // headless Chromium falls back to SwiftShader — a software rasteriser
  // — and profiling a renderer on one measures a CPU filling 1.44M
  // pixels, which is a fact about the harness rather than about the
  // game. Measured: default headless reports `SwiftShader Device`, and
  // these three flags report `NVIDIA GeForce RTX 3080, D3D11`.
  //
  // Launched here rather than in `playwright.config.ts` because every
  // other spec is about behaviour and does not care what drew the
  // pixels; only this one is lying without a GPU.
  const browser = await chromium.launch({
    args: ["--enable-gpu", "--ignore-gpu-blocklist", "--disable-software-rasterizer"],
  });
  const page = await browser.newPage();
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("http://localhost:3000/?seed=4242");
  await page.waitForFunction(() => window.__understory?.slotPoint !== undefined, null, {
    timeout: 20_000,
  });

  const shapes: { label: string; build: string[]; floors: number }[] = [
    { label: "starting tower", build: [], floors: 0 },
    {
      label: "big, busy tower",
      floors: 6,
      build: [
        "room.canteen",
        "room.bunk",
        "room.storeroom",
        "room.storeroom",
        "room.mill",
        "room.cutter_arm",
        "room.fiber_comb",
        "room.ropery",
        "room.thornwright",
        "room.dart_battery",
        "room.burner",
        "room.canopy_sails",
      ],
    },
  ];

  console.log("shape                    p50 ms   p95 ms   worst    quads  fps at p95");
  for (const shape of shapes) {
    const row = await page.evaluate(async (spec) => {
      const h = window.__understory!;
      // Materials, then rooms: this measures what a shape costs to draw,
      // not how long it takes to afford.
      const stock = () => {
        for (let i = 0; i < 30; i += 1) h.step(600);
      };
      stock();
      for (let i = 0; i < spec.floors; i += 1) {
        h.send("BuildFloor");
        stock();
      }
      for (const room of spec.build) {
        let done = false;
        for (let floor = 0; floor < 14 && !done; floor += 1) {
          for (let slot = 0; slot < 8; slot += 1) {
            if (h.send({ PlaceRoom: { room, floor, slot } }) === "Ok") {
              done = true;
              break;
            }
          }
        }
        stock();
      }
      h.send({ SetStriding: { walking: true } });

      // Let it settle, then sample real frames — `requestAnimationFrame`
      // deltas, so this is the whole frame including the sim step and
      // React, not just the WebGL call.
      await new Promise((done) => setTimeout(done, 500));
      const frames: number[] = [];
      await new Promise<void>((done) => {
        let last = performance.now();
        const tick = () => {
          const now = performance.now();
          frames.push(now - last);
          last = now;
          if (frames.length >= 240) {
            done();
            return;
          }
          requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
      });
      // **Where the frame goes.** The loop is two bridge calls and a
      // render, and `view()` is a `serde_json::to_string` in Rust, a
      // string across the WASM boundary, and a `JSON.parse` here — every
      // frame, for the whole snapshot. `DECISIONS.md` §3 minimised the
      // call *count*; nothing had ever measured what one call costs.
      const bridge = window.__understoryRaw;
      const split = { frame: 0, view: 0, parse: 0, bytes: 0 };
      if (bridge) {
        const N = 60;
        let t = performance.now();
        for (let i = 0; i < N; i += 1) bridge.frame(16_000);
        split.frame = (performance.now() - t) / N;
        t = performance.now();
        let raw = "";
        for (let i = 0; i < N; i += 1) raw = bridge.view();
        split.view = (performance.now() - t) / N;
        split.bytes = raw.length;
        t = performance.now();
        for (let i = 0; i < N; i += 1) JSON.parse(raw);
        split.parse = (performance.now() - t) / N;
      }

      frames.sort((a, b) => a - b);
      const at = (q: number) => frames[Math.min(frames.length - 1, Math.floor(frames.length * q))]!;
      return {
        p50: at(0.5),
        p95: at(0.95),
        worst: frames[frames.length - 1]!,
        // The quad count is not on the bridge — it belongs to the
        // renderer and reaches the screen through React. Read it off the
        // diagnostics line rather than plumbing a new hook for one
        // number a harness wants.
        quads: Number(/(\d+)\s+quads/.exec(document.body.textContent ?? "")?.[1] ?? -1),
        split,
      };
    }, shape);

    console.log(
      `  split: frame ${row.split.frame.toFixed(2)} ms, view ${row.split.view.toFixed(
        2,
      )} ms, JSON.parse ${row.split.parse.toFixed(2)} ms, snapshot ${row.split.bytes} chars`,
    );
    console.log(
      `${shape.label.padEnd(24)} ${row.p50.toFixed(1).padStart(6)} ${row.p95
        .toFixed(1)
        .padStart(8)} ${row.worst.toFixed(1).padStart(7)} ${String(row.quads).padStart(8)} ${(
        1000 / row.p95
      )
        .toFixed(0)
        .padStart(11)}`,
    );
  }

  console.log(
    "\n  16.7 ms is the 60 fps line. p95 is the number that matters: a mean hides the\n" +
      "  one frame in twenty that hitches, and a hitch is what a player feels.",
  );
});
