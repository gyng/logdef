use crate::command::{CommandError, GameCommand};
use crate::content::ShaftKind;
use crate::tests::{engine, stock_for_shaft};

#[test]
fn a_vent_stack_has_to_reach_the_roof() {
    let mut game = engine(0xE7A0_0001);
    stock_for_shaft(&mut game, "shaft.vent_stack", 1);
    let before = crate::replay::hash_state(game.state());
    let error = game
        .try_send(GameCommand::BuildShaft {
            shaft: "shaft.vent_stack".into(),
            low: 0,
            high: 3,
            slot: 7,
        })
        .expect_err("a flue ending below the roof vents into the tower");
    assert_eq!(error, CommandError::VentMustReachRoof { high: 3, roof: 4 });
    assert_eq!(crate::replay::hash_state(game.state()), before);
}

#[test]
fn an_intact_roof_stack_serves_rooms_above_its_low_end() {
    let mut game = engine(0xE7A0_0002);
    stock_for_shaft(&mut game, "shaft.vent_stack", 1);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.vent_stack".into(),
        low: 2,
        high: 4,
        slot: 7,
    })
    .expect("a clear roof-reaching stack should build");

    let stack = game
        .state()
        .tower
        .shafts
        .iter()
        .find(|shaft| shaft.kind == ShaftKind::VentStack)
        .expect("the stack exists");
    assert_eq!(game.content().shaft(stack.def).exhaust_capacity, 5);
    assert_eq!(stack.low, 2);
    assert_eq!(stack.high, game.state().tower.top_floor());
}

#[test]
fn a_stack_must_be_extended_after_the_roof_grows() {
    let mut game = engine(0xE7A0_0004);
    stock_for_shaft(&mut game, "shaft.vent_stack", 2);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.vent_stack".into(),
        low: 2,
        high: 4,
        slot: 7,
    })
    .unwrap();
    let id = game
        .state()
        .tower
        .shafts
        .iter()
        .find(|shaft| shaft.kind == ShaftKind::VentStack)
        .unwrap()
        .id;
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::BuildFloor).unwrap();
    assert_eq!(game.state().tower.shaft(id).unwrap().high, 4);
    game.try_send(GameCommand::ExtendShaft { shaft: id, high: 5 })
        .expect("paid clear extension reaches the new roof");
    assert_eq!(game.state().tower.shaft(id).unwrap().high, 5);
}

#[test]
fn severing_the_stack_marks_a_ready_forge_unvented() {
    let mut game = engine(0xE7A0_0003);
    crate::tests::stock_for(&mut game, "room.salvage_rig", 1);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.salvage_rig".into(),
        floor: 1,
        slot: 3,
    })
    .expect("the forge's physical prerequisite should stand");
    stock_for_shaft(&mut game, "shaft.vent_stack", 1);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.vent_stack".into(),
        low: 0,
        high: 4,
        slot: 7,
    })
    .expect("stack should build");

    // Put a forge in a clear cell and feed it directly: this test is
    // about the utility, not whether a porter happened to arrive.
    game.try_send(GameCommand::RemoveRoom { floor: 3, slot: 5 })
        .expect("clear the adjacent test cell");
    crate::tests::stock_for(&mut game, "room.sun_forge", 1);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.sun_forge".into(),
        floor: 3,
        slot: 5,
    })
    .expect("forge should build");
    let scrap = crate::tests::item(game.content(), "item.scrap");
    let state = game.state_mut_for_test();
    let forge = state
        .tower
        .floor_mut(3)
        .unwrap()
        .rooms
        .iter_mut()
        .find(|room| room.covers(5))
        .unwrap();
    forge
        .inputs
        .iter_mut()
        .find(|s| s.item == scrap)
        .unwrap()
        .count = 3;

    game.step(1);
    assert!(
        !game
            .state()
            .tower
            .floor(3)
            .unwrap()
            .room_at(5)
            .unwrap()
            .exhaust_refused
    );

    let stack = game
        .state_mut_for_test()
        .tower
        .shafts
        .iter_mut()
        .find(|shaft| shaft.kind == ShaftKind::VentStack)
        .unwrap();
    stack.health.hp = 0;
    game.step(1);
    assert!(
        game.state()
            .tower
            .floor(3)
            .unwrap()
            .room_at(5)
            .unwrap()
            .exhaust_refused
    );
}
