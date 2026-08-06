//! Is the shift rota a verb, or a setting the player is right to ignore?
//!
//! **Answer: it is a verb, and every use of it costs the tower.**
//! Splitting six crew across both bands loses 28-44% of the poles the
//! shipped default mills, on five seeds at two bunk counts, and the
//! *mixed* rotas -- the ones a thoughtful player picks, wanting cover
//! around the clock -- are the worst option on the board. Putting
//! everybody on nights beats splitting.
//!
//! **The question came from a played run.** A dogfood tower on day 3
//! held no poles at all, could build nothing in a menu of nineteen
//! rooms, and had four rooms backed up on their outputs. All three crew
//! were on `Day`, which is the engine's default and which nobody had
//! ever changed, and at dawn all three were asleep.
//!
//! The arithmetic underneath is not subtle. A day is 14,400 ticks. The
//! `Day` band runs permille 180-760, which is **8,352 ticks**; `Night`
//! is the other **6,048**. So a tower on the shipped default has 42% of
//! every day during which nobody hauls anything -- the mill fills its
//! output buffer, stops, and the hours are gone.
//!
//! Two readings fit that and they want opposite fixes:
//!
//! 1. **Coverage.** Split the rota and the tower works around the
//!    clock.
//! 2. **Total hands.** A night worker is awake 6,048 ticks against a
//!    day worker's 8,352, so moving somebody to nights *removes* 27.5%
//!    of their working time.
//!
//! Reading 2 wins, and the reason is that hauling is the binding
//! constraint (`AGENTS.md` §I) -- which is precisely the thing moving
//! somebody to nights takes away. Coverage is real and visible in the
//! `by night` column, which goes 1 -> 9 -> 9 -> 35 across the splits:
//! the verb does exactly what it says. It just buys hours in the band
//! where the tower has fewest hands, at the price of hours in the band
//! where it has most.
//!
//! **Why the mixed rotas lose to an all-night one** is the part worth
//! keeping. `rested_max_ticks` is 8,640 and a night shift is 6,048, so
//! a night worker **never crosses `tired_ticks`** and never pays
//! `tired_work_pct`; a day worker crosses it about four-fifths through
//! and finishes at 60%. An all-night tower has the fewest hands and no
//! tiredness tax at all. A split tower pays the tax *and* has fewer
//! hands, which is how 4d/2n and 3d/3n come out under 0d/6n.
//!
//! **Poles milled is the column**, counted off the mill's own output as
//! it rises -- the trick `glut.rs` had to invent after widenings turned
//! out to cap at three purchases. It has no ceiling and no policy in
//! it. Hauls and crafts are carried beside it as the uncorrelated
//! check, and they track it: 225 -> 195 -> 188 -> 191.
//!
//! Whole days only, six of them. A window that is not a whole number of
//! days is a measurement of what time it started (`AGENTS.md` §I), and
//! that rule has never mattered more than it does here, where the
//! subject *is* the time of day.
//!
//! **Three harness traps on the way, all three already written down in
//! this repo**, which is the argument for reading §II before building
//! an instrument rather than after:
//!
//! - *The tower parked at the first fork* and stood there five and a
//!   half days. The tell was `paces` reading **exactly 37,438 in all
//!   eight runs** -- identical across four rotas and two bunk counts.
//!   `watch.rs` recorded this one by name.
//! - *The mill was starved 98% of the run*, so every split reported the
//!   same flat number. No bamboo meant no burn, no burn meant no
//!   charge, and the tower crawled 3,700 paces where a walking one does
//!   12,754 in region 1 alone. Fixed with a standing top-up, asserted,
//!   because `harness::give` drops overflow silently.
//! - *One seed said 46, 46, 34, 45*, a 3d/3n row worse than putting
//!   everybody on nights. Non-monotone output from a monotone input is
//!   noise wearing a finding's clothes; five seeds and a printed range
//!   made the real order legible.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::content::Shift;
use understory_core::harness;
use understory_core::ids::ItemIdx;

/// A day, from the pack. Asserted against the content in `main` rather
/// than trusted, because the whole design of this instrument rests on
/// the window being a whole number of these.
const TICKS_PER_DAY: u32 = 14_400;

/// How many of the six aboard work nights, in the four splits worth
/// asking about: the shipped default, a third, half, and the inverse of
/// the default.
const SPLITS: [usize; 4] = [0, 2, 3, 6];

