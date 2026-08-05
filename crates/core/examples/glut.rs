//! Does the rope chain pay for itself, or does it eat the tower?
//!
//! **The question came from playing, not from an instrument.** A whole
//! run played through the agent tools arrived at the far edge with
//! `Meals 24, Rope 89` on the shelves, **no poles at all**, and every
//! single entry in the build menu marked "cannot pay". The tower had
//! forty-four times the rope an elevator wants and could not buy one.
//!
//! Two explanations fit that ending and they want opposite fixes:
//!
//! 1. **Competition at the source.** The comb and the ropery eat the
//!    same bamboo the mill and the burner do, so the rope chain starves
//!    poles and charge upstream.
//! 2. **Competition for the shelves.** Rope nothing consumes piles up,
//!    the storeroom fills, and the mill has nowhere to put a pole.
//!
//! **`AGENTS.md` warns about this exact shape** — "a shelf jam" is one of
//! the four confident wrong answers filed against §6.9 question 0, and
//! every one turned out to be a property of the harness — so this
//! measures rather than asserts. Explanation 2 is measurably **wrong**:
//! the shelves finish a third full and nothing is stuck in an output
//! buffer. The columns are kept so the next person can see that.
//!
//! **Two harness traps were walked into on the way, both of them ones
//! this repo has written down before.**
//!
//! *Widening is not an unbounded pole sink.* The hull runs 10 to 16 in
//! steps of two, so it is exactly three purchases and **both towers
//! reach the ceiling**. The first version reported "3 and 3, the rope
//! chain is free", which was a reading of `max_slots` — the same shape as
//! the plating comparison, caught only because the columns beside it
//! disagreed with it.
//!
//! *A stalled mill is not a backed-up mill.* `RoomView.stalled` means
//! "waiting on an input **or** backed up on an output", and reading it as
//! the second when it was the first is how "the rope glut jams the pole
//! supply" survived an hour. Both towers finish with **zero bamboo**: the
//! mill is starved, and the comb is a second mouth on the thing it is
//! starved of.
//!
//! So the tower still widens, to keep a real sink under both shapes, but
//! what is compared is **poles milled** — counted off the mill's own
//! output as it rises, which has no ceiling and no policy in it.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::harness;

/// The rooms both towers get on top of the opening ladder, which has
/// already stood up the garden, the cutter arm and the burner.
const CORE: [&str; 2] = ["room.mill", "room.storeroom"];
/// What the rope chain adds on top.
const ROPE: [&str; 2] = ["room.fiber_comb", "room.ropery"];

struct Run {
    label: &'static str,
    widenings: u32,
    /// Every pole the mill ever finished. The column with no ceiling in
    /// it, and the only one that answers the question.
    poles_milled: i64,
    poles: i64,
    rope: i64,
    bamboo: i64,
    shelves_used: usize,
    shelves_total: usize,
    mill_stalled_ticks: u32,
}

