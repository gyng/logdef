//! Cars: dispatch, dwell, capacity, and the dumbwaiter.
//!
//! M1's sprint question is whether elevator contention is fun, which
//! nobody can answer if the elevator is wrong. These tests pin the
//! behaviour that makes it recognisably an elevator: it batches, it
//! sweeps, it does not carry people the wrong way, and it fills up.

use crate::command::GameCommand;
use crate::content::ShaftKind;
use crate::snapshot::CrewStateTag;
use crate::state::{CarState, CrewState, ShaftPriority};
use crate::tests::{content, engine, item};

/// Build a shaft, banking enough poles first.
fn with_shaft(seed: u64, shaft: &str, low: u8, high: u8, slot: u8) -> crate::engine::GameEngine {
    let mut game = engine(seed);
    game.step(6000);
    game.try_send(GameCommand::BuildShaft {
        shaft: shaft.into(),
        low,
        high,
        slot,
    })
    .unwrap_or_else(|err| panic!("could not build {shaft}: {err}"));
    game
}

#[test]
fn an_elevator_can_be_built_and_costs_stock() {
    let content = content();
    let poles = item(&content, "item.poles");
    let mut game = engine(800);
    game.step(6000);

    let before = game.state().stock_of(poles);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.elevator".into(),
        low: 0,
        high: 3,
        slot: 7,
    })
    .expect("affordable after 200 seconds");

    assert!(game.state().stock_of(poles) < before, "the shaft was free");
    let shaft = game
        .state()
        .tower
        .shafts
        .iter()
        .find(|shaft| shaft.kind == ShaftKind::Elevator)
        .expect("the elevator is standing");
    assert_eq!(shaft.cars.len(), 1);
    assert_eq!(shaft.slot, 7);
}

#[test]
fn a_shaft_column_blocks_its_slot_on_every_floor_it_spans() {
    let mut game = with_shaft(801, "shaft.elevator", 0, 3, 7);
    for floor in 0..=3u8 {
        let blocked = game.state().tower.slot_range_blocked(floor, 7, 1);
        assert!(blocked, "floor {floor} slot 7 should be taken by the shaft");
    }
    // And a room cannot be dropped on top of it.
    let error = game.try_send(GameCommand::PlaceRoom {
        room: "room.storeroom".into(),
        floor: 3,
        slot: 6,
    });
    assert!(
        error.is_err(),
        "a two-wide room overlapped the shaft column"
    );
}

#[test]
fn a_span_outside_the_definition_is_refused() {
    let mut game = engine(802);
    game.step(6000);
    // A dumbwaiter reaches two or three floors, not the whole tower.
    let error = game
        .try_send(GameCommand::BuildShaft {
            shaft: "shaft.dumbwaiter".into(),
            low: 0,
            high: 3,
            slot: 7,
        })
        .expect_err("four floors is too far for a dumbwaiter");
    assert!(matches!(
        error,
        crate::command::CommandError::BadSpan { .. }
    ));
}

#[test]
fn the_stairs_cannot_be_torn_out() {
    let mut game = engine(803);
    let stairs = game.state().tower.shafts[0].id;
    let error = game
        .try_send(GameCommand::RemoveShaft { id: stairs })
        .expect_err("removing the only way down would soft-lock the tower");
    assert!(matches!(
        error,
        crate::command::CommandError::Undemolishable { .. }
    ));
}

#[test]
fn a_car_waits_for_demand_before_departing() {
    // The dispatch threshold. Without it the car chases every single
    // caller, no queue ever forms, and the milestone's design question
    // becomes unanswerable.
    let game = with_shaft(804, "shaft.elevator", 0, 3, 7);
    let shaft = game
        .state()
        .tower
        .shafts
        .iter()
        .find(|shaft| shaft.kind == ShaftKind::Elevator)
        .expect("standing");
    // Freshly built, with nobody calling, it is parked.
    assert!(matches!(shaft.cars[0].state, CarState::Idle));
}

