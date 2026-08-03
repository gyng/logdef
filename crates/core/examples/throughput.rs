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

const WARMUP: u32 = 6_000;
const WINDOW: u32 = 18_000;

struct Sample {
    hauls: u64,
    crafts: u64,
    harvested: u64,
    /// Ticks on which at least one crew member was queued at a shaft.
    queued_ticks: u32,
    peak_wait: u32,
    brownout_ticks: u32,
}

fn measure(label: &str, build_shaft: bool) -> Sample {
    let mut game = GameEngine::new(0xC0FFEE);
    game.step(WARMUP);

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
        queued_ticks,
        peak_wait,
        brownout_ticks,
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
}

fn row(label: &str, before: u64, after: u64) {
    let delta = after as i64 - before as i64;
    let sign = if delta >= 0 { "+" } else { "" };
    println!("{label:<18} {before:>12} {after:>12} {sign:>4}{delta}");
}
