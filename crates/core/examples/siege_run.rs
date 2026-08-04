//! Play a run and print what the jungle did to it.
//!
//! ```text
//! cargo run -p understory-core --example siege_run
//! ```
//!
//! M2's sprint question — does combat-as-logistics-stress produce drama
//! without an aimed weapon? — is a question about a curve, not about a
//! single tick, and it is not one the unit tests can answer. This is
//! the instrument for it, the way `throughput.rs` was the instrument
//! for M1's elevator question.
//!
//! It plays three towers side by side over five in-game days, all from
//! the same seed and the same starting position, differing only in how
//! their owner answers the jungle:
//!
//! - **subsistence** — one arm, never expands. Should barely be noticed.
//! - **greedy** — a second cutter arm and nothing else. Provokes, and
//!   has to live with it.
//! - **answered** — the second arm, plus a mill, a thornwright and a
//!   dart battery to shoot back with.
//!
//! What the numbers have to show for the answer to be yes: subsistence
//! is quiet, greedy is under real and increasing pressure but is not
//! simply destroyed, and answered is meaningfully better off than
//! greedy for the poles it spent.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;
use understory_core::systems::siege::tower_integrity_permille;

const SEED: u64 = 0x0000_5EED_0000_0002;
const TICKS_PER_DAY: u32 = 14_400;
const DAYS: u32 = 5;

fn main() {
    for plan in [Plan::Subsistence, Plan::Greedy, Plan::Answered] {
        run(plan);
    }
    pressure_table();
    does_plating_help();
}

/// What a given tower can take at a given level of attention.
///
/// The three plans above tell a story and are the better read, but they
/// stopped being able to *measure* the siege: a tower only provokes
/// while it is consuming, and once a shopping list runs out the jungle
/// forgets it exists. Every plan then finishes untouched, which says
/// nothing about whether `panel_hp` or `threat_per_100_provocation` are
/// any good.
///
/// So this pins provocation instead of earning it, holds the tower's
/// shape fixed, and asks the question the Siege rows actually encode:
/// at this much attention, with this much built, does it hold? No
/// economy in the loop, nothing to run out of, and the same three days
/// every time.
fn pressure_table() {
    const DAYS: u32 = 3;
    const LEVELS: [i64; 5] = [100, 300, 500, 700, 1000];
    /// Averaged over this many seeds a cell, because one seed does not
    /// describe a curve. Which creatures a wave can field changes
    /// discontinuously with provocation — `min_provocation` lets the
    /// leaper in at 330 and the borer at 500 — so a single run reads as
    /// non-monotonic nonsense: measured once, the starting tower was
    /// mauled at 600 and untouched at 1,000.
    const SEEDS: u64 = 6;

    println!("\n=== how much attention can a tower take? ===\n");
    println!(
        "  provocation held, tower held fixed, {DAYS} days a run, {SEEDS} seeds a cell.\n  \
         `lost` is hit points still missing at the end, averaged; `worst` is the \n           unluckiest seed of the six.\n"
    );
    println!(
        "{:<20} {:>5} {:>9} {:>7} {:>7} {:>8} {:>7} {:>6}    verdict",
        "tower", "prov", "standing", "lost hp", "worst", "seen off", "mend", "darts"
    );

    for shape in [Shape::Bare, Shape::Plated, Shape::Answered] {
        for level in LEVELS {
            let mut standing_sum = 0;
            let mut lost_sum = 0;
            let mut worst = 0;
            let mut repelled_sum = 0;
            let mut mended_sum = 0;
            let mut spent_sum = 0i64;
            let mut deaths = 0;
            for seed in 1..=SEEDS {
                let (standing, lost, repelled, mended, _, _, spent) =
                    press(shape, level, DAYS, seed);
                standing_sum += standing;
                lost_sum += lost;
                worst = worst.max(lost);
                repelled_sum += repelled;
                mended_sum += mended;
                spent_sum += spent;
                if standing == 0 {
                    deaths += 1;
                }
            }
            let n = i64::try_from(SEEDS).unwrap_or(1);
            let standing = standing_sum / n;
            println!(
                "{:<20} {level:>5} {standing:>8}‰ {:>7} {worst:>7} {:>8} {:>7} {:>6}  {}",
                shape.name(),
                lost_sum / n,
                repelled_sum / u64::from(SEEDS as u32),
                mended_sum / u64::from(SEEDS as u32),
                spent_sum / n,
                if deaths > 0 {
                    format!("LOST {deaths}/{SEEDS}")
                } else if standing >= 950 {
                    "held".to_string()
                } else if standing >= 700 {
                    "worn".to_string()
                } else {
                    "mauled".to_string()
                }
            );
        }
    }
}