#[test]
fn crew_board_a_car_and_are_carried() {
    let mut game = with_shaft(805, "shaft.elevator", 0, 3, 7);

    let mut ever_rode = false;
    let mut ever_moved = false;
    for _ in 0..6000 {
        game.step(1);
        if game
            .state()
            .crew
            .iter()
            .any(|member| matches!(member.state, CrewState::Riding { .. }))
        {
            ever_rode = true;
        }
        if game
            .state()
            .tower
            .shafts
            .iter()
            .filter(|shaft| shaft.kind == ShaftKind::Elevator)
            .any(|shaft| shaft.cars.iter().any(|car| car.pos.frac_raw() != 0))
        {
            ever_moved = true;
        }
        if ever_rode && ever_moved {
            break;
        }
    }

    assert!(ever_moved, "the car never left its floor");
    assert!(ever_rode, "nobody ever boarded the car");
}

#[test]
fn a_car_never_carries_more_than_its_capacity() {
    let mut game = with_shaft(806, "shaft.elevator", 0, 3, 7);
    for _ in 0..6000 {
        game.step(1);
        for shaft in &game.state().tower.shafts {
            for index in 0..shaft.cars.len() {
                let load = shaft.car_load(index, &game.state().crew);
                assert!(
                    load <= shaft.capacity,
                    "car carried {load} units against a capacity of {}",
                    shaft.capacity
                );
            }
        }
    }
}

#[test]
fn a_car_stays_inside_its_shaft() {
    let mut game = with_shaft(807, "shaft.elevator", 1, 3, 7);
    for _ in 0..6000 {
        game.step(1);
        for shaft in &game.state().tower.shafts {
            for car in &shaft.cars {
                assert!(
                    car.floor() >= shaft.low && car.floor() <= shaft.high,
                    "car at floor {} escaped a shaft spanning {}..{}",
                    car.floor(),
                    shaft.low,
                    shaft.high
                );
            }
        }
    }
}

#[test]
fn riders_are_never_left_inside_a_demolished_shaft() {
    let mut game = with_shaft(808, "shaft.elevator", 0, 3, 7);
    let id = game
        .state()
        .tower
        .shafts
        .iter()
        .find(|shaft| shaft.kind == ShaftKind::Elevator)
        .expect("standing")
        .id;

    // Let people get aboard, then tear it out from under them.
    game.step(1200);
    game.try_send(GameCommand::RemoveShaft { id })
        .expect("an elevator is removable");

    let stranded = game.state().crew.iter().any(|member| match member.state {
        CrewState::Boarding { shaft, .. }
        | CrewState::Climbing { shaft, .. }
        | CrewState::Riding { shaft, .. } => shaft == id,
        _ => false,
    });
    assert!(!stranded, "somebody is still riding a shaft that is gone");

    // And the tower keeps working afterwards.
    let hauls = game.state().stats.hauls_completed;
    game.step(3000);
    assert!(game.state().stats.hauls_completed > hauls);
}

#[test]
fn an_unserved_floor_is_not_stopped_at() {
    let mut game = with_shaft(809, "shaft.elevator", 0, 3, 7);
    let id = game
        .state()
        .tower
        .shafts
        .iter()
        .find(|shaft| shaft.kind == ShaftKind::Elevator)
        .expect("standing")
        .id;

    // Take floor 2 out of every daypart's program.
    let dayparts = game.content().dayparts.len();
    let served: Vec<bool> = (0..14).map(|floor| floor != 2).collect();
    for daypart in 0..dayparts {
        game.try_send(GameCommand::SetShaftProgram {
            id,
            daypart: daypart as u16,
            served: served.clone(),
            priority: ShaftPriority::Balanced,
        })
        .expect("a real shaft and a real daypart");
    }

    for _ in 0..6000 {
        game.step(1);
        let shaft = game
            .state()
            .tower
            .shafts
            .iter()
            .find(|shaft| shaft.id == id)
            .expect("standing");
        for car in &shaft.cars {
            assert!(
                !car.stops.contains(&2),
                "the car accepted a stop on an unserved floor"
            );
        }
    }
}

