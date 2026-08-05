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
    println!("=== what stalls the starting chain? ===\n");
    println!(
        "  A fed, housed tower over {DAYS} whole days, first day discarded as warm-up.\n\
         A room with a full outbox or an empty inbox does nothing, and which of the\n\
         two it is says whether a chain is supply-limited or demand-limited.\n"
    );

    let content = understory_core::content::Content::load_embedded().expect("pack");
    let mut game = GameEngine::new(0x0C4A_19E5);
    game.set_speed(SimSpeed::X1);
    // **The opening ladder first** (`SYSTEMS.md` §6.11). M6 cut the
    // starting tower to a Heartseed and a bed, so a harness that places
    // a canteen on turn one gets `CommandError::Locked` rather than a
    // tower. `chain_tower` grows to four floors and walks farm → cutter
    // arm → burner → mill, storeroom, cell bank, which is the tower
    // every instrument here was written against.
    understory_core::harness::chain_tower(&mut game, 4);
    // **Wherever they fit, rather than at named slots.** These used to
    // pin the canteen to floor 1 slot 4 and the bunk to floor 3 slot 1,
    // which were free in the old pre-built starting tower and are not
    // free in a tower this harness has just grown for itself. A slot
    // number is not what any of these instruments is measuring.
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
            understory_core::harness::give(&mut game, &item, n * 2);
        }
        assert!(
            understory_core::harness::place_anywhere(&mut game, room),
            "could not place {room}: this harness is measuring a tower it failed to build"
        );
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