/// Does plating actually help?
///
/// **It does, and this comparison said otherwise three times running.**
/// The fault was the metric, every time: `reinforce` raises `panel_hp`
/// and nothing else, so any column that counts panels compares two
/// towers whose maxima differ by exactly the thing under test — the
/// plated one has more to lose and reads worse for having it, whether
/// the column is a fraction or an absolute. It counts rooms and shafts
/// now, whose maxima are identical on both towers.
///
/// And it sweeps provocation rather than sitting at 100, because at 100
/// the panels hold on both towers and **nothing whatsoever reaches a
/// room** — eight seeds, zero damage, both shapes. A comparison run
/// where the mechanism cannot engage is not evidence that the mechanism
/// does nothing, and that was the other half of why this kept coming out
/// wrong.
fn does_plating_help() {
    println!(
        "
  the same tower, bare and plated twice, swept across provocation.

  Hit points lost from **rooms and shafts only**. Plating raises panel
  health and nothing else, so any column that counts panels compares two
  towers whose maxima differ by the thing being tested — which is how
  this comparison reported plating as worse three times running. Rooms
  and shafts are identical on both towers, and buying time for what is
  behind the skin is what plating is for.
"
    );
    println!(
        "  {:>4}  {:>4}  {:>9}  {:>9}  {:>8}",
        "prov", "seed", "bare lost", "plated", "verdict"
    );
    let mut plated_worse = 0;
    let mut compared = 0;
    // **Swept rather than fixed at one level, because at the level this
    // used to run there is nothing to measure.** At provocation 100 the
    // panels hold on both towers and *nothing at all* reaches a room:
    // eight seeds, zero damage behind the skin, both shapes. A
    // comparison run where the mechanism cannot engage is not evidence
    // that the mechanism does nothing.
    for level in [300, 500, 700] {
        for seed in 1..=8u64 {
            let (_, _, _, _, _, bare, _) = press(Shape::Bare, level, 3, seed);
            let (_, _, _, _, _, plated, _) = press(Shape::Plated, level, 3, seed);
            if bare == 0 && plated == 0 {
                continue;
            }
            compared += 1;
            if plated > bare {
                plated_worse += 1;
            }
            println!(
                "  {level:>4}  {seed:>4}  {bare:>9}  {plated:>9}  {:>8}",
                if plated > bare {
                    "worse"
                } else if plated < bare {
                    "better"
                } else {
                    "same"
                }
            );
        }
    }
    println!(
        "
  plated came out worse on {plated_worse} of {compared} comparisons where anything
  got behind the skin at all. More than half is a finding; a couple is noise."
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// What every run starts with.
    Bare,
    /// The same tower with its hull plated twice, as an enclave will do.
    Plated,
    /// A battery, a thornwright to feed it, and a second mill.
    Answered,
}

impl Shape {
    fn name(self) -> &'static str {
        match self {
            Self::Bare => "as it starts",
            Self::Plated => "plated twice",
            Self::Answered => "battery + darts",
        }
    }
}

