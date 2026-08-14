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
fn a_shelf_filter_reserves_empty_capacity_without_discarding_stock() {
    let mut game = engine(5005);
    let poles = item(game.content(), "item.poles");
    let bamboo = item(game.content(), "item.bamboo");
    let storeroom = game.content().room_idx("room.storeroom").unwrap();
    let room_id = {
        let state = game.state_mut_for_test();
        let room = state
            .tower
            .floors
            .iter_mut()
            .flat_map(|floor| floor.rooms.iter_mut())
            .find(|room| room.def == storeroom)
            .expect("fixture has storage");
        for shelf in &mut room.shelves {
            shelf.item = None;
            shelf.count = 0;
            shelf.filter = None;
        }
        room.id
    };

    game.try_send(GameCommand::SetShelfFilter {
        room: room_id,
        shelf: 0,
        item: Some("item.poles".into()),
    })
    .expect("an empty shelf may be reserved");

    let room = game
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .find(|room| room.id == room_id)
        .expect("room still stands");
    assert_eq!(room.shelves[0].filter, Some(poles));
    assert_eq!(room.shelves[0].space_for(bamboo), 0);
    assert_eq!(room.shelves[0].space_for(poles), room.shelves[0].max);

    let view = game.view();
    let rendered = view
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .find(|room| room.id == room_id.0)
        .expect("filtered room is visible");
    assert_eq!(rendered.shelves[0].filter, Some(poles.0));
}

#[test]
fn changing_an_occupied_shelf_filter_is_a_rejected_noop() {
    let mut game = engine(5006);
    let poles = item(game.content(), "item.poles");
    let storeroom = game.content().room_idx("room.storeroom").unwrap();
    let room_id = {
        let state = game.state_mut_for_test();
        let room = state
            .tower
            .floors
            .iter_mut()
            .flat_map(|floor| floor.rooms.iter_mut())
            .find(|room| room.def == storeroom)
            .expect("fixture has storage");
        room.shelves[0].item = Some(poles);
        room.shelves[0].count = 1;
        room.shelves[0].filter = None;
        room.id
    };
    let before = game.state_hash();

    let result = game.try_send(GameCommand::SetShelfFilter {
        room: room_id,
        shelf: 0,
        item: Some("item.bamboo".into()),
    });
    assert!(matches!(result, Err(CommandError::ShelfOccupied { .. })));
    assert_eq!(game.state_hash(), before, "a rejected filter mutated state");
}

#[test]
fn only_a_dedicated_storeroom_accepts_shelf_filters() {
    let mut game = engine(5007);
    let heart = game
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .find(|room| game.content().room(room.def).category == crate::content::RoomCategory::Heart)
        .expect("fixture has a Heartseed")
        .id;
    let before = game.state_hash();
    assert!(matches!(
        game.try_send(GameCommand::SetShelfFilter {
            room: heart,
            shelf: 0,
            item: Some("item.poles".into()),
        }),
        Err(CommandError::NotAStoreroom { .. })
    ));
    assert_eq!(game.state_hash(), before);
}

#[test]
fn an_empty_inactive_room_can_be_refitted_without_losing_identity() {
    let mut game = engine(5008);
    let storeroom = game.content().room_idx("room.storeroom").unwrap();
    let room_id = {
        let state = game.state_mut_for_test();
        let room = state
            .tower
            .floors
            .iter_mut()
            .flat_map(|floor| floor.rooms.iter_mut())
            .find(|room| room.def == storeroom)
            .expect("fixture has a storeroom");
        room.active = false;
        for shelf in &mut room.shelves {
            shelf.item = None;
            shelf.count = 0;
        }
        // A refit moves the machine; it does not silently erase the
        // work already inside its mechanism.
        room.progress = 17;
        room.work_acc = 23;
        room.burn_acc = 29;
        room.id
    };
    let (floor, slot) = game
        .state()
        .tower
        .floors
        .iter()
        .find_map(|floor| {
            (0..floor.slots.saturating_sub(2)).find_map(|slot| {
                (!game.state().tower.slot_range_blocked(floor.index, slot, 2))
                    .then_some((floor.index, slot))
            })
        })
        .expect("fixture has a clear two-slot cell");
    game.try_send(GameCommand::RelocateRoom {
        room: room_id,
        floor,
        slot,
    })
    .expect("empty inactive room should refit");
    let moved = game
        .state()
        .tower
        .floor(floor)
        .unwrap()
        .rooms
        .iter()
        .find(|room| room.id == room_id)
        .expect("same room identity remains");
    assert_eq!(moved.slot, slot);
    assert!(!moved.active);
    assert_eq!(
        (moved.progress, moved.work_acc, moved.burn_acc),
        (17, 23, 29)
    );
}

