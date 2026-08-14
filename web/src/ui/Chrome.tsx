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

import { useEffect, useRef, useState, type CSSProperties } from "react";

import { Economy } from "./Economy";
import { ItemIcon } from "./ItemIcon";
import type { Game, UiState } from "../engine/Game";
import type {
  CatalogSnapshot,
  CostInfo,
  CrewView,
  EnclaveInfo,
  HaltView,
  RoomInfo,
  PowerUse,
  ShaftInfo,
  ShaftView,
  SimSpeed,
  StoreView,
} from "../bridge/types";
import { crewArtCell } from "../engine/crewAtlas";
import { POWER_USES } from "../bridge/types";

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
  // **Closed by default**, like every panel that is not the game. The
  // chain graph answers "where does this go?", which is a question a
  // player asks between decisions rather than during one.
  const [chain, setChain] = useState(false);

  return (
    <div className="chrome">
      <TopBar game={game} ui={ui} />
      <Sidebar game={game} ui={ui} />
      <Roster game={game} ui={ui} />
      <button
        type="button"
        className={`chain-toggle${chain ? " on" : ""}`}
        data-testid="chain-toggle"
        aria-pressed={chain}
        title="Where every material comes from and what takes it"
        onClick={() => setChain((was) => !was)}
      >
        the chain
      </button>
      {chain && <Economy game={game} ui={ui} />}
      {ui.fork && <ForkCard game={game} ui={ui} />}
      {ui.landmarkAhead && !ui.waypoint && <LandmarkWarning game={game} ui={ui} />}
      {ui.waypoint && <WaypointCard game={game} ui={ui} />}
      {ui.atEnclave && <EnclaveBoard game={game} ui={ui} />}
      {/*
        **Not on by default.** tick/fps/quads sat permanently over the
        charge panel, which is fine for a session spent building the
        thing and wrong for the itch build — the first thing a stranger
        sees should not be a frame counter. `?debug` brings it back, and
        the capture harness passes it when a still needs to show one.
      */}
      {ui.debug && (
        <div className="diagnostics">
          <span>tick {ui.tick}</span>
          <span>{ui.fps} fps</span>
          <span>{ui.quads} quads</span>
        </div>
      )}
      {ui.lastError && (
        <div className="toast" role="status" data-testid="command-error">
          {ui.lastError}
        </div>
      )}
      {ui.lost && <Elegy ui={ui} />}
      {ui.arrived && !ui.lost && <Arrival game={game} ui={ui} />}
    </div>
  );
}

function LandmarkWarning({ game, ui }: Props) {
  const coming = ui.landmarkAhead;
  if (!coming) return null;
  const info = game.getCatalog().waypoints[coming.def];
  if (!info?.landmark) return null;
  const resident = info.resident === null ? null : game.getCatalog().enemies[info.resident];
  return (
    <aside className="landmark-warning" data-testid="landmark-warning" aria-live="polite">
      <span className="landmark-warning-lamp" aria-hidden="true" />
      <span>
        territory ahead · <b>{info.name}</b> · {Math.ceil(coming.ahead)} paces
        {resident ? ` · ${resident.name}` : ""}
      </span>
    </aside>
  );
}

/**
 * Who is aboard and what they are doing.
 *
 * **The shift toggle is gone** (`SYSTEMS.md` §6.32). Crew sleep when
 * they are tired and get up when they are rested, so there is nothing
 * here to set — what a player does about the night is build beds and
 * hire people, not assign a rota.
 *
 * **This is a schedule the player writes, not a readout of state**,
 * which is what makes it defensible under `DECISIONS.md` §8 — the same
 * category as the elevator's per-daypart programs, and a different
 * category from a dashboard. The rule it has to keep: this must never
 * become the primary place hunger and tiredness are read. Those belong
 * to the cross-section — a hungry crew member walks to the canteen, a
 * tired one moves visibly slower, a sleeping one is lying down — and if
 * the tower can only be understood through this list, the art pass
 * failed and no amount of polish here fixes it.
 *
 * So what a row shows is a *face*, a *name*, and what they are doing in
 * plain words. The two needs appear only as the state they produce
 * ("hungry", "asleep"), never as a number and never as a bar; the exact
 * tick counts are a hover title, which is the hover-only layer §8
 * allows.
 */
function Roster({ game, ui }: Props) {
  if (ui.crew.length === 0) return null;
  return (
    <aside className="roster panel" data-testid="roster">
      <h2 className="section-title">Aboard</h2>
      <ul className="roster-list">
        {ui.crew.map((member) => {
          return (
            <li className="roster-row" key={member.id} data-testid={`crew-${member.id}`}>
              <RosterPortrait member={member} />
              <span className="roster-who">
                <span className="roster-name">{member.name}</span>
                <span
                  className="roster-doing"
                  title={`fed ${Math.round(member.hunger / 30)}s ago, ${Math.round(
                    member.rested / 30,
                  )}s of work left`}
                >
                  {doing(member)}
                </span>
                {/*
                  **Who they are, under what they are doing.** One line,
                  in the tower's own words (`SYSTEMS.md` §6.25) — "sleeps
                  through daylight" rather than "deck_rest +100%". The
                  numbers are the pack's business; what a player needs is
                  a reason to remember this person.
                */}
                <Traits catalog={game.getCatalog()} member={member} />
              </span>
              <Practice catalog={game.getCatalog()} member={member} />
              <KitButton game={game} ui={ui} member={member} />
              <StationButton game={game} ui={ui} member={member} />
            </li>
          );
        })}
      </ul>
      <Schedules game={game} ui={ui} />
    </aside>
  );
}

/**
 * A dedicated 4x2 portrait atlas sits over the old full-body crop. Loading is
 * deliberately observed: development builds and old asset packs keep the
 * established crew-atlas crop instead of displaying an empty well.
 */
function RosterPortrait({ member }: { member: CrewView }) {
  const [portraitReady, setPortraitReady] = useState(false);
  const cell = crewArtCell(member.name, member.fidget);
  const column = cell % 4;
  const row = Math.floor(cell / 4);

  return (
    <span
      className="roster-face"
      aria-hidden="true"
      data-crew-cell={cell}
      style={
        {
          "--crew-fallback-x": `${(cell / 7) * 100}%`,
          "--crew-portrait-x": `${(column / 3) * 100}%`,
          "--crew-portrait-y": `${row * 100}%`,
        } as CSSProperties
      }
    >
      <span className={`roster-portrait${portraitReady ? " loaded" : ""}`} />
      <img
        className="roster-portrait-probe"
        src="/art/crew-portraits-atlas.png"
        alt=""
        onLoad={() => setPortraitReady(true)}
        onError={() => setPortraitReady(false)}
      />
    </span>
  );
}

/**
 * The per-daypart elevator programs, finally given somewhere to live.
 *
 * These have existed in the data model, the command layer and the replay
 * format since M1; §1.7 deferred the UI and said it "should land
 * alongside M4's shift rota if not before", and §2.9 carried that
 * forward unchanged. The rota's roster is the natural home because both
 * are **schedules written against the daypart clock** — the same
 * category of thing, edited the same way, and the reason a panel is
 * allowable here at all (`DECISIONS.md` §8).
 *
 * Only shafts with cars get one: stairs have no program to write,
 * because nothing dispatches them.
 *
 * The editor shows the *current* daypart and edits that one, rather
 * than offering a grid of every daypart against every floor. A player
 * setting a night program at midday cannot see what they are doing, and
 * the version of this that is a spreadsheet is the version that gets
 * built and then never opened.
 */
/**
 * What keeps running when the bank runs short.
 *
 * **Charge priority used to be the tick order and nothing else** — lifts
 * first because transport runs first, legs last because striding runs
 * last — so the most consequential scarcity in the game was resolved by
 * a constant. This hands the ranking over.
 *
 * Allowable under `DECISIONS.md` §8 for the same reason the shift rota
 * and the shaft programs are: **it is a schedule the player writes, not
 * a readout of state.** The diegetic half already exists and stays the
 * primary read — the lamps go out, the legs stutter, the roof rack
 * empties. This says what you want cut *first*, and the tower still
 * shows you what happened.
 *
 * Reordered by moving one entry at a time rather than by dragging: the
 * simulation wants all four exactly once, and a drag that can drop
 * outside the list has to invent a rule for what that means.
 */
