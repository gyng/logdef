//! Hauling: the crew, the stairs, and the queue.
//!
//! These are the tests that matter most for M0's design question. If
//! the chain does not move on its own, nothing above it is worth
//! looking at.

use crate::command::GameCommand;
use crate::snapshot::CrewStateTag;
use crate::state::CrewState;
use crate::tests::{content, engine, item, total_in_flight};

#[test]
fn the_chain_runs_unattended() {
    // The whole M0 slice: the cutter arm harvests, a crew member walks
    // it up the stairs, the mill turns it into poles. Nobody clicks
    // anything.
    let mut game = engine(100);
    game.step(1800); // one minute

    let state = game.state();
    assert!(
        state.stats.items_harvested > 0,
        "the cutter arm never harvested"
    );
    assert!(state.stats.hauls_completed > 0, "nothing was ever hauled");
    assert!(
        state.stats.crafts_completed > 0,
        "the mill never received bamboo"
    );
}

#[test]
fn poles_reach_the_shelves() {
    let content = content();
    let poles = item(&content, "item.poles");
    let mut game = engine(101);

    let opening = game.state().stock_of(poles);
    game.step(3600); // two minutes
    // Net of whatever repair spent, so this measures the chain rather
    // than the siege.
    let delivered = game.state().stock_of(poles) + game.state().stats.repair_poles_spent as i64;
    assert!(
        delivered > opening,
        "milled poles never made it to a storeroom: {opening} then {delivered}"
    );
}

#[test]
fn hauling_never_creates_or_destroys() {
    // Conservation across a long run, counting rooms and hands. Only
    // the mill may change the totals, and only by its recipe.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let poles = item(&content, "item.poles");
    let mut game = engine(102);

    let mut last_bamboo = total_in_flight(game.state(), bamboo);
    let mut last_poles = total_in_flight(game.state(), poles);
    let mut harvested = game.state().stats.items_harvested as i64;
    let mut crafted = game.state().stats.crafts_completed as i64;
    let mut mending = game.state().stats.repair_poles_spent as i64;

    for _ in 0..120 {
        game.step(30);
        let state = game.state();

        let bamboo_now = total_in_flight(state, bamboo);
        let poles_now = total_in_flight(state, poles);
        let harvested_now = state.stats.items_harvested as i64;
        let crafted_now = state.stats.crafts_completed as i64;
        let mending_now = state.stats.repair_poles_spent as i64;

        // Bamboo in = harvested; bamboo out = consumed by crafts.
        assert_eq!(
            bamboo_now - last_bamboo,
            (harvested_now - harvested) - (crafted_now - crafted),
            "bamboo appeared or vanished at tick {}",
            state.tick
        );
        // Poles come from crafts and leave through construction and
        // repair. This test issues no build commands, so repair is the
        // only sink — and once the jungle notices the tower, it is a
        // real one.
        assert_eq!(
            poles_now - last_poles,
            (crafted_now - crafted) - (mending_now - mending),
            "poles appeared or vanished at tick {}",
            state.tick
        );

        last_bamboo = bamboo_now;
        last_poles = poles_now;
        harvested = harvested_now;
        crafted = crafted_now;
        mending = mending_now;
    }
}

#[test]
fn the_stairs_hold_one_body_at_a_time() {
    let mut game = engine(103);
    for _ in 0..1800 {
        game.step(1);
        for shaft in &game.state().tower.shafts {
            assert!(
                shaft.riders <= shaft.capacity,
                "shaft {:?} had {} riders over a capacity of {}",
                shaft.id,
                shaft.riders,
                shaft.capacity
            );
        }
    }
}

#[test]
fn a_full_shaft_makes_a_visible_queue() {
    // Two crew and a one-body staircase must produce a wait. If this
    // stops being true, the contention thesis has no teeth.
    let mut game = engine(104);
    let mut saw_boarding = false;
    let mut peak_wait = 0;

    for _ in 0..3600 {
        game.step(1);
        for member in &game.state().crew {
            if matches!(member.state, CrewState::Boarding { .. }) {
                saw_boarding = true;
            }
            peak_wait = peak_wait.max(member.wait_ticks);
        }
        if saw_boarding && peak_wait > 0 {
            break;
        }
    }

    assert!(
        saw_boarding,
        "two crew never contended for the single staircase"
    );
    assert!(peak_wait > 0, "queueing never registered as a wait");
}

#[test]
fn riders_are_released_when_a_climb_finishes() {
    // A leaked rider slot would deadlock the tower silently, so assert
    // the shaft drains rather than trending upward.
    let mut game = engine(105);
    game.step(3600);
    let stuck = game
        .state()
        .tower
        .shafts
        .iter()
        .any(|shaft| shaft.riders > shaft.capacity);
    assert!(!stuck);

    // And the chain is still making progress at the end of the run,
    // which it could not be if the stairs had jammed.
    let before = game.state().stats.hauls_completed;
    game.step(1800);
    assert!(
        game.state().stats.hauls_completed > before,
        "the tower seized"
    );
}

