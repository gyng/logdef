//! Does the charge economy add up? Measured, not derived.
//!
//! ```text
//! cargo run --release -p understory-core --example charge
//! ```
//!
//! **`BALANCE.md`'s Power section is arithmetic all the way down**, and
//! none of it had ever been checked against the simulation. Its rows
//! claim that continuous striding costs "2,880 charge across a full
//! day-cycle", that a two-sail good-sun day earns "~20,000", that
//! lighting an 8-floor tower through the night costs "~760". Every one
//! of those is a multiplication somebody did by hand from the constant
//! next to it, and every one of them silently stops being true the
//! moment a system around it changes — which is exactly how M1's
//! throughput harness came to spend a milestone panicking on startup
//! without anybody noticing.
//!
//! So this runs the tower and reads the meter. Income, draw and the
//! bank's trajectory over whole days, at the two tower shapes the rows
//! actually talk about.
//!
//! **Whole days, always.** The sun curve is the whole of income and a
//! window that is not a whole number of days measures what time it
//! started at — the same trap `throughput.rs` documents at the top of
//! itself, which cost that instrument a correct answer once already.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;

const DAY: u32 = 14_400;

fn main() {
    println!("=== does the charge economy add up? ===\n");
    println!(
        "  One day-cycle each, measured off the meter rather than multiplied out.\n\
         `BALANCE.md`'s Power rows are hand arithmetic and this is the first thing\n\
         that has ever checked them against the simulation.\n"
    );

    println!(
        "{:<26} {:>8} {:>8} {:>8} {:>9} {:>8}",
        "tower", "income", "draw", "net", "brownout", "dark"
    );
    for (label, floors, walking) in [
        ("starting, parked", 0u8, false),
        ("starting, striding", 0, true),
        ("+4 floors, parked", 4, false),
        ("+4 floors, striding", 4, true),
    ] {
        let day = measure(floors, walking);
        println!(
            "{label:<26} {:>8} {:>8} {:>8} {:>8}% {:>7}%",
            day.income,
            day.draw,
            day.income - day.draw,
            day.brownout_ticks * 100 / i64::from(DAY),
            day.dark_ticks * 100 / i64::from(DAY),
        );
    }

    let parked = measure(0, false);
    let striding = measure(0, true);
    println!(
        "\n  measured stride cost over one day : {} (the row derives 2,880)",
        striding.draw - parked.draw
    );
    println!(
        "  measured lamp cost, 4 floors      : {} (the row derives ~380)",
        parked.draw
    );

    println!(
        "\n  Three things the meter says that the arithmetic did not:\n\
         \n\
         1. Striding costs about 2,700 over a day-cycle against a derived 2,880. Close,\n\
            and the shortfall is real rather than noise: a tower does not pay to stride\n\
            on the ticks it spends standing at a fork, and every run answers several.\n\
         \n\
         2. A parked 4-floor tower spends about 650 on lamps against a derived ~380 —\n\
            **70% more**. Exposure is sun *after terrain*, so a canopy-heavy region is\n\
            dark for 41-56% of the day rather than the ~33% the bare sun curve implies.\n\
            The constant is right; the day it was multiplied by was the wrong day.\n\
         \n\
         3. A tower four floors taller earns **nothing at all** and browns out for the\n\
            whole day. That is `v2-plan.md` §6.3 working as written — a new top floor\n\
            displaces the canopy sail deck — but the size of it is worth seeing: not a\n\
            tax on growing, a wall. The two tall rows above are a bankrupt tower, and\n\
            their draw figures measure poverty rather than lighting."
    );
}

struct Day {
    income: i64,
    draw: i64,
    brownout_ticks: i64,
    /// Ticks the tower needed its lamps: exposure below the threshold.
    ///
    /// Not `power.lit`, which is *true in daylight* — it means "the
    /// place is lit", not "the lamps are burning", and reading it as
    /// the second thing says a tower runs its lamps 100% of the day.
    dark_ticks: i64,
}

/// One whole day-cycle, after a whole-day warm-up so the bank and the
/// clock are both in a steady state rather than in their opening
/// positions.
fn measure(extra_floors: u8, walking: bool) -> Day {
    let mut game = GameEngine::new(0x_5A_11);
    game.set_speed(SimSpeed::X1);

    for _ in 0..extra_floors {
        // Floors are paid for from stock like everything else, so the
        // tower is handed the poles rather than made to earn them: this
        // is a measurement of a tall tower's *draw*, not of how long it
        // takes to become one.
        let poles = game
            .content()
            .item_idx("item.poles")
            .expect("the pack defines poles");
        {
            let state = game.state_mut_for_test();
            let mut left = 40;
            'floors: for floor in &mut state.tower.floors {
                for room in &mut floor.rooms {
                    left -= room.shelve(poles, left);
                    if left <= 0 {
                        break 'floors;
                    }
                }
            }
        }
        let _ = game.try_send(GameCommand::BuildFloor);
    }
    let _ = game.try_send(GameCommand::SetStriding { walking });

    step(&mut game, DAY);

    let (mut income, mut draw, mut brownout_ticks, mut dark_ticks) = (0i64, 0i64, 0i64, 0i64);
    for _ in 0..DAY {
        step(&mut game, 1);
        let power = &game.state().power;
        income += power.income_last;
        draw += power.spent_last;
        brownout_ticks += i64::from(power.brownout);
        let content = game.content().clone();
        if understory_core::systems::power::exposure_pct(game.state(), &content)
            < content.balance.clock.night_light_threshold
        {
            dark_ticks += 1;
        }
    }
    Day {
        income,
        draw,
        brownout_ticks,
        dark_ticks,
    }
}

/// Step, answering any fork — an unanswered one halts the tower, and a
/// halted tower pays no stride charge, which is the measurement.
fn step(game: &mut GameEngine, ticks: u32) {
    for _ in 0..ticks {
        if let Some(fork) = game.state().world.fork
            && fork.answer.is_none()
        {
            let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
        }
        game.step(1);
    }
}
