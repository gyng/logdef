//! Answer the milestone's design question with numbers.
//!
//! ```text
//! cargo run --release -p understory-core --example throughput
//! ```
//!
//! Runs the same tower with and without an added shaft and prints what
//! changed. "Is elevator contention fun?" is a question you answer by
//! playing, but "does relieving the bottleneck actually do anything?"
//! is a question you answer by measuring — and if the answer is no, no
//! amount of playing will make it fun.
//!
//! **This instrument and `lift.rs` disagree about the elevator, and the
//! disagreement is not resolved.** Here a four-floor tower hauls 121
//! without a shaft and 228 with one — **+88%**, with non-overlapping
//! seed ranges (114-129 against 201-278), so it is not noise. `lift.rs`
//! sweeping height puts a five-floor tower at **+11%** (§6.34). Both are
//! eight-seed means of the same verb.
//!
//! They are not measuring the same tower: this one buys a canteen and a
//! bunk and runs 28,800 ticks after a warm-up; `lift.rs` holds a room
//! plan fixed and grows the hull. One of those differences accounts for
//! it and nobody has found out which. **Do not quote either figure as
//! "what an elevator is worth" until somebody does** — quote the tower
//! it was measured on.

use understory_core::GameEngine;
use understory_core::command::GameCommand;

/// **Both are whole days, and that is not a detail.**
///
/// M4 gave the crew a rota, so a window that is not a whole number of
/// days measures what time of day it started at. The old figures — a
/// 6,000-tick warm-up and an 18,000-tick window — put two thirds of the
/// measurement across the night, when everybody on the default all-Day
/// shift is asleep, and a tower whose crew are in bed does not queue on
/// its staircase. The elevator had nothing to relieve and the harness
/// reported it as worthless. The effect had not moved; the instrument
/// had stopped pointing at it.
const DAY: u32 = 14_400;
const WARMUP: u32 = DAY;
const WINDOW: u32 = DAY * 2;

struct Sample {
    hauls: u64,
    crafts: u64,
    harvested: u64,
    /// Ticks on which at least one crew member was queued at a shaft.
    queued_ticks: u32,
    peak_wait: u32,
    brownout_ticks: u32,
    /// What the kitchen chain actually delivered. A tower that ate
    /// nothing is a tower whose numbers are about hunger, not shafts.
    meals: u64,
    /// What sleep costs, in the only unit that makes it comparable
    /// between two towers.
    asleep: u64,
    /// The lowest and highest `hauls` across the sweep. The headline
    /// column, and the one whose spread decides whether a delta quoted
    /// off it means anything.
    hauls_low: u64,
    hauls_high: u64,
}

/// Every seed this averages over.
///
/// **It ran on one until 2026-08-07.** The elevator's haul delta came
/// out at +137, +122 and +83 on three different seeds — the conclusion
/// survives at every one of them, the *magnitude* does not, and seven
/// `BALANCE.md` rows quote the magnitude.
const SEEDS: [u64; 8] = [0xC0FFEE, 1, 2, 3, 4, 5, 6, 7];

/// The mean of `SEEDS`, so a row is a figure rather than an anecdote.
fn measure(label: &str, build_shaft: bool) -> Sample {
    let each: Vec<Sample> = SEEDS
        .iter()
        .map(|&seed| measure_seed(seed, label, build_shaft))
        .collect();
    let n = each.len() as u64;
    Sample {
        hauls: each.iter().map(|s| s.hauls).sum::<u64>() / n,
        crafts: each.iter().map(|s| s.crafts).sum::<u64>() / n,
        harvested: each.iter().map(|s| s.harvested).sum::<u64>() / n,
        queued_ticks: (each.iter().map(|s| u64::from(s.queued_ticks)).sum::<u64>() / n) as u32,
        peak_wait: (each.iter().map(|s| u64::from(s.peak_wait)).sum::<u64>() / n) as u32,
        brownout_ticks: (each
            .iter()
            .map(|s| u64::from(s.brownout_ticks))
            .sum::<u64>()
            / n) as u32,
        meals: each.iter().map(|s| s.meals).sum::<u64>() / n,
        asleep: each.iter().map(|s| s.asleep).sum::<u64>() / n,
        hauls_low: each.iter().map(|s| s.hauls).min().unwrap_or(0),
        hauls_high: each.iter().map(|s| s.hauls).max().unwrap_or(0),
    }
}

