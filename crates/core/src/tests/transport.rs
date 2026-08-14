//! Cars: dispatch, dwell, capacity, and the freight the lift moves
//! when nobody is riding it.
//!
//! M1's sprint question is whether elevator contention is fun, which
//! nobody can answer if the elevator is wrong. These tests pin the
//! behaviour that makes it recognisably an elevator: it batches, it
//! sweeps, it does not carry people the wrong way, and it fills up.

use crate::command::{CommandError, GameCommand};
use crate::content::ShaftKind;
use crate::snapshot::CrewStateTag;
use crate::state::{CarState, CrewState, ShaftPriority};
use crate::tests::{content, engine, item};

/// Build a shaft, banking whatever it costs first.
///
/// Reads the cost off the pack rather than assuming poles: the elevator
/// became a tier-two building at M5 and a helper that hands out poles
/// would quietly stop building elevators.
fn with_shaft(seed: u64, shaft: &str, low: u8, high: u8, slot: u8) -> crate::engine::GameEngine {
    let mut game = engine(seed);
    crate::tests::stock_for_shaft(&mut game, shaft, 1);
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
    // Rope and mechanisms **before** the chain runs, not after. From M5
    // they are the other half of an elevator's cost and this tower has
    // no forge, so they have to be handed over — and a shelf holds one
    // kind, so by six thousand ticks in there is no free shelf to hand
    // them to. Stocking at tick zero claims a shelf while shelves are
    // still going spare. Poles are still earned, because that is the
    // half of the cost this test is actually about.
    crate::tests::stock_for_shaft(&mut game, "shaft.elevator", 1);
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
fn shaft_cost_is_fixed_plus_crossed_boundaries_and_extension_is_incremental() {
    let content = content();
    let poles = item(&content, "item.poles");
    let rope = item(&content, "item.rope");
    let mut game = engine(8_000);
    crate::tests::stock_for_shaft(&mut game, "shaft.elevator", 1);

    let poles_before = game.state().stock_of(poles);
    let rope_before = game.state().stock_of(rope);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.elevator".into(),
        low: 0,
        high: 2,
        slot: 7,
    })
    .expect("three-floor elevator");
    assert_eq!(
        poles_before - game.state().stock_of(poles),
        2,
        "two crossed boundaries should cost two poles"
    );
    assert_eq!(
        rope_before - game.state().stock_of(rope),
        2,
        "the fixed elevator frame should cost two rope"
    );

    let shaft = game
        .state()
        .tower
        .shafts
        .iter()
        .find(|shaft| shaft.kind == ShaftKind::Elevator)
        .expect("elevator standing")
        .id;
    let poles_before_extension = game.state().stock_of(poles);
    let rope_before_extension = game.state().stock_of(rope);
    game.try_send(GameCommand::ExtendShaft { shaft, high: 4 })
        .expect("two-floor extension");
    assert_eq!(
        poles_before_extension - game.state().stock_of(poles),
        2,
        "extension should charge only its two new boundaries"
    );
    assert_eq!(
        rope_before_extension,
        game.state().stock_of(rope),
        "extension must not repay the fixed frame cost"
    );
}

