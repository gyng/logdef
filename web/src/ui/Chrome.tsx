/**
 * The UI around the game view: readouts, speed, the build menu, and
 * whatever the player has selected.
 *
 * Kept thin on purpose. The cross-section is the interface — fill
 * levels, colour, and a crew member tinting red are the primary read
 * (`DECISIONS.md` §8). Everything here is either an action the player
 * needs a button for, or a number precise enough that eyeballing a bar
 * would not do.
 */

import { useEffect } from "react";

import type { Game, UiState } from "../engine/Game";
import type { RoomInfo, SimSpeed } from "../bridge/types";

const SPEEDS: { value: SimSpeed; label: string; key: string }[] = [
  { value: "Paused", label: "❚❚", key: "Space" },
  { value: "X1", label: "1×", key: "1" },
  { value: "X2", label: "2×", key: "2" },
  { value: "X4", label: "4×", key: "4" },
];

interface Props {
  game: Game;
  ui: UiState;
}

export function Chrome({ game, ui }: Props) {
  useKeyboardShortcuts(game, ui);

  return (
    <div className="chrome">
      <TopBar game={game} ui={ui} />
      <Sidebar game={game} ui={ui} />
      <div className="diagnostics">
        <span>tick {ui.tick}</span>
        <span>{ui.fps} fps</span>
        <span>{ui.quads} quads</span>
      </div>
      {ui.lastError && (
        <div className="toast" role="status" data-testid="command-error">
          {ui.lastError}
        </div>
      )}
    </div>
  );
}

function TopBar({ game, ui }: Props) {
  const catalog = game.getCatalog();
  return (
    <header className="topbar panel">
      <span className="brand">Understory</span>
      <dl className="readouts">
        <Readout label="Distance" value={`${ui.distance} paces`} />
        <Readout label="Terrain" value={ui.terrain} />
        <Readout label="Yield" value={`${ui.yieldPct}%`} warn={ui.yieldPct < 100} />
        <Readout label="Floors" value={`${ui.floors} / ${catalog.max_floors}`} />
        <Readout label="Queued" value={String(ui.waiting)} warn={ui.waiting > 0} />
      </dl>
      <div className="speeds" role="group" aria-label="Simulation speed">
        {SPEEDS.map((speed) => (
          <button
            key={speed.value}
            type="button"
            aria-pressed={ui.speed === speed.value}
            title={`${speed.value} (${speed.key})`}
            data-testid={`speed-${speed.value}`}
            onClick={() => game.setSpeed(speed.value)}
          >
            {speed.label}
          </button>
        ))}
      </div>
    </header>
  );
}

function Readout({ label, value, warn }: { label: string; value: string; warn?: boolean }) {
  return (
    <div className="readout">
      <dt>{label}</dt>
      <dd className={warn ? "warn" : undefined}>{value}</dd>
    </div>
  );
}

function Sidebar({ game, ui }: Props) {
  const catalog = game.getCatalog();
  // The Heartseed is pre-placed and unique; offering it in the menu
  // would only ever produce a rejection.
  const buildable = catalog.rooms.filter((room) => room.category !== "Heart");

  return (
    <aside className="sidebar panel">
      <section>
        <h2 className="section-title">Stores</h2>
        <div className="stock" data-testid="stock">
          {ui.stock.length === 0 ? (
            <span className="stock-empty">the shelves are bare</span>
          ) : (
            ui.stock.map((entry) => {
              const item = catalog.items[entry.item];
              return (
                <span className="stock-item" key={entry.item} title={item?.name}>
                  <span className="glyph">{item?.glyph}</span>
                  {entry.count}
                </span>
              );
            })
          )}
        </div>
      </section>

      <section>
        <h2 className="section-title">Build</h2>
        <ul className="build-list">
          <li>
            <button
              type="button"
              className="build-card"
              disabled={!game.canAffordFloor() || ui.floors >= catalog.max_floors}
              data-testid="build-floor"
              onClick={() => game.send("BuildFloor")}
            >
              <span className="build-name">
                Add a floor
                <span className="build-hint">the stairs grow with it</span>
              </span>
              <Cost game={game} costs={catalog.floor_cost} />
            </button>
          </li>
          {buildable.map((room) => (
            <li key={room.id}>
              <RoomCard game={game} ui={ui} room={room} />
            </li>
          ))}
        </ul>
      </section>

      {ui.selected && (
        <section className="selection" data-testid="selection">
          <h3>{ui.selected.info.name}</h3>
          <p>{describeRoom(game, ui.selected.info)}</p>
          <button
            type="button"
            className="danger"
            disabled={!ui.selected.removable}
            data-testid="remove-room"
            onClick={() => game.removeSelected()}
          >
            {ui.selected.removable ? "Tear down" : "Cannot be removed"}
          </button>
        </section>
      )}
    </aside>
  );
}