fn measure_seed(seed: u64, label: &str, build_shaft: bool) -> Sample {
    let mut game = GameEngine::new(seed);
    // **The opening ladder first** (`SYSTEMS.md` §6.11). M6 cut the
    // starting tower to a Heartseed and a bed, so the chain this
    // instrument measures is one it now has to build.
    understory_core::harness::chain_tower(&mut game, 4);

    // **A canteen and a bunk before anything is measured.** Without
    // them this harness measures a starving, exhausted tower and
    // reports it as an economy — crew on the slow multiplier queue
    // differently from crew who are merely busy, and the shaft question
    // is about the second thing. Bought when affordable rather than at
    // a fixed tick, so a balance change does not turn a measurement
    // into a panic.
    make_it_a_home(&mut game);
    step_walking(&mut game, WARMUP);

    // Both towers are given the elevator's price; only one spends it.
    endow(&mut game, "shaft.elevator");
    if build_shaft {
        game.try_send(GameCommand::BuildShaft {
            shaft: "shaft.elevator".into(),
            low: 0,
            high: 3,
            slot: 7,
        })
        .unwrap_or_else(|err| panic!("{label}: could not raise the elevator: {err}"));
    }

    let start = (
        game.state().stats.hauls_completed,
        game.state().stats.crafts_completed,
        game.state().stats.items_harvested,
        game.state().stats.meals_eaten,
        game.state().stats.crew_ticks_asleep,
    );

    let mut queued_ticks = 0;
    let mut peak_wait = 0;
    let mut brownout_ticks = 0;
    // **Dispatch, measured rather than derived.** The Transport rows
    // size a car's behaviour — `dwell_base_ticks`, `dispatch_threshold`,
    // `elevator_base_wait_ticks` — and none of them had ever been
    // watched running. What is observable from outside is how long a
    // car spends stopped and how long a person spends waiting for one.
    let mut car_moving = 0u32;
    let mut car_stopped = 0u32;
    let mut boarding_waits: Vec<u32> = Vec::new();
    let mut was_boarding: Vec<u32> = Vec::new();
    for _ in 0..WINDOW {
        // Nobody is playing, so nobody answers the fork the route
        // eventually offers — and a tower with an unanswered fork in
        // front of it stands still (`SYSTEMS.md` §3.9). The window
        // stops a few hundred paces short of the first one today, which
        // is close enough that a band-length roll could make this
        // harness quietly measure a parked tower.
        answer_any_fork(&mut game);
        game.step(1);
        let state = game.state();
        if state.crew.iter().any(|member| {
            matches!(
                member.state,
                understory_core::state::CrewState::Boarding { .. }
            )
        }) {
            queued_ticks += 1;
        }
        peak_wait = peak_wait.max(
            state
                .crew
                .iter()
                .map(|member| member.wait_ticks)
                .max()
                .unwrap_or(0),
        );
        if state.power.brownout {
            brownout_ticks += 1;
        }

        for shaft in state.tower.shafts.iter().filter(|s| !s.cars.is_empty()) {
            for car in &shaft.cars {
                match car.state {
                    understory_core::state::CarState::Moving => car_moving += 1,
                    understory_core::state::CarState::Dwelling { .. } => car_stopped += 1,
                    understory_core::state::CarState::Idle => {}
                }
            }
        }
        was_boarding.resize(state.crew.len(), 0);
        for (i, member) in state.crew.iter().enumerate() {
            if matches!(
                member.state,
                understory_core::state::CrewState::Boarding { .. }
            ) {
                was_boarding[i] += 1;
            } else if was_boarding[i] > 0 {
                boarding_waits.push(was_boarding[i]);
                was_boarding[i] = 0;
            }
        }
    }

    if !boarding_waits.is_empty() {
        boarding_waits.sort_unstable();
        let mean = boarding_waits.iter().sum::<u32>() / boarding_waits.len() as u32;
        let median = boarding_waits[boarding_waits.len() / 2];
        println!(
            "  [{label}] {} wait(s) at a shaft: {mean} mean, {median} median, {} worst               (`elevator_base_wait_ticks` 45, `queue_penalty_ticks` 60)",
            boarding_waits.len(),
            boarding_waits.last().copied().unwrap_or(0),
        );
    }
    // `checked_div` rather than a `> 0` guard and a plain divide:
    // clippy reads that pair as a hand-rolled version of this, and it is
    // right. A tower with no cars simply prints nothing.
    if let Some(moving) = (car_moving * 100).checked_div(car_moving + car_stopped) {
        println!(
            "  [{label}] of the ticks a car was busy: {moving}% moving, {}% dwelling  \
             (`dwell_base_ticks` 10 + `dwell_per_unit_ticks` 6 a unit)",
            100 - moving,
        );
    }

    let state = game.state();
    Sample {
        hauls: state.stats.hauls_completed - start.0,
        crafts: state.stats.crafts_completed - start.1,
        harvested: state.stats.items_harvested - start.2,
        meals: state.stats.meals_eaten - start.3,
        asleep: state.stats.crew_ticks_asleep - start.4,
        queued_ticks,
        peak_wait,
        brownout_ticks,
        // A single seed has no spread; `measure` fills these in.
        hauls_low: state.stats.hauls_completed - start.0,
        hauls_high: state.stats.hauls_completed - start.0,
    }
}