#[test]
fn active_or_loaded_rooms_cannot_be_refitted() {
    let mut game = engine(5009);
    let storeroom = game.content().room_idx("room.storeroom").unwrap();
    let room_id = game
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .find(|room| room.def == storeroom)
        .unwrap()
        .id;
    assert!(matches!(
        game.try_send(GameCommand::RelocateRoom {
            room: room_id,
            floor: 4,
            slot: 5
        }),
        Err(CommandError::RoomStillActive { .. })
    ));
}

#[test]
fn an_inactive_loaded_front_defence_keeps_its_rack_when_refitted() {
    let mut game = engine(5010);
    crate::harness::open_the_armoury(&mut game, "room.dart_battery");
    crate::tests::stock_for(&mut game, "room.dart_battery", 2);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.dart_battery".into(),
        floor: 3,
        slot: 9,
    })
    .expect("fixture can stand a front defence");
    let darts = crate::tests::item(game.content(), "item.darts");
    let (room_id, loaded) = {
        let state = game.state_mut_for_test();
        let room = state.tower.floors[3]
            .rooms
            .iter_mut()
            .find(|room| room.slot == 9)
            .expect("battery stands at the front");
        room.active = false;
        let rack = room
            .inputs
            .iter_mut()
            .find(|stack| stack.item == darts)
            .expect("battery has a dart rack");
        rack.count = 5;
        (room.id, rack.count)
    };

    game.try_send(GameCommand::RelocateRoom {
        room: room_id,
        floor: 2,
        slot: 9,
    })
    .expect("a switched-off front defence can move with its rack loaded");

    let moved = game.state().tower.floors[2]
        .rooms
        .iter()
        .find(|room| room.id == room_id)
        .expect("the same battery moved");
    assert_eq!(
        moved
            .inputs
            .iter()
            .find(|stack| stack.item == darts)
            .unwrap()
            .count,
        loaded,
        "relocation discarded committed ammunition"
    );
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
fn intake_that_reaches_the_ground_stays_near_it() {
    // `max_floor` still means what it meant. The **cutter arm** no
    // longer carries it (`SYSTEMS.md` §6.24) — bamboo grows tall, and an
    // arm on floor five cuts at floor five's height — but a salvage rig
    // genuinely reaches down into a ruin and cannot do that from the
    // roof.
    let mut game = engine(9);
    let error = game
        .try_send(GameCommand::PlaceRoom {
            room: "room.salvage_rig".into(),
            floor: 3,
            slot: 3,
        })
        .expect_err("a rig reaches the ground, not the roof");
    assert!(matches!(error, CommandError::FloorTooHigh { .. }));
}

#[test]
fn a_floor_that_harvests_cannot_also_shoot() {
    // **The trade the uncap creates** (§6.24). A cutter arm is
    // `front_only` and two slots wide; a floor's weapons deck is two
    // slots. So an arm fills it, and every storey is a choice between
    // feeding the tower and defending it.
    let mut game = crate::tests::engine(10);
    crate::tests::stock_poles(&mut game, 60);
    let slots = game.state().tower.floors[3].slots;
    let arm_width = game
        .content()
        .room_idx("room.cutter_arm")
        .map(|idx| game.content().room(idx).width)
        .expect("the pack defines a cutter arm");

    game.try_send(GameCommand::PlaceRoom {
        room: "room.cutter_arm".into(),
        floor: 3,
        slot: slots - arm_width,
    })
    .expect("an arm goes on any floor's leading edge now");

    let gun_width = game
        .content()
        .room_idx("room.thorn_gun")
        .map(|idx| game.content().room(idx).width)
        .expect("the pack defines a thorn gun");
    let err = game
        .try_send(GameCommand::PlaceRoom {
            room: "room.thorn_gun".into(),
            floor: 3,
            slot: slots - gun_width,
        })
        .expect_err("the arm is standing where the gun would");
    assert!(
        matches!(err, CommandError::SlotOccupied { .. }),
        "expected the deck to be full, got {err:?}"
    );
}

