//! Command validation. The contract is that a rejected command is a
//! no-op and an accepted one is fully paid for.

use crate::command::{CommandError, GameCommand};
use crate::state::SimSpeed;
use crate::tests::{content, engine, item, stock_poles};

#[test]
fn speed_round_trips() {
    let mut game = engine(1);
    for speed in [SimSpeed::X1, SimSpeed::X2, SimSpeed::X4, SimSpeed::Paused] {
        game.set_speed(speed);
        assert_eq!(game.state().speed, speed);
    }
}

#[test]
fn paused_engine_does_not_advance() {
    let mut game = engine(1);
    game.set_speed(SimSpeed::Paused);
    let before = game.state_hash();
    game.frame(1_000_000);
    assert_eq!(game.state().tick, 0);
    assert_eq!(game.state_hash(), before);
}

#[test]
fn speed_multiplies_ticks_per_frame() {
    let one_second = 1_000_000u64;

    let mut single = engine(1);
    single.set_speed(SimSpeed::X1);
    single.frame(one_second);

    let mut quad = engine(1);
    quad.set_speed(SimSpeed::X4);
    quad.frame(one_second);

    // Both are clamped by MAX_TICKS_PER_FRAME, so the assertion is
    // ordering, not an exact ratio.
    assert!(single.state().tick > 0);
    assert!(
        quad.state().tick >= single.state().tick,
        "4x ran {} ticks, 1x ran {}",
        quad.state().tick,
        single.state().tick
    );
}

#[test]
fn a_long_stall_cannot_spiral() {
    // A backgrounded tab returning after ten minutes must not try to
    // simulate ten minutes in one frame.
    let mut game = engine(1);
    game.set_speed(SimSpeed::X4);
    game.frame(600_000_000);
    assert!(game.state().tick <= u64::from(crate::engine::MAX_TICKS_PER_FRAME));
}

#[test]
fn building_a_floor_spends_stock_and_extends_the_stairs() {
    let content = content();
    let poles = item(&content, "item.poles");
    let mut game = engine(2);

    let before_floors = game.state().tower.floors.len();
    let before_stock = game.state().stock_of(poles);
    let cost: i64 = content
        .balance
        .tower
        .floor_cost
        .iter()
        .map(|c| c.amount)
        .sum();

    game.try_send(GameCommand::BuildFloor).expect("affordable");

    assert_eq!(game.state().tower.floors.len(), before_floors + 1);
    assert_eq!(game.state().stock_of(poles), before_stock - cost);

    let top = game.state().tower.top_floor();
    assert!(
        game.state().tower.shafts.iter().any(|s| s.high == top),
        "the stairs did not grow with the tower"
    );
}

#[test]
fn building_without_stock_is_refused_and_costs_nothing() {
    let content = content();
    let poles = item(&content, "item.poles");
    let mut game = engine(3);

    // Drain the shelves, then try to build.
    let stock = game.state().stock_of(poles);
    // No public "take" command, so build until it fails — which is the
    // behaviour under test anyway.
    let mut built = 0;
    loop {
        match game.try_send(GameCommand::BuildFloor) {
            Ok(()) => built += 1,
            Err(CommandError::InsufficientStock {
                needed, available, ..
            }) => {
                assert!(available < needed);
                break;
            }
            Err(other) => panic!("unexpected rejection: {other}"),
        }
        assert!(built < 50, "build loop never ran out of stock");
    }

    let floors_after = game.state().tower.floors.len();
    let hash = game.state_hash();
    assert!(game.try_send(GameCommand::BuildFloor).is_err());
    assert_eq!(game.state().tower.floors.len(), floors_after);
    assert_eq!(game.state_hash(), hash);
    assert!(stock > 0);
}

#[test]
fn the_floor_limit_holds() {
    let content = content();
    let max = content.balance.tower.max_floors;
    let mut game = engine(4);

    // Poles straight onto the shelves, rather than earned. This used to
    // run the economy for twelve thousand ticks, which worked only
    // because a parked tower kept harvesting: stacking floors adds
    // lamps the burner has to feed, the tower browns out, and from M3
    // a tower that cannot walk cannot harvest either (`SYSTEMS.md`
    // §3.6). Earning the poles now means never reaching the limit —
    // but the limit is what this test is about, and that spiral has
    // tests of its own.
    for _ in 0..max {
        stock_poles(&mut game, 12);
        game.step(30);
        let _ = game.send(GameCommand::BuildFloor);
        if game.state().tower.floors.len() >= max as usize {
            break;
        }
    }

    assert_eq!(game.state().tower.floors.len(), max as usize);
    assert!(matches!(
        game.try_send(GameCommand::BuildFloor),
        Err(CommandError::FloorLimit { .. })
    ));
}

#[test]
fn unknown_room_is_rejected_by_name() {
    let mut game = engine(5);
    let error = game
        .try_send(GameCommand::PlaceRoom {
            room: "room.does_not_exist".into(),
            floor: 0,
            slot: 6,
        })
        .expect_err("should reject");
    assert!(matches!(error, CommandError::UnknownRoom { .. }));
}