#[test]
fn a_shaft_column_blocks_its_slot_on_every_floor_it_spans() {
    let mut game = with_shaft(801, "shaft.elevator", 0, 3, 7);
    for floor in 0..=3u8 {
        let blocked = game.state().tower.slot_range_blocked(floor, 7, 1);
        assert!(blocked, "floor {floor} slot 7 should be taken by the shaft");
    }
    // And a room cannot be dropped on top of it. Slot 6 with a two-wide
    // room reaches into 7, which is the column — the overlap is the
    // point of the placement, not an accident of where there was room.
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
    // **One floor is not a shaft.** `min_span` is 2, and the lift's
    // `max_span` is 0 — unlimited — since it absorbed the dumbwaiter
    // (`SYSTEMS.md` §6.18), so the bound that can still be broken is
    // the lower one. This used to reach for the dumbwaiter's three-floor
    // ceiling, which no longer exists.
    let error = game
        .try_send(GameCommand::BuildShaft {
            shaft: "shaft.elevator".into(),
            low: 1,
            high: 1,
            slot: 7,
        })
        .expect_err("a shaft that spans one floor goes nowhere");
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
fn the_lift_moves_items_without_anybody_carrying_them() {
    let content = content();
    let bamboo = item(&content, "item.bamboo");

    // **Floor 0 to floor 2**, spanning the cutter arm and the
    // storeroom. It was 0 to 1 until M6 cut the opening tower down and
    // moved the storeroom up a floor (`SYSTEMS.md` §6.11); a
    // dumbwaiter's `max_span` is 3, so this still fits.
    let mut game = with_shaft(811, "shaft.elevator", 0, 2, 7);
    // Take the crew out entirely, so anything that moves was moved by
    // the machine.
    game.state_mut_for_test().crew.clear();

    // **Anywhere off floor 0, not specifically a shelf.** The mill is
    // in this span and a hungry inbox outranks a shelf, so counting
    // shelves alone measures which destination won rather than whether
    // anything moved — which is the *next* test's question. This one
    // only asks whether the machine works with nobody aboard.
    let moved = |game: &crate::engine::GameEngine| -> i64 {
        game.state()
            .tower
            .floors
            .iter()
            .filter(|floor| floor.index > 0)
            .flat_map(|floor| floor.rooms.iter())
            .flat_map(|room| {
                room.shelves
                    .iter()
                    .filter(|shelf| shelf.item == Some(bamboo))
                    .map(|shelf| shelf.count)
                    .chain(
                        room.inputs
                            .iter()
                            .filter(|stack| stack.item == bamboo)
                            .map(|stack| stack.count),
                    )
            })
            .sum()
    };

    let before = moved(&game);
    // No crew means no burner deliveries. Keep this transport fixture
    // supplied so the exact lift cost does not turn it into a test of
    // how long the opening bank lasts.
    for _ in 0..10 {
        game.step(300);
        let power = &mut game.state_mut_for_test().power;
        power.charge = power.capacity;
    }
    let after = moved(&game);

    assert!(
        after > before,
        "the dumbwaiter moved nothing with no crew aboard: {before} then {after}"
    );
}

#[test]
fn the_lift_never_loses_a_load() {
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let poles = item(&content, "item.poles");
    let mut game = with_shaft(812, "shaft.elevator", 0, 2, 7);
    crate::tests::disarm(&mut game);
    game.state_mut_for_test().crew.clear();

    let mut last = total_including_cars(&game, bamboo);
    let mut harvested = game.state().stats.harvested_by_item[bamboo.get()] as i64;
    let mut last_poles = total_including_cars(&game, poles);
    let mut burned = game.state().stats.fuel_burned as i64;

    for _ in 0..100 {
        game.step(30);
        let now = total_including_cars(&game, bamboo);
        let harvested_now = game.state().stats.harvested_by_item[bamboo.get()] as i64;
        let poles_now = total_including_cars(&game, poles);
        let burned_now = game.state().stats.fuel_burned as i64;
        // Bamboo arrives from the cutter arm and leaves through the
        // mill. Anything else is the dumbwaiter dropping a load, which
        // it must never do — mid-flight cargo is still cargo.
        assert_eq!(
            now - last,
            (harvested_now - harvested) - (poles_now - last_poles) - (burned_now - burned),
            "bamboo went missing at tick {}",
            game.state().tick
        );
        last = now;
        harvested = harvested_now;
        last_poles = poles_now;
        burned = burned_now;
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
    let elevator_def = game
        .content()
        .shaft_idx("shaft.elevator")
        .expect("lift def");
    let _cost = game.content().shaft(elevator_def).charge_per_floor;
    let noon = game.content().balance.clock.ticks_per_day / 2;
    // Put the car exactly on a landing at the start of a segment and
    // silence every other consumer. This asserts the lift's own draw,
    // rather than accidentally observing the night lamps as the old
    // version of this test did.
    {
        let state = game.state_mut_for_test();
        state.clock.tick_of_day = noon;
        state.walking = false;
        state.power.charge = state.power.capacity;
        state.power.trickle_acc = crate::fx::Fx::ZERO;
        for room in state
            .tower
            .floors
            .iter_mut()
            .flat_map(|floor| floor.rooms.iter_mut())
        {
            room.active = false;
        }
        let shaft = state
            .tower
            .shafts
            .iter_mut()
            .find(|shaft| shaft.def == elevator_def)
            .expect("elevator shaft");
        let car = shaft.cars.first_mut().expect("elevator car");
        car.pos = crate::fx::Fx::ZERO;
        car.dir = crate::state::CarDir::Up;
        car.state = CarState::Moving;
        car.stops = vec![1];
    }

    let before = game.state().power.charge;
    // A whole floor's worth of travel, because the cost is a rate now
    // rather than a lump paid at the landing.
    game.step(30);
    let spent = before - game.state().power.charge;
    // **A moving car spends while it moves** (`SYSTEMS.md` §6.39). It
    // used to buy a whole floor segment as it left a landing, because
    // four charge over eight ticks truncated to zero a tick; the rate is
    // stated honestly now, so what this measures is that a journey costs
    // something rather than that one tick costs everything.
    assert!(
        spent > 0,
        "a car crossed a floor without spending any charge"
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
    crate::tests::step_quietly(game, ticks);
    game.state().stats.crafts_completed - before
}

#[test]
fn adding_a_shaft_measurably_improves_throughput() {
    // M1's exit criterion, as an assertion rather than a vibe: a tower
    // whose crew queue on one staircase must move visibly more once a
    // second way up exists. If this stops being true, the contention
    // the whole design rests on has quietly stopped mattering.
    // **A whole day, warmed up over a whole day, and that is not a
    // detail.** This was 9,000 ticks after a 6,000-tick warm-up, which
    // is 0.625 of a day starting from wherever the warm-up happened to
    // land — and once M4 gave the crew a rota, two thirds of that
    // window fell across the night, when everybody on the default
    // all-Day shift is asleep. A tower whose crew are in bed does not
    // queue on its staircase, so the elevator had nothing to relieve
    // and the measurement said it was worthless: 19 crafts without
    // against 18 with. The effect had not gone anywhere; the instrument
    // had stopped pointing at it. Measured over a whole day from the
    // same hour, the same two towers read 18 against 36.
    //
    // The rule this leaves behind, and it applies to every harness in
    // `examples/` too: **a throughput window is a whole number of
    // days, or it is a measurement of what time it started.**
    const DAY: u32 = 14_400;
    const WINDOW: u32 = DAY;

    let mut cramped = engine(900);
    let mut relieved = engine(900);
    // The elevator costs rope from M5 and neither of these towers runs a
    // ropery, so the parts are handed over — **to both of them, before
    // the warm-up.** Both halves of that matter and both were learnt the
    // hard way. Stocking afterwards silently failed, because a shelf
    // holds one kind and a tower a day into a run has none spare.
    // Stocking only the tower that builds the shaft handed it a claimed
    // shelf its twin did not have, which is a second difference between
    // them and exactly the thing a controlled comparison may not have.
    for tower in [&mut cramped, &mut relieved] {
        crate::tests::stock_for_shaft(tower, "shaft.elevator", 1);
    }

    // Both towers bank the same poles over the same warm-up, so the
    // only difference between them is the shaft — and both enter the
    // window at the same hour, which is the point of warming up for a
    // whole day rather than a round number of ticks.
    crate::tests::step_quietly(&mut cramped, DAY);
    crate::tests::step_quietly(&mut relieved, DAY);
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

    // This four-floor tower is below the lift's strongest crossover.
    // The current measured gain is 25 -> 27 crafts; require that exact
    // direction and at least a two-craft margin without preserving the
    // obsolete 25% claim from the former transport/economy shape.
    assert!(
        with >= without + 2,
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
    estimate_laden(game, kind, queued, 0)
}

/// The same, for a crew member with `load` items on their back. The
/// stairs charge per item per floor and the other shafts do not, which
/// is the whole reason a built shaft has a job.
fn estimate_laden(
    game: &crate::engine::GameEngine,
    kind: ShaftKind,
    queued: u32,
    load: u32,
) -> u32 {
    let shaft = game
        .state()
        .tower
        .shafts
        .iter()
        .find(|shaft| shaft.kind == kind)
        .expect("shaft is standing");
    crate::systems::transport::estimated_trip_ticks(shaft, game.content(), 0, 3, queued, load)
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
        let one =
            crate::systems::transport::estimated_trip_ticks(shaft, game.content(), 0, 1, 0, 0);
        let three =
            crate::systems::transport::estimated_trip_ticks(shaft, game.content(), 0, 3, 0, 0);
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
            0,
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
fn nobody_is_routed_down_a_chute() {
    // **The last un-rideable shaft.** This used to be about the
    // dumbwaiter, which crew could not board because nothing rode it;
    // §6.18 folded that shaft into the lift and crew ride the result.
    // What is left is the chute, which goes one way, downward, and
    // whatever enters it is gone — a route no person should ever be
    // offered.
    let game = with_shaft(923, "shaft.elevator", 0, 2, 7);
    let content = game.content().clone();
    let rideable = content
        .shafts
        .iter()
        .filter(|shaft| shaft.kind != ShaftKind::Chute)
        .count();
    assert_eq!(
        rideable,
        content.shafts.len() - 1,
        "exactly one shaft in the pack should be un-rideable"
    );
    assert!(
        estimate(&game, ShaftKind::Elevator, 0) < u32::MAX,
        "the lift carries people as well as freight"
    );
}

// ---------------------------------------------------------------------------
// The job the lift does when nobody is calling it: feeding a recipe
// ---------------------------------------------------------------------------

#[test]
fn the_lift_feeds_a_hungry_recipe_in_preference_to_a_shelf() {
    // Spanning the cutter arm on floor 0 and the mill on floor 2, with
    // a storeroom on floor 1 in between. Both are valid destinations
    // for bamboo; the mill's inbox outranks the shelves, and this is
    // the path that actually keeps a chain running.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = with_shaft(924, "shaft.elevator", 0, 2, 7);
    game.state_mut_for_test().crew.clear();

    let crafts_before = game.state().stats.crafts_completed;
    // **Charge topped up as it goes.** A dumbwaiter draws
    // `charge_per_floor` every trip, and with the crew cleared nobody
    // is carrying fuel to a burner — so left alone the tower runs the
    // bank flat somewhere in the first thousand ticks and the shaft
    // stops, which reads as "the dumbwaiter never fed the mill" and is
    // a power measurement wearing a transport test's clothes.
    //
    // This test is about which destination a shaft prefers. The power
    // economy has `examples/charge.rs`.
    for _ in 0..20 {
        game.step(300);
        let power = &mut game.state_mut_for_test().power;
        power.charge = power.capacity;
    }

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
        "with no crew at all, the dumbwaiter never fed the mill:          {crafts_before} then {}, {in_mill} waiting in inboxes",
        game.state().stats.crafts_completed,
    );
    assert!(
        in_mill > 0 || game.state().stats.crafts_completed > crafts_before,
        "bamboo never reached an inbox: {in_mill} waiting"
    );
}

#[test]
fn the_lift_conserves_across_the_inbox_path_too() {
    // The conservation check again, but on the route that actually
    // deposits into a recipe rather than onto shelves — the one the
    // earlier test could not reach.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let poles = item(&content, "item.poles");
    let mut game = with_shaft(925, "shaft.elevator", 0, 2, 7);
    // **And disarmed.** The thorn gun eats two stalks a shot
    // (`SYSTEMS.md` §6.13), so a conservation check that does not know
    // about it reads a fired round as bamboo going missing — measured
    // here as -2 at tick 5,910. Ammo is a real sink; it is just not
    // this test's.
    crate::tests::disarm(&mut game);
    game.state_mut_for_test().crew.clear();

    let mut last = total_including_cars(&game, bamboo);
    let mut harvested = game.state().stats.harvested_by_item[bamboo.get()] as i64;
    let mut last_poles = total_including_cars(&game, poles);
    let mut burned = game.state().stats.fuel_burned as i64;

    for _ in 0..200 {
        game.step(30);
        let now = total_including_cars(&game, bamboo);
        let harvested_now = game.state().stats.harvested_by_item[bamboo.get()] as i64;
        let poles_now = total_including_cars(&game, poles);
        let burned_now = game.state().stats.fuel_burned as i64;
        assert_eq!(
            now - last,
            (harvested_now - harvested) - (poles_now - last_poles) - (burned_now - burned),
            "bamboo went missing at tick {}",
            game.state().tick
        );
        last = now;
        harvested = harvested_now;
        last_poles = poles_now;
        burned = burned_now;
    }
}

#[test]
fn the_lift_stops_when_there_is_nowhere_to_put_anything() {
    // Fill every destination and the car should sit still holding
    // nothing, rather than shuttling an empty box or dropping a load.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = with_shaft(926, "shaft.elevator", 0, 2, 7);
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

// ---------------------------------------------------------------------------
// A second car (`SYSTEMS.md` §6.20)
// ---------------------------------------------------------------------------

#[test]
fn a_shaft_can_be_given_another_car() {
    let mut game = with_shaft(940, "shaft.elevator", 0, 2, 7);
    let shaft = game.state().tower.shafts.last().expect("just built").id;
    let before = game
        .state()
        .tower
        .shafts
        .last()
        .expect("just built")
        .cars
        .len();
    crate::tests::stock_poles(&mut game, 40);
    crate::tests::stock_item(&mut game, "item.rope", 10);
    crate::tests::stock_item(&mut game, "item.mechanisms", 2);

    game.try_send(GameCommand::AddCar { shaft })
        .expect("a paid-for car should go in");

    let cars = &game.state().tower.shafts.last().expect("still there").cars;
    assert_eq!(cars.len(), before + 1, "the car did not arrive");
    // **At the bottom, not beside the other one.** A car spawned next to
    // its sibling shadows it — same sweep, same calls — and the whole
    // point of a second car is that it is somewhere else.
    assert_eq!(
        cars.last().expect("the new car").floor(),
        game.state().tower.shafts.last().expect("still there").low,
        "a new car should start at the foot of the shaft"
    );
}

#[test]
fn a_shaft_stops_taking_cars_somewhere() {
    let content = content();
    let mut game = with_shaft(941, "shaft.elevator", 0, 2, 7);
    let shaft = game.state().tower.shafts.last().expect("just built").id;
    let max = content
        .shaft_idx("shaft.elevator")
        .map(|idx| content.shaft(idx).max_cars)
        .expect("the pack defines a lift");

    let mut added = 0;
    loop {
        crate::tests::stock_poles(&mut game, 40);
        crate::tests::stock_item(&mut game, "item.rope", 10);
        crate::tests::stock_item(&mut game, "item.mechanisms", 2);
        match game.try_send(GameCommand::AddCar { shaft }) {
            Ok(()) => added += 1,
            Err(CommandError::FullOfCars { .. }) => break,
            Err(other) => panic!("adding a car failed for the wrong reason: {other:?}"),
        }
        assert!(added < 50, "the shaft took cars without limit");
    }
    assert!(added > 0, "the shaft would never take a second car");
    assert_eq!(
        game.state()
            .tower
            .shafts
            .last()
            .expect("still there")
            .cars
            .len(),
        max as usize,
        "the shaft went past its own ceiling"
    );
}

#[test]
fn adding_a_car_to_nothing_is_refused() {
    let mut game = crate::tests::engine(942);
    crate::tests::stock_poles(&mut game, 40);
    let err = game
        .try_send(GameCommand::AddCar {
            shaft: crate::ids::ShaftId(9999),
        })
        .expect_err("there is no such shaft");
    assert!(matches!(err, CommandError::NoSuchShaft { .. }), "{err:?}");
}

#[test]
fn an_unpaid_car_changes_nothing() {
    // `DECISIONS.md` §4: validate fully before mutating.
    let mut game = with_shaft(943, "shaft.elevator", 0, 2, 7);
    let shaft = game.state().tower.shafts.last().expect("just built").id;
    let before = game
        .state()
        .tower
        .shafts
        .last()
        .expect("just built")
        .cars
        .len();
    let hash = crate::replay::hash_state(game.state());

    let err = game
        .try_send(GameCommand::AddCar { shaft })
        .expect_err("the shelves are empty");
    assert!(
        matches!(err, CommandError::InsufficientStock { .. }),
        "{err:?}"
    );
    assert_eq!(
        game.state()
            .tower
            .shafts
            .last()
            .expect("still there")
            .cars
            .len(),
        before
    );
    assert_eq!(hash, crate::replay::hash_state(game.state()));
}

#[test]
fn a_second_car_moves_more_than_one_does() {
    // **The reason to sell one.** A car is a *turn*, not speed: capacity
    // is applied per car, so a second car is a second carload and what
    // it buys is queue rather than pace. Measured on the same seed with
    // the same tower, crowded enough that one car has a queue to work
    // through.
    // **Both towers are handed the price; only one spends it.** The
    // first version stocked only the tower that was buying, and three
    // cars came out at 50 hauls against one car's 185 — which was the
    // shelf jam, not the cars. A shelf holds one kind and the tower has
    // eight of them, so 80 poles and 20 rope granted to one side and
    // nothing to the other is a comparison of two different economies.
    // The rule is `AGENTS.md`'s: never let the setup differ by anything
    // but the thing under test.
    let hauls_with = |cars: u8| -> u64 {
        let mut game = with_shaft(944, "shaft.elevator", 0, 4, 7);
        let shaft = game.state().tower.shafts.last().expect("just built").id;
        for _ in 1..3 {
            crate::tests::stock_poles(&mut game, 40);
            crate::tests::stock_item(&mut game, "item.rope", 10);
            crate::tests::stock_item(&mut game, "item.mechanisms", 2);
        }
        for _ in 1..cars {
            game.try_send(GameCommand::AddCar { shaft })
                .expect("a paid-for car should go in");
        }
        assert_eq!(
            game.state()
                .tower
                .shafts
                .last()
                .expect("still there")
                .cars
                .len(),
            cars as usize,
            "the fixture did not end up with the cars it asked for"
        );
        crate::tests::step_quietly(&mut game, 20_000);
        game.state().stats.hauls_completed
    };

    let one = hauls_with(1);
    let three = hauls_with(3);
    assert!(
        three >= one,
        "three cars moved less than one: {three} against {one}"
    );
}
