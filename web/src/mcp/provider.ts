/**
 * Understory, as tools an agent can use.
 *
 * **WebMCP**: a page declares what it can do, and an agent in the
 * browser calls those declarations instead of clicking pixels. The API
 * is `navigator.modelContext.registerTool`, and it is a moving spec —
 * so this registers through it when it exists, and always mirrors the
 * same tools onto `window.__webmcp` so anything else (a test, a
 * different agent shim, a console) can drive them identically.
 *
 * ## The tools are the player's verbs, not the engine's
 *
 * `window.__understory` already exposes `step`, `grant` and a raw
 * command channel, and none of that is here. Those are debug hooks: an
 * agent handed `grant` does not play the game, it edits the save, and
 * whatever it then tells you about the design is worthless. Every tool
 * below is something a player can do with a mouse, and the failures are
 * the same failures — `Locked`, `InsufficientStock`, `NotAtTheFront`.
 *
 * ## One `look`, not twenty getters
 *
 * The bridge's whole design is one `view()` a frame rather than an
 * accessor per subsystem (`DECISIONS.md` §3), and the same argument
 * applies harder to an agent: twenty small reads is twenty round trips
 * and a model that has to remember which it called. `look` answers
 * "what is going on" in one call, in the tower's own words.
 */
import type { Game } from "../engine/Game";
import type {
  CatalogSnapshot,
  GameCommand,
  PowerUse,
  RoomInfo,
  ShaftInfo,
  ShiftTag,
  ViewSnapshot,
} from "../bridge/types";
import { placementFits } from "../engine/scene";

/** What a tool hands back. Text, because the caller is a model. */
interface ToolResult {
  content: { type: "text"; text: string }[];
  isError?: boolean;
}

interface ToolDef {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
  /** May be async: `understory_wait` lets the tower run for a while. */
  execute: (args: Record<string, unknown>) => ToolResult | Promise<ToolResult>;
}

/** The shape of the proposal, as much of it as we use. */
interface ModelContext {
  registerTool?: (tool: ToolDef) => void;
  provideContext?: (ctx: { tools: ToolDef[] }) => void;
}

const say = (text: string): ToolResult => ({ content: [{ type: "text", text }] });
const fail = (text: string): ToolResult => ({
  content: [{ type: "text", text }],
  isError: true,
});

/**
 * Turn a command result into something a model can act on.
 *
 * **The rejection is the interesting half.** `engine::commands` refuses
 * with a typed error that says exactly what was wrong — "that column
 * belongs to the weapons", "the journal has not learnt this yet" — and
 * an agent that only hears "failed" cannot correct itself. Passing the
 * error through verbatim is the whole reason these tools are worth more
 * than clicking.
 */
function report(result: string | { Error: unknown }, ok: string): ToolResult {
  if (result === "Ok") return say(ok);
  const why = typeof result === "string" ? result : JSON.stringify(result.Error);
  return fail(`refused: ${why}`);
}