/// **Five seeds, because one was not enough and said so.** The single-
/// seed version reported 46, 46, 34, 45 poles across the four splits —
/// a 3d/3n row worse than putting *everybody* on nights, which is not a
/// shape any mechanism in the game can produce. Non-monotone output
/// from a monotone input is noise wearing a finding's clothes.
const SEEDS: [u64; 5] = [4242, 7, 101, 2718, 31337];

struct Run {
    nights: usize,
    /// Extra bunks built on top of the one the tower starts with.
    extra: usize,
    /// How many people the tower can actually put in a bed at once,
    /// counted off the rooms rather than assumed — a bunk is two slots
    /// and sleeps two, and six of them do not fit above the ground
    /// floor of a four-floor tower.
    sleepers: usize,
    /// Every pole the mill ever finished, averaged over `SEEDS`. The
    /// headline.
    poles_milled: i64,
    /// The lowest and highest single seed behind that mean. **A mean
    /// whose spread straddles its neighbour's mean is not a finding**,
    /// and printing the range is the cheapest way to stop somebody
    /// (including me) reading a rank order into noise.
    poles_low: i64,
    poles_high: i64,
    /// The same, split by which shift band the tick belonged to. **This
    /// is the coverage question asked directly**: a tower that mills
    /// nothing between dusk and morning has 42% of its clock switched
    /// off, and no summary over a whole day can show you that.
    poles_day: i64,
    poles_night: i64,
    hauls: u64,
    crafts: u64,
    meals: u64,
    /// Ticks the mill spent unable to run — starved *or* backed up.
    /// `RoomView.stalled` cannot tell those apart, so this is only
    /// readable next to the bamboo column.
    mill_stalled: u32,
    /// Bamboo cut out of the ground, on top of the standing top-up.
    /// Kept because it is the health check on the tower's *own* income:
    /// a run where this collapses is a run where charge collapsed, and
    /// that is a different finding from anything about the rota.
    harvested: u64,
    /// Bamboo put up the burner's chimney. The other claim on the same
    /// material, and the only counter that reports what charge cost —
    /// `charge` is a level, not a bill.
    burned: u64,
    paces: i64,
    /// Ticks the tower spent dark. A night rota that never turns a lamp
    /// on is working at `dark_work_pct`, and this is how you would know.
    dark: u32,
    brownout: u32,
    /// **Why the burner was not burning**, split into the only two
    /// reasons `run_burners` has. `progress` only advances on a tick
    /// where *both* gates are open and `burn_ticks` is 100, so a burner
    /// that clears one gate 99% of the time and the other 5% makes
    /// almost no charge — and the two want completely opposite fixes.
    ///
    /// `no_headroom` is the bank being too full to take a whole burn.
    /// `no_fuel` is an empty inbox, which is a hauling failure and
    /// therefore the rota's business.
    no_headroom: u32,
    no_fuel: u32,
    capacity: i64,
}

