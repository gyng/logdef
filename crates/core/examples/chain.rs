//! Where the starting chain actually spends its time, and what stalls.
//!
//! ```text
//! cargo run --release -p understory-core --example chain
//! ```
//!
//! The Content rows size the first chain in the game against each other:
//! `cutter arm paces_per_item` 78 is "130 ticks a stalk on neutral
//! ground", `mill craft_ticks` 120 is "deliberately close to the cutter
//! arm's rate and on the near side of it", the buffers are "24 s of
//! runway on each side" and `storeroom shelves × per_shelf` is "80 items
//! across four independent shelves".
//!
//! Those are rates in isolation. What decides whether a chain runs is
//! the *stall* — a room whose outbox is full does nothing, and
//! `AGENTS.md` makes that visible-not-silent on purpose. So this counts
//! the ticks each room spends unable to work, and what it was waiting
//! for.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;

const DAY: u32 = 14_400;
const DAYS: u32 = 4;

fn main() {
    println!("=== what stalls the starting chain? ===\n");
    println!(
        "  A fed, housed tower over {DAYS} whole days, first day discarded as warm-up.\n\
         A room with a full outbox or an empty inbox does nothing, and which of the\n\
         two it is says whether a chain is supply-limited or demand-limited.\n"
    );

    let content = understory_core::content::Content::load_embedded().expect("pack");
    let mut game = GameEngine::new(0x0C4A_19E5);
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
    step(&mut game, DAY);

    // Per room: ticks with a full outbox, ticks with an empty inbox.
    let mut blocked: Vec<(String, u32, u32, u32)> = Vec::new();
    for _ in 0..DAY * (DAYS - 1) {
        step(&mut game, 1);
        let state = game.state();
        let mut at = 0usize;
        for floor in &state.tower.floors {
            for room in &floor.rooms {
                let def = content.room(room.def);
                if room.outputs.is_empty() && room.inputs.is_empty() {
                    continue;
                }
                if blocked.len() <= at {
                    blocked.push((def.name.clone(), 0, 0, 0));
                }
                blocked[at].1 += 1;
                if room
                    .outputs
                    .iter()
                    .any(understory_core::state::Stack::is_full)
                {
                    blocked[at].2 += 1;
                }
                if !room.inputs.is_empty() && room.inputs.iter().all(|s| s.count == 0) {
                    blocked[at].3 += 1;
                }
                at += 1;
            }
        }
    }

    println!("{:<16} {:>10} {:>12}", "room", "outbox full", "inbox empty");
    for (name, ticks, full, empty) in &blocked {
        println!(
            "{name:<16} {:>9.1}% {:>11.1}%",
            f64::from(*full) * 100.0 / f64::from((*ticks).max(1)),
            f64::from(*empty) * 100.0 / f64::from((*ticks).max(1)),
        );
    }

    let state = game.state();
    let days = f64::from(DAYS - 1);
    println!(
        "\nharvested a day                  {:.0}",
        state.stats.items_harvested as f64 / days
    );
    println!(
        "crafted a day                    {:.0}",
        state.stats.crafts_completed as f64 / days
    );
    println!(
        "\n  **Outbox full is backpressure and inbox empty is starvation**, and a chain\n\
         wants a little of the first and none of the second. A room stalled on its own\n\
         output is telling the player the next link is the bottleneck — which is what\n\
         `AGENTS.md` means by a stall being visible rather than silently discarded. A\n\
         room stalled on an empty inbox is waiting for a haul, and that is the crew.\n\
         \n\
         Read them against `haulcycle.rs`, where the same tower's crew are idle 0.2%\n\
         of the day: if both stall figures are high while nobody is idle, the tower is\n\
         not short of rooms or of buffers, it is short of hands."
    );
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