#[test]
fn a_tower_can_harvest_from_more_than_one_floor() {
    // **The uncap itself, and it was the ceiling on the whole economy.**
    // With `max_floor: 1` and `front_only`, a tower could own exactly
    // one arm: one per floor's leading edge, two floors allowed, and
    // the starting thorn gun already on floor 0's. `examples/lift.rs`
    // measured a tower whose hauls plateaued near 225 whatever crew or
    // cars it was given — because there was only ever one intake.
    let mut game = crate::tests::engine(11);
    let mut placed = 0;
    for floor in 1..5u8 {
        crate::tests::stock_poles(&mut game, 20);
        let slots = game.state().tower.floors[floor as usize].slots;
        let width = game
            .content()
            .room_idx("room.cutter_arm")
            .map(|idx| game.content().room(idx).width)
            .expect("the pack defines a cutter arm");
        if game
            .try_send(GameCommand::PlaceRoom {
                room: "room.cutter_arm".into(),
                floor,
                slot: slots - width,
            })
            .is_ok()
        {
            placed += 1;
        }
    }
    assert!(
        placed >= 2,
        "a tower could only place {placed} arm(s); intake is still capped at one"
    );
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
        state.stock_of(poles) >= 16,
        "the founding stores have to survive having no storeroom to sit in"
    );
}

#[test]
fn the_first_turn_offers_one_new_card() {
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
        vec!["room.bunk", "room.thorn_gun"],
        "only already-owned rooms should be intrinsically ungated; the standing Heartseed opens the cutter"
    );

    game.try_send(GameCommand::PlaceRoom {
        room: "room.cutter_arm".into(),
        floor: 1,
        slot: 8,
    })
    .expect("the standing Heartseed should offer the cutter on turn one");

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
    // Heartseed, then cutter arm, then burner. The Garden is an
    // optional branch after survival income exists.
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
    assert!(blocked(&mut game, "room.burner", 1, 5));
    game.try_send(GameCommand::PlaceRoom {
        room: "room.cutter_arm".into(),
        floor: 1,
        slot: 8,
    })
    .expect("the Heartseed opened the cutter arm");

    assert!(blocked(&mut game, "room.mill", 1, 5));
    assert!(blocked(&mut game, "room.garden", 1, 3));
    game.try_send(GameCommand::PlaceRoom {
        room: "room.burner".into(),
        floor: 1,
        slot: 5,
    })
    .expect("the cutter arm opened the burner");

    // And now the first branches are open.
    crate::tests::stock_item(&mut game, "item.poles", 60);
    game.try_send(GameCommand::BuildFloor).expect("affordable");
    game.try_send(GameCommand::PlaceRoom {
        room: "room.mill".into(),
        floor: 2,
        slot: 1,
    })
    .expect("the burner opened the basic production branch");
}

#[test]
fn the_burner_opens_branches_instead_of_the_whole_catalog() {
    let content = content();
    let burner = content.room_idx("room.burner").expect("burner exists");
    let mut directly_opened: Vec<_> = content
        .rooms
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            content
                .room_rt(crate::ids::RoomIdx(*index as u16))
                .unlocked_by
                == Some(burner)
        })
        .map(|(_, room)| room.id.as_str())
        .collect();
    directly_opened.sort_unstable();

    assert_eq!(
        directly_opened,
        [
            "room.cell_bank",
            "room.fiber_comb",
            "room.garden",
            "room.mill",
            "room.salvage_rig",
            "room.thornwright",
        ],
        "the first generator opened an undifferentiated future catalog"
    );

    for (room, prerequisite) in [
        ("room.canteen", "room.mill"),
        ("room.ropery", "room.fiber_comb"),
        ("room.resonator_works", "room.garden"),
        ("room.sun_forge", "room.salvage_rig"),
        ("room.fitter", "room.sun_forge"),
        ("room.cellwright", "room.sun_forge"),
    ] {
        let room = content.room_idx(room).expect("staged room exists");
        let prerequisite = content
            .room_idx(prerequisite)
            .expect("staged prerequisite exists");
        assert_eq!(content.room_rt(room).unlocked_by, Some(prerequisite));
    }
}