/// Buy the rooms that stop this being a measurement of neglect.
///
/// **Wherever they fit, rather than at named slots.** This used to pin
/// the canteen to floor 1 slot 4 and the bunk to floor 3 slot 1 — the
/// two-wide gaps the old pre-built starting tower left. M6 cut that
/// tower down to a Heartseed and a bed (`SYSTEMS.md` §6.11), so the
/// gaps are wherever the harness's own ladder did not land.
fn make_it_a_home(game: &mut GameEngine) {
    for room in ["room.canteen", "room.bunk"] {
        let cost: Vec<(String, i64)> = {
            let content = game.content();
            let idx = content.room_idx(room).expect("the pack defines it");
            content
                .room_rt(idx)
                .build_cost
                .iter()
                .map(|(item, n)| (content.item(*item).id.clone(), *n))
                .collect()
        };
        for (item, n) in cost {
            understory_core::harness::give(game, &item, n * 2);
        }
        assert!(
            understory_core::harness::place_anywhere(game, room),
            "could not place {room}: this is a measurement of a tower that was never built"
        );
    }
    // **And somewhere to save, which is a separate thing from somewhere
    // to live.**
    //
    // The starting tower has one storeroom. That was enough while an
    // elevator cost 12 poles; at M5's 18 it is not, and the failure is
    // not "the tower is poor" but "the tower has nowhere to put a pole".
    // Traced: harvest climbing steadily past 278 items, the cutter arm
    // untouched at 260 of 260, provocation at nothing, and the pole
    // count sitting at **zero** the whole time — every pole the mill
    // made was stuck in its own outbox because the shelves were full of
    // the bamboo waiting to become the next one. A tower that cannot
    // save cannot buy, however much it earns.
    //
    understory_core::harness::give(game, "item.poles", 12);
    assert!(
        understory_core::harness::place_anywhere(game, "room.storeroom"),
        "could not place a second storeroom: this measures a shelf jam rather than a shaft"
    );
}