#[test]
fn a_room_cannot_overlap_another_room() {
    let mut game = engine(6);
    // The starting mill sits on floor 2 at slot 3, two wide.
    let error = game
        .try_send(GameCommand::PlaceRoom {
            room: "room.storeroom".into(),
            floor: 2,
            slot: 4,
        })
        .expect_err("slot 4 is inside the mill");
    assert!(matches!(error, CommandError::SlotOccupied { .. }));
}

#[test]
fn a_room_cannot_overlap_a_shaft_column() {
    let mut game = engine(7);
    // The stairs own slot 0 on every floor.
    let error = game
        .try_send(GameCommand::PlaceRoom {
            room: "room.storeroom".into(),
            floor: 3,
            slot: 0,
        })
        .expect_err("slot 0 is the stairs");
    assert!(matches!(error, CommandError::SlotOccupied { .. }));
}

#[test]
fn a_room_cannot_hang_off_the_edge() {
    let mut game = engine(8);
    let slots = content().balance.tower.floor_slots;
    let error = game
        .try_send(GameCommand::PlaceRoom {
            room: "room.storeroom".into(),
            floor: 3,
            slot: slots - 1,
        })
        .expect_err("a two-wide room cannot start in the last slot");
    assert!(matches!(error, CommandError::SlotOutOfRange { .. }));
}

#[test]
fn intake_stays_near_the_ground() {
    let mut game = engine(9);
    let error = game
        .try_send(GameCommand::PlaceRoom {
            room: "room.cutter_arm".into(),
            floor: 3,
            slot: 3,
        })
        .expect_err("cutter arms reach the ground, not the roof");
    assert!(matches!(error, CommandError::FloorTooHigh { .. }));
}

#[test]
fn the_heartseed_is_unique_and_permanent() {
    let mut game = engine(10);

    let placed_twice = game.try_send(GameCommand::PlaceRoom {
        room: "room.heartseed".into(),
        floor: 3,
        slot: 3,
    });
    assert!(matches!(
        placed_twice,
        Err(CommandError::AlreadyPlaced { .. })
    ));

    let torn_out = game.try_send(GameCommand::RemoveRoom { floor: 0, slot: 1 });
    assert!(matches!(torn_out, Err(CommandError::Undemolishable { .. })));
}

#[test]
fn removing_a_room_clears_the_crew_tasks_that_pointed_at_it() {
    let mut game = engine(11);
    // Long enough that the crew are committed to hauls involving the
    // mill on floor 2.
    game.step(600);

    game.try_send(GameCommand::RemoveRoom { floor: 2, slot: 3 })
        .expect("the starting mill is removable");

    let dangling = game.state().crew.iter().any(|member| {
        member.task.as_ref().is_some_and(|task| {
            task.pickup
                .is_some_and(|pickup| pickup.floor == 2 && pickup.slot >= 3)
        })
    });
    assert!(!dangling, "a crew member is still hauling to a ghost room");

    // And the simulation keeps running without panicking.
    game.step(300);
}

#[test]
fn removing_nothing_is_an_error_not_a_silent_success() {
    let mut game = engine(12);
    // Slot 1 rather than 5: the roof's sails are three wide and cover
    // 4 to 6, so slot 5 has something in it now.
    let error = game
        .try_send(GameCommand::RemoveRoom { floor: 3, slot: 1 })
        .expect_err("nothing stands on floor 3 slot 1");
    assert!(matches!(error, CommandError::NoRoomThere { .. }));
}

#[test]
fn placing_on_a_floor_that_does_not_exist_is_refused() {
    let mut game = engine(13);
    let error = game
        .try_send(GameCommand::PlaceRoom {
            room: "room.storeroom".into(),
            floor: 200,
            slot: 2,
        })
        .expect_err("no such floor");
    assert!(matches!(error, CommandError::NoSuchFloor { .. }));
}

// ---------------------------------------------------------------------------
// The opening five minutes (`SYSTEMS.md` §6.11)
// ---------------------------------------------------------------------------

#[test]
fn the_opening_tower_is_a_heartseed_and_a_bed() {
    // The shipped opening, asserted as a shape rather than described in
    // a comment. Two floors, three crew, and nothing that works.
    let content = content();
    let game = crate::tests::opening(1);
    let state = game.state();

    assert_eq!(
        state.tower.floors.len(),
        2,
        "the opening tower is two floors"
    );
    assert_eq!(
        state.crew.len(),
        usize::from(content.balance.crew.starting_crew)
    );

    let rooms: Vec<&str> = state
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .map(|room| content.room(room.def).id.as_str())
        .collect();
    // The Heartseed, a bed, and one gun that eats raw bamboo — see
    // `SYSTEMS.md` §6.13. The gun is on the ground floor's
    // leading edge, so it sorts after the Heartseed.
    assert_eq!(rooms, vec!["room.heartseed", "room.thorn_gun", "room.bunk"]);

    // And the stores are aboard, because a build cost is paid off a
    // shelf and there is no storeroom to pay it from.
    let poles = item(&content, "item.poles");
    assert!(
        state.stock_of(poles) >= 24,
        "the founding stores have to survive having no storeroom to sit in"
    );
}