/** Everything a player can see, in one read. */
function look(view: ViewSnapshot, catalog: CatalogSnapshot): string {
  const item = (i: number) => catalog.items[i]?.name ?? "?";
  const lines: string[] = [];
  const shown = view.journey.enclave_at;
  const enclave = shown === null ? null : (catalog.regions[shown]?.enclave ?? null);

  lines.push(
    `Day ${String(view.clock.day + 1)}, ${catalog.dayparts[view.clock.daypart]?.name ?? "?"} · ` +
      `${String(Math.round(view.world.distance))} paces · ${view.journey.halt}`,
  );
  lines.push(
    `Charge ${String(view.power.charge)}/${String(view.power.capacity)}` +
      (view.power.brownout ? " — BROWN-OUT" : ""),
  );
  lines.push(
    `In ${catalog.regions[view.journey.region]?.name ?? "?"}, ` +
      `${String(Math.round(view.journey.region_permille / 10))}% through it · ` +
      `${String(Math.round(view.journey.remaining))} paces still to walk`,
  );

  lines.push("");
  lines.push("Tower:");
  const decks = [...view.tower.floors];
  decks.sort((a, b) => b.index - a.index);
  for (const floor of decks) {
    const rooms = floor.rooms
      .map((room) => {
        const info = catalog.rooms[room.def];
        const quiet = room.stall ? ` (${room.stall})` : "";
        return `${info?.name ?? "?"}@${String(room.slot)}${quiet}`;
      })
      .join(", ");
    lines.push(`  F${String(floor.index)} [${String(floor.slots)} slots] ${rooms || "empty"}`);
  }
  const shafts = view.tower.shafts
    .map(
      (s) =>
        `${s.kind}@${String(s.slot)} F${String(s.low)}-${String(s.high)} ${String(s.cars.length)} car(s)`,
    )
    .join(", ");
  lines.push(`  shafts: ${shafts || "stairs only"}`);

  lines.push("");
  lines.push("Crew:");
  for (const member of view.crew) {
    lines.push(
      `  ${member.name} — ${member.state}${member.stressed ? " (held up)" : ""}, ${member.shift} shift`,
    );
  }

  lines.push("");
  lines.push(
    `Shelves: ${view.stock.map((s) => `${item(s.item)} ${String(s.count)}`).join(", ") || "bare"}`,
  );

  // **Not the Heartseed.** It is unique and already standing, so
  // offering it is offering a rejection — the sidebar has filtered it
  // since M0 and this list did not, because `view.unlocked` is the
  // simulation's answer to "what is not locked" rather than to "what is
  // worth showing".
  // **Marked by whether the shelves can pay**, because "unlocked" and
  // "buildable right now" are different questions and an agent that
  // cannot tell them apart spends its turns being refused for money.
  const held = new Map(view.stock.map((s) => [s.item, s.count]));
  const buildable: string[] = [];
  for (const at of view.unlocked) {
    const room = catalog.rooms[at];
    if (!room || room.category === "Heart") continue;
    const paid = room.build_cost.every((c) => (held.get(c.item) ?? 0) >= c.amount);
    const cost = room.build_cost
      .map((c) => `${String(c.amount)} ${catalog.items[c.item]?.name ?? "?"}`)
      .join(" + ");
    buildable.push(`${room.id} (${cost})${paid ? "" : " — cannot pay"}`);
  }
  lines.push("");
  lines.push(`Can build: ${buildable.join("; ") || "nothing yet"}`);

  if (view.journey.fork) {
    // **An answered fork is not a question.** `world.fork` keeps its
    // answer until the tower crosses the split — the choice stays
    // changeable that whole time, which is the design (§3.3) — so a
    // reading of "is there a fork" is not a reading of "is anything
    // being asked". This said `A FORK is ahead ... the tower halts
    // until you choose` for the whole approach, and a dogfood agent
    // answered the same fork 398 times in 400 turns and never got on
    // with the run. The panel had this right all along.
    const fork = view.journey.fork;
    const ways = fork.branches
      .map((b, i) => `${String(i)}=${catalog.branches[b]?.name ?? "?"}`)
      .join(", ");
    lines.push("");
    if (fork.answer !== null) {
      lines.push(
        `The way is CHOSEN (${String(fork.answer)}), ${String(Math.round(fork.ahead))} paces ` +
          `off. Branches: ${ways}. Nothing to do — it stays changeable until the tower ` +
          "crosses, and then it is gone.",
      );
    } else {
      lines.push(
        `A FORK is ${String(Math.round(fork.ahead))} paces ahead and UNANSWERED. ` +
          `Branches: ${ways}. Choose before the tower reaches it, or it halts there.`,
      );
    }
  }
  if (view.journey.waypoint) {
    const way = catalog.waypoints[view.journey.waypoint.def];
    lines.push(
      `A waypoint is in reach: ${way?.name ?? "something"}` +
        (view.journey.waypoint.affordable ? "" : " (cannot pay for it)"),
    );
  }
  // **The settlements were invisible.** A run passes three of them and
  // they are where the shell work, the trades and the people are
  // (`SYSTEMS.md` §5); `look` reported none of it, so an agent walked
  // past every one. `enclave_ahead` counts down to it and goes null once
  // it is behind — there is no going back down the axis.
  const j = view.journey;
  if (j.at_enclave || j.enclave_ahead !== null) {
    const offers = j.offers
      .map((left, at) => {
        const offer = enclave?.offers[at];
        if (!offer || left === 0) return null;
        return (
          `${String(at)}: ${String(offer.give.amount)} ${item(offer.give.item)} for ` +
          `${String(offer.take.amount)} ${item(offer.take.item)} (${String(left)} left)`
        );
      })
      .filter((line): line is string => line !== null);
    lines.push("");
    if (j.at_enclave) {
      lines.push(
        `BERTHED at ${enclave?.name ?? "a settlement"}. While the tower is standing here:`,
      );
      lines.push(`  trades: ${offers.join("; ") || "none left"}`);
      lines.push(
        `  ${String(j.recruits)} people would come aboard` +
          (enclave && enclave.recruit_cost.length > 0
            ? ` (${enclave.recruit_cost.map((c) => `${String(c.amount)} ${item(c.item)}`).join(" + ")} each)`
            : ""),
      );
      lines.push(
        `  shell work ${String(j.shell_work)} left` +
          (enclave?.reinforce
            ? ` (${enclave.reinforce.cost.map((c) => `${String(c.amount)} ${item(c.item)}`).join(" + ")}, ` +
              `+${String(enclave.reinforce.panel_hp)} to every panel; ${String(j.shell_bonus)} added so far)`
            : ""),
      );
      lines.push("  None of it is available once the tower walks on.");
    } else {
      lines.push(
        `${enclave?.name ?? "A settlement"} is ${String(Math.round(j.enclave_ahead ?? 0))} paces ` +
          "ahead — trades, people and shell work, and only while the tower is standing at it.",
      );
    }
  }

  if (view.recruit) {
    const t = view.recruit.trait_at;
    lines.push(
      `Somebody wants to come aboard: ${view.recruit.name}` +
        (t !== null ? ` — ${catalog.traits[t]?.blurb ?? ""}` : ""),
    );
  }
  // **A wave is a situation, not a list of names.** This printed the
  // species and nothing else, which cannot tell an agent apart the two
  // things a player reads instantly off the cross-section: how far away
  // they still are, and whether anything is being chewed. M6 is
  // entirely about the verbs available while a wave lands (§6), and a
  // surface that cannot see a wave cannot use one.
  if (view.siege.enemies.length > 0) {
    lines.push("");
    lines.push(
      "Creatures (id · what · where · hurt):\n" +
        view.siege.enemies
          .map((e) => {
            const away = Math.round(Math.abs(e.at - view.world.distance));
            const where = e.state === "approach" ? `${String(away)} paces off` : "at the tower";
            const hurt = `${String(Math.round(e.hp_permille / 10))}% whole`;
            const mark = view.siege.focus === e.id ? " ← every emplacement prefers this one" : "";
            return (
              `  ${String(e.id)} · ${catalog.enemies[e.def]?.name ?? "?"} · ` +
              `${e.state}, ${where} · ${hurt}${mark}`
            );
          })
          .join("\n"),
    );
  }
  lines.push("");
  lines.push(
    `Tower ${String(Math.round(view.siege.integrity_permille / 10))}% whole` +
      (view.siege.repair_cost > 0
        ? `, ${String(view.siege.repair_cost)} poles of mending outstanding`
        : "") +
      ` · attention ${String(view.siege.provocation)}/${String(view.siege.provocation_max)}` +
      ` · charge goes to ${view.power.priority.join(" then ").toLowerCase()}`,
  );
  return lines.join("\n");
}

