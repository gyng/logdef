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

  // **The surface falls behind the game silently, so check it here.**
  // M6 is entirely about the verbs a player has while a wave is landing
  // (§6) and the first tool pass shipped with none of them — an agent
  // could read a list of species and do nothing about it, and nothing
  // failed. This is the cheap version of "does the tool layer still
  // cover the game": name the verbs that would be missed.
  for (const verb of [
    "understory_focus_creature",
    "understory_set_charge_priority",
    "understory_reinforce",
    "understory_set_work_order",
    "understory_add_car",
    "understory_station_crew",
    "understory_take_fork",
    "understory_recruit",
  ]) {
    expect(names, `the tools have fallen behind the game: no ${verb}`).toContain(verb);
  }

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
    if (look.text.includes("UNANSWERED")) await call("understory_take_fork", { branch: 0 });
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

  // ---------------------------------------------------------- the rest of it
  //
  // **The chain is the easy half.** Everything after it is the part a
  // player spends the run on: a lift once the rope is flowing, forks
  // answered, people taken aboard, and creatures arriving whether or not
  // anybody is ready. Play to arrival and write down what the tools
  // could not see or could not do.
  let lift = false;
  let sieges = 0;
  let recruited = 0;
  let forks = 0;
  let arrived = false;
  let waypoints = 0;
  let answered = 0;
  let rewired = false;

  // **Bounded by the clock, not by turns.** A run is 31–36 minutes at
  // 1× (§6.19) and this plays at 4×, so arriving is nine minutes of
  // wall time — more than a spec should cost, and the findings are the
  // gaps rather than the ending. Play until the budget runs out and
  // write down how far it got.
  const until = Date.now() + 80_000;
  while (!arrived && Date.now() < until) {
    const look = await call("understory_look");
    if (look.text.includes("reached the Refugia") || look.text.includes("Arrived")) {
      arrived = true;
      break;
    }
    if (look.text.includes("UNANSWERED")) {
      if ((await call("understory_take_fork", { branch: 0 })).ok) forks += 1;
      continue;
    }
    if (look.text.includes("wants to come aboard")) {
      if ((await call("understory_recruit")).ok) recruited += 1;
      continue;
    }
    // **A wave, answered.** M6 is entirely about the verbs a player has
    // while one is landing, and the tool surface had none of them until
    // this run: the agent could read a list of species and do nothing.
    // Prefer whatever is already hurt and at the tower — the shortest
    // path to one fewer thing chewing.
    if (look.text.includes("Creatures (")) {
      sieges += 1;
      const here = [...look.text.matchAll(/^ {2}(\d+) · .+ · attack, at the tower · (\d+)%/gm)].map(
        (m) => ({ id: Number(m[1]), hp: Number(m[2]) }),
      );
      // The one nearest to leaving, which is the shortest path to one
      // fewer thing chewing on the tower.
      let weakest: { id: number; hp: number } | undefined;
      for (const one of here) if (!weakest || one.hp < weakest.hp) weakest = one;
      if (weakest && (await call("understory_focus_creature", { creature: weakest.id })).ok) {
        answered += 1;
      }
    }
    // A brown-out is a decision the player is being asked to make, and
    // the tools can make it: keep the works fed, let the lamps go.
    if (look.text.includes("BROWN-OUT") && !rewired) {
      log.push(`  brown-out: ${look.text.split("\n").slice(0, 2).join(" / ")}`);
      rewired = (
        await call("understory_set_charge_priority", {
          order: ["Works", "Lifts", "Lamps", "Legs"],
        })
      ).ok;
    }
    if (look.text.includes("waypoint is in reach") && !look.text.includes("cannot pay")) {
      if ((await call("understory_take_waypoint")).ok) waypoints += 1;
      continue;
    }

    // A lift, once the chain has made rope for one. The agent has to
    // pick a column with nothing to tell it which are free — the room
    // tool has `where_can_it_go` and the shaft tool has nothing.
    if (!lift) {
      const spots = await call("understory_where_can_it_go", { shaft: "shaft.elevator" });
      const spot = /slot (\d+)/.exec(spots.text);
      if (spot) {
        const top = /F(\d+) \[/.exec(look.text);
        const r = await call("understory_build_shaft", {
          shaft: "shaft.elevator",
          low: 0,
          high: Number(top?.[1] ?? 1),
          slot: Number(spot[1]),
        });
        if (r.ok) {
          lift = true;
          log.push(`  lift: ${look.text.split("\n")[0]}`);
          continue;
        }
        if (!r.text.includes("InsufficientStock")) {
          gaps.push(`shaft.elevator: ${spots.text.slice(0, 70)} but ${r.text}`);
        }
      }
    }
    await call("understory_wait", { seconds: 4 });
  }
  log.push(
    `journey: ${arrived ? "arrived" : "still walking"}, lift ${lift ? "yes" : "no"}, ` +
      `${String(forks)} forks, ${String(recruited)} recruits, ${String(waypoints)} waypoints, ` +
      `${String(sieges)} siege beats (${String(answered)} answered)${rewired ? ", rewired" : ""}`,
  );

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