/**
 * How practised somebody is, as pips rather than as numbers.
 *
 * **Hover-only precision** (`DECISIONS.md` §8): what the card shows is
 * that this person has been doing something for a while, and the title
 * says which job and how far. There is no tick count anywhere, because
 * the rank *is* the fact — `Crew::rank` is an integer division and there
 * is nothing finer underneath for a player to chase.
 *
 * Only the best job is drawn. A four-column grid of everybody's skill at
 * everything is a spreadsheet, and the whole argument for letting
 * practice exist at all (`DESIGN.md` structural call 4) is that it stays
 * a fact about a person rather than a build to optimise.
 */
function Practice({ catalog, member }: { catalog: CatalogSnapshot; member: CrewView }) {
  let best = 0;
  for (let at = 1; at < member.ranks.length; at += 1) {
    if ((member.ranks[at] ?? 0) > (member.ranks[best] ?? 0)) best = at;
  }
  const rank = member.ranks[best] ?? 0;
  if (rank === 0) return null;
  const job = catalog.jobs[best]?.name ?? "the work";
  return (
    <span
      className="roster-practice"
      data-testid={`practice-${member.id}`}
      title={`${member.name} has done a lot of ${job} — ${rank} of ${catalog.max_rank}`}
      aria-label={`practised at ${job}`}
    >
      {"•".repeat(rank)}
    </span>
  );
}

/**
 * Put another car in this shaft.
 *
 * Reads how many it has from the view and how many it may have from the
 * catalog, so a pack that says a shaft cannot grow simply renders
 * nothing rather than a disabled button nobody can act on.
 */
function AddCarButton({
  game,
  shaft,
  info,
}: {
  game: Game;
  shaft: ShaftView;
  info: ShaftInfo | undefined;
}) {
  if (!info || info.max_cars <= shaft.cars.length) return null;
  const affordable = game.canAffordCar(info);
  return (
    <button
      type="button"
      className="schedule-car"
      disabled={!affordable}
      data-testid={`add-car-${shaft.id}`}
      aria-label={`Add a car to ${info.name}`}
      title={
        affordable
          ? `Put another car in. ${shaft.cars.length} of ${info.max_cars} — a car is a turn, not speed, so this buys queue rather than pace.`
          : "not enough on the shelves"
      }
      onClick={() => game.addCar(shaft.id)}
    >
      +car
    </button>
  );
}

/**
 * What is true about somebody, in words rather than percentages.
 *
 * Renders nothing for a person with no traits, so a pack without any is
 * a roster that looks exactly as it did before.
 */
function Traits({ catalog, member }: { catalog: CatalogSnapshot; member: CrewView }) {
  const mine = member.traits.map((at) => catalog.traits[at]).filter((entry) => entry !== undefined);
  if (mine.length === 0) return null;
  return (
    <span className="roster-traits" data-testid={`traits-${member.id}`}>
      {mine.map((entry) => (
        <span key={entry.id} className="roster-trait" title={entry.blurb}>
          {entry.name}
        </span>
      ))}
    </span>
  );
}

function PowerOrder({ game, ui }: Props) {
  const move = (from: number, by: number) => {
    const next = [...ui.powerPriority];
    const to = from + by;
    if (to < 0 || to >= next.length) return;
    [next[from], next[to]] = [next[to]!, next[from]!];
    game.setPowerPriority(next);
  };
  // **What the bank gauge cannot say.** Capacity is how long the tower
  // can keep going; generation is how hard it can spend right now, and a
  // full bank behind thin generation still runs everything slowly
  // (`SYSTEMS.md` §6.39).
  const asked = ui.powerDemand.reduce((sum, want) => sum + want, 0);
  const strained = ui.powerSatisfaction.some((served) => served < 1000);
  return (
    <>
      <h2 className="section-title schedule-title">Charge</h2>
      <p
        className={`power-rail${strained ? " power-rail-strained" : ""}`}
        data-testid="power-rail"
        title="What the burners, cell banks and Heartseed can deliver each tick, against what the tower is asking for. The bank says how long; this says how hard."
      >
        supply {ui.powerRail}/tick
        <span className="power-rail-slack">
          {` · asking ${asked}`}
          {strained ? " — running short" : ""}
        </span>
      </p>
      <ol className="power-order" data-testid="power-order">
        {ui.powerPriority.map((use, at) => {
          const circuit = POWER_USES.indexOf(use);
          const refused = ui.powerRefused[circuit] ?? false;
          const demand = ui.powerDemand[circuit] ?? 0;
          const served = ui.powerSatisfaction[circuit] ?? 1000;
          return (
            <li
              key={use}
              className={`power-row${refused ? " circuit-shed" : ""}`}
              title={
                refused
                  ? `${POWER_WORDS[use]} asked for ${demand} and got ${Math.round((served * demand) / 1000)}`
                  : undefined
              }
            >
              <span className="power-name">{POWER_WORDS[use]}</span>
              <span className="circuit-lamp" aria-label={refused ? "load shed" : "circuit ready"} />
              {/* **How well this circuit is being served, as a rate.**
                  A circuit is not on or off any more — it runs at a
                  fraction, and the fraction is what a player needs to
                  see to know whether to reorder or to go and build
                  another burner (`SYSTEMS.md` §6.39). Full service says
                  nothing at all, because a panel that shows "100%" on
                  five rows all day is noise. */}
              <span
                className={`power-served${served < 1000 ? " power-served-short" : ""}`}
                data-testid={`power-served-${use}`}
              >
                {served < 1000 ? `${Math.round(served / 10)}%` : ""}
              </span>
              <span className="power-moves">
                <button
                  type="button"
                  disabled={at === 0}
                  title={`Keep ${POWER_WORDS[use]} running before the one above`}
                  aria-label={`Move ${POWER_WORDS[use]} earlier`}
                  data-testid={`power-up-${use}`}
                  onClick={() => move(at, -1)}
                >
                  ▲
                </button>
                <button
                  type="button"
                  disabled={at === ui.powerPriority.length - 1}
                  title={`Let ${POWER_WORDS[use]} be cut before the one below`}
                  aria-label={`Move ${POWER_WORDS[use]} later`}
                  data-testid={`power-down-${use}`}
                  onClick={() => move(at, 1)}
                >
                  ▼
                </button>
              </span>
            </li>
          );
        })}
      </ol>
    </>
  );
}

/**
 * The tower's own words for its five draws. "Lifts" rather than
 * "transport", "works" rather than "production" — this is a place people
 * live, and the panel should sound like somebody who lives there.
 */
const POWER_WORDS: Record<PowerUse, string> = {
  Lifts: "the lifts",
  Works: "the works",
  Guns: "the guns",
  Lamps: "the lamps",
  Legs: "the legs",
};

/**
 * Post this person to the room the player has selected, or call them
 * back off wherever they are.
 *
 * **Reads the selection rather than offering a picker.** Clicking a room
 * is already how the player says which one they mean, and a second
 * chooser inside the roster would be a list of rooms competing with the
 * cross-section that *is* the list of rooms.
 *
 * The button says what it will do, not what is true — "to the mill",
 * "off the mill" — because a toggle whose label describes state leaves
 * the player working out the verb.
 */
/**
 * What this person is carrying, and what the tower could lend them.
 *
 * **The equip system, and it is deliberately about a person.** The
 * tower's half of "equip" already exists — what you feed a dart battery
 * *is* the choice — so a kit belongs to somebody named. `DESIGN.md` §2
 * structural call 4 says crew are individuals rather than stat blocks,
 * and this is the smallest control that makes that mechanical.
 *
 * A cycle rather than a menu: there are three kits in the whole game,
 * the tower rarely owns more than one, and a dropdown for a list that
 * short is a click to open something the button could have said.
 * Clicking moves to the next kit on the shelves and then back to
 * nothing, so handing one in is always one click away.
 */
function KitButton({ game, ui, member }: Props & { member: UiState["crew"][number] }) {
  const catalog = game.getCatalog();
  const held = member.kit === null ? null : catalog.items[member.kit];
  // What could be lent right now: kits on the shelves, plus whatever
  // this person already has, so the cycle can always return to it.
  const offerable = ui.stock
    .map((entry) => catalog.items[entry.item])
    .filter((info): info is NonNullable<typeof info> => Boolean(info?.kit));
  if (offerable.length === 0 && !held) return null;

  const ring = [null, ...offerable.map((info) => info.id)];
  const at = ring.indexOf(held?.id ?? null);
  const next = ring[(at + 1) % ring.length] ?? null;

  return (
    <button
      type="button"
      className={`kit-toggle${held ? " on" : ""}`}
      data-testid={`kit-${member.id}`}
      aria-label={held ? `Change ${member.name}'s kit` : `Give ${member.name} a kit`}
      title={
        held
          ? `${member.name} is carrying the ${held.name}. Click to hand it in.`
          : `Lend ${member.name} a kit from the shelves.`
      }
      onClick={() => game.equipCrew(member.id, next)}
    >
      {held ? <ItemIcon item={held} decorative /> : "·"}
    </button>
  );
}

