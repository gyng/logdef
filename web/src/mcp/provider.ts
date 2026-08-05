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
import type { CatalogSnapshot, GameCommand, ViewSnapshot } from "../bridge/types";

/** What a tool hands back. Text, because the caller is a model. */
interface ToolResult {
  content: { type: "text"; text: string }[];
  isError?: boolean;
}

interface ToolDef {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
  execute: (args: Record<string, unknown>) => ToolResult;
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

  lines.push(
    `Day ${String(view.clock.day + 1)}, ${catalog.dayparts[view.clock.daypart]?.name ?? "?"} · ` +
      `${String(view.world.distance)} paces · ${view.journey.halt}`,
  );
  lines.push(
    `Charge ${String(view.power.charge)}/${String(view.power.capacity)}` +
      (view.power.brownout ? " — BROWN-OUT" : ""),
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
  const buildable: string[] = [];
  for (const at of view.unlocked) {
    const room = catalog.rooms[at];
    if (room && room.category !== "Heart") buildable.push(room.name);
  }
  lines.push(`Can build: ${buildable.join(", ") || "nothing yet"}`);

  if (view.journey.fork) {
    lines.push("");
    lines.push(
      `A FORK is ahead. Branches: ${view.journey.fork.branches
        .map((b, i) => `${String(i)}=${catalog.branches[b]?.name ?? "?"}`)
        .join(", ")}. The tower halts until you choose.`,
    );
  }
  if (view.journey.waypoint) {
    const way = catalog.waypoints[view.journey.waypoint.def];
    lines.push(
      `A waypoint is in reach: ${way?.name ?? "something"}` +
        (view.journey.waypoint.affordable ? "" : " (cannot pay for it)"),
    );
  }
  if (view.recruit) {
    const t = view.recruit.trait_at;
    lines.push(
      `Somebody wants to come aboard: ${view.recruit.name}` +
        (t !== null ? ` — ${catalog.traits[t]?.blurb ?? ""}` : ""),
    );
  }
  if (view.siege.enemies.length > 0) {
    lines.push("");
    lines.push(
      `Creatures: ${view.siege.enemies.map((e) => catalog.enemies[e.def]?.name ?? "?").join(", ")}`,
    );
  }
  return lines.join("\n");
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
    call: (name: string, args: Record<string, unknown> = {}) => {
      const tool = tools.find((t) => t.name === name);
      if (!tool) return fail(`no such tool: ${name}`);
      return tool.execute(args);
    },
  };

  return () => {
    delete target.__webmcp;
  };
}