/// Hand the tower a shaft's whole price, so the two samples differ by
/// the shaft and by nothing else.
///
/// **This exists because M5 gated the elevator on rope and this harness
/// stopped running**, panicking with "could not build the elevator: need
/// 18 item.poles, have 0". It stayed broken because nothing runs it on
/// the way past.
///
/// The obvious repair — wait until the tower can afford one — is wrong,
/// and measurably so. A bare tower needs about 75,000 ticks to save 18
/// poles, so the two samples then start at completely different points
/// in the run, on different ground, with different provocation and
/// different accumulated damage. Measured that way the elevator came out
/// **34 hauls and 50 harvest behind**, which is not a fact about
/// elevators; it is a fact about measuring one tower three in-game days
/// later than the other. The header comment above already records this
/// instrument being fooled once by a window that was not a whole number
/// of days, and this is the same mistake wearing a different hat.
///
/// So both towers are handed the price and only one spends it. The
/// question here is whether an elevator **earns its slot**, not whether
/// a tower can afford one — that question belongs to `journey.rs`, which
/// builds the chain that pays for it.
fn endow(game: &mut GameEngine, shaft: &str) {
    let costs: Vec<(understory_core::ids::ItemIdx, i64)> = game
        .content()
        .shaft_rt(
            game.content()
                .shaft_idx(shaft)
                .unwrap_or_else(|| panic!("the pack has no {shaft}")),
        )
        .build_cost
        .clone();
    let state = game.state_mut_for_test();
    for (item, amount) in costs {
        let mut left = amount;
        'floors: for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                left -= room.shelve(item, left);
                if left <= 0 {
                    break 'floors;
                }
            }
        }
        assert!(left <= 0, "nowhere to put {shaft}'s own price");
    }
}

/// Step, answering any fork before it can bring the tower to a halt.
fn step_walking(game: &mut GameEngine, ticks: u32) {
    let mut left = ticks;
    while left > 0 {
        answer_any_fork(game);
        let chunk = left.min(300);
        game.step(chunk);
        left -= chunk;
    }
}

/// Take the left-hand branch of whatever fork is pending, if any.
fn answer_any_fork(game: &mut GameEngine) {
    if game
        .state()
        .world
        .fork
        .is_some_and(|fork| fork.answer.is_none())
    {
        let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
    }
}

fn main() {
    // **A hardcoded day goes stale silently.** `journey.rs` reported a
    // 57,372-tick run as "3 days" when the pack said 7,200 and the
    // answer was eight, because it kept its own copy of the day length.
    // Every instrument here windows on whole days, so a stale copy makes
    // the window a measurement of what time it started at — the trap
    // `throughput.rs` documents at the top of itself. Fail loudly rather
    // than quietly measure a different game.
    assert_eq!(
        DAY,
        understory_core::content::Content::load_embedded()
            .expect("the shipped pack should load")
            .balance
            .clock
            .ticks_per_day,
        "the pack's day length has moved; update this file's day constant"
    );
    let stairs_only = measure("stairs only", false);
    let with_elevator = measure("with elevator", true);

    let seconds = WINDOW / 30;
    println!("{WINDOW} ticks ({seconds}s of simulation) after a {WARMUP}-tick warm-up\n");
    println!(
        "{:<18} {:>12} {:>12} {:>8}",
        "", "stairs only", "+ elevator", "delta"
    );
    row("hauls", stairs_only.hauls, with_elevator.hauls);
    // **The spread beside the headline.** Three seeds gave the elevator
    // +137, +122 and +83 hauls; the conclusion held every time and the
    // magnitude did not. A delta quoted off a mean whose inputs span
    // this much is a delta with a range attached whether it says so or
    // not, so it says so.
    println!(
        "{:<18} {:>12} {:>12}",
        "  (hauls range)",
        format!("{}-{}", stairs_only.hauls_low, stairs_only.hauls_high),
        format!("{}-{}", with_elevator.hauls_low, with_elevator.hauls_high),
    );
    row("crafts", stairs_only.crafts, with_elevator.crafts);
    row("harvested", stairs_only.harvested, with_elevator.harvested);
    row(
        "queued ticks",
        u64::from(stairs_only.queued_ticks),
        u64::from(with_elevator.queued_ticks),
    );
    row(
        "peak wait",
        u64::from(stairs_only.peak_wait),
        u64::from(with_elevator.peak_wait),
    );
    row(
        "brownout ticks",
        u64::from(stairs_only.brownout_ticks),
        u64::from(with_elevator.brownout_ticks),
    );
    row("meals eaten", stairs_only.meals, with_elevator.meals);
    row(
        "crew-ticks asleep",
        stairs_only.asleep,
        with_elevator.asleep,
    );
}

fn row(label: &str, before: u64, after: u64) {
    let delta = after as i64 - before as i64;
    let sign = if delta >= 0 { "+" } else { "" };
    println!("{label:<18} {before:>12} {after:>12} {sign:>4}{delta}");
}