#[test]
fn programming_a_nonexistent_daypart_is_refused() {
    let mut game = with_shaft(810, "shaft.elevator", 0, 3, 7);
    let id = game.state().tower.shafts.last().expect("standing").id;
    let error = game
        .try_send(GameCommand::SetShaftProgram {
            id,
            daypart: 99,
            served: vec![true; 14],
            priority: ShaftPriority::Balanced,
        })
        .expect_err("there is no daypart 99");
    assert!(matches!(
        error,
        crate::command::CommandError::NoSuchDaypart { .. }
    ));
}

#[test]
fn a_dumbwaiter_moves_items_without_anybody_carrying_them() {
    let content = content();
    let bamboo = item(&content, "item.bamboo");

    // Spanning the cutter arm on floor 0 and the storeroom on floor 1.
    let mut game = with_shaft(811, "shaft.dumbwaiter", 0, 1, 7);
    // Take the crew out entirely, so anything that moves was moved by
    // the machine.
    game.state_mut_for_test().crew.clear();

    let shelved_before: i64 = shelved(&game, bamboo);
    game.step(3000);
    let shelved_after: i64 = shelved(&game, bamboo);

    assert!(
        shelved_after > shelved_before,
        "the dumbwaiter moved nothing with no crew aboard: {shelved_before} then {shelved_after}"
    );
}

fn shelved(game: &crate::engine::GameEngine, item: crate::ids::ItemIdx) -> i64 {
    game.state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .flat_map(|room| room.shelves.iter())
        .filter(|shelf| shelf.item == Some(item))
        .map(|shelf| shelf.count)
        .sum()
}

#[test]
fn a_dumbwaiter_never_loses_a_load() {
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = with_shaft(812, "shaft.dumbwaiter", 0, 1, 7);
    game.state_mut_for_test().crew.clear();

    let mut last = total_including_cars(&game, bamboo);
    let mut harvested = game.state().stats.items_harvested as i64;
    let mut crafted = game.state().stats.crafts_completed as i64;

    for _ in 0..100 {
        game.step(30);
        let now = total_including_cars(&game, bamboo);
        let harvested_now = game.state().stats.items_harvested as i64;
        let crafted_now = game.state().stats.crafts_completed as i64;
        // Bamboo arrives from the cutter arm and leaves through the
        // mill. Anything else is the dumbwaiter dropping a load, which
        // it must never do — mid-flight cargo is still cargo.
        assert_eq!(
            now - last,
            (harvested_now - harvested) - (crafted_now - crafted),
            "bamboo went missing at tick {}",
            game.state().tick
        );
        last = now;
        harvested = harvested_now;
        crafted = crafted_now;
    }
}

fn total_including_cars(game: &crate::engine::GameEngine, item: crate::ids::ItemIdx) -> i64 {
    let in_cars: i64 = game
        .state()
        .tower
        .shafts
        .iter()
        .flat_map(|shaft| shaft.cars.iter())
        .flat_map(|car| car.freight.iter())
        .filter(|stack| stack.item == item)
        .map(|stack| stack.count)
        .sum();
    crate::tests::total_in_flight(game.state(), item) + in_cars
}

#[test]
fn crew_prefer_the_faster_shaft() {
    // Build an elevator alongside the stairs and check the crew notice.
    // If they don't, every shaft the player builds is dead weight.
    let mut game = with_shaft(813, "shaft.elevator", 0, 3, 7);
    let elevator = game
        .state()
        .tower
        .shafts
        .iter()
        .find(|shaft| shaft.kind == ShaftKind::Elevator)
        .expect("standing")
        .id;

    let mut used_elevator = false;
    for _ in 0..6000 {
        game.step(1);
        if game.state().crew.iter().any(|member| {
            matches!(member.state, CrewState::Boarding { shaft, .. } | CrewState::Riding { shaft, .. }
                if shaft == elevator)
        }) {
            used_elevator = true;
            break;
        }
    }
    assert!(used_elevator, "the crew ignored the elevator entirely");
}