function StationButton({ game, ui, member }: Props & { member: UiState["crew"][number] }) {
  const posted = member.stationed;
  const here = ui.selected;

  // Already posted: the button calls them back, wherever they are.
  //
  // It does not name the room. The cross-section already says where
  // somebody is — they are standing in it — and a name here would be a
  // second, textual copy of a fact the picture carries better
  // (`DECISIONS.md` §8).
  if (posted !== null) {
    return (
      <button
        type="button"
        className="station-toggle on"
        data-testid={`station-${member.id}`}
        aria-label={`Return ${member.name} to hauling`}
        title={`${member.name} is working a room rather than hauling. Send them back to the stairs.`}
        onClick={() => game.stationCrew(member.id, null)}
      >
        ⏻
      </button>
    );
  }

  // Nothing selected: nothing to post them to, so the control is not
  // offered at all rather than offered and disabled — a dead button is a
  // question the player has to answer before they can ignore it.
  if (!here) return null;
  return (
    <button
      type="button"
      className="station-toggle"
      data-testid={`station-${member.id}`}
      aria-label={`Post ${member.name} to ${here.info.name}`}
      title={`Put ${member.name} to work in the ${here.info.name}. They stop hauling while they are there.`}
      onClick={() => game.stationCrew(member.id, here.id)}
    >
      ＋
    </button>
  );
}

/**
 * Physical shaft changes, without an always-present timetable.
 *
 * Per-daypart stop masks remain replay-compatible in the simulation,
 * but no instrument has shown them creating a worthwhile decision.
 * Placement, extension and another car already answer the visible
 * questions: where the column goes, how high it reaches, and whether a
 * queue needs another turn.
 */
