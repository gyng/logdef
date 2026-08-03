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
use understory_core::content::{IntakeSource, Shift};
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

    // A canteen, bought early because it is cheap and because a fixture
    // of a starving tower is a fixture of a tower that is about to work
    // at 60% and blame the shafts. It is also the only consumer bamboo
    // has that is not the mill, so without one the whole meals chain —
    // the recipe, the errand, the `Eating` state, `MealServed` — is
    // absent from the recording.
    place_when_affordable(&mut engine, "room.canteen", 3, 1);
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
    // **Rope, because from M5 that is what an elevator is partly made
    // of** — and a chute, because fiber is about to become the fifth
    // material wanting a shelf and the storeroom has four.
    //
    // The chute has to be standing *before* the jam rather than after
    // it: measured, a recorder that waited until it noticed could never
    // build its way out, because by then there were no poles on any
    // shelf to pay with — they were stuck in a mill whose outbox had
    // nowhere to empty to. A chute prevents; it does not resurrect.
    place_when_affordable(&mut engine, "room.fiber_comb", 1, 4);
    place_when_affordable(&mut engine, "room.ropery", 2, 1);
    // **And switched off again once there is rope for a shaft.**
    //
    // Rope's only consumer is a build cost, and a build cost is a
    // *one-off*. Left running, a ropery fills the tower with something
    // nothing eats and starves everything else of shelf space:
    // measured, 140 rope across every shelf, no poles anywhere, and an
    // elevator that could never be afforded even though rope was the
    // only thing the tower had. A chute does not save you from this and
    // should not — rope is a material the tower builds with, so it is
    // *wanted*, and a chute that threw away wanted materials is the
    // version of this that lost the economy instead (see
    // `haul::find_destination`).
    //
    // **A room whose consumer is a one-off is a room you turn off**, and
    // that is a real thing this economy asks of a player rather than a
    // quirk of the fixture. `SYSTEMS.md` §5.11 carries it as an open
    // question, because "remember to switch it off" is a poor answer.
    off_when_stocked(&mut engine, "room.ropery", "item.rope", 24);
    // And the comb behind it, one material along and for exactly the
    // same reason: with the ropery off, fiber's consumer is gone too,
    // and an intake room with no consumer fills shelves precisely as
    // fast as a production room with none. **This is a pattern, not two
    // incidents.**
    off_when_stocked(&mut engine, "room.fiber_comb", "item.fiber", 8);
    // **No chute in the fixture, and the reason is structural rather
    // than incidental.** A shaft needs one free column on every floor it
    // spans, and once the Heartseed, the cutter arm, a cell bank, two
    // storerooms, a mill, a ropery and a comb are placed there is
    // exactly one full-height column left — which the elevator has. Two
    // shafts want the same slot and only one can have it, which is
    // `DESIGN.md` pillar 2 working as designed and not something a
    // fixture should paper over.
    //
    // Chute and spill behaviour is covered by `tests/haul.rs` instead,
    // which is the better home for it: a spill is a haul decision, and
    // testing it needs a deliberately jammed tower rather than a healthy
    // one that happens to own a chute.
    // Beds, on the new top floor. Two of them, which at a three-crew
    // tower with everybody on the day shift is one short — deliberately,
    // so the fixture records both halves of sleep: somebody in a
    // hammock, and somebody on the deck at half the rest rate.
    place_when_affordable(&mut engine, "room.bunk", 4, 4);
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
    //
    // Waited for rather than assumed, for the same reason the rooms are
    // (`place_when_affordable`). Eighteen poles was comfortably banked
    // in twelve thousand ticks before M4; a tower whose crew sleep
    // through the night and stop to eat takes longer to get there, and
    // hardcoding the wait meant the recorder panicked eight poles short
    // rather than recording a slower tower.
    build_shaft_when_affordable(&mut engine, "shaft.elevator", 0, 4, 7);
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

    // The rota. One crew member onto the night shift, which is the one
    // command M4 adds and the only way the fixture covers a tower whose
    // crew are not all asleep at the same time. Recorded as a decision
    // about a named person, which is what `SetShift` is for.
    let night_worker = engine
        .state()
        .crew
        .last()
        .expect("a run starts with crew")
        .id;
    engine
        .try_send(GameCommand::SetShift {
            crew: night_worker,
            shift: Shift::Night,
        })
        .expect("anybody aboard can be reshifted");

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
    place_when_affordable(&mut engine, "room.thornwright", 4, 6);
    step_walking(&mut engine, 1800);

    // And a demolition, so the stale-task path is covered too.
    engine
        .try_send(GameCommand::RemoveRoom { floor: 4, slot: 6 })
        .expect("the thornwright placed above should still be there");
    step_walking(&mut engine, 900);

    // A battery, and then long enough at speed for the jungle to notice
    // the tower and come and have a look. This is what puts the siege,
    // the damage model, and the repair loop into the fixture — a
    // fixture that never sees a wave cannot catch the siege drifting.
    // Floor 2 rather than floor 1: the elevator's column took slot 7 on
    // every floor it spans, and floor 1's remaining two-wide gap is the
    // only place on the two ground floors a salvage rig can stand.
    place_when_affordable(&mut engine, "room.dart_battery", 1, 6);
    step_walking(&mut engine, 9000);

    // A berth. The rig goes in the last two-wide gap on the ground
    // floor, then the tower walks until a ruin is inside its reach and
    // stops — which is all berthing is, since there is no `Berth`
    // command (`SYSTEMS.md` §3.4). This puts the ruin intake, the
    // salvage draw-down, the rousing, and the wardens' own spawn path
    // into the fixture. It also records the one situation in the game
    // where a wave cannot be walked away from, because walking away is
    // what ends the salvage.
    // **No salvage rig in the fixture from M5, and the reason is a slot
    // problem worth writing down.**
    //
    // Three rooms reach the ground and therefore carry `max_floor: 1` —
    // the cutter arm, the salvage rig and now the fiber comb — and
    // between the Heartseed, the cell bank, a storeroom and the
    // elevator's column there are not two floors' worth of room for all
    // three. Something had to go, and it is the rig: berthing, wardens
    // and ruin intake are covered by `tests/journey.rs` end to end,
    // while the chute, the comb and the ropery are covered nowhere else
    // and are what M5 added. The fixture trades M3 coverage it
    // duplicates for M5 coverage it does not.
    //
    // That three ground rooms do not fit on two ground floors is a real
    // tension rather than a fixture problem — see `SYSTEMS.md` §5.11.
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
    // **No rig in the fixture from M5, so no scrap and no wardens.**
    // The berth is still recorded — the tower still stops with a ruin in
    // reach, which exercises the halt, the world state and the intake
    // system's berthed branch — but there is nothing aboard to extract
    // with. See the note above the walk for why the rig lost its slot,
    // and `tests/journey.rs` for where ruin intake and wardens are
    // covered end to end.
    let scrap = scrap_held(&engine);
    let roused = wardens_out(&engine);

    // And that the home half of M4 is actually in the recording. A
    // fixture with a canteen the crew never reached and bunks nobody
    // ever lay in would look identical to one with neither, and would
    // catch neither drifting.
    let meals_eaten = engine.state().stats.meals_eaten;
    let crew_ticks_asleep = engine.state().stats.crew_ticks_asleep;
    assert!(
        meals_eaten > 0,
        "the fixture has a canteen in it but nobody ever ate"
    );
    assert!(
        crew_ticks_asleep > 0,
        "the fixture has bunks in it but nobody ever slept"
    );

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
    println!(
        "  the home: {} meal(s) eaten, {} crew-tick(s) asleep",
        meals_eaten, crew_ticks_asleep
    );
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