/**
 * Every legal placement for a room, right now.
 *
 * **The agent's version of the ghost highlights.** A player sees every
 * slot a room could take before committing (`drawPlaceMode`); an agent
 * had nothing, and a dogfood run spent about thirty refused calls per
 * room walking the floor by hand. Refusals are cheap for the simulation
 * and expensive for a model — they fill its context with
 * `SlotOccupied`.
 *
 * Shares `placementFits` with the renderer, so the answer cannot drift
 * from the highlight, and neither can drift from the command layer
 * without the smoke spec noticing.
 */
function legalSpots(
  view: ViewSnapshot,
  catalog: CatalogSnapshot,
  info: RoomInfo,
): { floor: number; slot: number }[] {
  const top = view.tower.floors.length - 1;
  const mode = {
    kind: "room" as const,
    id: info.id,
    width: info.width,
    frontOnly: info.front_only,
    maxFloor: info.max_floor,
    minFloor: info.min_floor,
    span: 1,
    hover: null,
  };
  const out: { floor: number; slot: number }[] = [];
  for (const floor of view.tower.floors) {
    // A garden grows nothing anywhere but the roof, and the command
    // layer says so — `placementFits` does not know about it because
    // the renderer greys those floors a different way.
    if (info.top_floor_only && floor.index !== top) continue;
    for (let slot = 0; slot + info.width <= floor.slots; slot += 1) {
      if (placementFits(view, mode, floor.index, slot, catalog.front_slots)) {
        out.push({ floor: floor.index, slot });
      }
    }
  }
  return out;
}