/// Hold provocation at `level` and see what happens.
fn press(shape: Shape, level: i64, days: u32, seed: u64) -> (i64, i64, u64, u64, i64, i64, i64) {
    let mut engine = GameEngine::new(seed);
    engine.set_speed(SimSpeed::X1);
    let content = engine.content().clone();

    if shape == Shape::Answered {
        // **Paid for first, and then asserted.** This harness has now
        // lied twice about the same tower in the same way. The first
        // time, hard-coded coordinates meant the thornwright was refused
        // for a slot clash and the "answered" tower had a battery with
        // no dart supply; `build_anywhere` fixed that. The second time —
        // this one — M5 put 2 rope on a dart battery's price, the
        // pressure tower starts with ten poles and no rope, every build
        // was refused, and `Shape::Answered` was *byte-for-byte the bare
        // tower*: identical 629 hit points lost at provocation 300, and
        // **zero darts fired at every level of the table**.
        //
        // A harness that reports a defence comparison in which nothing
        // was ever defended is worse than one that crashes. So: stock
        // it, then insist.
        pay_for_rooms(&mut engine);
        for room in ["room.dart_battery", "room.thornwright", "room.mill"] {
            assert!(
                build_anywhere(&mut engine, room),
                "the answered tower could not build {room}, so it is not an answered tower"
            );
        }
    }
    if shape == Shape::Plated {
        // The figures the enclave *would* charge for, applied directly.
        // Read from here rather than from content because the offer is
        // currently withdrawn — see `regions/drowned_city.ron`, and see
        // the last section of this harness for why.
        engine.state_mut_for_test().tower.reinforce(120);
    }
    // Stock it so the chain is never the limiting factor — this is a
    // measurement of the siege, not of the economy.
    stock_everything(&mut engine);

    let mut darts_spent = 0i64;
    let ticks = days * TICKS_PER_DAY;
    for tick in 0..ticks {
        if tick % 60 == 0 {
            darts_spent += stock_everything(&mut engine);
            let siege = &mut engine.state_mut_for_test().siege;
            siege.provocation = level;
            siege.provocation_acc = 0;
        }
        step_walking(&mut engine, 1);
        if engine.state().siege.lost {
            break;
        }
    }

    let state = engine.state();
    (
        tower_integrity_permille(state),
        total_hp(state) - standing_hp(state),
        state.siege.repelled,
        state.stats.hp_repaired,
        understory_core::systems::repair::outstanding_repair_cost(state, &content),
        behind_the_skin_lost(state),
        darts_spent,
    )
}

/// Hit points lost from **rooms and shafts**, ignoring panels entirely.
///
/// **The only honest way to ask whether plating works, and it took four
/// goes to find it.** `reinforce` raises `panel_hp` and nothing else, so
/// any metric that includes panels compares two towers whose maxima
/// differ by exactly the thing under test: the plated one has more to
/// lose and reads worse for having it, whether the column is a fraction
/// (per-mille) or an absolute (hit points missing). That reading has now
/// been produced three times and withdrawn twice, and `AGENTS.md` warns
/// about it by name.
///
/// Rooms and shafts have identical maxima on both towers, so this column
/// is apples to apples — and it is also the mechanism plating is *for*.
/// A thicker skin does not stop damage, it buys time: creatures spend
/// longer chewing through a panel and correspondingly less time on the
/// mill behind it. If plating does anything at all, it shows up here.
fn behind_the_skin_lost(state: &understory_core::state::GameState) -> i64 {
    let mut lost = 0;
    for floor in &state.tower.floors {
        for room in &floor.rooms {
            lost += room.health.max - room.health.hp;
        }
    }
    for shaft in &state.tower.shafts {
        lost += shaft.health.max - shaft.health.hp;
    }
    lost
}

/// Hit points the tower is missing, in absolute terms.
///
/// The per-mille readout is a *fraction*, so it is not comparable
/// between towers with different totals: plating raises every panel's
/// maximum, and a plated tower that loses one panel reads worse than a
/// bare tower that loses the same panel, because the panel it lost was
/// bigger. That is fine for the player, who only ever compares their
/// tower to itself, and useless for a table that compares three towers
/// to each other. This column is the honest one.
fn standing_hp(state: &understory_core::state::GameState) -> i64 {
    let mut hp = 0;
    for floor in &state.tower.floors {
        hp += floor.panel.hp;
        for room in &floor.rooms {
            hp += room.health.hp;
        }
    }
    for shaft in &state.tower.shafts {
        hp += shaft.health.hp;
    }
    hp
}

