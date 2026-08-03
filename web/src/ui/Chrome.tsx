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

import { useEffect, useState } from "react";

import type { Game, UiState } from "../engine/Game";
import type {
  CatalogSnapshot,
  CostInfo,
  EnclaveInfo,
  HaltView,
  RoomInfo,
  ShaftInfo,
  SimSpeed,
} from "../bridge/types";

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
      <Roster game={game} ui={ui} />
      {ui.fork && <ForkCard game={game} ui={ui} />}
      {ui.atEnclave && <EnclaveBoard game={game} ui={ui} />}
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
      {ui.arrived && !ui.lost && <Arrival game={game} ui={ui} />}
    </div>
  );
}

/**
 * Who is aboard, what they are doing, and which shift they work.
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
          const night = member.shift === "Night";
          return (
            <li className="roster-row" key={member.id} data-testid={`crew-${member.id}`}>
              <span className="roster-face" aria-hidden="true">
                {FACES[member.fidget % FACES.length]}
              </span>
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
              </span>
              <button
                type="button"
                className={`shift-toggle${night ? " night" : ""}`}
                data-testid={`shift-${member.id}`}
                aria-pressed={night}
                title={
                  night
                    ? `${member.name} works the night. Set them back to days.`
                    : `${member.name} works the day. Put them on nights — they will be up while the day crew sleep, and their bed is free for somebody else.`
                }
                onClick={() => game.setShift(member.id, night ? "Day" : "Night")}
              >
                {night ? "night" : "day"}
              </button>
            </li>
          );
        })}
      </ul>
      <Schedules game={game} ui={ui} />
    </aside>
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
function Schedules({ game, ui }: Props) {
  const catalog = game.getCatalog();
  const dispatched = ui.shafts.filter((shaft) => shaft.kind !== "Stairs");
  if (dispatched.length === 0) return null;
  const daypart = ui.daypartIndex;

  return (
    <>
      <h2 className="section-title schedule-title">
        Shafts · {catalog.dayparts[daypart]?.name ?? "now"}
      </h2>
      <ul className="schedule-list">
        {dispatched.map((shaft) => {
          const program = shaft.programs[daypart];
          const served = program?.served ?? [];
          const info = catalog.shafts[shaft.def];
          return (
            <li className="schedule-row" key={shaft.id} data-testid={`schedule-${shaft.id}`}>
              <span className="schedule-name">{info?.name ?? "shaft"}</span>
              <span className="schedule-floors">
                {Array.from({ length: shaft.high - shaft.low + 1 }, (_, i) => {
                  const floor = shaft.low + i;
                  const on = served[floor] ?? true;
                  return (
                    <button
                      type="button"
                      key={floor}
                      className={`floor-pip${on ? " on" : ""}`}
                      data-testid={`stop-${shaft.id}-${floor}`}
                      aria-pressed={on}
                      title={
                        on
                          ? `Stops at F${floor}. Click to skip it this daypart.`
                          : `Skips F${floor}. Click to stop there this daypart.`
                      }
                      onClick={() => {
                        const next = [...served];
                        while (next.length <= floor) next.push(true);
                        next[floor] = !on;
                        game.setShaftProgram(
                          shaft.id,
                          daypart,
                          next,
                          program?.priority ?? "Balanced",
                        );
                      }}
                    >
                      {floor}
                    </button>
                  );
                })}
              </span>
            </li>
          );
        })}
      </ul>
    </>
  );
}

/**
 * Faces, chosen by `fidget` — the per-crew cosmetic draw that already
 * exists for the renderer's idle phase.
 *
 * `portrait = fidget % faces` is the whole mechanism. It costs no new
 * state and no new roll, it is stable for the life of a crew member,
 * and it is reproducible from a seed. Crucially it is drawn from the
 * `cosmetic` stream, so adding or removing a face can never perturb an
 * economic roll (`DECISIONS.md` §2).
 */
const FACES = ["🌱", "🍃", "🪴", "🌿", "🌾", "🌻", "🌴", "🍂"] as const;

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
      return "idle";
  }
}

function TopBar({ game, ui }: Props) {
  const catalog = game.getCatalog();
  return (
    <header className="topbar panel">
      <span className="brand">Understory</span>
      <Journey ui={ui} />
      <dl className="readouts">
        <Readout label="Day" value={`${ui.day + 1} · ${ui.daypart}`} />
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
      <SoundToggle game={game} />
      <button
        type="button"
        className={`stride-toggle stride-${ui.halt}`}
        aria-pressed={ui.walking}
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
      title={on ? "Mute" : "Listen to the tower"}
      onClick={() => {
        const next = !on;
        setOn(next);
        game.setAudioEnabled(next);
      }}
    >
      {on ? "🔊" : "🔇"}
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
            </button>
          );
        })}
      </div>
    </section>
  );
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
              <span className="offer-terms">{terms(catalog, ui.enclave, index)}</span>
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
          {ui.shellWork > 0
            ? `Have them plate the hull · ${costLine(catalog, ui.enclave.reinforce.cost)}`
            : "The hull is as plated as they will make it"}
          <span className="enclave-note">
            {ui.shellWork > 0
              ? `+${ui.enclave.reinforce.panel_hp} to every panel, and to every floor built after`
              : `+${ui.shellBonus} already on every panel`}
          </span>
        </button>
      )}
      <button
        type="button"
        className="enclave-recruit"
        disabled={ui.recruits <= 0}
        data-testid="recruit"
        onClick={() => game.recruit()}
      >
        {ui.recruits > 0
          ? `Ask someone to come aboard · ${costLine(catalog, ui.enclave?.recruit_cost ?? [])}`
          : "Nobody else is coming"}
      </button>
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
      <p className="elegy-note">{game.journalNow().runs.length} runs written down.</p>
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

/**
 * What an offer asks and what it gives, in the goods themselves.
 *
 * "4⚙️ → 3🎋" rather than a price: there is no currency in this game,
 * and inventing a unit to display would be inventing one.
 */
function terms(
  catalog: CatalogSnapshot | null,
  enclave: EnclaveInfo | null,
  index: number,
): string {
  const offer = enclave?.offers[index];
  if (!catalog || !offer) return "an exchange";
  const side = (cost: CostInfo) => `${cost.amount}${catalog.items[cost.item]?.glyph ?? ""}`;
  return `${side(offer.give)} → ${side(offer.take)}`;
}

/** A list of costs, in the same shorthand. */
function costLine(catalog: CatalogSnapshot | null, costs: CostInfo[]): string {
  if (!catalog) return "";
  return costs.map((cost) => `${cost.amount}${catalog.items[cost.item]?.glyph ?? ""}`).join(" ");
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