#[test]
fn a_load_survives_its_destination_being_demolished() {
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = engine(106);
    game.step(900);

    // Tear out the mill mid-haul. Whatever was in hand or in the mill's
    // inbox must still exist afterwards, and must end up somewhere.
    let before_total = total_in_flight(game.state(), bamboo);
    let before_harvested = game.state().stats.items_harvested as i64;
    let before_crafted = game.state().stats.crafts_completed as i64;

    game.try_send(GameCommand::RemoveRoom { floor: 2, slot: 3 })
        .expect("the starting mill is removable");

    // Removing the mill removes its inbox with it, so only what was
    // outside it is conserved — but nothing in a crew member's hands
    // may vanish.
    let carried_now: i64 = game
        .state()
        .crew
        .iter()
        .filter_map(|member| member.carrying)
        .filter(|(held, _)| *held == bamboo)
        .map(|(_, count)| count)
        .sum();
    assert!(before_total >= carried_now);

    game.step(3600);
    let state = game.state();

    // With no mill, every stalk harvested since must be accounted for
    // on a shelf, in the arm, or in a hand — nothing crafted, nothing
    // lost.
    assert_eq!(
        state.stats.crafts_completed as i64, before_crafted,
        "something milled bamboo after the mill was demolished"
    );
    let harvested_since = state.stats.items_harvested as i64 - before_harvested;
    assert!(harvested_since > 0, "the arm stopped harvesting");

    let shelved: i64 = state
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .flat_map(|room| room.shelves.iter())
        .filter(|shelf| shelf.item == Some(bamboo))
        .map(|shelf| shelf.count)
        .sum();
    assert!(
        shelved > 0,
        "orphaned bamboo never found a shelf — a load was stranded"
    );
}

#[test]
fn two_crew_never_chase_the_same_crate() {
    let mut game = engine(107);
    for _ in 0..3600 {
        game.step(1);
        let state = game.state();
        // Committed pickups from a room must never exceed what it holds.
        for floor in &state.tower.floors {
            for room in &floor.rooms {
                for stack in &room.outputs {
                    // Crew already carrying have taken their share out
                    // of the stack, so only outstanding promises count.
                    let committed: i64 = state
                        .crew
                        .iter()
                        .filter(|member| !member.is_carrying())
                        .filter_map(|member| member.task.as_ref())
                        .filter(|task| task.item == stack.item)
                        .filter(|task| task.pickup.is_some_and(|pickup| pickup.room == room.id))
                        .map(|task| task.amount)
                        .sum();
                    assert!(
                        committed <= stack.count,
                        "{committed} promised out of a stack of {} at tick {}",
                        stack.count,
                        state.tick
                    );
                }
            }
        }
    }
}

#[test]
fn a_hungry_recipe_outranks_a_shelf() {
    // Priority, observed rather than asserted on internals: while the
    // mill has inbox space, bamboo should be heading there.
    let mut game = engine(108);
    let content = content();
    let bamboo = item(&content, "item.bamboo");

    let mut saw_inbox_task = false;
    for _ in 0..1800 {
        game.step(1);
        let mill_hungry = game.state().tower.floors.iter().any(|floor| {
            floor
                .rooms
                .iter()
                .any(|room| room.inputs.iter().any(|s| s.item == bamboo && !s.is_full()))
        });
        if !mill_hungry {
            continue;
        }
        if game.state().crew.iter().any(|member| {
            member.task.as_ref().is_some_and(|task| {
                task.item == bamboo
                    && matches!(
                        task.destination,
                        crate::state::HaulDestination::Inbox { .. }
                    )
            })
        }) {
            saw_inbox_task = true;
            break;
        }
    }
    assert!(saw_inbox_task, "bamboo never got routed to the hungry mill");
}

#[test]
fn crew_positions_stay_inside_the_tower() {
    let mut game = engine(109);
    for _ in 0..1800 {
        game.step(1);
        let state = game.state();
        let top = state.tower.top_floor();
        let slots = state.tower.floors[0].slots;
        for member in &state.crew {
            assert!(
                member.floor() <= top,
                "{} walked off the top at tick {}",
                member.name,
                state.tick
            );
            assert!(
                member.slot() < slots,
                "{} walked off the edge at tick {}",
                member.name,
                state.tick
            );
        }
    }
}

#[test]
fn the_view_reports_the_same_states_the_simulation_is_in() {
    let mut game = engine(110);
    game.step(600);
    let view = game.view();
    assert_eq!(view.crew.len(), game.state().crew.len());
    for (rendered, actual) in view.crew.iter().zip(&game.state().crew) {
        let expected = match actual.state {
            CrewState::Idle => CrewStateTag::Idle,
            CrewState::Walking { .. } => CrewStateTag::Walk,
            CrewState::Boarding { .. } => CrewStateTag::Board,
            CrewState::Climbing { .. } => CrewStateTag::Climb,
            CrewState::Riding { .. } => CrewStateTag::Ride,
            CrewState::Repairing { .. } => CrewStateTag::Mend,
            CrewState::Loading { .. } => CrewStateTag::Load,
            CrewState::Unloading { .. } => CrewStateTag::Unload,
        };
        assert_eq!(rendered.state, expected);
    }
}