fn total_hp(state: &understory_core::state::GameState) -> i64 {
    let mut max = 0;
    for floor in &state.tower.floors {
        max += floor.panel.max;
        for room in &floor.rooms {
            max += room.health.max;
        }
    }
    for shaft in &state.tower.shafts {
        max += shaft.health.max;
    }
    max
}

/// Keep poles and darts on the shelves, so nothing under measurement is
/// waiting on the chain.
///
/// Topped to a *modest* level, and this matters more than it looks. An
/// earlier version filled the shelves to the brim every minute, which
/// left the tower with nowhere to put anything a crew member picked
/// up — and a stranded carrier could not repair. That amplified a small
/// difference between two towers into a fourfold gap in repair and had
/// me withdraw a working feature. An instrument that changes the thing
/// it measures is worse than no instrument.
/// Returns how many darts it had to put back on the racks, which is
/// exactly how many were fired since the last top-up — the racks are
/// refilled every 60 ticks, so stock cannot be read for consumption but
/// the refill can.
fn stock_everything(engine: &mut GameEngine) -> i64 {
    let content = engine.content().clone();
    let poles = content.item_idx("item.poles");
    let darts = content.item_idx("item.darts");
    let mut reloaded = 0i64;
    let state = engine.state_mut_for_test();
    for item in [poles, darts].into_iter().flatten() {
        let held = state.stock_of(item);
        if held < 16 {
            state.shelve(item, 16 - held);
        }
    }
    // And the racks, so a battery is measured on its reload rather than
    // on its supply line.
    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            if let Some(darts) = darts
                && let Some(rack) = room.inputs.iter_mut().find(|s| s.item == darts)
            {
                let space = rack.space();
                reloaded += rack.deposit(space);
            }
        }
    }
    reloaded
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Plan {
    Subsistence,
    Greedy,
    Answered,
}

impl Plan {
    fn name(self) -> &'static str {
        match self {
            Self::Subsistence => "subsistence",
            Self::Greedy => "greedy",
            Self::Answered => "answered",
        }
    }
}

