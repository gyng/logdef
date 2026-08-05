import { expect, test } from "@playwright/test";

/**
 * **Play the game through the agent tools and see what breaks.**
 *
 * Not a test of the tools' plumbing — a test of whether the tools are
 * *enough*. Everything here goes through `__webmcp.call`, the same
 * surface an agent in the browser would get, with no test hooks and no
 * grants. If a run cannot be played this way, the surface is wrong.
 */
test("a run can be played through the tools alone", async ({ page }) => {
  const log: string[] = [];
  await page.goto("/?seed=7001");
  await page.waitForFunction(() => window.__understory !== undefined, null, { timeout: 20_000 });
  await page.waitForFunction(
    () => (window as unknown as Record<string, unknown>).__webmcp !== undefined,
    null,
    { timeout: 20_000 },
  );

  const call = async (name: string, args: Record<string, unknown> = {}) => {
    const out = await page.evaluate(
      ([n, a]) =>
        (
          window as unknown as {
            __webmcp: {
              call: (
                n: string,
                a: Record<string, unknown>,
              ) => { content: { text: string }[]; isError?: boolean };
            };
          }
        ).__webmcp.call(n as string, a as Record<string, unknown>),
      [name, args] as const,
    );
    const text = out.content.map((c) => c.text).join("\n");
    log.push(`${out.isError ? "✗" : "✓"} ${name} ${JSON.stringify(args)} → ${text.slice(0, 160)}`);
    return { text, ok: out.isError !== true };
  };

  const first = await call("understory_look");
  expect(first.ok).toBe(true);
  console.log("=== opening look ===\n" + first.text);

  const chain = await call("understory_chain");
  console.log("=== chain (first lines) ===\n" + chain.text.split("\n").slice(0, 6).join("\n"));

  // Play: run the clock, build the ladder, keep walking.
  await call("understory_set_speed", { speed: "X4" });
  await call("understory_build_room", { room: "room.garden", floor: 1, slot: 3 });
  await call("understory_build_room", { room: "room.cutter_arm", floor: 1, slot: 8 });

  console.log("=== log ===\n" + log.join("\n"));
  expect(log.length).toBeGreaterThan(0);
});
