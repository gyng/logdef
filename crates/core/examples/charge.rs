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
    for (label, floors, walking, reroof) in [
        ("starting, parked", 0u8, false, false),
        ("starting, striding", 0, true, false),
        ("+4 floors, parked", 4, false, false),
        ("+4 floors, striding", 4, true, false),
        // The same towers, re-roofed. Everything above grows without
        // replacing the sail deck it shaded, which measures
        // `top_floor_only` rather than height.
        ("re-roofed, parked", 0, false, true),
        ("+4, re-roofed, parked", 4, false, true),
        ("+10, re-roofed, parked", 10, false, true),
        // Striding, so the bank is actually being drained and income is
        // income rather than a mirror of draw. A parked tower with a
        // full bank "earns" exactly what it spends, which is why the
        // parked rows above cannot tell a bigger roof from a smaller one.
        ("+4, re-roofed, striding", 4, true, true),
        ("+10, re-roofed, striding", 10, true, true),
    ] {
        let day = if reroof {
            measure_reroofed(floors, walking)
        } else {
            measure(floors, walking)
        };
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
            their draw figures measure poverty rather than lighting.\n\
         \n\
         4. **Re-roof and the wall is gone.** The same tower with a fresh sail deck on\n\
            its new top floor strides at 0% brown-out ten floors up. So the wall was\n\
            never height — it was growing without replacing what you shaded, which is a\n\
            decision, and a far better one than a ceiling.\n\
         \n\
         5. **A parked tower cannot answer this**, and the parked rows are here to show\n\
            why: with a full bank, income is exactly draw, so a bigger roof and a\n\
            smaller one report the same figure. `canopy_climb_pct_per_floor` is\n\
            invisible on them at any value. It shows on the striding rows — ten floors\n\
            up, 4,542 income and 5% brown-out at 0, against 4,836 and 0% at 5."
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
    measure_tower(extra_floors, walking, false)
}

/// The same, with a fresh sail deck bought for the new roof.
///
/// **The row that tells `canopy_climb_pct_per_floor` from nothing.** The
/// note below records a four-floors-taller tower earning *nothing at
/// all* — but that tower had no sails on its new roof, so it measures
/// `top_floor_only` doing its job and cannot say anything about whether
/// a higher roof sees more sky. Re-roofing is what a player would do,
/// and it is the only shape where the height term is worth a point.
fn measure_reroofed(extra_floors: u8, walking: bool) -> Day {
    measure_tower(extra_floors, walking, true)
}

fn measure_tower(extra_floors: u8, walking: bool, reroof: bool) -> Day {
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
    if reroof {
        // Bamboo for the canvas, then the deck itself, on whatever slot
        // of the new top floor will take it. Asserted, because a sail
        // that silently failed to go up would make this row identical to
        // the one above it and read as "height buys nothing".
        let bamboo = game
            .content()
            .item_idx("item.bamboo")
            .expect("the pack defines bamboo");
        {
            let state = game.state_mut_for_test();
            let mut left = 40;
            'floors: for floor in &mut state.tower.floors {
                for room in &mut floor.rooms {
                    left -= room.shelve(bamboo, left);
                    if left <= 0 {
                        break 'floors;
                    }
                }
            }
        }
        let top = game.state().tower.floors.len() as u8 - 1;
        let slots = game.content().balance.tower.floor_slots;
        let up = (0..slots).any(|slot| {
            game.try_send(GameCommand::PlaceRoom {
                room: "room.canopy_sails".into(),
                floor: top,
                slot,
            })
            .is_ok()
        });
        assert!(
            up,
            "no sail deck went onto floor {top}; this row measures nothing"
        );
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
