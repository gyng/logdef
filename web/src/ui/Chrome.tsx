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
import type { RoomInfo, ShaftInfo, SimSpeed } from "../bridge/types";

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
      {ui.lost && <Elegy ui={ui} />}
    </div>
  );
}

function TopBar({ game, ui }: Props) {
  const catalog = game.getCatalog();
  return (
    <header className="topbar panel">
      <span className="brand">Understory</span>
      <dl className="readouts">
        <Readout label="Day" value={`${ui.day + 1} · ${ui.daypart}`} />
        <Readout label="Distance" value={`${ui.distance} paces`} />
        <Readout label="Terrain" value={ui.terrain} />
        <Readout label="Yield" value={`${ui.yieldPct}%`} warn={ui.yieldPct < 100} />
        <Readout label="Sun" value={`${ui.exposurePct}%`} warn={ui.exposurePct < 30} />
        <Readout label="Floors" value={`${ui.floors} / ${catalog.max_floors}`} />
        <Readout label="Queued" value={String(ui.waiting)} warn={ui.waiting > 0} />
        <Readout
          label="Standing"
          value={`${Math.round(ui.integrity / 10)}%`}
          warn={ui.integrity < 1000}
        />
        {/* Both of these are silent until there is something to say.
            A permanent "0 poles owed" would be a dashboard number for
            a state the tower is in for most of a run. */}
        {ui.repairCost > 0 && <Readout label="To mend" value={`${ui.repairCost} poles`} warn />}
        {ui.repelled > 0 && <Readout label="Seen off" value={String(ui.repelled)} />}
      </dl>
      <Weather ui={ui} />
      <ChargeGauge ui={ui} />
      <button
        type="button"
        className={ui.walking ? "stride-toggle walking" : "stride-toggle"}
        aria-pressed={ui.walking}
        title="Halting the legs banks the charge they would burn (W)"
        data-testid="stride-toggle"
        onClick={() => game.setStriding(!ui.walking)}
      >
        {ui.walking ? "Striding" : "Halted"}
      </button>
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

/**
 * The charge gauge. A bar first and a number second: how full the banks
 * are is a glance question, and only the net flow needs digits.
 */
function ChargeGauge({ ui }: { ui: UiState }) {
  const fill = Math.max(0, Math.min(100, ui.chargeFill / 10));
  const net = ui.chargeIncome - ui.chargeSpend;
  const level = ui.brownout ? "empty" : fill < 25 ? "low" : "ok";
  return (
    <div className={`charge charge-${level}`} data-testid="charge">
      <div
        className="charge-bar"
        role="meter"
        aria-label="Charge"
        aria-valuenow={Math.round(fill)}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div className="charge-fill" style={{ width: `${fill}%` }} />
      </div>
      <span className="charge-figures">
        {ui.charge}/{ui.chargeCapacity}
        <span className={net < 0 ? "charge-net down" : "charge-net up"}>
          {net >= 0 ? "+" : ""}
          {net}
        </span>
      </span>
    </div>
  );
}

/**
 * How roused the forest is, keyed by percentage of the ceiling. Words
 * rather than a figure, because the question the player is actually
 * asking is "should I ease off", not "what is the number". Anything
 * past the last threshold falls through to "roused".
 */
const MOODS: [number, string][] = [
  [12, "still"],
  [34, "stirring"],
  [62, "restless"],
  [85, "watchful"],
];

/**
 * Provocation, read as weather.
 *
 * How much attention the tower has drawn is the only difficulty dial
 * in the game, and the player turns it by harvesting hard and burning
 * bamboo rather than from a menu — so it belongs on the bar next to
 * the charge, in the same shape. What it must never look like is a
 * threat meter: the creatures are defending their territory and the
 * tower is the thing passing through it (`DECISIONS.md` §8). Hence a
 * word for the mood of the forest and no number at all.
 */
function Weather({ ui }: { ui: UiState }) {
  const fill = Math.max(0, Math.min(100, (ui.provocation / Math.max(1, ui.provocationMax)) * 100));
  const mood = MOODS.find(([ceiling]) => fill < ceiling)?.[1] ?? "roused";
  const band = fill < 34 ? "calm" : fill < 85 ? "stirring" : "roused";
  return (
    <div className={`weather weather-${band}`} data-testid="weather">
      <div
        className="weather-bar"
        role="meter"
        aria-label="Attention"
        aria-valuenow={Math.round(fill)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuetext={mood}
      >
        <div className="weather-fill" style={{ width: `${fill}%` }} />
      </div>
      <span className="weather-word">the canopy is {mood}</span>
    </div>
  );
}

/**
 * The end of a run.
 *
 * Not a fail screen and not a scoreboard. The tower stopped somewhere,
 * and the only things worth saying about it are how long it stood and
 * how far it got.
 */
function Elegy({ ui }: { ui: UiState }) {
  return (
    <div className="elegy" role="status" data-testid="elegy">
      <h1>The Heartseed is gone</h1>
      <p>The tower stands where it stopped. The green will have it back before the season turns.</p>
      <dl className="elegy-facts">
        <div>
          <dt>Stood</dt>
          <dd>{ui.day + 1} days</dd>
        </div>
        <div>
          <dt>Walked</dt>
          <dd>{ui.distance} paces</dd>
        </div>
        <div>
          <dt>Seen off</dt>
          <dd>{ui.repelled}</dd>
        </div>
      </dl>
      <button type="button" className="elegy-again" onClick={walkAgain}>
        walk again
      </button>
    </div>
  );
}

/**
 * Start over. An explicit seed is dropped on the way out — otherwise
 * "walk again" would deal the same run and the same ending.
 */
function walkAgain(): void {
  const url = new URL(window.location.href);
  url.searchParams.delete("seed");
  window.location.replace(url.toString());
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

      <section>
        <h2 className="section-title">Transport</h2>
        <ul className="build-list">
          {catalog.shafts
            // The stairs are built in and cannot be added or removed.
            .filter((shaft) => shaft.kind !== "Stairs")
            .map((shaft) => (
              <li key={shaft.id}>
                <ShaftCard game={game} ui={ui} shaft={shaft} />
              </li>
            ))}
        </ul>
      </section>

      {ui.selected && (
        <section className="selection" data-testid="selection">
          <h3>{ui.selected.info.name}</h3>
          <p>{describeRoom(game, ui.selected.info)}</p>
          <div className="selection-actions">
            {(ui.selected.info.burner || ui.selected.info.power_draw > 0) && (
              <button
                type="button"
                className="toggle"
                aria-pressed={ui.selectedActive}
                data-testid="toggle-room"
                onClick={() => game.toggleSelectedRoom()}
              >
                {ui.selectedActive ? "Running" : "Shut down"}
              </button>
            )}
            <button
              type="button"
              className="danger"
              disabled={!ui.selected.removable}
              data-testid="remove-room"
              onClick={() => game.removeSelected()}
            >
              {ui.selected.removable ? "Tear down" : "Cannot be removed"}
            </button>
          </div>
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

function ShaftCard({ game, ui, shaft }: Props & { shaft: ShaftInfo }) {
  const affordable = game.canAffordShaft(shaft);
  const perFloor = (shaft.ticks_per_floor / 30).toFixed(1);
  const hint = affordable
    ? shaft.kind === "Dumbwaiter"
      ? `items only · ${shaft.min_span}–${shaft.max_span} floors · ${perFloor}s a floor`
      : `${shaft.capacity} aboard · ${perFloor}s a floor · ${shaft.charge_per_floor}⚡ a floor`
    : "not enough on the shelves";

  return (
    <button
      type="button"
      className="build-card"
      aria-pressed={ui.placing === shaft.id}
      disabled={!affordable}
      data-testid={`build-${shaft.id}`}
      onClick={() => game.beginPlacingShaft(ui.placing === shaft.id ? null : shaft.id)}
    >
      <span className="build-name">
        {shaft.name}
        <span className="build-hint">{hint}</span>
      </span>
      <Cost game={game} costs={shaft.build_cost} />
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
    case "Production": {
      const craft = `${(room.craft_ticks / 30).toFixed(0)}s a craft`;
      return room.power_draw > 0 ? `${craft} · ${room.power_draw}⚡ a tick` : craft;
    }
    case "Storage":
      return `${room.shelves} shelves`;
    case "Energy":
      if (room.solar) return "roof only · charge from sun";
      if (room.burner) return "burns bamboo for charge";
      if (room.bank_capacity > 0) return `holds ${room.bank_capacity}⚡`;
      return "";
    case "Defence":
      // The catalog exposes that a room shoots back but not its range
      // or its rate, and it should stay that way: where you put it and
      // whether the chain keeps it fed are the decisions, not the
      // numbers on the card.
      return room.power_draw > 0
        ? `shoots back · ${room.power_draw}⚡ a tick`
        : "shoots back · fed off the shelves";
    default:
      return "";
  }
}

function describeRoom(game: Game, info: RoomInfo): string {
  const catalog = game.getCatalog();
  const name = (index: number) => catalog.items[index]?.name ?? "something";

  if (info.solar) {
    return "Drinks sunlight, but only from the roof. Build a floor above it and it goes dark.";
  }
  if (info.burner) {
    return "Burns bamboo for charge. The dirty fallback — every stalk burned is a stalk not built with.";
  }
  if (info.bank_capacity > 0) {
    return `Holds ${info.bank_capacity} charge. Storage is something you build, not something you find.`;
  }
  if (info.defence) {
    // No ammo named: the catalog does not carry which item an
    // emplacement eats, and guessing would go stale the first time one
    // ships that does not eat darts.
    return "Answers whatever comes close, off an ordinary rack that ordinary crew have to keep filled. Run it dry and it goes quiet, for exactly the same reason a mill does.";
  }
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
  const walking = ui.walking;

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.target instanceof HTMLInputElement) return;
      if (event.metaKey || event.ctrlKey || event.altKey) return;
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
        case "w":
        case "W":
          game.setStriding(!walking);
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
  }, [game, speed, placing, walking]);
}