fn main() {
    let days: u32 = std::env::var("UNDERSTORY_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6);
    let ticks = TICKS_PER_DAY * days;

    println!("Does splitting the shift rota pay?");
    println!(
        "  Six crew, {days} whole days ({ticks} ticks, {:.0} minutes at 1x), {} seeds averaged.",
        f64::from(ticks) / 30.0 / 60.0,
        SEEDS.len()
    );
    println!("  A day shift is 8,352 ticks and a night shift 6,048, so a tower on the default has");
    println!("  6,048 ticks a day — 42% — during which nobody hauls anything at all.\n");

    // Bunks are the obvious confound: half the argument for splitting a
    // rota anywhere is that two shifts can share one bed. Swept rather
    // than fixed, so it cannot quietly be the thing being measured.
    let mut runs = Vec::new();
    for extra in [0usize, 2] {
        for nights in SPLITS {
            let each: Vec<Run> = SEEDS
                .iter()
                .map(|&seed| measure(nights, extra, ticks, seed))
                .collect();
            runs.push(mean(&each));
        }
    }

    println!(
        "  {:<8} {:>8} {:>6} {:>8} {:>7} {:>8} {:>6} {:>6} {:>6} {:>5} {:>6} {:>7} {:>6} {:>5} {:>9}",
        "rota",
        "sleepers",
        "poles",
        "range",
        "by day",
        "by night",
        "hauls",
        "crafts",
        "meals",
        "cut",
        "burned",
        "paces",
        "stall",
        "dark",
        "brownout"
    );
    for run in &runs {
        println!(
            "  {:<8} {:>8} {:>6} {:>8} {:>7} {:>8} {:>6} {:>6} {:>6} {:>5} {:>6} {:>7} {:>5}% {:>4}% {:>8}%",
            format!("{}d/{}n", 6 - run.nights, run.nights),
            run.sleepers,
            run.poles_milled,
            format!("{}-{}", run.poles_low, run.poles_high),
            run.poles_day,
            run.poles_night,
            run.hauls,
            run.crafts,
            run.meals,
            run.harvested,
            run.burned,
            run.paces / 256,
            run.mill_stalled * 100 / ticks.max(1),
            run.dark * 100 / ticks.max(1),
            run.brownout * 100 / ticks.max(1),
        );
    }

    println!();
    // **The burner's own diagnosis, printed whether or not it is the
    // answer.** `AGENTS.md` §II rule 3: before believing that something
    // does nothing, check the run gave it something to do. The first
    // two versions of this instrument reported a flat rota effect off a
    // tower making three burns in six days.
    let one = &runs[0];
    println!(
        "  Burner: bank {} against a burn of 800. Blocked on headroom {}% of ticks, on fuel {}% —",
        one.capacity,
        one.no_headroom * 100 / ticks.max(1),
        one.no_fuel * 100 / ticks.max(1),
    );
    println!(
        "  and `burn_ticks` is 100, so progress needs a hundred ticks with BOTH gates open.
"
    );

    for extra in [0usize, 2] {
        let of = |nights: usize| -> &Run {
            runs.iter()
                .find(|r| r.nights == nights && r.extra == extra)
                .expect("every split was measured at every bunk count")
        };
        let base = of(0);
        let best = SPLITS
            .iter()
            .map(|&n| of(n))
            .max_by_key(|r| r.poles_milled)
            .expect("SPLITS is not empty");
        let delta = best.poles_milled - base.poles_milled;
        let pct = (delta * 100) as f64 / base.poles_milled.max(1) as f64;
        println!(
            "  {} sleepers: the default (6d/0n) mills {}, the best split ({}d/{}n) mills {} — {}{pct:.0}%.",
            base.sleepers,
            base.poles_milled,
            6 - best.nights,
            best.nights,
            best.poles_milled,
            if delta >= 0 { "+" } else { "" },
        );
    }
}

/// Average a config's seeds into one row, and carry the spread on the
/// headline column so a reader can see whether the mean means anything.
fn mean(each: &[Run]) -> Run {
    let n = each.len() as i64;
    let un = each.len() as u32;
    let first = &each[0];
    let poles: Vec<i64> = each.iter().map(|r| r.poles_milled).collect();
    Run {
        nights: first.nights,
        extra: first.extra,
        sleepers: first.sleepers,
        poles_milled: poles.iter().sum::<i64>() / n,
        poles_low: *poles.iter().min().unwrap_or(&0),
        poles_high: *poles.iter().max().unwrap_or(&0),
        poles_day: each.iter().map(|r| r.poles_day).sum::<i64>() / n,
        poles_night: each.iter().map(|r| r.poles_night).sum::<i64>() / n,
        hauls: each.iter().map(|r| r.hauls).sum::<u64>() / n as u64,
        crafts: each.iter().map(|r| r.crafts).sum::<u64>() / n as u64,
        meals: each.iter().map(|r| r.meals).sum::<u64>() / n as u64,
        mill_stalled: each.iter().map(|r| r.mill_stalled).sum::<u32>() / un,
        harvested: each.iter().map(|r| r.harvested).sum::<u64>() / n as u64,
        burned: each.iter().map(|r| r.burned).sum::<u64>() / n as u64,
        paces: each.iter().map(|r| r.paces).sum::<i64>() / n,
        dark: each.iter().map(|r| r.dark).sum::<u32>() / un,
        brownout: each.iter().map(|r| r.brownout).sum::<u32>() / un,
        no_headroom: each.iter().map(|r| r.no_headroom).sum::<u32>() / un,
        no_fuel: each.iter().map(|r| r.no_fuel).sum::<u32>() / un,
        capacity: first.capacity,
    }
}

fn measure(nights: usize, extra: usize, ticks: u32, seed: u64) -> Run {
    let mut game = GameEngine::new(seed);
    // Four floors, the same shape `glut.rs` measures on, so the two
    // instruments' pole figures can be read against each other.
    harness::chain_tower(&mut game, 4);

    // **A tower that starves is measuring hunger, not the rota.**
    // `hungry_ticks` is 4,800 and `starving_ticks` 7,200, so six days
    // without a kitchen puts everybody at `hungry_work_pct` for most of
    // the run and swamps the signal. The canteen is not the subject; it
    // is the thing that has to be there for the subject to be visible.
    build(&mut game, "room.canteen");
    for _ in 0..extra {
        build(&mut game, "room.bunk");
    }

    crew_of(&mut game, 6, nights);

    // Counted, not assumed. `min_floor: 1` and a two-slot footprint mean
    // the tower's beds are limited by the floors above the works, and a
    // column that said what the harness *asked for* rather than what it
    // got is how this instrument would come to compare two identical
    // towers.
    let sleepers: usize = {
        let content = game.content();
        game.state()
            .tower
            .floors
            .iter()
            .flat_map(|floor| &floor.rooms)
            .filter_map(|room| content.room(room.def).quarters.as_ref())
            .map(|q| q.sleepers as usize)
            .sum()
    };

    let content = game.content().clone();
    let mill = content.room_idx("room.mill").expect("the pack has a mill");
    let poles = content.item_idx("item.poles").expect("the pack has poles");
    let bamboo_idx = content
        .item_idx("item.bamboo")
        .expect("the pack has bamboo");

    let mut poles_milled = 0i64;
    let mut poles_day = 0i64;
    let mut poles_night = 0i64;
    let mut in_the_mill = 0i64;
    let mut mill_stalled = 0u32;
    let mut dark = 0u32;
    let mut brownout = 0u32;
    let mut no_headroom = 0u32;
    let mut no_fuel = 0u32;

    for _ in 0..ticks {
        game.step(1);

        // **A standing bamboo supply, because the question is about the
        // clock and not about the ground.** The first version of this
        // instrument measured a tower that cut 54 bamboo in six days and
        // milled twelve poles: no bamboo meant no burn, no burn meant no
        // charge, and a tower on the Heartseed trickle alone crawled
        // 3,700 paces where a walking one does 12,754 in region 1 alone.
        // It was a charge death-crawl with the rota on top, and every
        // split came out the same because the mill was starved 98% of
        // the run — `AGENTS.md` §II rule 3, "check the run gave the
        // mechanism something to do", in its purest form.
        //
        // Topped up rather than granted once: a lump sum is spent and
        // the tower is back where it started by day two.
        top_up(&mut game, bamboo_idx, 24);

        // **Answer every fork, or the tower parks at the first one.**
        // `paces` came out at exactly 3,700 in all eight runs of the
        // previous version — identical across four rotas and two bunk
        // counts, which is the tell that it is not a measurement of
        // anything the sweep varied. The tower had walked to the first
        // split and stood there for five and a half days, harvesting
        // nothing and spending nothing, and the burner's "blocked on
        // headroom 98%" was a fact about a parked tower rather than
        // about the game. `watch.rs` recorded this exact trap and this
        // instrument walked into it anyway.
        let _ = game.try_send(GameCommand::TakeFork { branch: 0 });

        let now: i64 = game
            .state()
            .tower
            .floors
            .iter()
            .flat_map(|floor| &floor.rooms)
            .filter(|room| room.def == mill)
            .flat_map(|room| &room.outputs)
            .filter(|stack| stack.item == poles)
            .map(|stack| stack.count)
            .sum();
        if now > in_the_mill {
            let made = now - in_the_mill;
            poles_milled += made;
            // Which band the tick belonged to, read off the engine's own
            // definition rather than off the tick number — `shift_now`
            // is what decides who is awake, so it is what has to decide
            // which column a pole lands in.
            if understory_core::systems::needs::shift_now(game.state(), &content) == Shift::Day {
                poles_day += made;
            } else {
                poles_night += made;
            }
        }
        in_the_mill = now;

        if !game.state().power.lit {
            dark += 1;
        }
        if game.state().power.brownout {
            brownout += 1;
        }

        // Read after the tick, against the same two conditions
        // `run_burners` tests, in the same order.
        {
            let state = game.state();
            let headroom = state.power.capacity - state.power.charge;
            for floor in &state.tower.floors {
                for room in &floor.rooms {
                    let Some(def) = content.room(room.def).burner.as_ref() else {
                        continue;
                    };
                    if def.charge_per_burn > headroom {
                        no_headroom += 1;
                    } else if room
                        .inputs
                        .first()
                        .is_none_or(|f| f.count < def.fuel_per_burn)
                    {
                        no_fuel += 1;
                    }
                }
            }
        }
        let view = game.view();
        for floor in &view.tower.floors {
            for room in &floor.rooms {
                if room.stalled && room.def == mill.0 {
                    mill_stalled += 1;
                }
            }
        }
    }

    let paces = game.state().world.distance;
    let capacity = game.state().power.capacity;
    let stats = &game.state().stats;
    Run {
        nights,
        extra,
        sleepers,
        poles_milled,
        poles_low: poles_milled,
        poles_high: poles_milled,
        poles_day,
        poles_night,
        hauls: stats.hauls_completed,
        crafts: stats.crafts_completed,
        meals: stats.meals_eaten,
        mill_stalled,
        harvested: stats
            .harvested_by_item
            .get(bamboo_idx.get())
            .copied()
            .unwrap_or(0),
        burned: stats.fuel_burned,
        paces,
        dark,
        brownout,
        no_headroom,
        no_fuel,
        capacity,
    }
}

/// Keep at least `want` of an item on the tower's shelves.
///
/// **Asserted, because `harness::give` silently drops overflow.**
/// `watch.rs` handed a tower eight items of forty each into one
/// storeroom, the late ones landed nowhere, and a dart battery was
/// refused for rope it had never actually been given. A top-up that
/// tops nothing up is the same bug with a slower fuse: it would leave
/// the mill starved again and the instrument would report the same
/// flat zeros it did the first time.
fn top_up(game: &mut GameEngine, idx: ItemIdx, want: i64) -> i64 {
    let held = held_of(game, idx);
    if held >= want {
        return held;
    }
    let id = game.content().item(idx).id.clone();
    harness::give(game, &id, want - held);
    let now = held_of(game, idx);
    assert!(
        now > held,
        "the tower would take none of the {id} it was offered — every shelf is full, \
         so this instrument is measuring a shelf jam and not a rota"
    );
    now
}

fn held_of(game: &GameEngine, idx: ItemIdx) -> i64 {
    game.state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| &floor.rooms)
        .flat_map(|room| &room.shelves)
        .filter(|shelf| shelf.item == Some(idx))
        .map(|shelf| shelf.count)
        .sum()
}

