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
}

fn measure(label: &str, build_shaft: bool) -> Sample {
    let mut game = GameEngine::new(0xC0FFEE);

    // **A canteen and a bunk before anything is measured.** Without
    // them this harness measures a starving, exhausted tower and
    // reports it as an economy — crew on the slow multiplier queue
    // differently from crew who are merely busy, and the shaft question
    // is about the second thing. Bought when affordable rather than at
    // a fixed tick, so a balance change does not turn a measurement
    // into a panic.
    make_it_a_home(&mut game);
    step_walking(&mut game, WARMUP);

    if build_shaft {
        game.try_send(GameCommand::BuildShaft {
            shaft: "shaft.elevator".into(),
            low: 0,
            high: 3,
            slot: 7,
        })
        .unwrap_or_else(|err| panic!("{label}: could not build the elevator: {err}"));
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
    }
}

/// Buy the two rooms that stop this being a measurement of neglect.
///
/// Floor 1 slot 4 and floor 3 slot 1 are the two-wide gaps the starting
/// layout leaves on floors that are not the one the elevator's column
/// will take.
fn make_it_a_home(game: &mut GameEngine) {
    place_when_affordable(game, "room.canteen", 1, 4);
    place_when_affordable(game, "room.bunk", 3, 1);
}

/// Place `room` as soon as the tower can pay for it. Waiting for the
/// money rather than hardcoding "by now there will be five poles" is
/// what keeps a harness measuring the same tower across a balance
/// change instead of panicking mid-script.
fn place_when_affordable(game: &mut GameEngine, room: &str, floor: u8, slot: u8) {
    for _ in 0..400 {
        match game.try_send(GameCommand::PlaceRoom {
            room: room.into(),
            floor,
            slot,
        }) {
            Ok(()) => return,
            Err(understory_core::command::CommandError::InsufficientStock { .. }) => {
                step_walking(game, 300);
            }
            Err(other) => panic!("could not place {room} at {floor}.{slot}: {other}"),
        }
    }
    panic!("{room} never became affordable");
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
    let stairs_only = measure("stairs only", false);
    let with_elevator = measure("with elevator", true);

    let seconds = WINDOW / 30;
    println!("{WINDOW} ticks ({seconds}s of simulation) after a {WARMUP}-tick warm-up\n");
    println!(
        "{:<18} {:>12} {:>12} {:>8}",
        "", "stairs only", "+ elevator", "delta"
    );
    row("hauls", stairs_only.hauls, with_elevator.hauls);
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