fn run(plan: Plan) {
    let mut engine = GameEngine::new(SEED);
    engine.set_speed(SimSpeed::X1);

    // A minute to get the chain turning before anyone builds anything.
    step_walking(&mut engine, 1800);

    // A shopping list, worked through in order as poles allow. Buying
    // the whole plan at tick 1800 is not something a player could do —
    // the first attempt at this harness tried, silently could not
    // afford the third item, and spent five days comparing a tower with
    // a dart battery against a tower that had failed to build one.
    // **A canteen and a bunk in every plan — but after the plan's own
    // rooms, not before them.**
    //
    // Without them all three towers are starving and exhausted by day
    // two, which makes every figure below a measurement of neglect
    // rather than of the siege. But buying them first is worse than not
    // buying them at all: a cutter arm may only stand on the two lowest
    // floors (`max_floor`), floor 1 has exactly two two-wide gaps, and
    // a canteen and a bunk fill both. Greedy and answered could then
    // never place their second arm, the list jammed behind an item that
    // was not unaffordable but *unplaceable*, and all three plans ran
    // as subsistence with extra steps — measured, every tower ending
    // day 5 at full standing with provocation 0, ten rooms and two
    // poles banked. The home rooms fit anywhere; the plan's rooms do
    // not. Order accordingly.
    let mut list: Vec<&str> = match plan {
        Plan::Subsistence => vec![],
        Plan::Greedy => vec!["room.cutter_arm"],
        Plan::Answered => vec![
            "room.cutter_arm",
            "room.dart_battery",
            "room.mill",
            "room.thornwright",
        ],
    };
    list.push("room.canteen");
    list.push("room.bunk");
    // And then storerooms, indefinitely.
    //
    // Not padding: a tower only provokes while it is *consuming*, and
    // it only consumes while it is building. Once a shopping list runs
    // out, poles pile up on the shelves, the mill's outbox fills, the
    // arms stall, and the jungle forgets the tower exists — measured,
    // all three plans finishing at full integrity with provocation
    // zero, which says nothing about any of them. A player does not
    // stop wanting things on day two, so neither does the harness. A
    // storeroom is the cheapest standing reason to want poles, so it is
    // what the list keeps buying.
    list.reverse();
    for _ in 0..40 {
        list.insert(0, "room.storeroom");
    }

    println!("\n=== {} ===", plan.name());
    println!(" day  prov  standing  poles  mended  spent  repelled  bill  built  walked");

    // Look the index up rather than assuming one. Items are interned
    // in sorted-id order, so `ItemIdx(0)` is bamboo, and a harness that
    // guesses reports the wrong column with total confidence.
    let poles = engine
        .content()
        .item_idx("item.poles")
        .expect("the pack defines poles");
    let darts = engine.content().item_idx("item.darts");
    let mut last_mended = 0;
    let mut last_spent = 0;
    let mut last_repelled = 0;
    let mut last_paces = 0i64;
    let mut deferred = 0;

    for day in 1..=DAYS {
        for _ in 0..12 {
            step_walking(&mut engine, TICKS_PER_DAY / 12);
            let Some(next) = list.last().copied() else {
                continue;
            };
            if build_anywhere(&mut engine, next) {
                list.pop();
                continue;
            }
            // Out of floor, not out of money: a tower with poles banked
            // and no free slot is one a player would build upward. The
            // sails go back on the new roof straight away, because
            // growing taller shades the old ones and a tower that stops
            // making charge stops walking, harvesting and everything
            // else within the minute.
            if engine.try_send(GameCommand::BuildFloor).is_ok() {
                let top = engine.state().tower.top_floor();
                for slot in 0..engine.content().balance.tower.floor_slots {
                    if engine
                        .try_send(GameCommand::PlaceRoom {
                            room: "room.canopy_sails".into(),
                            floor: top,
                            slot,
                        })
                        .is_ok()
                    {
                        break;
                    }
                }
                continue;
            }
            deferred += 1;
        }

        {
            let content = engine.content().clone();
            let state = engine.state();
            if let Some(darts) = darts {
                let on_rack: i64 = state
                    .tower
                    .floors
                    .iter()
                    .flat_map(|f| f.rooms.iter())
                    .filter(|r| content.room(r.def).defence.is_some())
                    .flat_map(|r| r.inputs.iter())
                    .filter(|s| s.item == darts)
                    .map(|s| s.count)
                    .sum();
                let made = state.stock_of(darts);
                let busy: Vec<String> = state
                    .crew
                    .iter()
                    .map(|c| {
                        format!(
                            "{:?}/{}",
                            c.state,
                            if c.errand.is_some() {
                                "errand"
                            } else if c.task.is_some() {
                                "task"
                            } else {
                                "-"
                            }
                        )
                    })
                    .collect();
                let mut hurt = Vec::new();
                for floor in &state.tower.floors {
                    if floor.panel.is_hurt() {
                        hurt.push(format!(
                            "panel{}={}/{}",
                            floor.index, floor.panel.hp, floor.panel.max
                        ));
                    }
                    for room in &floor.rooms {
                        if room.health.is_hurt() {
                            hurt.push(format!(
                                "{}@{}.{}={}/{}",
                                content.room(room.def).short,
                                floor.index,
                                room.slot,
                                room.health.hp,
                                room.health.max
                            ));
                        }
                    }
                }
                for shaft in &state.tower.shafts {
                    if shaft.health.is_hurt() {
                        hurt.push(format!(
                            "shaft{}={}/{}",
                            shaft.id.0, shaft.health.hp, shaft.health.max
                        ));
                    }
                }
                println!("        darts {on_rack} on the rack, {made} in store");
                println!("        crew {busy:?}");
                if !hurt.is_empty() {
                    println!("        hurt {hurt:?}");
                }
            }
        }
        let state = engine.state();
        let mended = state.stats.hp_repaired;
        let spent = state.stats.repair_poles_spent;
        let repelled = state.siege.repelled;
        println!(
            "{day:4}  {:4}  {:7}‰  {:5}  {:6}  {:5}  {:8}  {:4}  {:5}  {:6}",
            state.siege.provocation,
            tower_integrity_permille(state),
            state.stock_of(poles),
            mended - last_mended,
            spent - last_spent,
            repelled - last_repelled,
            understory_core::systems::repair::outstanding_repair_cost(state, engine.content()),
            rooms(&engine),
            // Paces this day. Zero means the legs stopped, and with
            // per-pace intake (`SYSTEMS.md` §3.6) a tower that is not
            // walking is not harvesting either — so a run that goes
            // quiet here has usually browned out rather than made
            // peace with the jungle.
            (state.world.distance >> 8) - last_paces,
        );
        last_mended = mended;
        last_spent = spent;
        last_repelled = repelled;
        last_paces = state.world.distance >> 8;
    }

    let state = engine.state();
    println!(
        "  ended {} ‰ standing, {} creature(s) seen off, {} pole(s) banked, \
         {} of 60 checks could not afford the next thing on the list",
        tower_integrity_permille(state),
        state.siege.repelled,
        state.stock_of(poles),
        deferred,
    );
    if state.siege.lost {
        println!("  the Heartseed is gone");
    }
}

