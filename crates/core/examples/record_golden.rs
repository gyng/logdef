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
use understory_core::content::IntakeSource;
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
    step_walking(&mut engine, 1800);

    // A second storeroom, bought out of the starting poles and bought
    // first. The tower ships with one, four shelves wide, and a shelf
    // holds a single kind — so once bamboo has claimed all four there
    // is nowhere to put a pole, the mill's outbox fills, and the whole
    // chain stops with the shelves apparently only three-quarters full.
    // Faster intake reaches that inside five minutes, and once it does
    // the tower cannot afford its way out, because affording anything
    // needs the poles that are stuck in the mill. This is the shelf
    // bottleneck working exactly as designed; the script answers it the
    // way a player has to, and early.
    place_when_affordable(&mut engine, "room.storeroom", 2, 5);
    step_walking(&mut engine, 600);

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
    place_when_affordable(&mut engine, "room.canopy_sails", 4, 1);
    // An elevator costs a lot of poles. Bank them before asking for
    // one, and before adding a second consumer of the same item — a
    // thornwright eats poles as fast as the mill can supply them, so
    // the order here is the order a player is forced into.
    //
    // Twelve thousand ticks of it, because a tower being visited
    // regularly spends a real part of what one mill makes on putting
    // itself back together. That is the M2 economy, and a fixture
    // recorded against a quieter one would not be recording this game.
    step_walking(&mut engine, 12_000);

    // The elevator: cars, dispatch, dwell, and a charge draw per floor.
    engine
        .try_send(GameCommand::BuildShaft {
            shaft: "shaft.elevator".into(),
            low: 0,
            high: 4,
            slot: 7,
        })
        .expect("slot 7 should be clear all the way up");
    step_walking(&mut engine, 1800);

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
    step_walking(&mut engine, 900);

    // Halt, bank some charge, then set off again.
    engine
        .try_send(GameCommand::SetStriding { walking: false })
        .expect("always legal");
    step_walking(&mut engine, 900);
    engine
        .try_send(GameCommand::SetStriding { walking: true })
        .expect("always legal");
    step_walking(&mut engine, 900);

    // Darts, and somewhere for them to go. Banked for first: the
    // script has just spent a stretch of it standing still, and a
    // stopped tower harvests nothing now, so what it could afford
    // before the legs stopped is not what it can afford after.
    place_when_affordable(&mut engine, "room.thornwright", 4, 3);
    step_walking(&mut engine, 1800);

    // And a demolition, so the stale-task path is covered too.
    engine
        .try_send(GameCommand::RemoveRoom { floor: 4, slot: 3 })
        .expect("the thornwright placed above should still be there");
    step_walking(&mut engine, 900);

    // A battery, and then long enough at speed for the jungle to notice
    // the tower and come and have a look. This is what puts the siege,
    // the damage model, and the repair loop into the fixture — a
    // fixture that never sees a wave cannot catch the siege drifting.
    // Floor 2 rather than floor 1: the elevator's column took slot 7 on
    // every floor it spans, and floor 1's remaining two-wide gap is the
    // only place on the two ground floors a salvage rig can stand.
    place_when_affordable(&mut engine, "room.dart_battery", 2, 1);
    step_walking(&mut engine, 9000);

    // A berth. The rig goes in the last two-wide gap on the ground
    // floor, then the tower walks until a ruin is inside its reach and
    // stops — which is all berthing is, since there is no `Berth`
    // command (`SYSTEMS.md` §3.4). This puts the ruin intake, the
    // salvage draw-down, the rousing, and the wardens' own spawn path
    // into the fixture. It also records the one situation in the game
    // where a wave cannot be walked away from, because walking away is
    // what ends the salvage.
    place_when_affordable(&mut engine, "room.salvage_rig", 1, 1);
    let reach = rig_reach(&engine);
    for _ in 0..40_000 {
        answer_any_fork(&mut engine);
        engine.step(1);
        if engine.state().world.ruin_in_reach(reach).is_some() {
            break;
        }
    }
    assert!(
        engine.state().world.ruin_in_reach(reach).is_some(),
        "the recorder never walked past a ruin, so the fixture has no berth in it"
    );
    engine
        .try_send(GameCommand::SetStriding { walking: false })
        .expect("always legal");
    step_walking(&mut engine, 4500);

    // What the berth actually produced, asserted rather than assumed. A
    // fixture that walked past a ruin, stopped, and extracted nothing
    // would still be a valid recording — of a tower standing still.
    let scrap = scrap_held(&engine);
    let roused = wardens_out(&engine);
    assert!(scrap > 0, "the berth put no scrap in the fixture");
    assert!(roused > 0, "the ruin gave up scrap without waking anything");

    engine
        .try_send(GameCommand::SetStriding { walking: true })
        .expect("always legal");
    step_walking(&mut engine, 3000);

    // The enclave is deliberately **not** in this fixture. Reaching it
    // means walking into region 2, which took the recording from 40,000
    // ticks to 115,000 and the file from 90 KB to 260 KB — a third of a
    // second of every `cargo test`, three times the browser check, and
    // a quarter of a megabyte embedded in the WASM bundle, all to cover
    // two command handlers that are pure state arithmetic.
    //
    // What the fixture is for is native/wasm parity of the simulation,
    // and that is already covered by the chain, the elevator, a fork, a
    // berth, a waking and a siege. `Trade` and `Recruit` are covered by
    // the tests in `tests/journey.rs`, and their survival through the
    // replay format by `every_command_survives_the_replay_format`.
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
    println!("  the berth: {scrap} scrap out, {roused} warden(s) woken");
    println!("rebuild so the embedded copy picks it up");
}