fn main() {
    let ticks: u32 = std::env::var("UNDERSTORY_TICKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30 * 60 * 20);

    println!("Does the rope chain pay for itself?");
    println!(
        "  {ticks} ticks ({:.0} minutes at 1x), both towers widening whenever they can afford it.\n",
        f64::from(ticks) / 30.0 / 60.0
    );

    let runs: Vec<Run> = [("chain only", &[][..]), ("chain + rope", &ROPE[..])]
        .into_iter()
        .map(|(label, extra)| measure(label, extra, ticks))
        .collect();

    println!(
        "  {:<14} {:>10} {:>13} {:>11} {:>7} {:>8} {:>9} {:>14}",
        "tower",
        "widenings",
        "poles milled",
        "poles held",
        "rope",
        "bamboo",
        "shelves",
        "mill stalled"
    );
    for run in &runs {
        println!(
            "  {:<14} {:>10} {:>13} {:>11} {:>7} {:>8} {:>9} {:>13}%",
            run.label,
            run.widenings,
            run.poles_milled,
            run.poles,
            run.rope,
            run.bamboo,
            format!("{}/{}", run.shelves_used, run.shelves_total),
            run.mill_stalled_ticks * 100 / ticks.max(1),
        );
    }

    let bare = &runs[0];
    let roped = &runs[1];
    let lost = bare.poles_milled - roped.poles_milled;
    let pct = (lost.abs() * 100) as f64 / bare.poles_milled.max(1) as f64;
    println!();
    println!(
        "  Poles milled: {} without the rope chain, {} with it — the rope chain {} {pct:.0}% of the tower's poles.",
        bare.poles_milled,
        roped.poles_milled,
        if lost > 0 { "costs" } else { "gains" },
    );
    println!(
        "  The mill sat stalled {}% of the run with the rope chain and {}% without, and both towers",
        roped.mill_stalled_ticks * 100 / ticks.max(1),
        bare.mill_stalled_ticks * 100 / ticks.max(1),
    );
    println!(
        "  ended with no bamboo at all — so the mill is *starved*, not backed up. Bamboo is the"
    );
    println!("  constraint under everything here, and the comb is a second mouth on it.");
    println!(
        "  (The played run's mill was genuinely BACKED UP, which is a different tower: three crew,"
    );
    println!(
        "  two floors and no lift, so poles were made and never carried. Both stalls are real and"
    );
    println!("  they want opposite fixes — `snapshot.rs` tells them apart and so should you.)");
    println!(
        "  Widenings are NOT a verdict: the hull caps at three purchases and both towers reach it."
    );
    println!(
        "  Shelves finish {}/{} and {}/{} full, so nothing here is a shelf jam.",
        bare.shelves_used, bare.shelves_total, roped.shelves_used, roped.shelves_total
    );
}

fn measure(label: &'static str, extra: &[&str], ticks: u32) -> Run {
    let mut game = GameEngine::new(4242);
    // Four floors: the rope chain needs somewhere to stand, and both
    // shapes get the same room whether they use it or not.
    harness::open_the_ladder(&mut game, 4);

    // Every placement is asserted: a silent `false` from a place-a-room
    // helper is what made `siege_run.rs` compare a tower against a
    // byte-identical copy of itself, twice.
    for room in CORE.iter().chain(extra) {
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
            harness::give(&mut game, &item, n * 2);
        }
        assert!(
            harness::place_anywhere(&mut game, room),
            "{label}: could not place {room} — this harness has no working chain"
        );
    }

    let mill = game
        .content()
        .room_idx("room.mill")
        .expect("the pack has a mill");
    let poles = game
        .content()
        .item_idx("item.poles")
        .expect("the pack has poles");

    let mut widenings = 0;
    let mut mill_stalled_ticks = 0;
    let mut poles_milled = 0i64;
    let mut in_the_mill = 0i64;

    for _ in 0..ticks {
        game.step(1);

        // A standing reason to want poles. Bounded — see the header —
        // but a sink either way, so neither tower simply fills up.
        if game.try_send(GameCommand::WidenTower).is_ok() {
            widenings += 1;
        }

        // **Poles milled, counted off the mill's own output as it
        // rises.** Every rise is a finished pole; every fall is one
        // carried away, which is not this column's business.
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
            poles_milled += now - in_the_mill;
        }
        in_the_mill = now;

        // A stall is only interesting attached to the room it happened
        // to: on a ropery it is the rope chain doing its job, and on a
        // mill it is the tower failing.
        let view = game.view();
        for floor in &view.tower.floors {
            for room in &floor.rooms {
                if room.stalled && room.def == mill.0 {
                    mill_stalled_ticks += 1;
                }
            }
        }
    }

    let held = |id: &str| -> i64 {
        let content = game.content();
        let Some(idx) = content.item_idx(id) else {
            return 0;
        };
        game.state()
            .tower
            .floors
            .iter()
            .flat_map(|floor| &floor.rooms)
            .flat_map(|room| &room.shelves)
            .filter(|shelf| shelf.item == Some(idx))
            .map(|shelf| shelf.count)
            .sum()
    };
    let shelf_use = {
        let rooms: Vec<_> = game
            .state()
            .tower
            .floors
            .iter()
            .flat_map(|floor| &floor.rooms)
            .collect();
        let total: usize = rooms.iter().map(|room| room.shelves.len()).sum();
        let used = rooms
            .iter()
            .flat_map(|room| &room.shelves)
            .filter(|shelf| shelf.item.is_some() && shelf.count > 0)
            .count();
        (used, total)
    };

    Run {
        label,
        widenings,
        poles_milled,
        poles: held("item.poles"),
        rope: held("item.rope"),
        bamboo: held("item.bamboo"),
        shelves_used: shelf_use.0,
        shelves_total: shelf_use.1,
        mill_stalled_ticks,
    }
}
