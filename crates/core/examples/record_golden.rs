//! Regenerate the golden replay fixture.
//!
//! ```text
//! cargo run -p understory-core --example record_golden
//! ```
//!
//! Plays a fixed script that touches every system the simulation has —
//! the chain, construction, the sun, an elevator with a programmed
//! schedule, halting and restarting the legs, and a demolition — and
//! writes the recording to `assets/replays/golden.json`. That file is
//! embedded into the binary, so after regenerating you must rebuild
//! before the native test and the browser check see the new bytes.
//!
//! Add to the script whenever a milestone adds a system. A fixture that
//! never exercises the elevator cannot catch the elevator drifting.
//!
//! Run this whenever a deliberate simulation change makes the old
//! fixture stale. If it changes when you did **not** intend a
//! simulation change, you have found a determinism bug — do not
//! regenerate, go fix it.

use std::path::PathBuf;

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::{ShaftPriority, SimSpeed};

/// Fixed seed so the fixture is reproducible.
const SEED: u64 = 0x0000_5EED_0000_0001;

fn main() {
    let mut engine = GameEngine::new(SEED);

    // Let the opening chain run long enough for the cutter arm to fill,
    // the crew to make several round trips, and the mill to craft. Also
    // long enough to cross from predawn into real daylight, so the sun
    // curve and the sails are both exercised.
    engine.set_speed(SimSpeed::X1);
    engine.step(1800);

    // Build upward. This exercises construction, stock spending, the
    // stairs extending — and shades the sails, which were sitting on
    // what used to be the roof.
    engine
        .try_send(GameCommand::BuildFloor)
        .expect("a floor should be affordable after a minute of milling");
    // Growing taller put the sails in the shade, which on a tower this
    // small means no income at all — the legs stop within the minute
    // and never start again. Re-roofing is not optional, and a script
    // that skipped it recorded a tower standing still being eaten,
    // which exercises far less of the simulation than one that walks.
    engine
        .try_send(GameCommand::PlaceRoom {
            room: "room.canopy_sails".into(),
            floor: 4,
            slot: 1,
        })
        .expect("the new roof is bare");
    // An elevator costs a lot of poles. Bank them before asking for
    // one, and before adding a second consumer of the same item — a
    // thornwright eats poles as fast as the mill can supply them, so
    // the order here is the order a player is forced into.
    //
    // Twelve thousand ticks of it, because a tower being visited
    // regularly spends a real part of what one mill makes on putting
    // itself back together. That is the M2 economy, and a fixture
    // recorded against a quieter one would not be recording this game.
    engine.step(12_000);

    // The elevator: cars, dispatch, dwell, and a charge draw per floor.
    engine
        .try_send(GameCommand::BuildShaft {
            shaft: "shaft.elevator".into(),
            low: 0,
            high: 4,
            slot: 7,
        })
        .expect("slot 7 should be clear all the way up");
    engine.step(1800);

    // Reprogram it, so the per-daypart program path is in the fixture.
    let elevator = engine
        .state()
        .tower
        .shafts
        .last()
        .expect("the elevator was just built")
        .id;
    engine
        .try_send(GameCommand::SetShaftProgram {
            id: elevator,
            daypart: 0,
            served: vec![true, false, true, true, true],
            priority: ShaftPriority::FreightFirst,
        })
        .expect("daypart 0 exists");

    // A speed change mid-recording, to prove it round-trips.
    engine.set_speed(SimSpeed::X4);
    engine.step(900);

    // Halt, bank some charge, then set off again.
    engine
        .try_send(GameCommand::SetStriding { walking: false })
        .expect("always legal");
    engine.step(900);
    engine
        .try_send(GameCommand::SetStriding { walking: true })
        .expect("always legal");
    engine.step(900);

    // Darts, and somewhere for them to go.
    engine
        .try_send(GameCommand::PlaceRoom {
            room: "room.thornwright".into(),
            floor: 4,
            slot: 3,
        })
        .expect("the new top floor is empty");
    engine.step(1800);

    // And a demolition, so the stale-task path is covered too.
    engine
        .try_send(GameCommand::RemoveRoom { floor: 4, slot: 3 })
        .expect("the thornwright placed above should still be there");
    engine.step(900);

    // A battery, and then long enough at speed for the jungle to notice
    // the tower and come and have a look. This is what puts the siege,
    // the damage model, and the repair loop into the fixture — a
    // fixture that never sees a wave cannot catch the siege drifting.
    engine
        .try_send(GameCommand::PlaceRoom {
            room: "room.dart_battery".into(),
            floor: 1,
            slot: 1,
        })
        .expect("floor 1 slot 1 should be free");
    engine.step(9000);

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
        .join("golden.json")
}