/// Scrap anywhere in the tower — the rig's outbox, the shelves, or a
/// pair of hands on the stairs.
fn scrap_held(engine: &GameEngine) -> i64 {
    let Some(scrap) = engine.content().item_idx("item.scrap") else {
        return 0;
    };
    let state = engine.state();
    let stored: i64 = state
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .map(|room| {
            let outbox: i64 = room
                .outputs
                .iter()
                .filter(|s| s.item == scrap)
                .map(|s| s.count)
                .sum();
            let shelved: i64 = room
                .shelves
                .iter()
                .filter(|s| s.item == Some(scrap))
                .map(|s| s.count)
                .sum();
            outbox + shelved
        })
        .sum();
    let carried: i64 = state
        .crew
        .iter()
        .filter_map(|member| member.carrying)
        .filter(|(item, _)| *item == scrap)
        .map(|(_, count)| count)
        .sum();
    stored + carried
}

/// Creatures out that an ordinary wave could not have produced — which
/// is to say, wardens.
fn wardens_out(engine: &GameEngine) -> usize {
    engine
        .state()
        .siege
        .enemies
        .iter()
        .filter(|enemy| !engine.content().enemy(enemy.def).wave_eligible)
        .count()
}

/// Step, answering any fork before it can bring the tower to a halt.
///
/// The route splits about fifteen thousand paces in, and a tower with
/// no answer stands at the split indefinitely (`SYSTEMS.md` §3.9). A
/// recorder that answered only at the end would produce a fixture of a
/// parked tower — which, now that intake accrues per pace, is a fixture
/// of a tower that has also stopped harvesting.
fn step_walking(engine: &mut GameEngine, ticks: u32) {
    let mut left = ticks;
    while left > 0 {
        answer_any_fork(engine);
        let chunk = left.min(300);
        engine.step(chunk);
        left -= chunk;
    }
}

/// Place `room` as soon as the tower can pay for it, so the script's
/// shopping list survives a change to how fast the chain runs.
///
/// A recorder that hardcodes "by now there will be five poles" breaks
/// every time the economy moves, and breaks by *panicking mid-script*,
/// which at least is loud. Waiting for the money instead means the
/// fixture keeps covering the same systems across a balance change.
fn place_when_affordable(engine: &mut GameEngine, room: &str, floor: u8, slot: u8) {
    for _ in 0..200 {
        let result = engine.try_send(GameCommand::PlaceRoom {
            room: room.into(),
            floor,
            slot,
        });
        match result {
            Ok(()) => return,
            // Only ever wait for money. A slot clash or a bad floor is
            // a mistake in the script and should still be loud.
            Err(understory_core::command::CommandError::InsufficientStock { .. }) => {
                step_walking(engine, 300);
            }
            Err(other) => panic!("could not place {room} at {floor}.{slot}: {other}"),
        }
    }
    panic!("{room} never became affordable");
}

/// Take the left-hand branch of whatever fork is pending, if any.
fn answer_any_fork(engine: &mut GameEngine) {
    if engine
        .state()
        .world
        .fork
        .is_some_and(|fork| fork.answer.is_none())
    {
        engine
            .try_send(GameCommand::TakeFork { branch: 0 })
            .expect("a pending fork always offers a branch 0");
    }
}

/// How far the shipped rig reaches, read from the pack rather than
/// written down twice.
fn rig_reach(engine: &GameEngine) -> i64 {
    let rig = engine
        .content()
        .room_idx("room.salvage_rig")
        .expect("the pack ships a salvage rig");
    match engine.content().room_rt(rig).intake_source {
        Some(IntakeSource::Ruin { range_paces, .. }) => range_paces,
        _ => panic!("the salvage rig should draw from a ruin"),
    }
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/replays")
        .join("golden.json")
}