#[test]
fn the_garden_tends_itself_without_consuming_a_porter() {
    // The Garden already commits roof space, sunlight, hauling and five
    // poles. Its former permanent staffing requirement made resin-first
    // lose the elevator on every measured seed, so it now grows without
    // taking one of the opening's three porters out of circulation.
    let content = content();
    let produce = item(&content, "item.resin_feedstock");
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

    let garden = content
        .room_idx("room.garden")
        .map(|idx| content.room(idx))
        .expect("the Garden exists");
    assert_eq!(garden.crew_required, 0);

    let mut game = crate::tests::engine(40);
    game.state_mut_for_test().clock.tick_of_day = noon;
    game.step(2400);
    assert!(grown(&game) > 0, "an unstaffed Garden grew nothing at noon");
}

// ---------------------------------------------------------------------------
// Growing sideways (`SYSTEMS.md` §6.16)
// ---------------------------------------------------------------------------

#[test]
fn widening_adds_slots_at_the_back_and_keeps_the_front() {
    // **The whole reason it grows backwards.** Weapons are `front_only`
    // (§6.13), so a hull that grew at the nose would leave every gun it
    // owns standing two slots *inside* the tower — at the place the
    // front used to be. Adding at the back and sliding everything up
    // costs a loop and keeps the leading edge where it was.
    let content = content();
    let mut game = crate::tests::opening(40);
    crate::tests::stock_item(&mut game, "item.poles", 60);

    let by = content.balance.tower.widen_slots;
    let before: Vec<(u8, u8)> = game
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter().map(move |room| (floor.index, room.slot)))
        .collect();
    let width_before = game.state().tower.floors[0].slots;

    game.try_send(GameCommand::WidenTower)
        .expect("affordable, and the hull is not at its widest");

    let after: Vec<(u8, u8)> = game
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter().map(move |room| (floor.index, room.slot)))
        .collect();

    assert_eq!(game.state().tower.floors[0].slots, width_before + by);
    assert_eq!(
        after,
        before
            .iter()
            .map(|(floor, slot)| (*floor, slot + by))
            .collect::<Vec<_>>(),
        "the rooms did not slide back with the hull"
    );

    // And the front weapon is still on the front: the gun the tower
    // ships with sits at `slots - width` before and after.
    let gun = game
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .find(|room| game.content().room(room.def).id == "room.thorn_gun")
        .expect("the tower sets out with one");
    assert_eq!(
        gun.slot,
        game.state().tower.floors[0].slots - game.content().room(gun.def).width,
        "widening left the gun behind its own leading edge"
    );
}

#[test]
fn a_widened_hull_has_somewhere_new_to_build() {
    // The purchase has to *buy* something, or it is a number going up.
    let mut game = crate::tests::opening(41);
    crate::tests::stock_item(&mut game, "item.poles", 90);

    // Fill the ground floor's usable span, so nothing more fits.
    let mut placed = 0;
    while game
        .try_send(GameCommand::PlaceRoom {
            room: "room.bunk".into(),
            floor: 1,
            slot: 3 + placed * 2,
        })
        .is_ok()
    {
        placed += 1;
        crate::tests::stock_item(&mut game, "item.poles", 20);
    }
    let full = placed;
    assert!(full > 0, "nothing fitted on floor 1 to begin with");

    game.try_send(GameCommand::WidenTower).expect("affordable");
    crate::tests::stock_item(&mut game, "item.poles", 20);

    // **The new deck is at the back**, which follows from where the
    // frame went: everything aboard slid *up* by `widen_slots`, so the
    // slots it vacated are the low ones. Slot 0 is new hull, and it is
    // new hull the stairs used to stand in.
    let _ = full;
    assert!(
        game.try_send(GameCommand::PlaceRoom {
            room: "room.bunk".into(),
            floor: 1,
            slot: 0,
        })
        .is_ok(),
        "a widened hull had nowhere new to put anything"
    );
}