/**
 * Every span a shaft may legally rise through, right now.
 *
 * **The harder of the two questions, and it had no tool.** A room needs
 * one free footprint on one floor and the agent can see the floor; a
 * shaft needs the same column free on *every* floor it spans, which is
 * a fact about a vertical slice that no text dump of the tower makes
 * legible. An agent could only guess a column and be told `Occupied`.
 *
 * Reported as the tallest legal span per column rather than every
 * combination, because a shaft that does not reach the top is nearly
 * always a mistake — the second car and the freight runs both want the
 * whole height.
 */
function legalSpans(
  view: ViewSnapshot,
  info: ShaftInfo,
): { slot: number; low: number; high: number }[] {
  const floors = view.tower.floors.length;
  const ceiling = info.max_span === 0 ? floors : Math.min(info.max_span, floors);
  const out: { slot: number; low: number; high: number }[] = [];
  const widest = Math.max(...view.tower.floors.map((f) => f.slots));
  for (let slot = 0; slot < widest; slot += 1) {
    let best: { low: number; high: number } | null = null;
    for (let low = 0; low < floors; low += 1) {
      for (let span = ceiling; span >= info.min_span; span -= 1) {
        if (low + span > floors) continue;
        const mode = {
          kind: "shaft" as const,
          id: info.id,
          width: 1,
          frontOnly: false,
          maxFloor: null,
          minFloor: null,
          span,
          hover: null,
        };
        if (placementFits(view, mode, low, slot)) {
          if (!best || span > best.high - best.low + 1) best = { low, high: low + span - 1 };
          break;
        }
      }
    }
    if (best) out.push({ slot, low: best.low, high: best.high });
  }
  return out;
}

/**
 * Why a wait should stop early, if it should.
 *
 * **A wait can walk past a decision that cannot be taken back.** The
 * berth window is 160 paces wide and the tower covers 288 in four
 * seconds at 4×, so an agent that asked for four seconds could go from
 * "a settlement is 300 paces ahead" to "it is behind you" without ever
 * being offered the choice — and there is no going back down the axis.
 * A player watching the screen simply sees it coming.
 *
 * This is the agent's version of looking up. Everything here is either
 * irreversible (a settlement passed, a fork crossed, a waypoint missed)
 * or wants answering now (something chewing the tower, the bank empty,
 * the run over). Nothing here is advice — it is only the reason the
 * clock stopped, and the fresh `look` underneath says the rest.
 */
function interruption(view: ViewSnapshot, catalog: CatalogSnapshot): string | null {
  if (view.journey.arrived) return "the tower has arrived";
  if (view.siege.lost) return "the Heartseed is gone";
  if (view.journey.fork && view.journey.fork.answer === null && view.journey.fork.ahead < 400) {
    return "a fork is close and unanswered";
  }
  if (view.journey.at_enclave) {
    const at = view.journey.enclave_at;
    const enclave = at === null ? null : catalog.regions[at]?.enclave;
    return `the tower is berthed at ${enclave?.name ?? "a settlement"} — trades, people and shell work, and only while it is standing here`;
  }
  if (view.journey.enclave_ahead !== null && view.journey.enclave_ahead < 300) {
    return `a settlement is ${String(Math.round(view.journey.enclave_ahead))} paces ahead — stop the tower to berth, or it is lost for good`;
  }
  if (view.journey.waypoint) return "a waypoint is alongside";
  if (view.recruit) return `${view.recruit.name} wants to come aboard`;
  if (view.siege.enemies.some((e) => e.state === "attack")) {
    return "something is at the tower";
  }
  if (view.power.brownout) return "the tower is in a brown-out";
  return null;
}

/**
 * Register the game's tools with whatever agent surface is present.
 *
 * Returns a teardown, because the page can rebuild its `Game` (a hot
 * reload, a new seed) and a tool closing over a disposed engine would
 * be worse than no tool at all.
 */
