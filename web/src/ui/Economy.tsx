/**
 * The economy, as a graph you can look at.
 *
 * **The question this answers is "where does this go?"** — and it is a
 * question the game has been unable to answer since M1. `SYSTEMS.md`
 * §6.27's review found a garden growing produce nothing could eat and
 * two weapons needing the deepest chain in the game to feed, and both
 * were invisible from inside the game: you found out by building the
 * thing and watching it sit there.
 *
 * Materials are the nodes, because materials are what a player is asking
 * about. Rooms are the *edges* — the transformation between one material
 * and the next — which is the honest shape of a production chain and
 * keeps the graph readable at the pack's twenty-four rooms.
 *
 * Everything here is derived from the catalog and the view. There is no
 * second model of the economy to drift out of step with the first, which
 * matters more than it sounds: a diagram that lies is worse than none,
 * and this one cannot, because it is drawn from the same numbers the
 * simulation runs on.
 */
import type { Game, UiState } from "../engine/Game";
import type { CatalogSnapshot, RoomInfo } from "../bridge/types";
import { ItemIcon } from "./ItemIcon";

/** One transformation: the room, what it eats, what it makes. */
interface Flow {
  room: RoomInfo;
  eats: number[];
  makes: number[];
}

/**
 * Every way the pack turns one material into another.
 *
 * Four kinds of consumer, and missing any of them makes a material look
 * like a dead end when it is not: a recipe, a burner's fuel, an
 * emplacement's ammunition, and the crew's dinner.
 */
function flows(catalog: CatalogSnapshot): Flow[] {
  return catalog.rooms.map((room) => {
    const eats = new Set<number>();
    const makes = new Set<number>();
    for (const input of room.inputs) eats.add(input.item);
    for (const output of room.outputs) makes.add(output.item);
    if (room.intake_item !== null) makes.add(room.intake_item);
    if (room.burner_fuel !== null) eats.add(room.burner_fuel);
    if (room.defence) eats.add(room.defence.ammo);
    return { room, eats: [...eats], makes: [...makes] };
  });
}

/**
 * Lay the materials out in columns, each one to the right of everything
 * it is made from.
 *
 * Intakes sit in column zero because nothing makes them — they come out
 * of the ground the tower is walking over. The pass is repeated rather
 * than recursive so a cycle in a content pack settles instead of
 * hanging: a chain that feeds itself simply stops moving right.
 */
function columns(catalog: CatalogSnapshot, all: Flow[]): number[][] {
  const depth = new Map<number, number>();
  for (let i = 0; i < catalog.items.length; i += 1) depth.set(i, 0);
  for (let pass = 0; pass < catalog.items.length; pass += 1) {
    let moved = false;
    for (const flow of all) {
      if (flow.eats.length === 0) continue;
      const from = Math.max(...flow.eats.map((item) => depth.get(item) ?? 0));
      for (const made of flow.makes) {
        if ((depth.get(made) ?? 0) < from + 1) {
          depth.set(made, from + 1);
          moved = true;
        }
      }
    }
    if (!moved) break;
  }

  // Only materials the pack actually moves: an item nothing makes and
  // nothing eats is a fact about the item list, not about the economy.
  const live = new Set<number>();
  for (const flow of all) {
    for (const item of [...flow.eats, ...flow.makes]) live.add(item);
  }

  const widest = Math.max(0, ...[...live].map((item) => depth.get(item) ?? 0));
  const out: number[][] = Array.from({ length: widest + 1 }, () => []);
  const ordered: number[] = [...live];
  ordered.sort((a, b) => a - b);
  for (const item of ordered) {
    out[depth.get(item) ?? 0]!.push(item);
  }
  return out;
}

export function Economy({ game, ui }: { game: Game; ui: UiState }) {
  const catalog = game.getCatalog();
  const all = flows(catalog);
  const lanes = columns(catalog, all);

  // What the tower is actually holding, and what it is actually running.
  const held = new Map<number, number>();
  for (const store of ui.stock) held.set(store.item, store.count);
  const standing = new Map<string, { stalled: boolean; why: string | null }>();
  for (const floor of ui.tower?.floors ?? []) {
    for (const room of floor.rooms) {
      const info = catalog.rooms[room.def];
      if (!info) continue;
      const was = standing.get(info.id);
      // A tower with three mills is running if any one of them is.
      standing.set(info.id, {
        stalled: (was?.stalled ?? true) && room.stalled,
        why: room.stall ?? was?.why ?? null,
      });
    }
  }

  return (
    <aside className="economy panel" data-testid="economy">
      <h2 className="section-title">The chain</h2>
      <p className="economy-note">
        Materials, left to right. A line is a room — solid if you have built one, faint if you have
        not.
      </p>
      <div className="economy-lanes">
        {lanes.map((lane, at) => (
          <div className="economy-lane" key={at}>
            {lane.map((item) => {
              const info = catalog.items[item];
              if (!info) return null;
              const count = held.get(item) ?? 0;
              // **Nothing eats this.** The finding §6.27 was built to
              // make visible: produce grew for a whole run with one
              // consumer three rooms deep, and no way to see it.
              const eaten = all.filter((f) => f.eats.includes(item));
              const dead = eaten.length === 0;
              const reachable = eaten.some((f) => standing.has(f.room.id));
              const explanation = dead
                ? `${info.name} — nothing in the pack consumes this`
                : reachable
                  ? `${info.name} — ${eaten
                      .filter((f) => standing.has(f.room.id))
                      .map((f) => f.room.name.toLowerCase())
                      .join(", ")} take it`
                  : `${info.name} — only ${eaten
                      .map((f) => f.room.name.toLowerCase())
                      .join(", ")} take it, and you have none`;
              return (
                <div
                  className={`economy-item${dead ? " dead" : reachable ? " flowing" : ""}`}
                  key={item}
                  data-testid={`economy-item-${info.id}`}
                  title={explanation}
                  aria-label={explanation}
                  tabIndex={0}
                >
                  <ItemIcon item={info} className="economy-glyph" decorative />
                  <span className="economy-name">{info.name}</span>
                  <span className="economy-count">{count}</span>
                </div>
              );
            })}
          </div>
        ))}
      </div>

      <h2 className="section-title economy-rooms-title">What moves it</h2>
      <ul className="economy-rooms">
        {all
          .filter((flow) => flow.eats.length > 0 || flow.makes.length > 0)
          .map((flow) => {
            const state = standing.get(flow.room.id);
            const name = (i: number) => catalog.items[i]?.name ?? "?";
            return (
              <li
                key={flow.room.id}
                className={`economy-flow${state ? (state.stalled ? " stalled" : " running") : " unbuilt"}`}
                data-testid={`economy-flow-${flow.room.id}`}
              >
                <span className="economy-flow-name">{flow.room.name}</span>
                <span className="economy-flow-arrow">
                  {flow.eats.map(name).join(" + ") || "the ground"}
                  {" → "}
                  {/*
                    **Not everything makes an item.** A burner turns
                    bamboo into charge and an emplacement turns ammo
                    into shots, and neither has an `outputs` entry —
                    they read "→ —" until you say so, which makes the
                    tower's two most important sinks look like dead
                    ends.
                  */}
                  {flow.makes.map(name).join(" + ") ||
                    (flow.room.burner ? "charge" : flow.room.defence ? "shots" : "—")}
                </span>
                {state?.stalled && state.why ? (
                  <span className="economy-flow-why">{state.why}</span>
                ) : null}
              </li>
            );
          })}
      </ul>
    </aside>
  );
}
