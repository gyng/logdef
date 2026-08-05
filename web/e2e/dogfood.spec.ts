import { expect, test } from "@playwright/test";

/**
 * **Play a whole run through the agent tools and write down what hurt.**
 *
 * Not a test of the tools' plumbing — a test of whether they are
 * *enough*. Everything goes through `__webmcp.call`, the surface an
 * agent in the browser gets: no test hooks, no grants, no reaching past
 * the buttons a player has. If a run cannot be played this way, the
 * surface is wrong, and the log at the end says where.
 */
test("a whole run can be played through the tools alone", async ({ page }) => {
  test.setTimeout(180_000);
  const log: string[] = [];
  const gaps: string[] = [];

  await page.goto("/?seed=7001");
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
    if (out.isError === true) log.push(`  ✗ ${name} ${JSON.stringify(args)} → ${text}`);
    return { text, ok: out.isError !== true };
  };

  // A tool an agent would want and does not have is a gap in the
  // surface, not a failure of the run — recorded, not thrown.
  const names = await page.evaluate(() =>
    (window as unknown as { __webmcp: { tools: { name: string }[] } }).__webmcp.tools.map(
      (t) => t.name,
    ),
  );
  log.push(`tools: ${names.join(", ")}`);

  await call("understory_set_speed", { speed: "X4" });

  // Play it: the ladder, then the chain, widening rather than growing
  // because the rooms that reach the ground never get another floor.
  const plan = [
    "room.garden",
    "room.cutter_arm",
    "room.burner",
    "room.mill",
    "room.fiber_comb",
    "room.storeroom",
    "room.ropery",
  ];
  const built: string[] = [];

  for (let beat = 0; beat < 220; beat += 1) {
    const look = await call("understory_look");

    // Anything asking for a decision, answered the way a player would.
    if (look.text.includes("A FORK")) await call("understory_take_fork", { branch: 0 });
    if (look.text.includes("wants to come aboard")) await call("understory_recruit");
    if (look.text.includes("waypoint is in reach") && !look.text.includes("cannot pay"))
      await call("understory_take_waypoint");
    if (look.text.includes("reached the Refugia") || look.text.includes("· arrived")) break;

    const next = plan.find((room) => !built.includes(room));
    if (!next) break;

    // **Ask where it may stand, then put it there.** The first version
    // of this walked every slot on every floor by hand and spent about
    // thirty refused calls per room — cheap for the simulation and
    // ruinous for a model's context, which is the whole argument for
    // the tool.
    const spots = await call("understory_where_can_it_go", { room: next });
    const spot = /floor (\d+) slot (\d+)/.exec(spots.text);
    if (spot) {
      const r = await call("understory_build_room", {
        room: next,
        floor: Number(spot[1]),
        slot: Number(spot[2]),
      });
      if (r.ok) {
        built.push(next);
        continue;
      }
      // A legal spot the command still refused is the two layers
      // disagreeing, and worth shouting about.
      if (r.text.includes("InsufficientStock")) {
        await call("understory_wait", { seconds: 3 });
      } else {
        gaps.push(`${next}: ${spots.text.slice(0, 60)} but ${r.text}`);
      }
      continue;
    }
    // Nowhere to stand. Width is the only thing that helps the rooms
    // that reach the ground.
    const w = await call("understory_widen_hull");
    if (!w.ok) {
      const f = await call("understory_build_floor");
      // **Wait rather than ask again.** Nothing changes on the agent's
      // turn; it changes because time passed. Without this the loop
      // spent its whole budget being told the same price it could not
      // meet — 5 of 7 rooms, and the log was one refusal repeated.
      if (!f.ok) await call("understory_wait", { seconds: 3 });
    }
  }

  log.push(`built ${String(built.length)}/${String(plan.length)}: ${built.join(", ")}`);
  const final = await call("understory_look");
  log.push("--- final ---");
  log.push(final.text);
  if (gaps.length > 0) log.push(`GAPS: ${gaps.join("; ")}`);
  console.log(log.join("\n"));
  // **The tool layer and the command layer must agree.** A `where` that
  // offers a slot the build then refuses is the two halves of the game
  // disagreeing, and it happened: `viewForTool` handed back the last
  // snapshot published to React, which is behind any command the agent
  // itself just sent. It reads fresh now, and this is what holds it.
  expect(gaps, "the placement tool offered a slot the command refused").toEqual([]);
  // And the surface has to be *enough*: a whole plan playable through
  // the tools alone, with no test hooks and no grants.
  expect(built.length, "the plan could not be built through the tools").toBe(plan.length);
});