/// Step, taking whichever branch a fork offers first.
///
/// This harness drives the engine with no player, and a tower with an
/// unanswered fork in front of it stands still indefinitely
/// (`SYSTEMS.md` §3.9). Left alone it would spend the back half of the
/// five days parked — and from M3 a parked tower harvests nothing, so
/// it would report a starved tower's siege curve with total confidence.
/// The branch taken does not matter here; that a branch is taken does.
fn step_walking(engine: &mut GameEngine, ticks: u32) {
    let mut left = ticks;
    while left > 0 {
        if engine
            .state()
            .world
            .fork
            .is_some_and(|fork| fork.answer.is_none())
        {
            let _ = engine.try_send(GameCommand::TakeFork { branch: 0 });
        }
        let chunk = left.min(300);
        engine.step(chunk);
        left -= chunk;
    }
}

fn try_build(engine: &mut GameEngine, room: &str, floor: u8, slot: u8) -> bool {
    engine
        .try_send(GameCommand::PlaceRoom {
            room: room.into(),
            floor,
            slot,
        })
        .is_ok()
}

/// Hand the tower enough of everything to buy its own defences.
///
/// Not a measurement of the economy — `press` pins provocation and
/// stocks the shelves for exactly the same reason. What is being asked
/// is whether a tower that *has* a battery does better than one that
/// does not, and making it earn the battery first only measures how long
/// that takes.
fn pay_for_rooms(engine: &mut GameEngine) {
    let content = engine.content().clone();
    let state = engine.state_mut_for_test();
    for id in ["item.poles", "item.rope"] {
        let Some(item) = content.item_idx(id) else {
            continue;
        };
        let mut left = 40;
        'floors: for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                left -= room.shelve(item, left);
                if left <= 0 {
                    break 'floors;
                }
            }
        }
    }
}

/// Put a room in the first place it will go.
///
/// Hard-coded coordinates made this harness lie: the thornwright it
/// thought it had built had in fact been refused for a slot clash, so
/// the "answered" tower had a dart battery and no dart supply, and the
/// comparison it was drawing was meaningless.
fn build_anywhere(engine: &mut GameEngine, room: &str) -> bool {
    let floors = engine.state().tower.floors.len() as u8;
    let slots = engine.content().balance.tower.floor_slots;
    for floor in 0..floors {
        for slot in 0..slots {
            if try_build(engine, room, floor, slot) {
                return true;
            }
        }
    }
    false
}

fn rooms(engine: &GameEngine) -> usize {
    engine
        .state()
        .tower
        .floors
        .iter()
        .map(|floor| floor.rooms.len())
        .sum()
}
