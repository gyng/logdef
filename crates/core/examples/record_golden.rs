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

    // Long enough to cross from predawn into real daylight, so the sun
    // curve and the lamp threshold are both exercised.
    engine.set_speed(SimSpeed::X1);
    step_walking(&mut engine, 900);

    // ---------------------------------------------------------------
    // **The opening ladder** (`SYSTEMS.md` §6.11).
    //
    // M6 cut the starting tower to two floors, three crew, a Heartseed
    // and a bed. There is no chain to let run any more: the fixture has
    // to build one, in the order the gate allows — farm, cutter arm,
    // burner — and only then does the rest of the menu exist. That is
    // worth recording rather than skipping, because it is now the first
    // five minutes of every run.
    // ---------------------------------------------------------------

    // The farm, on the roof it starts with. It is the only card on turn
    // one and the only room in the pack that refuses to work without
    // people in it.
    place_when_affordable(&mut engine, "room.garden", 1, 3);
    // **Two of the three, posted.** `crew_required` is 2, so this is
    // not a bonus, it is the difference between a farm and an
    // ornament — and it puts `StationCrew`, `Errand::Station` and
    // `CrewState::Manning` into the fixture on the first minute rather
    // than nowhere at all.
    staff(&mut engine, 1, 3, 2);
    step_walking(&mut engine, 600);

    // The cutter arm, which the farm unlocked. Ground floor, beside the
    // Heartseed: `max_floor` is 1 because it reaches the ground, and
    // floor 0's slots 4-6 are the only three contiguous the tower has.
    place_when_affordable(&mut engine, "room.cutter_arm", 0, 4);
    step_walking(&mut engine, 900);

    // **Upward before the burner**, which is a placement argument
    // rather than an economic one. Floor 1 has six usable slots, the
    // bunk holds two of them and the farm two more; a burner in the
    // last two would leave the fiber comb — two wide and `max_floor` 1,
    // because fiber is stripped off the ground — with nowhere in the
    // tower to stand. So the tower grows first and the burner goes on
    // the new deck.
    build_floor_when_affordable(&mut engine);

    // The burner, which the cutter arm unlocked, and with it the rest
    // of the menu.
    place_when_affordable(&mut engine, "room.burner", 2, 1);
    step_walking(&mut engine, 600);

    // ---------------------------------------------------------------
    // The menu is open. Everything below is the game rather than the
    // opening.
    // ---------------------------------------------------------------

    // The mill: bamboo into poles, and the reason anything else is
    // affordable.
    place_when_affordable(&mut engine, "room.mill", 2, 3);
    step_walking(&mut engine, 600);

    // A second storeroom — the Heartseed carries two shelves and a
    // shelf holds a single kind, so once bamboo and poles have claimed
    // both there is nowhere to put a third material, the mill's outbox
    // fills, and the whole chain stops. Faster intake reaches that
    // inside five minutes, and once it does the tower cannot afford its
    // way out, because affording anything needs the poles that are
    // stuck in the mill. This is the shelf bottleneck working exactly
    // as designed; the script answers it the way a player has to, and
    // early.
    place_when_affordable(&mut engine, "room.storeroom", 2, 5);
    step_walking(&mut engine, 600);

    // A canteen, bought early because it is cheap and because a fixture
    // of a starving tower is a fixture of a tower that is about to work
    // at 60% and blame the shafts. It is also the only consumer bamboo
    // has that is not the mill, so without one the whole meals chain —
    // the recipe, the errand, the `Eating` state, `MealServed` — is
    // absent from the recording.
    build_floor_when_affordable(&mut engine);
    place_when_affordable(&mut engine, "room.canteen", 3, 1);
    step_walking(&mut engine, 600);

    // Higher again, and a second burner with it: charge is no longer
    // free, growing the tower adds lamps, and the fixture should record
    // a tower that can pay for the height it just bought.
    //
    // **Waits for the money, like every room does.** This used to
    // assert a floor was affordable after a fixed minute of milling,
    // which held only while the mill had the tower's bamboo to itself.
    // M6 gave the burner an appetite for the same stalks, the opening
    // chain got slower, and the recorder died at 2 poles of 6 — a
    // fixture that cannot be recorded, from a script that was making a
    // timing assumption it never said out loud.
    build_floor_when_affordable(&mut engine);
    place_when_affordable(&mut engine, "room.burner", 4, 1);
    // **Rope, because from M5 that is what an elevator is partly made
    // of** — and a chute, because fiber is about to become the fifth
    // material wanting a shelf and the storeroom has four.
    //
    // The chute has to be standing *before* the jam rather than after
    // it: measured, a recorder that waited until it noticed could never
    // build its way out, because by then there were no poles on any
    // shelf to pay with — they were stuck in a mill whose outbox had
    // nowhere to empty to. A chute prevents; it does not resurrect.
    place_when_affordable(&mut engine, "room.fiber_comb", 1, 5);
    place_when_affordable(&mut engine, "room.ropery", 3, 3);
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
    // **Six, not twenty-four**, and the difference is shelf space rather
    // than thrift. A shelf holds one kind, the tower has eight of them,
    // and twenty-four rope claims two — which it then never gives back,
    // because nothing eats rope. Measured on a 40-minute journey: 28
    // rope banked, bamboo with nowhere to land, the mill starved, and a
    // tower holding four poles of the twelve its elevator costs. Six is
    // an elevator (4) and a dart battery (2) and not one coil more.
    // **The canteen off once the larder is full**, the same move the ropery
    // and the comb get here, and for a reason that only started
    // biting at M6.
    //
    // A canteen eats 6 bamboo a craft behind a 12-deep inbox, against
    // the mill's 1 behind 6, and `find_destination` feeds the emptiest
    // inbox first — so a running canteen outbids the mill for every
    // stalk. That was survivable while the sails paid for the tower
    // and the mill only had one rival. Cutting the sails added a
    // third mouth for the same material, and the recorder then walked
    // 27,671 paces over 63,000 ticks to finish with **one pole and
    // nothing else on any shelf**: three shelves squatted by 29 meals
    // nobody was going to eat, a stalled mill, and a fixture that
    // could not be recorded at all.
    //
    // Twelve meals is four days of eating for three crew, which is
    // what a player banks and then stops.
    //
    // **A second cutter arm is the obvious other answer and it is the
    // wrong one.** Tried: harvest doubled, and so did provocation —
    // `provocation_per_100_harvested` is 300 — and the tower was
    // dismantled by the jungle at tick 20,100 with both arms wrecked
    // and no poles to mend them. Cutting harder is not free.
    off_when_stocked(&mut engine, "room.canteen", "item.meals", 12);
    off_when_stocked(&mut engine, "room.ropery", "item.rope", 6);
    // And the comb behind it, one material along and for exactly the
    // same reason: with the ropery off, fiber's consumer is gone too,
    // and an intake room with no consumer fills shelves precisely as
    // fast as a production room with none. **This is a pattern, not two
    // incidents.**
    off_when_stocked(&mut engine, "room.fiber_comb", "item.fiber", 4);
    // **And the kitchen, which is the same pattern a third time.** Meals
    // do have a consumer — crew eat them — but a canteen outruns three
    // appetites easily, and 24 of them banked claims two of the tower's
    // eight shelves and never gives them back. Measured on a 40-minute
    // journey: 24 meals, 12 fiber, 9 rope, bamboo with nowhere to land,
    // and six poles of the twelve an elevator costs. Twelve is a couple
    // of days' eating for this crew and one shelf.
    off_when_stocked(&mut engine, "room.canteen", "item.meals", 12);
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
    place_when_affordable(&mut engine, "room.bunk", 4, 3);
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

    // **The battery before the elevator, and the order is the finding.**
    // A dart battery is 6 poles and 2 rope; an elevator is 12 and 4.
    // Bought after the shaft it never became affordable at all once the
    // journey was scaled for a 40-minute run: the tower spent everything
    // on the lift, walked its 43,972 paces, *arrived*, and then stood
    // still earning nothing for the rest of the script. Measured — the
    // wait ran to tick 141,300 with `arrived true` and no poles at all,
    // having already built and demolished a thornwright and made twelve
    // darts, so it was one purchase short of the whole recording.
    //
    // A run is now about two fifths as long, so a tower's whole income
    // is about two fifths of what this script was written against, and
    // the order it buys in stopped being free. Cheap and load-bearing
    // first — the rule the shopping lists in `examples/` already follow,
    // arrived at here the hard way.
    place_when_affordable(&mut engine, "room.dart_battery", 3, 5);
    step_walking(&mut engine, 1800);

    // Darts to put in it, from the same argument: a thornwright is 5
    // poles and it used to sit after the elevator, where the tower had
    // four poles and needed five. One short, with the whole journey
    // already walked. Both cheap rooms now come before the expensive
    // shaft, which is the only ordering a 40-minute run can pay for.
    place_when_affordable(&mut engine, "room.thornwright", 4, 5);
    step_walking(&mut engine, 1800);

    // And a demolition, so the stale-task path is covered too.
    engine
        .try_send(GameCommand::RemoveRoom { floor: 4, slot: 5 })
        .expect("the thornwright placed above should still be there");
    step_walking(&mut engine, 900);

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

    // A battery, and then long enough at speed for the jungle to notice
    // the tower and come and have a look. This is what puts the siege,
    // the damage model, and the repair loop into the fixture — a
    // fixture that never sees a wave cannot catch the siege drifting.
    // Floor 2 rather than floor 1: the elevator's column took slot 7 on
    // every floor it spans, and floor 1's remaining two-wide gap is the
    // only place on the two ground floors a salvage rig can stand.
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
/// Let a room stock up, then switch it off.
///
/// **The waiting budget is small on purpose, and it used to be 400
/// blocks.** This is a chore rather than a gate: switching the room off
/// early is the *safe* outcome, because the whole reason it exists is
/// to stop a room squatting shelves. Waiting is only worth anything if
/// the stock actually arrives.
///
/// Three of these in a row at 400 blocks is 360,000 ticks of budget
/// against a journey that is about 74,000 ticks long. While the tower's
/// income comfortably beat the thresholds that never showed; when M6
/// cut the sails and the mill lost 31% of its crafts to the burner,
/// each call started spending its whole budget, the tower walked to the
/// far edge mid-chore, and every purchase after it was made by a tower
/// that had *arrived* and was earning nothing. The recorder died on a
/// bunk at tick 374,700 with no poles, having walked all 43,972 paces.
///
/// A blocking wait in a fixed script spends a resource the rest of the
/// script needs, and the resource here is the journey.
fn off_when_stocked(engine: &mut GameEngine, room: &str, item: &str, enough: i64) {
    let Some(idx) = engine.content().item_idx(item) else {
        return;
    };
    for _ in 0..40 {
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
/// Grow, once the poles are there.
///
/// The floor equivalent of `place_when_affordable`, and it exists for
/// the same reason: how long a tower takes to afford six poles is a
/// property of the economy, and a recorder that hardcodes it breaks
/// every time the economy moves.
fn build_floor_when_affordable(engine: &mut GameEngine) {
    for _ in 0..200 {
        match engine.try_send(GameCommand::BuildFloor) {
            Ok(()) => return,
            Err(understory_core::command::CommandError::InsufficientStock { .. }) => {
                step_walking(engine, 300);
            }
            Err(other) => panic!("could not grow the tower: {other}"),
        }
    }
    let state = engine.state();
    panic!(
        "a floor never became affordable at tick {} — {} paces walked, shelves hold {}",
        state.tick,
        state.world.distance >> 8,
        shelf_report(engine),
    );
}

/// Post `count` crew to the room at `floor`.`slot`.
///
/// The farm is the one room in the pack with `crew_required`, so
/// building it is only half of building it.
fn staff(engine: &mut GameEngine, floor: u8, slot: u8, count: usize) {
    let Some(room) = engine
        .state()
        .tower
        .find_room(floor, slot)
        .map(|room| room.id)
    else {
        panic!("nothing at {floor}.{slot} to post anybody to");
    };
    let crew: Vec<_> = engine.state().crew.iter().map(|member| member.id).collect();
    for who in crew.into_iter().take(count) {
        engine
            .try_send(GameCommand::StationCrew {
                crew: who,
                room: Some(room),
                until_tired: false,
            })
            .expect("posting somebody to a room that exists is always legal");
    }
}

fn place_when_affordable(engine: &mut GameEngine, room: &str, floor: u8, slot: u8) {
    for _ in 0..200 {
        let result = engine.try_send(GameCommand::PlaceRoom {
            room: room.into(),
            floor,
            slot,
        });
        match result {
            Ok(()) => {
                // **A timeline, because every failure of this script so
                // far has been "which step ate the journey".** The tower
                // has about 74,000 ticks of walking before it arrives
                // and stops earning; knowing where they went is the
                // difference between a diagnosis and a bisection.
                println!(
                    "  {room:<20} placed at {floor}.{slot}, tick {}",
                    engine.state().tick
                );
                return;
            }
            // Only ever wait for money. A slot clash or a bad floor is
            // a mistake in the script and should still be loud.
            Err(understory_core::command::CommandError::InsufficientStock { .. }) => {
                step_walking(engine, 300);
            }
            Err(other) => panic!("could not place {room} at {floor}.{slot}: {other}"),
        }
    }
    // **Says where it gave up, not just that it did.** "Never became
    // affordable" is the same sentence whether the tower is jammed, poor,
    // or standing at the far edge of the journey having already arrived —
    // and those want opposite fixes. Costs nothing until something fails.
    let state = engine.state();
    panic!(
        "{room} never became affordable at tick {} — {} paces walked, arrived {},          shelves hold {}",
        state.tick,
        state.world.distance >> 8,
        state.arrived,
        shelf_report(engine),
    );
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
    panic!(
        "{shaft} never became affordable at tick {} — {} paces walked, arrived {},          shelves hold {}",
        state.tick,
        state.world.distance >> 8,
        state.arrived,
        shelf_report(engine),
    );
}

/// What is on the shelves, by name. Shared by both "never became
/// affordable" panics, because the first question either of them raises
/// is whether the tower was poor or jammed — and those look completely
/// different here and want opposite fixes.
fn shelf_report(engine: &GameEngine) -> String {
    let state = engine.state();
    engine
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
        .collect::<Vec<_>>()
        .join(" ")
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