export function registerGameTools(game: Game): () => void {
  const send = (cmd: GameCommand) => game.sendForTool(cmd);

  const tools: ToolDef[] = [
    {
      name: "understory_look",
      description:
        "Look at the walking tower: the day, where it is, its floors and rooms and who is " +
        "aboard, what is on the shelves, what can be built, and anything asking for a " +
        "decision (a fork, a waypoint, somebody wanting to join, creatures). Call this " +
        "first and after anything that might have changed the situation.",
      inputSchema: { type: "object", properties: {} },
      execute: () => say(look(game.viewForTool(), game.getCatalog())),
    },
    {
      name: "understory_wait",
      description:
        "Let the tower run for a few seconds and then say what changed. Use this whenever you " +
        "are waiting for something — poles to be milled, a room to fill, ground to be covered " +
        "— rather than asking again immediately. Nothing about the tower changes on your turn; " +
        "it changes because time passed. Capped at 30 seconds; raise the speed first if you " +
        "want more done per second. **It stops early** when something arrives that cannot wait " +
        "— a settlement coming into reach, an unanswered fork closing, a creature reaching the " +
        "tower — and says so, so a long wait is safe to ask for.",
      inputSchema: {
        type: "object",
        properties: {
          seconds: { type: "number", description: "Real seconds to let run, 1-30" },
        },
        required: ["seconds"],
      },
      execute: async (a) => {
        const want = Math.max(1, Math.min(30, Number(a.seconds) || 1));
        // **Checked often enough to catch a berth**, which is 160 paces
        // wide and about two seconds at 4×. A quarter-second poll costs
        // nothing and is the difference between being offered a
        // settlement and reading about one going past.
        const until = Date.now() + want * 1000;
        let stopped: string | null = null;
        const before = interruption(game.viewForTool(), game.getCatalog());
        while (Date.now() < until) {
          await new Promise((done) => setTimeout(done, 250));
          const now = interruption(game.viewForTool(), game.getCatalog());
          // Only a *new* reason stops the clock. Waiting during a
          // brown-out to see whether it clears is a reasonable thing to
          // ask for, and a wait that returned instantly every time
          // would be no wait at all.
          if (now !== null && now !== before) {
            stopped = now;
            break;
          }
        }
        const waited = Math.round((want * 1000 - Math.max(0, until - Date.now())) / 100) / 10;
        return say(
          `waited ${String(waited)}s${stopped === null ? "" : ` and stopped early: ${stopped}`}.

${look(game.viewForTool(), game.getCatalog())}`,
        );
      },
    },
    {
      name: "understory_chain",
      description:
        "The production chain: every material, what makes it and what takes it, and which of " +
        "those rooms this tower actually has. Use this to work out why something is not " +
        "arriving, or what a room you are thinking of building would need feeding with.",
      inputSchema: { type: "object", properties: {} },
      execute: () => {
        const catalog = game.getCatalog();
        const view = game.viewForTool();
        const standing = new Set(
          view.tower.floors.flatMap((f) => f.rooms.map((r) => catalog.rooms[r.def]?.id)),
        );
        const lines = catalog.rooms
          .map((room) => {
            const eats = [
              ...room.inputs.map((i) => catalog.items[i.item]?.name),
              ...(room.burner_fuel !== null ? [catalog.items[room.burner_fuel]?.name] : []),
              ...(room.defence ? [catalog.items[room.defence.ammo]?.name] : []),
            ].filter(Boolean);
            const makes = [
              ...room.outputs.map((o) => catalog.items[o.item]?.name),
              ...(room.intake_item !== null ? [catalog.items[room.intake_item]?.name] : []),
            ].filter(Boolean);
            if (eats.length === 0 && makes.length === 0) return null;
            const built = standing.has(room.id) ? "BUILT" : "not built";
            const out = makes.join(" + ") || (room.burner ? "charge" : "shots");
            return `  ${room.name}: ${eats.join(" + ") || "the ground"} -> ${out} [${built}]`;
          })
          .filter((l): l is string => l !== null);
        return say(`The chain:\n${lines.join("\n")}`);
      },
    },
    {
      name: "understory_where_can_it_go",
      description:
        "Every floor and slot a given room may legally stand on, right now. Ask this before " +
        "building: the rules are real and interlocking — weapons and the cutter arm must be " +
        "on the leading edge, ordinary rooms may not stand there at all, some rooms only " +
        "reach the ground, a garden needs the roof, and a shaft's column is taken on every " +
        "floor it spans. An empty answer means widen the hull or add a floor. Pass `shaft` " +
        "instead of `room` to ask the same about a lift, which is the harder question — a " +
        "shaft needs its column free on every floor it passes through.",
      inputSchema: {
        type: "object",
        properties: {
          room: { type: "string", description: 'Content id, e.g. "room.mill"' },
          shaft: {
            type: "string",
            description: 'Content id of a shaft instead, e.g. "shaft.elevator"',
          },
        },
      },
      execute: (a) => {
        const catalog = game.getCatalog();
        const view = game.viewForTool();
        if (a.shaft !== undefined) {
          const want = String(a.shaft);
          const shaft = catalog.shafts.find((s) => s.id === want);
          if (!shaft) return fail(`no such shaft: ${want}`);
          const spans = legalSpans(view, shaft);
          if (spans.length === 0) {
            return say(
              `${shaft.name} has no free column on this tower. Every column is taken on ` +
                "at least one floor it would have to pass through — widen the hull.",
            );
          }
          return say(
            `${shaft.name} can rise at: ` +
              spans
                .map(
                  (s) => `slot ${String(s.slot)} from floor ${String(s.low)} to ${String(s.high)}`,
                )
                .join(", "),
          );
        }
        const id = String(a.room);
        const info = catalog.rooms.find((room) => room.id === id);
        if (!info) return fail(`no such room or shaft: ${id}`);
        const spots = legalSpots(view, catalog, info);
        if (spots.length === 0) {
          return say(
            `${info.name} cannot stand anywhere on this tower. ` +
              (info.max_floor !== null
                ? "It reaches the ground, so a new floor will not help it — widen the hull."
                : "Widen the hull or add a floor."),
          );
        }
        return say(
          `${info.name} (${String(info.width)} wide) can go at: ` +
            spots.map((s) => `floor ${String(s.floor)} slot ${String(s.slot)}`).join(", "),
        );
      },
    },
    {
      name: "understory_build_room",
      description:
        "Put a room on a floor at a slot. Weapons and the cutter arm must stand on the " +
        "leading edge (the highest slots); ordinary rooms may not stand there at all. Some " +
        "rooms only reach the ground and cannot go above floor 1. Refusals say which rule " +
        "you hit.",
      inputSchema: {
        type: "object",
        properties: {
          room: { type: "string", description: 'Content id, e.g. "room.mill"' },
          floor: { type: "number" },
          slot: { type: "number" },
        },
        required: ["room", "floor", "slot"],
      },
      execute: (a) =>
        report(
          send({
            PlaceRoom: {
              room: String(a.room),
              floor: Number(a.floor),
              slot: Number(a.slot),
            },
          }),
          `built ${String(a.room)}`,
        ),
    },
    {
      name: "understory_build_floor",
      description:
        "Add a floor on top. Costs poles. Note that rooms which reach the ground can never " +
        "use it — for those, widen the hull instead.",
      inputSchema: { type: "object", properties: {} },
      execute: () => report(send("BuildFloor"), "a floor went up"),
    },
    {
      name: "understory_widen_hull",
      description:
        "Widen every floor by a couple of slots, at the back. This is the only way to give " +
        "the ground floors more room, and the rooms that reach the ground are stuck with " +
        "two floors however tall the tower gets.",
      inputSchema: { type: "object", properties: {} },
      execute: () => report(send("WidenTower"), "the hull is wider"),
    },
    {
      name: "understory_build_shaft",
      description:
        "Raise a shaft up a column. It carries crew, and fetches stock by itself whenever " +
        "nobody is riding. A shaft needs its column free on every floor it spans.",
      inputSchema: {
        type: "object",
        properties: {
          shaft: { type: "string", description: 'e.g. "shaft.elevator"' },
          low: { type: "number" },
          high: { type: "number" },
          slot: { type: "number" },
        },
        required: ["shaft", "low", "high", "slot"],
      },
      execute: (a) =>
        report(
          send({
            BuildShaft: {
              shaft: String(a.shaft),
              low: Number(a.low),
              high: Number(a.high),
              slot: Number(a.slot),
            },
          }),
          "the shaft is up",
        ),
    },
    {
      name: "understory_focus_creature",
      description:
        "Ask every emplacement to prefer one creature, by the id `look` gives it. This is " +
        "the whole of it — nobody is ordered to fire, and a gun with a better shot in front " +
        "of it still takes that shot. Pass no id to go back to no preference. Use it to " +
        "finish something already hurt, or to pull fire onto whatever is chewing a room.",
      inputSchema: {
        type: "object",
        properties: {
          creature: {
            type: "number",
            description: "Creature id from `look`. Omit to clear the preference.",
          },
        },
      },
      execute: (a) =>
        report(
          send({ FocusEnemy: { enemy: a.creature === undefined ? null : Number(a.creature) } }),
          a.creature === undefined
            ? "no creature is preferred now"
            : "every emplacement has been told to prefer it",
        ),
    },
    {
      name: "understory_set_charge_priority",
      description:
        "Say what the bank pays for first when there is not enough for everything. Lifts, " +
        "Works, Lamps and Legs in some arrangement — what comes first is served until the " +
        "charge runs out, and what comes last simply stops. This is the answer to a " +
        "brown-out: decide what the tower gives up, rather than letting it decide.",
      inputSchema: {
        type: "object",
        properties: {
          order: {
            type: "array",
            items: { type: "string" },
            description: 'All four of "Lifts", "Works", "Lamps", "Legs", most important first',
          },
        },
        required: ["order"],
      },
      execute: (a) => {
        const want = (a.order as unknown[]).map(String);
        const legal = ["Lifts", "Works", "Lamps", "Legs"];
        if (want.length !== 4 || !legal.every((use) => want.includes(use))) {
          return fail(`give all four of ${legal.join(", ")}, most important first`);
        }
        return report(
          send({ SetPowerPriority: { order: want as PowerUse[] } }),
          `charge now goes to ${want.join(" then ").toLowerCase()}`,
        );
      },
    },
    {
      name: "understory_reinforce",
      description:
        "Have the berthed settlement plate the hull: every panel, present and future, gains " +
        "hit points. Only while the tower is standing at a settlement, only a few times per " +
        "settlement for the whole run, and paid in stock. Worth it before a stretch you " +
        "expect to be chewed on — it cannot be bought in the middle of one.",
      inputSchema: { type: "object", properties: {} },
      execute: () => report(send("Reinforce"), "the crew are thickening the hull"),
    },
    {
      name: "understory_set_work_order",
      description:
        "Rank the kinds of work the crew take on. An idle person goes down this list and " +
        "takes the first job of a kind that has one waiting, so putting hauling last means " +
        "doors and mending come first and the shelves fill more slowly. Use it when a wave " +
        "is landing, and put it back afterwards.",
      inputSchema: {
        type: "object",
        properties: {
          order: {
            type: "array",
            items: { type: "string" },
            description: "Every job id, most important first",
          },
        },
        required: ["order"],
      },
      execute: (a) => {
        const catalog = game.getCatalog();
        const want = (a.order as unknown[]).map(String);
        const known = catalog.jobs.map((job) => job.id);
        if (want.length !== known.length || !known.every((job) => want.includes(job))) {
          return fail(`give all of ${known.join(", ")}, most important first`);
        }
        return report(
          send({ SetWorkOrder: { order: want } }),
          `crew now prefer ${want.join(", ")}`,
        );
      },
    },
    {
      name: "understory_add_car",
      description:
        "Put another car in a shaft that has room for one. Two cars on one shaft move nearly " +
        "twice the stock without costing another column, which is the cheap answer to a " +
        "queue at the doors.",
      inputSchema: {
        type: "object",
        properties: { shaft: { type: "number", description: "Shaft id from `look`" } },
        required: ["shaft"],
      },
      execute: (a) =>
        report(send({ AddCar: { shaft: Number(a.shaft) } }), "another car is running"),
    },
    {
      name: "understory_trade",
      description:
        "Take one of the berthed settlement's posted swaps, by the number `look` gives it. " +
        "Only while the tower is standing at the settlement, and each swap has a limited " +
        "number of takes for the whole run — walking on ends the chance for good.",
      inputSchema: {
        type: "object",
        properties: { offer: { type: "number", description: "Offer number from `look`" } },
        required: ["offer"],
      },
      execute: (a) => report(send({ Trade: { offer: Number(a.offer) } }), "traded"),
    },
    {
      name: "understory_set_room_active",
      description:
        "Switch a room off, or back on. A room that is off draws no charge and does no work — " +
        "the cheapest answer to a brown-out that does not cost anything permanent, and the " +
        "way to stop a chain filling a buffer you would rather keep for something else.",
      inputSchema: {
        type: "object",
        properties: {
          floor: { type: "number" },
          slot: { type: "number" },
          active: { type: "boolean", description: "false switches it off" },
        },
        required: ["floor", "slot", "active"],
      },
      execute: (a) =>
        report(
          send({
            SetRoomActive: {
              floor: Number(a.floor),
              slot: Number(a.slot),
              active: Boolean(a.active),
            },
          }),
          a.active ? "it is working again" : "it is off",
        ),
    },
    {
      name: "understory_set_shift",
      description:
        "Put somebody on the day or the night shift. Crew sleep through their off band, so a " +
        "tower with everybody on days does nothing for a third of the clock — and one with " +
        "everybody on nights harvests in the dark with the lamps on. Balance them.",
      inputSchema: {
        type: "object",
        properties: {
          crew: { type: "number", description: "Crew id from `look`" },
          shift: { type: "string", enum: ["Day", "Night"] },
        },
        required: ["crew", "shift"],
      },
      execute: (a) =>
        report(
          send({ SetShift: { crew: Number(a.crew), shift: String(a.shift) as ShiftTag } }),
          `they are on the ${String(a.shift).toLowerCase()} shift now`,
        ),
    },
    {
      name: "understory_set_speed",
      description:
        'How fast the simulation runs: "Paused", "X1", "X2" or "X4". Nothing happens while ' +
        "paused, so set a speed before expecting the tower to do anything.",
      inputSchema: {
        type: "object",
        properties: { speed: { type: "string", enum: ["Paused", "X1", "X2", "X4"] } },
        required: ["speed"],
      },
      execute: (a) =>
        report(
          send({ SetSpeed: { speed: String(a.speed) as "Paused" | "X1" | "X2" | "X4" } }),
          `speed ${String(a.speed)}`,
        ),
    },
    {
      name: "understory_take_fork",
      description:
        "Choose a branch at a fork. The tower stands still until you do, so answer one as " +
        "soon as `look` reports it.",
      inputSchema: {
        type: "object",
        properties: { branch: { type: "number", description: "Index from `look`" } },
        required: ["branch"],
      },
      execute: (a) =>
        report(send({ TakeFork: { branch: Number(a.branch) } }), "the route is chosen"),
    },
    {
      name: "understory_take_waypoint",
      description: "Take what the thing beside the route is offering, while it is in reach.",
      inputSchema: { type: "object", properties: {} },
      execute: () => report(send("TakeWaypoint"), "taken"),
    },
    {
      name: "understory_recruit",
      description:
        "Take on the person the settlement is offering. `look` says who they are and what is " +
        "true about them. Walking on spends the chance.",
      inputSchema: { type: "object", properties: {} },
      execute: () => report(send("Recruit"), "somebody came aboard"),
    },
    {
      name: "understory_set_striding",
      description:
        "Start or stop walking. A stopped tower harvests no ground but can strip a ruin it " +
        "is standing at; a walking one cannot be caught by most things.",
      inputSchema: {
        type: "object",
        properties: { walking: { type: "boolean" } },
        required: ["walking"],
      },
      execute: (a) =>
        report(
          send({ SetStriding: { walking: Boolean(a.walking) } }),
          a.walking === true ? "walking" : "stopped",
        ),
    },
    {
      name: "understory_station_crew",
      description:
        "Post somebody to a room, or call them back with room=null. A posted person works " +
        "that room and stops hauling — which is the whole trade.",
      inputSchema: {
        type: "object",
        properties: {
          crew: { type: "number", description: "Crew id from `look`" },
          room: { type: ["number", "null"], description: "Room id, or null to call them back" },
        },
        required: ["crew"],
      },
      execute: (a) =>
        report(
          send({
            StationCrew: {
              crew: Number(a.crew),
              room: a.room === null || a.room === undefined ? null : Number(a.room),
              until_tired: false,
            },
          }),
          "posted",
        ),
    },
  ];

  const nav = navigator as Navigator & { modelContext?: ModelContext };
  const ctx = nav.modelContext;
  if (ctx?.registerTool) {
    for (const tool of tools) ctx.registerTool(tool);
  } else if (ctx?.provideContext) {
    ctx.provideContext({ tools });
  }

  // **Always mirrored.** `navigator.modelContext` is a proposal and is
  // not in most browsers; without this, the tools would exist only where
  // the spec has already shipped and could not be exercised at all.
  const target = window as unknown as Record<string, unknown>;
  target.__webmcp = {
    tools: tools.map((t) => ({
      name: t.name,
      description: t.description,
      inputSchema: t.inputSchema,
    })),
    call: async (name: string, args: Record<string, unknown> = {}) => {
      const tool = tools.find((t) => t.name === name);
      if (!tool) return fail(`no such tool: ${name}`);
      return await tool.execute(args);
    },
  };

  return () => {
    delete target.__webmcp;
  };
}