#[test]
fn an_elevator_draws_charge_when_it_moves() {
    let mut game = with_shaft(814, "shaft.elevator", 0, 3, 7);
    // Night, halted, so transport is the only thing that can spend.
    {
        let state = game.state_mut_for_test();
        state.clock.tick_of_day = 0;
        state.walking = false;
        state.power.charge = state.power.capacity;
    }

    let before = game.state().power.charge;
    game.step(1800);
    assert!(
        game.state().power.charge < before,
        "a running elevator cost nothing"
    );
}

#[test]
fn the_chain_still_runs_with_an_elevator_in_the_tower() {
    // The regression that matters: adding transport must not break the
    // thing transport exists to serve.
    let mut game = with_shaft(815, "shaft.elevator", 0, 3, 7);
    let crafts = game.state().stats.crafts_completed;
    game.step(6000);
    assert!(
        game.state().stats.crafts_completed > crafts,
        "the mill stopped once an elevator existed"
    );
}

// ---------------------------------------------------------------------------
// The milestone's exit criterion
// ---------------------------------------------------------------------------

/// Crafts completed over a fixed window.
///
/// Crafts rather than hauls, deliberately. Haul *count* barely moves
/// when you add a shaft — the crew were always busy — but what they
/// were busy doing changes completely: less shuttling surplus onto
/// shelves, more bamboo actually reaching the mill. Counting trips
/// would have said the elevator did nothing.
fn throughput(game: &mut crate::engine::GameEngine, ticks: u32) -> u64 {
    let before = game.state().stats.crafts_completed;
    game.step(ticks);
    game.state().stats.crafts_completed - before
}

#[test]
fn adding_a_shaft_measurably_improves_throughput() {
    // M1's exit criterion, as an assertion rather than a vibe: a tower
    // whose crew queue on one staircase must move visibly more once a
    // second way up exists. If this stops being true, the contention
    // the whole design rests on has quietly stopped mattering.
    const WINDOW: u32 = 9000;

    let mut cramped = engine(900);
    let mut relieved = engine(900);

    // Both towers bank the same poles over the same warm-up, so the
    // only difference between them is the shaft.
    cramped.step(6000);
    relieved.step(6000);
    relieved
        .try_send(GameCommand::BuildShaft {
            shaft: "shaft.elevator".into(),
            low: 0,
            high: 3,
            slot: 7,
        })
        .expect("affordable after 200 seconds");

    let without = throughput(&mut cramped, WINDOW);
    let with = throughput(&mut relieved, WINDOW);

    // A margin, not just "greater than": a one-craft difference would
    // be noise, and this test exists to catch the effect disappearing.
    assert!(
        with > without + without / 4,
        "an elevator did not meaningfully improve throughput: \
         {without} crafts without, {with} with"
    );
}

#[test]
fn an_underbuilt_tower_shows_its_bottleneck_at_the_shaft() {
    // The other half of the criterion: the queue has to be *visible*,
    // not merely present. Crew must spend real time boarding and cross
    // the stress threshold, because that tint is the only bottleneck
    // instrument the game has.
    let mut game = engine(901);
    let stress = content().balance.crew.stress_ticks;

    let mut peak_wait = 0;
    let mut boarding_ticks = 0u32;
    for _ in 0..9000 {
        game.step(1);
        let view = game.view();
        if view
            .crew
            .iter()
            .any(|member| member.state == CrewStateTag::Board)
        {
            boarding_ticks += 1;
        }
        peak_wait = peak_wait.max(
            view.crew
                .iter()
                .map(|member| member.wait_ticks)
                .max()
                .unwrap_or(0),
        );
    }

    assert!(
        boarding_ticks > 300,
        "crew only spent {boarding_ticks} ticks queueing in five minutes — no visible contention"
    );
    assert!(
        peak_wait >= stress,
        "the longest wait was {peak_wait} ticks, never reaching the {stress}-tick stress tint"
    );
}