function Schedules({ game, ui }: Props) {
  const catalog = game.getCatalog();
  const dispatched = ui.shafts.filter((shaft) => shaft.kind !== "Stairs");
  if (dispatched.length === 0) return null;

  return (
    <>
      <h2 className="section-title schedule-title">Shafts</h2>
      <ul className="schedule-list">
        {dispatched.map((shaft) => {
          const info = catalog.shafts[shaft.def];
          return (
            <li className="schedule-row" key={shaft.id} data-testid={`schedule-${shaft.id}`}>
              <span className="schedule-name">{info?.name ?? "shaft"}</span>
              {/*
                **Another car, not another column** (`SYSTEMS.md`
                §6.20). A shaft costs a slot on every floor it passes
                through, for ever; a car costs none. So the answer to a
                queue late on lives here, on the shaft that already
                exists, rather than in the build menu.
              */}
              {shaft.kind === "Elevator" && <AddCarButton game={game} shaft={shaft} info={info} />}
              {shaft.high < ui.floors - 1 && (
                <button
                  type="button"
                  className="schedule-car"
                  data-testid={`extend-shaft-${shaft.id}`}
                  disabled={
                    !info || !game.canAffordShaftExtension(info, ui.floors - 1 - shaft.high)
                  }
                  onClick={() => game.extendShaft(shaft.id, ui.floors - 1)}
                >
                  extend to roof
                  {info && (
                    <Cost
                      game={game}
                      costs={game.shaftExtensionCost(info, ui.floors - 1 - shaft.high)}
                    />
                  )}
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </>
  );
}

/**
 * What somebody is doing, in words a person would use.
 *
 * Hunger and tiredness appear here as states rather than as numbers,
 * and only once they are *visible in the tower anyway* — "hungry" means
 * they are on their way to eat, which you can watch them do.
 */
function doing(member: UiState["crew"][number]): string {
  switch (member.state) {
    case "sleep":
      return "asleep";
    case "eat":
      return "eating";
    case "mend":
      return "mending";
    case "man":
      return "at their station";
    // Not "chasing it off" and not "defending": nobody fights, they are
    // simply in the room, which is the whole of what a person does about
    // a thief (`DECISIONS.md` §8).
    case "shoo":
      return "seeing something out";
    case "board":
      return member.stressed ? "held up at the stairs" : "waiting for a way up";
    case "climb":
      return "on the stairs";
    case "ride":
      return "riding up";
    case "load":
      return "picking up";
    case "unload":
      return "setting down";
    case "walk":
      return member.carrying ? "carrying" : "on their way";
    default:
      // **"idle" is the roster's most common word and its least useful**,
      // and it hid the same distinction `StallTag` exists for
      // (`SYSTEMS.md` §6.26): somebody with nothing to do and somebody
      // holding a crate the tower has no room for read identically, and
      // they are answered by opposite actions.
      //
      // A stranded carrier is the one worth naming. `haul.rs` computes
      // exactly this state — hands full, no inbox, no shelf, no chute —
      // and lets them sleep and eat *because* of it; until now the only
      // outward sign was somebody standing still.
      return member.carrying ? "holding something, nowhere to put it" : "nothing to do";
  }
}

function TopBar({ game, ui }: Props) {
  const catalog = game.getCatalog();
  return (
    <header className="topbar panel">
      <span className="brand">Understory</span>
      <Journey ui={ui} />
      <dl className="readouts">
        {/*
          **Four readouts left, from nine.** `DECISIONS.md` §8 says to
          look for the diegetic version before adding a number to the
          screen, and the bar had stopped doing that:

          - *Terrain* is the ground the tower is walking over.
          - *Sun* is the sky, and the lamps coming on at dusk.
          - *Floors* is countable — the tower is right there.
          - *Queued* was the worst of them. §8 names the crew tinting
            red as the bottleneck instrument and says in as many words
            that it "does not get a second, numeric representation";
            this was that second representation.

          All four are on the Day readout's hover instead, which is
          where §8 puts precision. What stays is the day (time is
          invisible), the yield (the one fact the scene cannot show),
          and under siege the things that need answering.
        */}
        <Readout
          label="Day"
          value={`${ui.day + 1} · ${ui.daypart}`}
          hint={`${ui.terrain} · sun ${ui.exposurePct}% · ${ui.floors} of ${catalog.max_floors} floors${
            ui.waiting > 0 ? ` · ${ui.waiting} queued at a shaft` : ""
          }`}
        />
        <Readout label="Yield" value={`${ui.yieldPct}%`} warn={ui.yieldPct < 100} />
        {/* All three of these are silent until there is something to
            say. A permanent "0 poles owed" would be a dashboard number
            for a state the tower is in for most of a run — and by that
            same rule, so was a permanent "Standing 100%". An undamaged
            tower is the ordinary case, and the interesting one is
            already drawn: `drawSplits` cracks the panelling and
            `drawPanel` opens a hole in the skin. */}
        {/* Rounded down, not to nearest: this only appears once
            something is broken, and 998 per-mille rounding up to a
            warn-coloured "100%" is a readout arguing with itself. */}
        {ui.integrity < 1000 && (
          <Readout label="Standing" value={`${Math.floor(ui.integrity / 10)}%`} warn />
        )}
        {ui.repairCost > 0 && <Readout label="To mend" value={`${ui.repairCost} poles`} warn />}
        {ui.repelled > 0 && <Readout label="Seen off" value={String(ui.repelled)} />}
      </dl>
      <Weather ui={ui} />
      <ChargeGauge ui={ui} />
      <SoundToggle game={game} />
      <button
        type="button"
        className={`stride-toggle stride-${ui.halt}`}
        aria-pressed={ui.walking}
        aria-label={ui.walking ? "Halt the tower" : "Set the tower striding"}
        title="Halting the legs banks the charge they would burn (W)"
        data-testid="stride-toggle"
        onClick={() => game.setStriding(!ui.walking)}
      >
        {HALT_WORDS[ui.halt]}
      </button>
      <div className="speeds" role="group" aria-label="Simulation speed">
        {SPEEDS.map((speed) => (
          <button
            key={speed.value}
            type="button"
            aria-pressed={ui.speed === speed.value}
            aria-label={`Simulation speed ${speed.value}`}
            title={`${speed.value} (${speed.key})`}
            data-testid={`speed-${speed.value}`}
            onClick={() => game.setSpeed(speed.value)}
          >
            {speed.label}
          </button>
        ))}
      </div>
      {/*
        **The weapon bar, FTL-shaped.** Everything the tower can point
        at something, with what is on its rack — because a weapon that
        is dry is the single most important thing on the screen during a
        wave and it was previously visible only by finding the room in
        the cross-section and reading its inbox.

        It is a *readout of rooms*, not a second place to build them:
        the placement is still the decision (`SYSTEMS.md` §6.13), and
        this says what that decision bought. Empty until the tower has a
        weapon, which on turn one it does — the thorn gun it sets out
        with.
      */}
      <Weapons game={game} ui={ui} />
      {/*
        Zoom. Discoverable rather than only on the wheel: a control
        nobody knows about is a control nobody has, and the tower now
        starts at two floors where the fitted view leaves it very small
        (`SYSTEMS.md` §6.11).
      */}
      <div className="speeds" role="group" aria-label="Zoom">
        <button
          type="button"
          title="Zoom out (-)"
          aria-label="Zoom out"
          data-testid="zoom-out"
          onClick={() => game.zoomBy(1 / 1.15)}
        >
          −
        </button>
        <button
          type="button"
          title="Fit the tower to the frame (0)"
          aria-label="Fit tower to frame"
          data-testid="zoom-reset"
          onClick={() => game.resetZoom()}
        >
          {`${Math.round(ui.zoom * 100)}%`}
        </button>
        <button
          type="button"
          title="Zoom in (+)"
          aria-label="Zoom in"
          data-testid="zoom-in"
          onClick={() => game.zoomBy(1.15)}
        >
          +
        </button>
      </div>
    </header>
  );
}

/**
 * Why the tower is standing still, in one word on the control that
 * stopped it.
 *
 * Four states share one silhouette and the cross-section carries the
 * difference (`scene.ts` draws each of them differently), but the
 * toggle used to say "Halted" for all four — including the two the
 * player did not choose. A brown-out is not the same answer to "why
 * aren't we moving" as a fork is, and this is the control they would
 * reach for to find out.
 */
const HALT_WORDS: Record<HaltView, string> = {
  walking: "Striding",
  stopped: "Halted",
  berthed: "Berthed",
  brownout: "No charge",
  fork: "At the fork",
  arrived: "Arrived",
};

/**
 * Where the run has got to.
 *
 * The palette says which region the tower is in — the drowned city does
 * not look like the deep jungle and is not supposed to need a caption.
 * What the strip genuinely cannot say is *how far through* it you are,
 * because there is no horizon feature for "two thirds of the way", so
 * that is the one thing here that earns chrome: an unlabelled line, no
 * percentage, next to the name of the place and the paces walked.
 */
/**
 * The one piece of chrome the audio pass needs, and the reason it needs
 * one: **audio cannot start without a gesture.**
 *
 * Browser autoplay policy keeps an `AudioContext` suspended until the
 * player interacts, so something on screen has to be the interaction.
 * Making that thing the mute button rather than a modal "click to
 * enable sound" gate means the game is playable from the first frame
 * and the sound arrives when it is asked for — and it is why the
 * Playwright smoke test never hears anything, which is correct rather
 * than a failure.
 */
function SoundToggle({ game }: { game: Game }) {
  const [on, setOn] = useState(false);
  return (
    <button
      type="button"
      className={`sound-toggle${on ? " on" : ""}`}
      data-testid="sound-toggle"
      aria-pressed={on}
      aria-label={on ? "Mute the tower" : "Listen to the tower"}
      title={on ? "Mute" : "Listen to the tower"}
      onClick={() => {
        const next = !on;
        setOn(next);
        game.setAudioEnabled(next);
      }}
    >
      <span className="sound-lamp" aria-hidden="true" />
      <span className="sound-word" aria-hidden="true">
        aud
      </span>
    </button>
  );
}

function Journey({ ui }: { ui: UiState }) {
  const through = Math.max(0, Math.min(100, ui.regionPermille / 10));
  return (
    <div className="journey" data-testid="journey">
      <span className="journey-place">
        {ui.region}
        {ui.branch && <span className="journey-branch"> · {ui.branch}</span>}
      </span>
      <div
        className="journey-bar"
        role="meter"
        aria-label="Through this region"
        aria-valuenow={Math.round(through)}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div className="journey-fill" style={{ width: `${through}%` }} />
      </div>
      <span className="journey-paces">{ui.distance} paces</span>
    </div>
  );
}

/**
 * A word for how loud a branch is, never a number.
 *
 * `threat_pct` multiplies the region's own, so 100 is "the same as
 * around here" rather than an absolute. Three words is the whole
 * vocabulary: the choice is which way to go, not which multiplier to
 * prefer (`SYSTEMS.md` §3.3).
 */
function threatWord(pct: number): string {
  if (pct < 100) return "quieter";
  if (pct > 100) return "louder";
  return "as usual";
}

/**
 * The beat the tower is passing.
 *
 * **A moment, not a menu** (`SYSTEMS.md` §6.14). It appears when the
 * thing comes alongside, offers one button, and goes away when the
 * tower has walked past — ignoring it is free and is the default, so
 * there is deliberately no dismiss control and no way to bring it back.
 *
 * Drawn beside the fork card rather than as a modal, because a modal
 * would stop the world and the whole point is that the tower keeps
 * walking while you decide.
 */
function WaypointCard({ game, ui }: Props) {
  const here = ui.waypoint;
  if (!here) return null;
  const catalog = game.getCatalog();
  const info = catalog.waypoints[here.def];
  if (!info) return null;
  const resident = info.resident === null ? null : catalog.enemies[info.resident];

  const ground =
    info.paces === 0
      ? null
      : info.paces > 0
        ? `${info.paces} paces gained`
        : `${-info.paces} paces lost`;
  const attention =
    info.provocation === 0
      ? null
      : `canopy: ${attentionMood(ui.provocation, ui.provocationMax)} → ${attentionMood(
          Math.max(0, Math.min(ui.provocationMax, ui.provocation + info.provocation)),
          ui.provocationMax,
        )}`;
  const offered = [
    ...info.gives.map((give) => `${give.amount} ${catalog.items[give.item]?.name ?? "?"}`),
    ...(ground ? [ground] : []),
    ...(attention ? [attention] : []),
  ];

  return (
    <section className="waypoint panel" data-testid="waypoint" aria-label={info.name}>
      <header className="waypoint-head">
        <h2>{info.name}</h2>
        <p>{info.said}</p>
      </header>
      <button
        type="button"
        className="waypoint-take"
        disabled={!here.affordable}
        data-testid="waypoint-take"
        onClick={() => game.takeWaypoint()}
      >
        <span className="waypoint-verb">{info.take}</span>
        <Cost game={game} costs={info.costs} />
      </button>
      <p className="waypoint-terms">{offered.join(" · ") || "nothing but the time"}</p>
      {resident && (
        <div className="waypoint-postures" aria-label="Ways through the nesting ground">
          <p>
            <b>Go around</b> — leave it undisturbed and give up the shortcut.
          </p>
          <p>
            <b>Break away</b> — cut through, keep the legs moving, and outlast its grip.
          </p>
          <p>
            <b>Hold ground</b> — cut through, halt, and spend ammunition, charge, and repairs for
            {resident.drops.length > 0
              ? ` ${resident.drops
                  .map(
                    (drop) =>
                      `${String(drop.amount)} ${catalog.items[drop.item]?.name ?? "material"}`,
                  )
                  .join(" + ")}.`
              : " what it carries."}
          </p>
        </div>
      )}
    </section>
  );
}

/**
 * The fork.
 *
 * Not a modal and not a pause — the tower keeps walking toward it while
 * this is up, and a player who answers early never stops at all. It
 * appears the moment the split does, roughly fifty seconds out at 1×,
 * and stays answerable until the tower crosses.
 *
 * There is deliberately no authored description of either way. The card
 * is built out of the branch's own palette and threat multiplier, so it
 * cannot drift out of step with what the branch actually does during
 * tuning — a game that misdescribes the only informed choice it asks
 * for is worse than one that describes it drily (`SYSTEMS.md` §3.3).
 */
function ForkCard({ game, ui }: Props) {
  const fork = ui.fork;
  if (!fork) return null;
  const catalog = game.getCatalog();
  const waiting = ui.halt === "fork";
  // Against `stream_ahead_paces`: the fork exists from the moment the
  // generator can see it, and this fills as the tower closes on it.
  const closing = Math.max(0, Math.min(100, (1 - fork.ahead / 900) * 100));

  return (
    <section
      className={waiting ? "fork panel fork-waiting" : "fork panel"}
      data-testid="fork"
      aria-label="The way splits"
    >
      <header className="fork-head">
        <h2>The way splits</h2>
        <p>
          {waiting
            ? "the tower is standing at it, waiting to be told"
            : fork.answer !== null
              ? "the way is chosen, and stays changeable until the tower crosses"
              : "say which way before you reach it and you never stop"}
        </p>
        <div className="fork-closing" aria-hidden="true">
          <div className="fork-closing-fill" style={{ width: `${closing}%` }} />
        </div>
      </header>
      <div className="fork-ways">
        {fork.branches.map((index, side) => {
          const info = catalog.branches[index];
          if (!info) return null;
          const ground = info.terrain
            .slice(0, 2)
            .map((terrain) => catalog.terrain[terrain]?.name ?? "?")
            .join(" and ");
          return (
            <button
              key={info.id}
              type="button"
              className="fork-way"
              aria-pressed={fork.answer === side}
              data-testid={`fork-${String(side)}`}
              onClick={() => game.takeFork(side)}
            >
              <span className="fork-name">{info.name}</span>
              <span className="fork-ground">{ground}</span>
              <span className={`fork-threat threat-${threatWord(info.threat_pct)}`}>
                {threatWord(info.threat_pct)}
              </span>
              <span className="fork-pressure">{approachForecast(info.approaches)}</span>
            </button>
          );
        })}
      </div>
    </section>
  );
}

function approachForecast(weights: [number, number, number]): string {
  const labels = ["ground", "canopy", "burrow"] as const;
  const total = weights.reduce((sum, weight) => sum + weight, 0);
  const highest = Math.max(...weights);
  const leaders = weights.filter((weight) => weight === highest).length;
  const index = weights.indexOf(highest);
  if (total <= 0 || highest * 2 < total || leaders !== 1) return "mixed approaches";
  return `${labels[index] ?? "mixed"} pressure`;
}

/**
 * The enclave's posted board.
 *
 * The berth itself is diegetic — the tower stops next to a place and
 * walking on ends it — but the transaction is not, and `SYSTEMS.md`
 * §3.5 says so outright: taking an offer is a button on a board and the
 * goods appear on the shelves. The genuinely diegetic version needs a
 * haul destination outside the tower, and that was reasoned about and
 * cut for M3. So this is the smallest honest thing.
 *
 * Every offer names its own terms, in the goods themselves rather than
 * in a price: four scrap for three poles, and how many times more they
 * will do it. `journey.offers` carries what is *left*, which is state;
 * the terms come from the catalog, which is content.
 */
function EnclaveBoard({ game, ui }: Props) {
  const catalog = game.catalogInfo();
  return (
    <aside className="enclave panel" data-testid="enclave" aria-label="The posted board">
      <h2 className="enclave-name">{ui.enclave?.name ?? "A settlement"}</h2>
      <p className="enclave-note">people live here; the tower is passing through</p>
      <ul className="enclave-offers">
        {ui.offers.map((left, index) => (
          <li key={index}>
            <button
              type="button"
              className="enclave-offer"
              disabled={left <= 0}
              data-testid={`trade-${String(index)}`}
              onClick={() => game.trade(index)}
            >
              <span className="offer-terms">
                <Terms catalog={catalog} enclave={ui.enclave} index={index} />
              </span>
              <span className="offer-left">{left > 0 ? `${left} to be had` : "spoken for"}</span>
            </button>
          </li>
        ))}
      </ul>
      {ui.enclave?.reinforce && (
        <button
          type="button"
          className="enclave-reinforce"
          disabled={ui.shellWork <= 0}
          data-testid="reinforce"
          onClick={() => game.reinforce()}
        >
          {ui.shellWork > 0 ? (
            <>
              Have them plate the hull ·{" "}
              <CostItems catalog={catalog} costs={ui.enclave.reinforce.cost} />
            </>
          ) : (
            "The hull is as plated as they will make it"
          )}
          <span className="enclave-note">
            {ui.shellWork > 0
              ? `+${ui.enclave.reinforce.panel_hp} to every panel, and to every floor built after`
              : `+${ui.shellBonus} already on every panel`}
          </span>
        </button>
      )}
      {/*
        **Somebody in particular, before you pay** (`SYSTEMS.md` §6.29).
        This read "Ask someone to come aboard" and handed over the next
        name off a list — forty traits existed and you never chose
        between them, because you never saw one first. The offer is
        drawn when the tower berths and held until it walks on, so
        passing costs you the visit rather than nothing.
      */}
      <div className="enclave-candidates" aria-label="People willing to come aboard">
        {ui.recruits > 0 && ui.recruit.length > 0 ? (
          ui.recruit.map((candidate, index) => (
            <button
              key={`${candidate.name}-${String(index)}`}
              type="button"
              className="enclave-recruit"
              data-testid={`recruit-${String(index)}`}
              onClick={() => game.recruit(index)}
            >
              <span className="recruit-who">{candidate.name}</span>
              <span className="recruit-trait">
                {candidate.trait_at !== null
                  ? (catalog?.traits[candidate.trait_at]?.blurb ?? "wants to come aboard")
                  : "wants to come aboard"}
              </span>
              <span className="recruit-cost">
                <CostItems catalog={catalog} costs={ui.enclave?.recruit_cost ?? []} />
              </span>
            </button>
          ))
        ) : (
          <span className="enclave-empty">Nobody else is coming</span>
        )}
      </div>
    </aside>
  );
}

/**
 * The other end of a run.
 *
 * The Elegy's sibling and deliberately the same register: it reports
 * where the tower got to and does not grade it. Region 2's far edge is
 * a placeholder finish line — M5 adds the third region and the actual
 * Refugia — so there is nothing here to congratulate anybody for, and
 * pretending otherwise would be the wrong tone twice over
 * (`SYSTEMS.md` §3.7, `DECISIONS.md` §8).
 */
/**
 * The Refugia, reached.
 *
 * **A description, not a score** (`DECISIONS.md` §8). No rating, no rank,
 * no stars, and nothing that could be read as a mark out of ten. What
 * arriving shows is what the tower has, who is aboard by name, the route
 * it walked, and the seed — so the run can be handed to somebody else,
 * which is the whole of what `v2-plan.md` §6.6 promises about seeds.
 *
 * And what the journal learnt, if anything, phrased as a thing the crew
 * did rather than a thing the player earned.
 */
function Arrival({ game, ui }: Props) {
  return (
    <div className="elegy arrival" role="status" data-testid="arrival">
      <h1>The Refugia</h1>
      <p>
        The ground runs out here, and this is where it was going. The tower stands with its legs
        still, and the people who walked it here are already talking about what to plant.
      </p>
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
          <dt>Aboard</dt>
          <dd>{ui.crew.map((member) => member.name).join(", ") || "nobody"}</dd>
        </div>
      </dl>
      <Learned ui={ui} />
      <Seed ui={ui} />
      <button type="button" className="elegy-again" onClick={walkAgain}>
        walk again
      </button>
      {/* Read rather than shown: the whole journal is a lot, and the
          arrival is not the place for a table. */}
      <RunLogs game={game} />
    </div>
  );
}

/**
 * What this run taught, if anything.
 *
 * Deeds rather than achievements: "ran a forge" is a thing you did, and
 * a list of things you did is a diary. The moment this reads as a
 * checklist with ticks on it, it has become the thing `v2-plan.md` §3
 * rules out.
 */
function Learned({ ui }: { ui: UiState }) {
  if (ui.learned.length === 0) return null;
  return (
    <div className="learned" data-testid="learned">
      <h2 className="section-title">The journal gains</h2>
      <ul>
        {ui.learned.map((deed) => (
          <li key={deed.said}>
            {deed.said}
            {deed.opens.length > 0 && (
              <span className="learned-opens"> — and how to build with what it left.</span>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

/**
 * The run log, and a way to hand it over.
 *
 * **`SYSTEMS.md` §5.8 asks for "a file the player can read and hand
 * over", and until now it was neither.** Every run has been recorded to
 * the journal since M5 and there was no way to get one out — the elegy
 * said how many runs were written down and stopped there. That is the
 * whole gap between somebody playing and `BALANCE.md`'s constants being
 * graded from run logs rather than from scripted harnesses, which is
 * what that criterion asks for in as many words.
 *
 * A copy button rather than a table, for two reasons. The arrival is not
 * the place for a dashboard (`DECISIONS.md` §8 — no scores, no grades,
 * nothing that reads as a mark out of ten), and the reader this is *for*
 * is somebody doing a difficulty pass with a text file open, not a
 * player admiring their statistics. So: one line saying how many runs
 * exist, and a way to take them away.
 */
function RunLogs({ game }: { game: Game }) {
  const [copied, setCopied] = useState(false);
  const runs = game.journalNow().runs;
  const hand = () => {
    void navigator.clipboard
      .writeText(JSON.stringify(runs, null, 2))
      .then(() => setCopied(true))
      .catch(() => setCopied(false));
  };
  return (
    <p className="elegy-note">
      {runs.length} runs written down.{" "}
      {runs.length > 0 && (
        <button type="button" className="elegy-copy" onClick={hand} data-testid="copy-runs">
          {copied ? "copied" : "copy them"}
        </button>
      )}
    </p>
  );
}

/** The seed, plainly, so a run can be handed to somebody else. */
function Seed({ ui }: { ui: UiState }) {
  return (
    <p className="elegy-seed">
      seed <code data-testid="run-seed">{ui.seed}</code>
    </p>
  );
}

/**
 * The charge gauge. A bar first and a number second: how full the banks
 * are is a glance question, and only the net flow needs digits.
 */
function ChargeGauge({ ui }: { ui: UiState }) {
  const fill = Math.max(0, Math.min(100, ui.chargeFill / 10));
  const shed = ui.powerRefused.some(Boolean) || ui.brownout;
  const level = ui.charge <= 0 ? "empty" : shed ? "shed" : fill < 25 ? "low" : "ok";
  const condition =
    level === "empty"
      ? "empty"
      : level === "shed"
        ? "load shed"
        : level === "low"
          ? "reserve"
          : "holding";
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
        <span className="charge-condition">{condition}</span>
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
  const mood = attentionMood(ui.provocation, ui.provocationMax);
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

function attentionMood(value: number, maximum: number): string {
  const fill = Math.max(0, Math.min(100, (value / Math.max(1, maximum)) * 100));
  return MOODS.find(([ceiling]) => fill < ceiling)?.[1] ?? "roused";
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

function Readout({
  label,
  value,
  warn,
  hint,
}: {
  label: string;
  value: string;
  warn?: boolean;
  /** The precision layer (`DECISIONS.md` §8): hover only, never a badge. */
  hint?: string;
}) {
  return (
    <div className="readout" title={hint}>
      <dt>{label}</dt>
      <dd className={warn ? "warn" : undefined}>{value}</dd>
    </div>
  );
}

function Sidebar({ game, ui }: Props) {
  const catalog = game.getCatalog();
  const selectionRef = useRef<HTMLElement>(null);
  useEffect(() => {
    if (ui.selected) selectionRef.current?.scrollIntoView({ block: "nearest" });
  }, [ui.selected]);
  // The Heartseed is pre-placed and unique; offering it in the menu
  // would only ever produce a rejection.
  //
  // **And the menu opens out as the tower grows** (`SYSTEMS.md` §6.11).
  // A first turn used to show twenty cards in a tower that already had
  // a working chain in it, and nothing on that screen said which of the
  // twenty mattered. `ui.unlocked` is the simulation's own answer to
  // "what can be built right now", so the menu cannot drift from it.
  const open = new Set(ui.unlocked);
  const buildable = catalog.rooms.filter(
    (room, index) => room.category !== "Heart" && open.has(index),
  );

  // **Unlocked first, locked last — and nothing finer than that.**
  //
  // §5.7 argues for showing rooms you have not unlocked: a newcomer
  // sees the shape of what the game becomes. It does not argue for
  // giving that the same weight as a room you could press right now,
  // and late in a run the menu was mostly the former.
  //
  // **Sorting by affordability was tried and is wrong.** Stock moves
  // every few ticks, so cards rose and fell continuously — the list
  // reordered under the cursor and a button could not be clicked at
  // all. (Playwright found it: "element is not stable", forever.) A
  // menu that rearranges itself while you reach for it is worse than
  // one with some greyed cards in it.
  //
  // `locked` changes a handful of times a run, so this is stable.
  const sorted = [...buildable];
  sorted.sort((a, b) => Number(ui.locked.includes(a.id)) - Number(ui.locked.includes(b.id)));

  return (
    <aside className="sidebar panel">
      <section>
        <h2 className="section-title">Stores</h2>
        <div className="stock" data-testid="stock">
          {ui.stock.length === 0 ? (
            <span className="stock-empty">the shelves are bare</span>
          ) : (
            ui.stock.map((entry) => <Store key={entry.item} catalog={catalog} entry={entry} />)
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
          <li>
            {/*
              **Widening, beside growing.** A floor goes on top of what
              is already there; a wider hull is new frame along the
              whole height, and it costs more for that reason
              (`SYSTEMS.md` §6.16). The new deck arrives at the *back* —
              everything aboard slides forward — which is what keeps the
              weapons on the leading edge.
            */}
            <button
              type="button"
              className="build-card"
              disabled={!game.canAffordWidening() || ui.slots >= catalog.max_slots}
              data-testid="widen-tower"
              onClick={() => game.send("WidenTower")}
            >
              <span className="build-name">
                Widen the hull
                <span className="build-hint">
                  {/*
                    **Says what width is for** (`SYSTEMS.md` §6.30). It
                    read "2 more slots, at the back", which is true and
                    silent about the only thing width really buys: the
                    rooms that reach the ground carry `max_floor: 1` and
                    have two floors for ever, however tall the tower
                    gets. Width is their only relief, and it is what
                    decides whether a run has a shaft in it.
                  */}
                  {ui.slots >= catalog.max_slots
                    ? "as wide as it goes"
                    : `${catalog.widen_slots} more slots on every floor — the only room the ground floors will ever get`}
                </span>
              </span>
              <Cost game={game} costs={game.wideningCost()} />
            </button>
          </li>
          {sorted.map((room) => (
            <li key={room.id}>
              <RoomCard game={game} ui={ui} room={room} />
            </li>
          ))}
        </ul>
      </section>

      <section className="power-bus">
        <PowerOrder game={game} ui={ui} />
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
        <section ref={selectionRef} className="selection" data-testid="selection">
          <h3>{ui.selected.info.name}</h3>
          <p>{describeRoom(game, ui.selected.info)}</p>
          {ui.selected.info.category === "Storage" && ui.selected.room.shelves.length > 0 && (
            <ShelfFilters game={game} ui={ui} />
          )}
          <div className="selection-actions">
            <button
              type="button"
              className="selection-key"
              disabled={
                !ui.selected.removable ||
                ui.selected.room.active ||
                [...ui.selected.room.inputs, ...ui.selected.room.outputs].some(
                  (stack) => stack.count > 0,
                ) ||
                ui.selected.room.shelves.some((shelf) => shelf.count > 0)
              }
              data-testid="relocate-room"
              onClick={() => game.beginRelocatingSelected()}
            >
              Refit elsewhere
            </button>
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

/** Physical shelf labels: a reservation, not an abstract routing rule. */
function ShelfFilters({ game, ui }: Props) {
  const selected = ui.selected;
  if (!selected || selected.info.category !== "Storage") return null;
  const catalog = game.getCatalog();
  // A shelf label is planning, not a content encyclopedia. Offer what the
  // tower already holds and what its currently unlocked rooms consume or
  // produce. Preserve an existing label even if its chain is not open now.
  const relevant = new Set(ui.stock.map((entry) => entry.item));
  for (const roomIndex of ui.unlocked) {
    const room = catalog.rooms[roomIndex];
    if (!room) continue;
    for (const cost of [...room.build_cost, ...room.inputs, ...room.outputs]) {
      relevant.add(cost.item);
    }
  }
  for (const shelf of selected.room.shelves) {
    if (shelf.item !== null) relevant.add(shelf.item);
    if (shelf.filter !== null) relevant.add(shelf.filter);
  }
  const relevantItems = catalog.items.filter((_item, index) => relevant.has(index));
  return (
    <fieldset className="shelf-filters" data-testid="shelf-filters">
      <legend>Shelf labels</legend>
      {selected.room.shelves.map((shelf, index) => (
        <label key={index} className="shelf-filter">
          <span>{index + 1}</span>
          <select
            aria-label={`Reserve shelf ${index + 1}`}
            value={shelf.filter === null ? "" : (catalog.items[shelf.filter]?.id ?? "")}
            onChange={(event) =>
              game.setShelfFilter(selected.id, index, event.currentTarget.value || null)
            }
          >
            <option value="">anything</option>
            {relevantItems.map((item) => (
              <option
                key={item.id}
                value={item.id}
                disabled={
                  shelf.count > 0 &&
                  shelf.item !== null &&
                  catalog.items[shelf.item]?.id !== item.id
                }
              >
                {item.name}
              </option>
            ))}
          </select>
        </label>
      ))}
    </fieldset>
  );
}

/**
 * One item on the shelves, drawn as a shelf.
 *
 * **The number was the whole interface.** `🎋 15` says nothing about
 * whether fifteen is a lot, and nothing at all about the fact the panel
 * most needs to carry: a shelf that is *full* is why a chain stops.
 * `SYSTEMS.md` §5.11 open question 0 is that every chain terminates in
 * a buffer and a full buffer caps the tower's whole harvest — and that
 * was visible only as a row of pips inside a room in the cross-section,
 * which is the last place a player looks when wondering why the cutter
 * arm has gone quiet.
 *
 * So it fills. The count goes to the hover, where `DECISIONS.md` §8
 * puts precision, and the level leads. A full shelf gets a lip rather
 * than a colour, because a full store is not an error — it is a tower
 * that has everything it needs and is telling you to spend some.
 */
function Store({ catalog, entry }: { catalog: CatalogSnapshot; entry: StoreView }) {
  const item = catalog.items[entry.item];
  const space = Math.max(1, entry.space);
  const full = entry.count >= entry.space;
  const fill = Math.max(0, Math.min(100, (entry.count / space) * 100));
  return (
    <span
      className={`stock-item${full ? " full" : ""}`}
      title={`${entry.count} of ${entry.space} ${item?.name ?? ""}${full ? " · the shelf is full" : ""}`}
      role="meter"
      aria-label={item?.name}
      aria-valuenow={entry.count}
      aria-valuemin={0}
      aria-valuemax={entry.space}
    >
      <span className="stock-level" style={{ height: `${fill}%` }} />
      <ItemIcon item={item} className="glyph" decorative />
    </span>
  );
}

function RoomCard({ game, ui, room }: Props & { room: RoomInfo }) {
  const affordable = game.canAfford(room);
  const fits = game.hasRoomFor(room);
  const placed = room.unique && !fits;
  // **Locked rooms are greyed, not hidden.** A newcomer can see the
  // shape of what the game becomes without being able to reach for it,
  // and an unlock is then a thing that *opens* rather than a thing that
  // appears from nowhere and has to be explained (`SYSTEMS.md` §5.7).
  const locked = ui.locked.includes(room.id);
  const disabled = locked || !affordable || !fits;

  let hint = costHint(room);
  if (locked) hint = "the journal has not learnt this yet";
  else if (!fits) hint = placed ? "already standing" : "no room for it";
  else if (!affordable) hint = "not enough on the shelves";
  const tradeoff = roomTradeoff(game.getCatalog(), ui, room);

  return (
    <button
      type="button"
      className={`build-card${locked ? " locked" : ""}`}
      aria-pressed={ui.placing === room.id}
      disabled={disabled}
      data-testid={`build-${room.id}`}
      onClick={() => game.beginPlacing(ui.placing === room.id ? null : room.id)}
    >
      <span className="build-name">
        {room.name}
        <span className="build-hint">{hint}</span>
        {tradeoff && (
          <span className="build-tradeoff" data-testid={`tradeoff-${room.id}`}>
            {tradeoff}
          </span>
        )}
      </span>
      <Cost game={game} costs={room.build_cost} />
    </button>
  );
}

function ShaftCard({ game, ui, shaft }: Props & { shaft: ShaftInfo }) {
  const affordable = game.canAffordShaft(shaft);
  const perFloor = (shaft.ticks_per_floor / 30).toFixed(1);
  const span = game.shaftBuildSpan(shaft);
  // **One built shaft, two jobs, and the card says both** — the lift
  // carries people, and fetches stock by itself whenever nobody is
  // calling it (`SYSTEMS.md` §6.18). This used to branch on a
  // Dumbwaiter kind that no longer exists.
  const role =
    shaft.kind === "Chute"
      ? `automatic relief for surplus in a touching storeroom · one way down · ${perFloor}s a floor`
      : shaft.kind === "Busbar"
        ? `primary ${shaft.power_capacity} charge/tick trunk between floors`
        : shaft.kind === "VentStack"
          ? `${shaft.exhaust_capacity} smoke capacity · hot rooms must touch · must reach the roof`
          : `${shaft.capacity} aboard, or ${shaft.batch} in stock · ${perFloor}s a floor · ${shaft.charge_per_floor}⚡ a floor`;
  const pricedRole = `${span}-floor installation · ${role}`;
  const hint = affordable ? pricedRole : `${pricedRole} · not enough on the shelves`;

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
        <span className="build-tradeoff" data-testid={`tradeoff-${shaft.id}`}>
          {shaftTradeoff(shaft)}
        </span>
      </span>
      <Cost game={game} costs={game.shaftBuildCost(shaft)} />
    </button>
  );
}

/**
 * The card's second line says what a thing does. This line says what
 * choosing it gives up. It is deliberately derived from the existing
 * catalog rather than becoming another authored tuning field: when two
 * unlocked recipes consume the same item, that competition is the fact
 * the player needs to see.
 */
function roomTradeoff(catalog: CatalogSnapshot, ui: UiState, room: RoomInfo): string | null {
  if (room.burner) {
    return "tradeoff · charge now, but less bamboo for poles and more attention";
  }
  if (room.bank_capacity > 0) {
    return "tradeoff · local reserve and discharge; a riser is still needed to share it between floors";
  }
  if (room.category === "Storage") {
    return "tradeoff · safer buffers, but no new harvest or throughput";
  }
  if (room.top_floor_only && room.category === "Intake") {
    return "tradeoff · claims the roof for a weather-dependent harvest";
  }
  if (room.category === "Defence") {
    const mount = room.top_floor_only
      ? "the roof"
      : room.shaft_adjacent
        ? "a shaft-side bay"
        : room.front_only
          ? "the leading edge"
          : `${room.width} deck slot${room.width === 1 ? "" : "s"}`;
    return `tradeoff · commits ${mount} and its ammunition chain`;
  }

  const open = new Set(ui.unlocked);
  const inputItems = new Set(room.inputs.map((input) => input.item));
  const rivals = catalog.rooms
    .filter(
      (candidate, index) =>
        candidate.id !== room.id &&
        open.has(index) &&
        candidate.inputs.some((input) => inputItems.has(input.item)),
    )
    .slice(0, 2);
  if (rivals.length > 0) {
    const shared = room.inputs
      .filter((input) =>
        rivals.some((candidate) => candidate.inputs.some((it) => it.item === input.item)),
      )
      .map((input) => catalog.items[input.item]?.name.toLowerCase() ?? "stock")
      .filter((name, index, names) => names.indexOf(name) === index)
      .join(" and ");
    return `tradeoff · shares ${shared} with ${rivals.map((candidate) => candidate.name).join(" / ")}`;
  }
  if (room.crew_required > 0) {
    return `tradeoff · commits ${room.crew_required} porter${room.crew_required === 1 ? "" : "s"} while staffed`;
  }
  if (room.category === "Intake") {
    return `tradeoff · ${room.width} low-deck slot${room.width === 1 ? "" : "s"} for one terrain yield`;
  }
  if (room.power_draw > 0) {
    return `tradeoff · adds ${room.power_draw} charge/tick to its floor circuit`;
  }
  // A universal "uses deck slots" sentence is not a decision; it made
  // every card taller while hiding the few conflicts that deserve a
  // pause. Quiet cards are intentional.
  return null;
}

function shaftTradeoff(shaft: ShaftInfo): string {
  switch (shaft.kind) {
    case "Elevator":
      return "tradeoff · movement and a medium trunk, in the column a Busbar would occupy";
    case "Busbar":
      return "tradeoff · strongest trunk and redundancy, but carries no people or stock";
    case "VentStack":
      return "tradeoff · clean hot rooms, but carries neither charge nor cargo";
    case "Chute":
      return "tradeoff · clears unwanted surplus downward, with no return journey";
    case "Stairs":
      return "tradeoff · emergency movement and wiring, slowly";
  }
}

function Cost({ game, costs }: { game: Game; costs: { item: number; amount: number }[] }) {
  const catalog = game.getCatalog();
  if (costs.length === 0) return <span className="build-cost">free</span>;
  return (
    <span className="build-cost">
      <CostItems catalog={catalog} costs={costs} />
    </span>
  );
}

/**
 * What an offer asks and what it gives, in the goods themselves.
 *
 * "4⚙️ → 3🎋" rather than a price: there is no currency in this game,
 * and inventing a unit to display would be inventing one.
 */
function Terms({
  catalog,
  enclave,
  index,
}: {
  catalog: CatalogSnapshot | null;
  enclave: EnclaveInfo | null;
  index: number;
}) {
  const offer = enclave?.offers[index];
  if (!catalog || !offer) return "an exchange";
  return (
    <>
      <CostItem catalog={catalog} cost={offer.give} />
      <span aria-hidden="true"> → </span>
      <CostItem catalog={catalog} cost={offer.take} />
    </>
  );
}

function CostItem({ catalog, cost }: { catalog: CatalogSnapshot; cost: CostInfo }) {
  const item = catalog.items[cost.item];
  return (
    <span className="item-amount">
      <span>{cost.amount}</span>
      <ItemIcon item={item} />
    </span>
  );
}

/** A list of costs, in the same shorthand. */
function CostItems({ catalog, costs }: { catalog: CatalogSnapshot | null; costs: CostInfo[] }) {
  if (!catalog) return null;
  return (
    <span className="cost-items">
      {costs.map((cost, index) => (
        <CostItem key={`${String(cost.item)}-${String(index)}`} catalog={catalog} cost={cost} />
      ))}
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
    case "Quarters":
      return room.min_floor === null ? "beds" : `beds · floor ${room.min_floor} and up`;
    case "Energy":
      if (room.burner) {
        return room.min_floor === null
          ? "burns bamboo for charge"
          : `burns bamboo for charge · floor ${room.min_floor} and up`;
      }
      if (room.bank_capacity > 0) {
        // **Both halves, because they answer different problems.**
        // Capacity is how long the tower lasts; discharge is how hard it
        // can spend at once, and a tower browning out with a full bank
        // needs the second number to understand why (`SYSTEMS.md` §6.36).
        return `holds ${room.bank_capacity}⚡ · gives ${room.bank_discharge}⚡ a tick`;
      }
      return "";
    case "Defence": {
      // **Still no damage, rate or range on the card**, and the reason
      // is unchanged: where you put it and whether the chain keeps it
      // fed are the decisions, not the numbers.
      //
      // What *has* changed is that there is now a third decision and it
      // is not a number. §6.22 gave each weapon an approach it answers —
      // a lantern mast cannot see the ground, a root ward cannot see
      // the trees — so "shoots back" stopped being enough to choose
      // with. What it answers and what it eats are facts about what the
      // thing is *for*; the numbers stay off.
      const answers = room.defence?.targets.length
        ? `answers ${room.defence.targets.join(" and ")}`
        : "answers anything";
      const effect = room.defence ? defenceEffect(room.defence.effect) : "strikes one creature";
      const mount = room.top_floor_only
        ? " · roof only"
        : room.shaft_adjacent
          ? " · must touch a shaft"
          : "";
      const power = room.power_draw > 0 ? ` · ${room.power_draw}⚡ a tick` : "";
      // **A shot's charge is on the card, and damage still is not.** The
      // rule has not moved: the numbers stay off because placement and
      // supply are the decisions. This is one of those — a thorn gun
      // costs nothing to fire and a lantern mast costs 40, so which
      // weapon a tower can afford to *run* is now a real question and an
      // invisible one until it is written down (`SYSTEMS.md` §6.36).
      const shot =
        room.defence && room.defence.charge_per_shot > 0
          ? ` · ${room.defence.charge_per_shot}⚡ a shot`
          : "";
      return `${effect} · ${answers}${mount}${power}${shot}`;
    }
    default:
      return "";
  }
}

function describeRoom(game: Game, info: RoomInfo): string {
  const catalog = game.getCatalog();
  const name = (index: number) => catalog.items[index]?.name ?? "something";

  if (info.burner) {
    return "Burns bamboo for charge, and nothing else in the tower makes any. Every stalk burned is a stalk not built with — but it only lights when the bank has room, so what it really costs you is walking far and working hard.";
  }
  if (info.bank_capacity > 0) {
    return `Holds ${info.bank_capacity} charge and can give back ${info.bank_discharge} a tick. Storage is something you build, not something you find — and the second number is the one that matters when the tower is full and still shedding circuits, because that is the tower asking for a wider pipe rather than a deeper well.`;
  }
  if (info.defence) {
    // **It names the ammo now**, and the comment that used to sit here
    // said the catalog did not carry it — true when every emplacement
    // ate darts, and stale since §6.22 gave each one a chain of its
    // own. Naming it is the point: a tanglenet eating rope and a
    // lantern mast eating charge cells are two different production
    // lines, and that is the decision.
    const eats = name(info.defence.ammo).toLowerCase();
    const answers = info.defence.targets.length
      ? `what comes out of ${info.defence.targets.join(" and ")}`
      : "whatever comes close";
    const effect = defenceEffect(info.defence.effect);
    const mount = info.top_floor_only
      ? " It only mounts on the current roof."
      : info.shaft_adjacent
        ? " It must touch a shaft column so it can interrupt a creature already inside circulation."
        : "";
    const draw =
      info.defence.charge_per_shot > 0
        ? ` Every shot also draws ${info.defence.charge_per_shot} charge off the bank, so a wave is a load on the switchboard and not only on the shelves.`
        : " It draws no charge, so it keeps answering through a brown-out when the powered emplacements have gone quiet.";
    return `${effect[0]?.toUpperCase() ?? ""}${effect.slice(1)} and answers ${answers}, off a rack of ${eats} that ordinary crew have to keep filled.${mount} Run it dry and it goes quiet, for exactly the same reason a mill does.${draw}`;
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

function defenceEffect(effect: NonNullable<RoomInfo["defence"]>["effect"]): string {
  switch (effect) {
    case "Tangle":
      return "anchors one approach";
    case "Resonance":
      return "staggers a nearby cluster";
    case "Repel":
      return "turns one canopy creature away";
    case "RootWard":
      return "interrupts and pins one burrower";
    case "Direct":
      return "strikes one creature";
  }
}

/**
 * Space to pause, 1/2/4 for speed, Escape to drop out of placement.
 * Bound on window so they work wherever the pointer is.
 */
/**
 * The tower's weapons, and whether they can fire.
 *
 * Ammo counts rather than a green light: "3 darts" is a number a player
 * can plan with and "ready" is not. The bar goes quiet — dimmed, not
 * red — when a rack is empty, which is the same language a starved mill
 * speaks (`DECISIONS.md` §8).
 */
function Weapons({ ui }: Props) {
  if (ui.weapons.length === 0) return null;

  return (
    <div className="weapons" role="group" aria-label="Weapons" data-testid="weapons">
      {ui.weapons.map((weapon) => (
        <span
          key={weapon.id}
          className={weapon.ammo === 0 ? "weapon dry" : "weapon"}
          data-testid={`weapon-${weapon.id}`}
          title={`${weapon.name} — floor ${weapon.floor}`}
        >
          <span className="weapon-name">{weapon.short}</span>
          <span className="weapon-ammo">{weapon.ammo}</span>
        </span>
      ))}
    </div>
  );
}

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
          if (placing) game.cancelPlacement();
          break;
        // Zoom. `=` as well as `+` because reaching plus means holding
        // shift on most layouts, and a keyboard shortcut you have to
        // use two hands for is a keyboard shortcut nobody uses.
        case "+":
        case "=":
          game.zoomBy(1.15);
          break;
        case "-":
        case "_":
          game.zoomBy(1 / 1.15);
          break;
        case "0":
          game.resetZoom();
          break;
        default:
          break;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [game, speed, placing, walking]);
}
