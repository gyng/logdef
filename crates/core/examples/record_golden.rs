//! Regenerate the golden replay fixture.
//!
//! ```text
//! cargo run -p understory-core --example record_golden
//! ```
//!
//! Plays a fixed script — build a floor, add a second mill, run for a
//! while at speed — and writes the recording to
//! `assets/replays/m0-golden.json`. That file is embedded into the
//! binary, so after regenerating you must rebuild before the native
//! test and the browser check see the new bytes.
//!
//! Run this whenever a deliberate simulation change makes the old
//! fixture stale. If it changes when you did **not** intend a
//! simulation change, you have found a determinism bug — do not
//! regenerate, go fix it.

use std::path::PathBuf;

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;

/// Fixed seed so the fixture is reproducible.
const SEED: u64 = 0x0000_5EED_0000_0001;

fn main() {
    let mut engine = GameEngine::new(SEED);

    // Let the opening chain run long enough for the cutter arm to fill,
    // the crew to make several round trips, and the mill to craft.
    engine.set_speed(SimSpeed::X1);
    engine.step(600);

    // Build upward, then put a second mill on the new floor. This
    // exercises construction, stock spending, and the stairs extending.
    engine
        .try_send(GameCommand::BuildFloor)
        .expect("floor should be affordable after 20 seconds of milling");
    engine
        .try_send(GameCommand::PlaceRoom {
            room: "room.mill".into(),
            floor: 3,
            slot: 3,
        })
        .expect("floor 3 slot 3 should be free");
    engine.step(600);

    // A speed change mid-recording, to prove it round-trips.
    engine.set_speed(SimSpeed::X4);
    engine.step(300);

    // And a demolition, so the stale-task path is covered too.
    engine
        .try_send(GameCommand::RemoveRoom { floor: 3, slot: 3 })
        .expect("the mill placed above should still be there");
    engine.step(300);

    let replay = engine.export_replay();
    let path = fixture_path();
    std::fs::create_dir_all(path.parent().expect("fixture has a parent directory"))
        .expect("fixture directory must be writable");
    std::fs::write(&path, replay.to_json()).expect("fixture must be writable");

    println!(
        "wrote {} — {} tick(s), {} command(s), {} checkpoint(s)",
        path.display(),
        replay.final_tick,
        replay.commands.len(),
        replay.checkpoints.len()
    );
    println!("rebuild so the embedded copy picks it up");
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/replays")
        .join("m0-golden.json")
}