function RoomCard({ game, ui, room }: Props & { room: RoomInfo }) {
  const affordable = game.canAfford(room);
  const fits = game.hasRoomFor(room);
  const placed = room.unique && !fits;
  const disabled = !affordable || !fits;

  let hint = costHint(room);
  if (!fits) hint = placed ? "already standing" : "no room for it";
  else if (!affordable) hint = "not enough on the shelves";

  return (
    <button
      type="button"
      className="build-card"
      aria-pressed={ui.placing === room.id}
      disabled={disabled}
      data-testid={`build-${room.id}`}
      onClick={() => game.beginPlacing(ui.placing === room.id ? null : room.id)}
    >
      <span className="build-name">
        {room.name}
        <span className="build-hint">{hint}</span>
      </span>
      <Cost game={game} costs={room.build_cost} />
    </button>
  );
}

function Cost({ game, costs }: { game: Game; costs: { item: number; amount: number }[] }) {
  const catalog = game.getCatalog();
  if (costs.length === 0) return <span className="build-cost">free</span>;
  return (
    <span className="build-cost">
      {costs.map((cost) => `${cost.amount}${catalog.items[cost.item]?.glyph ?? ""}`).join(" ")}
    </span>
  );
}

function costHint(room: RoomInfo): string {
  switch (room.category) {
    case "Intake":
      return room.max_floor === null
        ? "harvests as you walk"
        : `harvests as you walk · up to floor ${room.max_floor}`;
    case "Production":
      return `${(room.craft_ticks / 30).toFixed(0)}s a craft`;
    case "Storage":
      return `${room.shelves} shelves`;
    default:
      return "";
  }
}

function describeRoom(game: Game, info: RoomInfo): string {
  const catalog = game.getCatalog();
  const name = (index: number) => catalog.items[index]?.name ?? "something";

  if (info.intake_item !== null) {
    return `Strips ${name(info.intake_item).toLowerCase()} from the terrain the tower is walking through.`;
  }
  if (info.inputs.length > 0 && info.outputs.length > 0) {
    const from = info.inputs.map((io) => name(io.item).toLowerCase()).join(" and ");
    const to = info.outputs.map((io) => name(io.item).toLowerCase()).join(" and ");
    return `Turns ${from} into ${to}, ${(info.craft_ticks / 30).toFixed(0)} seconds at a time.`;
  }
  if (info.shelves > 0) {
    return `Holds ${info.shelves} shelves of anything the crew bring up. Construction spends straight off these.`;
  }
  return "The tower's living core.";
}

/**
 * Space to pause, 1/2/4 for speed, Escape to drop out of placement.
 * Bound on window so they work wherever the pointer is.
 */
function useKeyboardShortcuts(game: Game, ui: UiState): void {
  const speed = ui.speed;
  const placing = ui.placing;

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.target instanceof HTMLInputElement) return;
      switch (event.key) {
        case " ":
          event.preventDefault();
          game.setSpeed(speed === "Paused" ? "X1" : "Paused");
          break;
        case "1":
          game.setSpeed("X1");
          break;
        case "2":
          game.setSpeed("X2");
          break;
        case "4":
          game.setSpeed("X4");
          break;
        case "Escape":
          if (placing) game.beginPlacing(null);
          break;
        default:
          break;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [game, speed, placing]);
}