/// Pay for a room out of thin air and put it somewhere it fits.
///
/// Asserted, because a silent `false` from a place-a-room helper is
/// what made `siege_run.rs` compare a tower against a byte-identical
/// copy of itself, twice (`AGENTS.md` §II).
fn build(game: &mut GameEngine, room: &str) {
    let cost: Vec<(String, i64)> = {
        let content = game.content();
        let idx = content
            .room_idx(room)
            .unwrap_or_else(|| panic!("the pack should define {room}"));
        content
            .room_rt(idx)
            .build_cost
            .iter()
            .map(|(item, n)| (content.item(*item).id.clone(), *n))
            .collect()
    };
    for (item, n) in cost {
        harness::give(game, &item, n * 2);
    }
    assert!(
        harness::place_anywhere(game, room),
        "could not place {room}: this harness has no tower to measure"
    );
}

/// Put six people aboard and post `nights` of them on the night rota.
///
/// The shift goes through `GameCommand::SetShift` rather than the
/// field, so what is measured is the verb a player actually has. A
/// harness that wrote `member.shift` directly would be measuring a
/// state the game might not let anybody reach.
fn crew_of(game: &mut GameEngine, crew: usize, nights: usize) {
    {
        let content = game.content().clone();
        let state = game.state_mut_for_test();
        while state.crew.len() > crew {
            state.crew.pop();
        }
        while state.crew.len() < crew {
            state.add_crew(&content);
        }
    }
    let ids: Vec<_> = game.state().crew.iter().map(|member| member.id).collect();
    for id in ids.into_iter().take(nights) {
        game.try_send(GameCommand::SetShift {
            crew: id,
            shift: Shift::Night,
        })
        .unwrap_or_else(|err| panic!("a crew member should be re-shiftable: {err}"));
    }
}
