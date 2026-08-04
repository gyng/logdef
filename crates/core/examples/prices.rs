//! When can a tower first afford each thing it can build?
//!
//! ```text
//! cargo run --release -p understory-core --example prices
//! ```
//!
//! A `build_cost` row is a price, and a price makes exactly one testable
//! claim: **is it payable at the point in a run where the thing it buys
//! is wanted?** The rows argue that in relative terms — a rig is "priced
//! above a mill and level with a dumbwaiter", a cell bank is "the
//! priciest of the small Energy rooms", a burner is "same tier as the
//! mill, an easy fallback to stand up". Those are comparisons between
//! numbers in a table, not statements about a run.
//!
//! So this walks one tower, buys nothing, and records the first moment
//! each buildable becomes affordable — which is what a player actually
//! experiences a price as. A thing that arrives on day one is an opening
//! move; a thing that arrives on day six is a mid-run decision; a thing
//! that never arrives is gated on a chain rather than on a price, and
//! should be read as a chain.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;

const DAY: u32 = 14_400;
const DAYS: u32 = 8;

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
    println!("=== when can a tower first afford each thing? ===\n");
    println!(
        "  One tower, walking, buying nothing, for {DAYS} days. A price is only\n\
         testable as \"payable when the thing is wanted\", so this records the first\n\
         tick each buildable could have been paid for.\n\
         \n\
         The tower buys a kitchen and a bunk and nothing else, so what is measured is\n\
         a price against an *undiverted* income — the best case. Anything that never\n\
         becomes affordable here is gated on a chain, not on a price.\n"
    );

    let content = understory_core::content::Content::load_embedded().expect("pack");
    let mut game = GameEngine::new(0x0BE1_9CE5);
    game.set_speed(SimSpeed::X1);
    for (room, floor, slot) in [("room.canteen", 1u8, 4u8), ("room.bunk", 3, 1)] {
        for _ in 0..600 {
            match game.try_send(GameCommand::PlaceRoom {
                room: room.into(),
                floor,
                slot,
            }) {
                Ok(()) => break,
                Err(understory_core::command::CommandError::InsufficientStock { .. }) => {
                    step(&mut game, 300)
                }
                Err(other) => panic!("could not place {room}: {other}"),
            }
        }
    }

    let mut first: Vec<(String, String, Option<u32>)> = content
        .rooms
        .iter()
        .map(|r| ("room".to_string(), r.name.clone(), None))
        .chain(
            content
                .shafts
                .iter()
                .map(|s| ("shaft".to_string(), s.name.clone(), None)),
        )
        .collect();

    for tick in 0..DAY * DAYS {
        step(&mut game, 1);
        let state = game.state();
        let mut at = 0usize;
        for room in &content.room_runtime {
            if first[at].2.is_none() && affordable(state, &room.build_cost) {
                first[at].2 = Some(tick);
            }
            at += 1;
        }
        for shaft in &content.shaft_runtime {
            if first[at].2.is_none() && affordable(state, &shaft.build_cost) {
                first[at].2 = Some(tick);
            }
            at += 1;
        }
    }

    let mut rows: Vec<&(String, String, Option<u32>)> = first.iter().collect();
    rows.sort_by_key(|(_, name, at)| (at.unwrap_or(u32::MAX), name.clone()));
    println!(
        "{:<8} {:<18} {:>10} {:>8}",
        "kind", "thing", "first at", "day"
    );
    for (kind, name, at) in rows {
        match at {
            Some(t) => println!(
                "{kind:<8} {name:<18} {t:>10} {:>8.1}",
                f64::from(*t) / f64::from(DAY)
            ),
            None => println!("{kind:<8} {name:<18} {:>10} {:>8}", "never", "-"),
        }
    }

    println!(
        "\n  `never` is the important column. A price a tower cannot reach in eight days\n\
         of undiverted income is not a price, it is a **chain** — the thing is gated on\n\
         building whatever makes its inputs, and its row should say so rather than\n\
         comparing its number to a mill's."
    );
}

fn affordable(
    state: &understory_core::state::GameState,
    cost: &[(understory_core::ids::ItemIdx, i64)],
) -> bool {
    cost.iter()
        .all(|(item, amount)| state.stock_of(*item) >= *amount)
}

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