/// Switch a room off once the tower holds enough of what it makes.
///
/// The scripted version of a decision a player makes by looking: a chain
/// whose consumer is a one-off build cost has to be stopped by hand, or
/// it fills the shelves with something nothing eats.
fn off_when_stocked(engine: &mut GameEngine, room: &str, item: &str, enough: i64) {
    let Some(idx) = engine.content().item_idx(item) else {
        return;
    };
    for _ in 0..400 {
        if engine.state().stock_of(idx) >= enough {
            break;
        }
        step_walking(engine, 300);
    }
    let found = engine.state().tower.floors.iter().find_map(|floor| {
        floor
            .rooms
            .iter()
            .find(|r| engine.content().room(r.def).id == room)
            .map(|r| (floor.index, r.slot))
    });
    if let Some((floor, slot)) = found {
        let _ = engine.try_send(GameCommand::SetRoomActive {
            floor,
            slot,
            active: false,
        });
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

/// The same, for a shaft.
fn build_shaft_when_affordable(engine: &mut GameEngine, shaft: &str, low: u8, high: u8, slot: u8) {
    for _ in 0..200 {
        let result = engine.try_send(GameCommand::BuildShaft {
            shaft: shaft.into(),
            low,
            high,
            slot,
        });
        match result {
            Ok(()) => return,
            Err(understory_core::command::CommandError::InsufficientStock { .. }) => {
                step_walking(engine, 300);
            }
            Err(other) => panic!("could not build {shaft} at slot {slot}: {other}"),
        }
    }
    let state = engine.state();
    let held: Vec<String> = engine
        .content()
        .items
        .iter()
        .enumerate()
        .map(|(i, def)| {
            format!(
                "{}={}",
                def.name,
                state.stock_of(understory_core::ids::ItemIdx(i as u16))
            )
        })
        .collect();
    panic!(
        "{shaft} never became affordable; shelves hold {}",
        held.join(" ")
    );
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