#[test]
fn the_first_turn_offers_one_card() {
    // **The whole point of the gate.** Twenty of twenty-one rooms used
    // to be buildable on turn one. Exactly one is now, and every other
    // refusal has to be a `Locked` rather than a slot clash or a price
    // — otherwise this is measuring the tower's shape, not the rule.
    let content = content();
    let mut game = crate::tests::opening(2);
    // Money, so nothing is refused for being unaffordable.
    crate::tests::stock_item(&mut game, "item.poles", 60);

    let mut open = Vec::new();
    for room in &content.rooms {
        if room.unique {
            continue;
        }
        let locked = content
            .room_rt(content.room_idx(&room.id).expect("a room the pack defines"))
            .unlocked_by
            .is_some();
        if !locked {
            open.push(room.id.as_str());
        }
    }
    assert_eq!(
        open,
        vec!["room.bunk", "room.garden", "room.thorn_gun"],
        "turn one should offer the farm, the bed and the gun the tower already has"
    );

    // And the rule bites at the command boundary, not just in a menu.
    let refused = game
        .try_send(GameCommand::PlaceRoom {
            room: "room.mill".into(),
            floor: 1,
            slot: 3,
        })
        .expect_err("a mill is four rooms down the ladder");
    assert!(
        matches!(refused, CommandError::Locked { .. }),
        "a locked room was refused for the wrong reason: {refused:?}"
    );
}

#[test]
fn the_ladder_opens_one_rung_at_a_time() {
    // Farm, then cutter arm, then burner, and then everything.
    let mut game = crate::tests::opening(3);
    crate::tests::stock_item(&mut game, "item.poles", 60);

    let blocked = |game: &mut crate::engine::GameEngine, room: &str, floor, slot| {
        matches!(
            game.try_send(GameCommand::PlaceRoom {
                room: room.into(),
                floor,
                slot,
            }),
            Err(CommandError::Locked { .. })
        )
    };

    // Slot 8 on floor 1: a cutter arm is `front_only` and two wide, so
    // the front is `floor_slots - width` (`SYSTEMS.md` §6.13), and
    // floor 0's front is where the tower's own gun stands.
    assert!(blocked(&mut game, "room.cutter_arm", 1, 8));
    game.try_send(GameCommand::PlaceRoom {
        room: "room.garden".into(),
        floor: 1,
        slot: 3,
    })
    .expect("the farm is turn one's card");

    assert!(blocked(&mut game, "room.burner", 1, 5));
    game.try_send(GameCommand::PlaceRoom {
        room: "room.cutter_arm".into(),
        floor: 1,
        slot: 8,
    })
    .expect("the farm opened the cutter arm");

    assert!(blocked(&mut game, "room.mill", 1, 5));
    game.try_send(GameCommand::PlaceRoom {
        room: "room.burner".into(),
        floor: 1,
        slot: 5,
    })
    .expect("the cutter arm opened the burner");

    // And now the menu is open.
    crate::tests::stock_item(&mut game, "item.poles", 60);
    game.try_send(GameCommand::BuildFloor).expect("affordable");
    game.try_send(GameCommand::PlaceRoom {
        room: "room.mill".into(),
        floor: 2,
        slot: 1,
    })
    .expect("the burner opened everything");
}

#[test]
fn a_farm_with_nobody_in_it_grows_nothing() {
    // `crew_required` is a requirement rather than M6's bonus, and the
    // farm is the only room in the pack that carries it. Two towers,
    // same seed, same hour: one with people posted and one without.
    let content = content();
    let produce = item(&content, "item.produce");
    let noon = content.balance.clock.ticks_per_day / 2;

    let grown = |game: &crate::engine::GameEngine| -> i64 {
        game.state()
            .tower
            .floors
            .iter()
            .flat_map(|floor| floor.rooms.iter())
            .flat_map(|room| room.outputs.iter())
            .filter(|stack| stack.item == produce)
            .map(|stack| stack.count)
            .sum()
    };

    let run = |posted: usize| {
        let mut game = crate::tests::opening(4);
        crate::tests::stock_item(&mut game, "item.poles", 60);
        game.try_send(GameCommand::PlaceRoom {
            room: "room.garden".into(),
            floor: 1,
            slot: 3,
        })
        .expect("the farm is turn one's card");
        assert_eq!(crate::tests::staff(&mut game, 1, 3, posted), posted);
        game.state_mut_for_test().clock.tick_of_day = noon;
        game.step(2400);
        grown(&game)
    };

    assert_eq!(run(0), 0, "a farm nobody is standing in grew a crop");
    assert_eq!(run(1), 0, "one person ran a farm that asks for two");
    assert!(run(2) > 0, "two people posted to a farm grew nothing");
}
