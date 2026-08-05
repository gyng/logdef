//! What a tick costs, and where the budget goes.
//!
//! ```text
//! cargo run --release -p understory-core --example profile
//! ```
//!
//! `AGENTS.md` §IV says profile before optimising and do not optimise
//! speculatively, so this exists to say whether there is anything to
//! optimise at all. It measures the sim only — the renderer's half is in
//! `web/e2e/profile.spec.ts`, because a frame budget spent in WebGL is
//! not visible from here.
//!
//! **The budget.** The sim runs at a fixed 30 Hz, and the player can ask
//! for 4×, so the worst case is **120 ticks a second** — and those have
//! to fit alongside a 60 fps render in the same single browser thread.
//! Call it 4 ms a frame for the sim at 4×, which is 8 ticks: **500 µs a
//! tick** is the line, and anything under about 100 µs means the sim is
//! not the thing to look at.

use std::time::Instant;

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;

const WARMUP: u32 = 30_000;
const SAMPLE: u32 = 120_000;

fn main() {
    println!("=== what does a tick cost? ===\n");
    println!(
        "  {SAMPLE} ticks a shape, after a {WARMUP}-tick warm-up. The budget is 120 ticks a\n\
         second at 4x, sharing a thread with a 60 fps render — so ~500us a tick is the line\n\
         and under ~100us means the sim is not worth optimising.\n"
    );
    println!(
        "{:<26} {:>9} {:>10} {:>9} {:>8}",
        "tower", "us/tick", "ticks/s", "rooms", "crew"
    );

    for (label, floors, rooms) in [
        ("starting", 0, &[][..]),
        (
            "a working chain",
            0,
            &["room.canteen", "room.bunk", "room.storeroom"][..],
        ),
        (
            "big, busy",
            6,
            &[
                "room.canteen",
                "room.bunk",
                "room.storeroom",
                "room.storeroom",
                "room.mill",
                "room.cutter_arm",
                "room.fiber_comb",
                "room.ropery",
                "room.thornwright",
                "room.dart_battery",
                "room.salvage_rig",
                "room.burner",
                "room.garden",
            ][..],
        ),
    ] {
        let (us, rooms_built, crew) = measure(floors, rooms);
        println!(
            "{label:<26} {us:>9.1} {:>10.0} {rooms_built:>9} {crew:>8}",
            1_000_000.0 / us
        );
    }

    println!(
        "\n  A tick is the same work whatever the speed multiplier — 4x runs four of them,\n\
         it does not make one bigger. So multiply the figure above by 4 to compare\n\
         against a frame, and by 120 to compare against a second."
    );
}

fn measure(extra_floors: u32, rooms: &[&str]) -> (f64, usize, usize) {
    let mut game = GameEngine::new(0x000B_EEF5);
    game.set_speed(SimSpeed::X1);

    // Hand it the materials rather than making it earn them: this is a
    // measurement of what a tick costs on a tower of a given shape, not
    // of how long that shape takes to reach.
    endow(&mut game);
    for _ in 0..extra_floors {
        let _ = game.try_send(GameCommand::BuildFloor);
        endow(&mut game);
    }
    for room in rooms {
        let floors = game.state().tower.floors.len() as u8;
        let slots = game.content().balance.tower.floor_slots;
        'place: for floor in 0..floors {
            for slot in 0..slots {
                if game
                    .try_send(GameCommand::PlaceRoom {
                        room: (*room).into(),
                        floor,
                        slot,
                    })
                    .is_ok()
                {
                    break 'place;
                }
            }
        }
        endow(&mut game);
    }
    let _ = game.try_send(GameCommand::SetStriding { walking: true });

    // Warm up so the measurement is of a running tower rather than of
    // one still filling its buffers, and so waves have started.
    for _ in 0..WARMUP {
        answer(&mut game);
        game.step(1);
    }

    let rooms_built = game
        .state()
        .tower
        .floors
        .iter()
        .map(|f| f.rooms.len())
        .sum::<usize>();
    let crew = game.state().crew.len();

    let start = Instant::now();
    for _ in 0..SAMPLE {
        answer(&mut game);
        game.step(1);
    }
    let elapsed = start.elapsed();

    (
        elapsed.as_secs_f64() * 1_000_000.0 / f64::from(SAMPLE),
        rooms_built,
        crew,
    )
}

fn answer(game: &mut GameEngine) {
    if let Some(fork) = game.state().world.fork
        && fork.answer.is_none()
    {
        let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
    }
}

fn endow(game: &mut GameEngine) {
    let content = game.content().clone();
    let state = game.state_mut_for_test();
    for id in ["item.poles", "item.rope", "item.bamboo", "item.darts"] {
        let Some(item) = content.item_idx(id) else {
            continue;
        };
        let mut left = 60;
        'floors: for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                left -= room.shelve(item, left);
                if left <= 0 {
                    break 'floors;
                }
            }
        }
    }
}