#[test]
fn the_hull_stops_widening_somewhere() {
    // `max_slots` is the same promise `max_floors` makes vertically:
    // the cross-section fits on one screen, which the whole art
    // direction rests on.
    let content = content();
    let mut game = crate::tests::opening(42);
    let mut widenings = 0;
    loop {
        crate::tests::stock_item(&mut game, "item.poles", 40);
        match game.try_send(GameCommand::WidenTower) {
            Ok(()) => widenings += 1,
            Err(CommandError::AlreadyWidest { .. }) => break,
            Err(other) => panic!("widening failed for the wrong reason: {other:?}"),
        }
        assert!(widenings < 50, "the hull widened without limit");
    }
    assert!(widenings > 0, "the hull could never widen at all");
    assert!(
        game.state().tower.floors[0].slots <= content.balance.tower.max_slots,
        "the hull went past its own ceiling"
    );
}

// ---------------------------------------------------------------------------
// The weapons' deck (`SYSTEMS.md` §6.21)
// ---------------------------------------------------------------------------

#[test]
fn an_ordinary_room_cannot_stand_on_the_weapons_deck() {
    // `front_only` said a gun must be at the front. It did not say the
    // front was *for* guns, so a storeroom could take the edge and the
    // floor became unarmable with nothing saying why.
    let content = content();
    let mut game = crate::tests::engine(1800);
    crate::tests::stock_poles(&mut game, 60);

    let slots = game.state().tower.floors[1].slots;
    let deck_from = slots - content.balance.tower.front_slots;
    let err = game
        .try_send(GameCommand::PlaceRoom {
            room: "room.storeroom".into(),
            floor: 3,
            slot: deck_from,
        })
        .expect_err("that column belongs to the weapons");
    assert!(
        matches!(err, CommandError::OnTheWeaponsDeck { .. }),
        "{err:?}"
    );
}

#[test]
fn a_wide_room_cannot_lean_onto_the_deck_either() {
    // Checked against the whole footprint, not the left edge — the same
    // reason `slot_range_blocked` is. A two-wide room one slot short of
    // the deck still covers the first column of it.
    let content = content();
    let mut game = crate::tests::engine(1801);
    crate::tests::stock_poles(&mut game, 60);

    let slots = game.state().tower.floors[1].slots;
    let deck_from = slots - content.balance.tower.front_slots;
    let width = content
        .room_idx("room.storeroom")
        .map(|idx| content.room(idx).width)
        .expect("the pack defines a storeroom");
    assert!(width > 1, "this test needs a room wider than one slot");

    let err = game
        .try_send(GameCommand::PlaceRoom {
            room: "room.storeroom".into(),
            floor: 3,
            slot: deck_from + 1 - width,
        })
        .expect_err("its far edge is on the deck");
    assert!(
        matches!(err, CommandError::OnTheWeaponsDeck { .. }),
        "{err:?}"
    );
}

#[test]
fn a_weapon_still_reaches_the_edge_it_is_reserved() {
    // The point of reserving it. A gun goes exactly where `front_only`
    // puts it, and the deck is what keeps that column free.
    let mut game = crate::tests::engine(1802);
    crate::tests::stock_poles(&mut game, 60);
    let slots = game.state().tower.floors[1].slots;
    let width = game
        .content()
        .room_idx("room.thorn_gun")
        .map(|idx| game.content().room(idx).width)
        .expect("the pack defines a thorn gun");

    game.try_send(GameCommand::PlaceRoom {
        room: "room.thorn_gun".into(),
        floor: 3,
        slot: slots - width,
    })
    .expect("the leading edge is exactly where a weapon goes");
}

#[test]
fn the_deck_is_wide_enough_for_the_widest_weapon() {
    // A deck that could not hold the widest weapon in the pack would be
    // a deck with a footnote.
    let content = content();
    let widest = content
        .rooms
        .iter()
        .filter(|room| room.front_only)
        .map(|room| room.width)
        .max()
        .expect("the pack has weapons");
    assert!(
        content.balance.tower.front_slots >= widest,
        "front_slots {} is narrower than the widest weapon ({widest})",
        content.balance.tower.front_slots
    );
}