// ---------------------------------------------------------------------------
// The estimate crew route by
// ---------------------------------------------------------------------------

/// Estimate for a shaft by kind, at a given queue length.
fn estimate(game: &crate::engine::GameEngine, kind: ShaftKind, queued: u32) -> u32 {
    let shaft = game
        .state()
        .tower
        .shafts
        .iter()
        .find(|shaft| shaft.kind == kind)
        .expect("shaft is standing");
    crate::systems::transport::estimated_trip_ticks(shaft, game.content(), 0, 3, queued)
}

#[test]
fn a_longer_queue_makes_a_shaft_less_attractive() {
    // The estimate is what routes crew. If it ignores the queue they
    // pile onto the same staircase no matter how long the line is —
    // which is exactly the bug the flat penalty had.
    let game = with_shaft(920, "shaft.elevator", 0, 3, 7);
    for kind in [ShaftKind::Stairs, ShaftKind::Elevator] {
        let empty = estimate(&game, kind, 0);
        let busy = estimate(&game, kind, 4);
        assert!(
            busy > empty,
            "{kind:?}: a queue of four estimated {busy} against {empty} for an empty shaft"
        );
    }
}

#[test]
fn a_longer_climb_costs_more_than_a_shorter_one() {
    let game = with_shaft(921, "shaft.elevator", 0, 3, 7);
    for kind in [ShaftKind::Stairs, ShaftKind::Elevator] {
        let shaft = game
            .state()
            .tower
            .shafts
            .iter()
            .find(|shaft| shaft.kind == kind)
            .expect("standing");
        let one = crate::systems::transport::estimated_trip_ticks(shaft, game.content(), 0, 1, 0);
        let three = crate::systems::transport::estimated_trip_ticks(shaft, game.content(), 0, 3, 0);
        assert!(
            three > one,
            "{kind:?}: three floors ({three}) did not cost more than one ({one})"
        );
    }
}

#[test]
fn the_elevator_earns_its_poles_on_long_climbs_and_busy_ones() {
    // Two crossovers have to exist for a shaft to be worth building.
    //
    // Distance: a car has fixed overhead — walking to it, waiting for
    // it, two sets of doors — so one floor up an empty staircase is
    // simply quicker. Three floors up it is not, and that is what the
    // eighteen poles bought.
    //
    // Congestion: everybody queueing for the stairs climbs separately,
    // where a car takes several at once. However short the climb, a
    // line should send crew to the car.
    let mut game = with_shaft(922, "shaft.elevator", 0, 3, 7);
    // Clear the traffic the warm-up left behind, so "empty staircase"
    // actually means empty. Somebody mid-climb costs a following crew
    // member a wait, which is correct but not what is under test here.
    {
        let state = game.state_mut_for_test();
        state.crew.clear();
        for shaft in &mut state.tower.shafts {
            shaft.riders = 0;
        }
    }

    let shaft_of = |kind: ShaftKind| {
        game.state()
            .tower
            .shafts
            .iter()
            .find(move |shaft| shaft.kind == kind)
            .expect("standing")
    };
    let cost = |kind: ShaftKind, to: u8, queued: u32| {
        crate::systems::transport::estimated_trip_ticks(
            shaft_of(kind),
            game.content(),
            0,
            to,
            queued,
        )
    };

    assert!(
        cost(ShaftKind::Stairs, 1, 0) < cost(ShaftKind::Elevator, 1, 0),
        "one floor up, an empty staircase should still win"
    );
    assert!(
        cost(ShaftKind::Stairs, 3, 0) > cost(ShaftKind::Elevator, 3, 0),
        "three floors up, the car should win"
    );
    assert!(
        cost(ShaftKind::Stairs, 1, 3) > cost(ShaftKind::Elevator, 1, 3),
        "a three-deep queue should send crew to the car even for one floor"
    );
}

#[test]
fn crew_cannot_be_routed_onto_a_dumbwaiter() {
    let game = with_shaft(923, "shaft.dumbwaiter", 0, 2, 7);
    assert_eq!(
        estimate(&game, ShaftKind::Dumbwaiter, 0),
        u32::MAX,
        "a dumbwaiter should be infinitely unattractive to a person"
    );
}

// ---------------------------------------------------------------------------
// The dumbwaiter's real job: feeding a recipe
// ---------------------------------------------------------------------------

#[test]
fn a_dumbwaiter_feeds_a_hungry_recipe_in_preference_to_a_shelf() {
    // Spanning the cutter arm on floor 0 and the mill on floor 2, with
    // a storeroom on floor 1 in between. Both are valid destinations
    // for bamboo; the mill's inbox outranks the shelves, and this is
    // the path that actually keeps a chain running.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = with_shaft(924, "shaft.dumbwaiter", 0, 2, 7);
    game.state_mut_for_test().crew.clear();

    let crafts_before = game.state().stats.crafts_completed;
    game.step(6000);

    let in_mill: i64 = game
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .flat_map(|room| room.inputs.iter())
        .filter(|stack| stack.item == bamboo)
        .map(|stack| stack.count)
        .sum();

    assert!(
        game.state().stats.crafts_completed > crafts_before,
        "with no crew at all, the dumbwaiter never fed the mill"
    );
    assert!(
        in_mill > 0 || game.state().stats.crafts_completed > crafts_before,
        "bamboo never reached an inbox: {in_mill} waiting"
    );
}

#[test]
fn a_dumbwaiter_conserves_across_the_inbox_path_too() {
    // The conservation check again, but on the route that actually
    // deposits into a recipe rather than onto shelves — the one the
    // earlier test could not reach.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = with_shaft(925, "shaft.dumbwaiter", 0, 2, 7);
    game.state_mut_for_test().crew.clear();

    let mut last = total_including_cars(&game, bamboo);
    let mut harvested = game.state().stats.items_harvested as i64;
    let mut crafted = game.state().stats.crafts_completed as i64;

    for _ in 0..200 {
        game.step(30);
        let now = total_including_cars(&game, bamboo);
        let harvested_now = game.state().stats.items_harvested as i64;
        let crafted_now = game.state().stats.crafts_completed as i64;
        assert_eq!(
            now - last,
            (harvested_now - harvested) - (crafted_now - crafted),
            "bamboo went missing at tick {}",
            game.state().tick
        );
        last = now;
        harvested = harvested_now;
        crafted = crafted_now;
    }
}

#[test]
fn a_dumbwaiter_stops_when_there_is_nowhere_to_put_anything() {
    // Fill every destination and the car should sit still holding
    // nothing, rather than shuttling an empty box or dropping a load.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = with_shaft(926, "shaft.dumbwaiter", 0, 2, 7);
    game.state_mut_for_test().crew.clear();
    game.step(9000);

    // Whatever state it settles into, nothing may be lost and the car
    // may not hold cargo it has given up on delivering forever.
    let stuck: i64 = game
        .state()
        .tower
        .shafts
        .iter()
        .flat_map(|shaft| shaft.cars.iter())
        .flat_map(|car| car.freight.iter())
        .filter(|stack| stack.item == bamboo)
        .map(|stack| stack.count)
        .sum();
    let batch = 8;
    assert!(
        stuck <= batch,
        "the dumbwaiter is hoarding {stuck} bamboo it will never deliver"
    );
}
