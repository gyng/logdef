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
            CrewState::Eating { .. } => CrewStateTag::Eat,
            CrewState::Sleeping => CrewStateTag::Sleep,
        };
        assert_eq!(rendered.state, expected);
    }
}

#[test]
fn a_storeroom_is_a_buffer_and_not_a_bin() {
    // Everything a crew member could pick up used to come out of a
    // room's *outbox*, never off a shelf. So the mill could only ever
    // be fed straight from the cutter arm, and any bamboo that reached
    // a shelf on the way — which is where it goes the moment the mill's
    // inbox is full — was stranded there for the rest of the run.
    // Nothing draws bamboo back off a shelf: `take_stock` spends poles
    // for building and repair, and that is the only other way out.
    //
    // The visible form was a tower stopping with its shelves
    // three-quarters full and no error anywhere.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = engine(4100);

    // Put bamboo on the shelves and nowhere else, with an empty mill
    // waiting for it and no arm to feed it directly.
    {
        let state = game.state_mut_for_test();
        state.siege.provocation = 0;
        state.siege.enemies.clear();
        state.siege.next_wave_tick = u64::MAX;
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                for stack in room.outputs.iter_mut().chain(room.inputs.iter_mut()) {
                    stack.count = 0;
                }
            }
        }
        // The arms go, so the only bamboo in the tower is the shelved
        // kind. If the mill runs, it ran on that.
        for floor in &mut state.tower.floors {
            floor
                .rooms
                .retain(|room| content.room_rt(room.def).intake_source.is_none());
        }
        state.shelve(bamboo, 20);
    }

    let shelved = game.state().stock_of(bamboo);
    assert_eq!(shelved, 20, "the test did not manage to shelve anything");
    let crafts = game.state().stats.crafts_completed;

    game.step(3000);

    assert!(
        game.state().stock_of(bamboo) < shelved,
        "bamboo went onto a shelf and could not come off it again"
    );
    assert!(
        game.state().stats.crafts_completed > crafts,
        "the mill starved with twenty bamboo sitting on a shelf beside it"
    );
}

#[test]
fn nothing_carries_a_thing_from_one_shelf_to_another() {
    // The other half of the rule. A shelf pickup exists to feed a room
    // that eats the item; allowing a shelf as its destination too would
    // let a crew member move bamboo between storerooms for ever, which
    // scores as work and achieves nothing.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = engine(4101);
    {
        let state = game.state_mut_for_test();
        state.siege.provocation = 0;
        state.siege.enemies.clear();
        state.siege.next_wave_tick = u64::MAX;
        // No arms and no mill: nothing produces bamboo and nothing eats
        // it, so any movement at all is a crew member going in circles.
        for floor in &mut state.tower.floors {
            floor.rooms.retain(|room| {
                let rt = content.room_rt(room.def);
                rt.intake_source.is_none() && rt.recipe_inputs.is_empty()
            });
        }
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                for stack in room.outputs.iter_mut().chain(room.inputs.iter_mut()) {
                    stack.count = 0;
                }
            }
        }
        state.shelve(bamboo, 12);
    }

    let before = game.state().stats.hauls_completed;
    game.step(3000);
    assert_eq!(
        game.state().stats.hauls_completed,
        before,
        "crew hauled something with nowhere to take it"
    );
    assert_eq!(
        game.state().stock_of(bamboo),
        12,
        "shelved bamboo moved with nothing to consume it"
    );
}

#[test]
fn a_crew_member_holding_something_with_nowhere_to_put_it_still_mends() {
    // Repair used to be gated on empty hands, full stop: a crew member
    // carrying a load finished the delivery first, because putting it
    // down somewhere it does not belong would lose it.
    //
    // That is right until there is nowhere to put it at all. A tower
    // with no free shelf and no hungry room strands whoever is holding
    // something, and a stranded carrier was then lost to repair for the
    // rest of the run — `is_carrying` said no and `find_destination`
    // said no, so they stood there holding a crate while the wall came
    // down.
    //
    // Found by an instrument that had stuffed its own shelves to keep
    // the chain fed, which turned a small difference between two towers
    // into a fourfold gap in repair and nearly got a working feature
    // withdrawn. Mending while holding a crate costs nothing:
    // `repair::run` never touches `carrying`, so the load stays held
    // and goes where it was going once somewhere opens up.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let poles = item(&content, "item.poles");
    let mut game = engine(4300);

    {
        let state = game.state_mut_for_test();
        state.siege.provocation = 0;
        state.siege.enemies.clear();
        state.siege.next_wave_tick = u64::MAX;

        // Damage to mend, and the poles to mend it with.
        state.tower.floor_mut(0).expect("ground floor").panel.hp -= 100;

        // Fill every shelf and every input in the tower, so nothing a
        // crew member picks up has anywhere to go.
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                for stack in room.inputs.iter_mut().chain(room.outputs.iter_mut()) {
                    let space = stack.space();
                    stack.deposit(space);
                }
                for shelf in &mut room.shelves {
                    if shelf.item.is_none() {
                        shelf.item = Some(bamboo);
                    }
                    shelf.count = shelf.max;
                }
            }
        }
        // Except the poles repair will draw on: one shelf's worth,
        // which is the tower's stock rather than somewhere to deliver.
        if let Some(shelf) = state
            .tower
            .floors
            .iter_mut()
            .flat_map(|floor| floor.rooms.iter_mut())
            .flat_map(|room| room.shelves.iter_mut())
            .next()
        {
            shelf.item = Some(poles);
            shelf.count = shelf.max;
        }

        // And put a crate in somebody's hands that they cannot deliver.
        if let Some(member) = state.crew.first_mut() {
            member.carrying = Some((bamboo, 1));
            member.state = crate::state::CrewState::Idle;
            member.task = None;
        }
    }

    let hurt = game.state().tower.floor(0).expect("ground floor").panel.hp;
    game.step(1200);

    let after = game.state().tower.floor(0).expect("ground floor").panel.hp;
    assert!(
        after > hurt,
        "nobody mended the panel: {hurt} then {after}, on a tower where every \
         crew member had their hands full and nowhere to empty them"
    );
    // And the load was not dropped on the floor to do it.
    assert!(
        game.state().stats.hp_repaired > 0,
        "the panel healed without any repair being recorded"
    );
}

// ---------------------------------------------------------------------------
// Chutes: the escape hatch for a jammed tower
// ---------------------------------------------------------------------------

/// Jam a tower the way `BALANCE.md`'s `storeroom` row describes: fill
/// every shelf with one thing and switch off whatever eats it.
///
/// **Fiber, not bamboo, and the swap is the point of the fixture.** A
/// chute may only spill what nothing wants, and "wants" now includes
/// anything a settlement will take — so bamboo, which the drowned city
/// buys ten at a time, is not rubbish however full the shelves are. It
/// is also the wrong material for this test on its own terms: bamboo has
/// a live mill waiting for it in any tower anybody would build, so
/// jamming with it needs the mill switched off and then measures a
/// situation that cannot occur. Fiber is what §5.4 was actually written
/// about — harvested by a comb, eaten only by a ropery, and worthless
/// the moment the ropery stops.
fn jammed(seed: u64) -> (crate::engine::GameEngine, crate::ids::ItemIdx) {
    let mut game = engine(seed);
    let fiber = item(game.content(), "item.fiber");
    // Pay for the chute *before* jamming, because a jammed tower cannot
    // pay for anything — which is the finding that moved the chute's own
    // cost off rope and onto poles alone, and is exactly why a chute
    // prevents rather than resurrects.
    crate::tests::stock_for_shaft(&mut game, "shaft.chute", 1);
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                // Nothing eats fiber any more, which is what makes it
                // rubbish rather than stock.
                room.active = false;
                for shelf in &mut room.shelves {
                    // Everything except what is already paying for the
                    // chute. A tower with literally nothing on a shelf
                    // cannot build, and this fixture is about a tower
                    // that is stuck rather than one that is bankrupt.
                    if shelf.item.is_some() {
                        continue;
                    }
                    shelf.item = Some(fiber);
                    shelf.count = shelf.max;
                }
            }
        }
    }
    (game, fiber)
}

#[test]
fn without_a_chute_a_jammed_tower_stays_jammed() {
    // The control, and the thing the chute exists to change. Recorded as
    // a test rather than as a comment because "it was already broken"
    // is the claim a fix rests on.
    let (mut game, fiber) = jammed(4100);
    let before = total_in_flight(game.state(), fiber);
    game.step(6000);
    assert_eq!(
        total_in_flight(game.state(), fiber),
        before,
        "something cleared a jam with no chute in the tower"
    );
}

#[test]
fn a_chute_empties_shelves_of_what_nothing_wants() {
    let (mut game, fiber) = jammed(4101);
    crate::tests::stock_for_shaft(&mut game, "shaft.chute", 1);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.chute".into(),
        low: 0,
        high: 2,
        slot: 7,
    })
    .expect("slot 7 is clear on the lower floors");

    let before = total_in_flight(game.state(), fiber);
    assert!(before > 0, "the fixture did not jam the tower");
    game.step(12_000);
    assert!(
        total_in_flight(game.state(), fiber) < before,
        "a chute stood in a jammed tower and nothing was thrown away"
    );
}

#[test]
fn a_chute_never_throws_away_something_a_room_is_waiting_for() {
    // The rule that makes a chute safe to leave standing, and the one
    // the first version got wrong: offered to anything that merely had
    // nowhere to go *right now*, crew threw the economy away — every
    // stalk the arm cut and every pole the mill made — because those
    // were the loads in hand while the shelves were full of the things
    // that caused the jam.
    let mut game = engine(4102);
    let bamboo = item(game.content(), "item.bamboo");
    crate::tests::stock_for_shaft(&mut game, "shaft.chute", 1);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.chute".into(),
        low: 0,
        high: 2,
        slot: 7,
    })
    .expect("slot 7 is clear on the lower floors");

    // A healthy tower: the mill is live, so bamboo is wanted.
    let mut ever_spilled = false;
    for _ in 0..12_000 {
        let before = crate::tests::total_in_flight(game.state(), bamboo);
        game.step(1);
        let after = crate::tests::total_in_flight(game.state(), bamboo);
        // The mill consumes bamboo, so a fall of one is ordinary. What
        // must never happen is a fall while no mill crafted — which is
        // what a spill looks like from outside.
        if after < before && game.state().stats.crafts_completed == 0 {
            ever_spilled = true;
        }
    }
    assert!(
        !ever_spilled,
        "a chute threw away bamboo a live mill was waiting for"
    );
}

#[test]
fn a_chute_never_throws_away_salvage() {
    // **The one a player would have found the hard way.** Scrap has no
    // room that wants it unless the tower has built a sun forge, and it
    // is not a build cost for anything — so on the two questions
    // `wanted` used to ask, scrap answered no twice and a chute was
    // free to dump it.
    //
    // Which is the worst possible thing for it to dump. Scrap is the
    // entire point of berthing at a ruin (`SYSTEMS.md` §3.4): the player
    // stopped, woke the wardens, took the damage and paid the poles to
    // mend it. Its real consumers are an enclave's `Trade`, `Recruit`
    // and `Reinforce`, which are *commands* — nothing about them appears
    // in any room's inputs, so a check that only reads rooms cannot see
    // them.
    //
    // Caught by the screenshot harness rather than by a test, which is
    // its own small lesson: the capture run berthed, salvaged, and then
    // photographed an enclave board it could not afford to buy from.
    let mut game = engine(4104);
    let scrap = item(game.content(), "item.scrap");
    crate::tests::stock_for_shaft(&mut game, "shaft.chute", 1);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.chute".into(),
        low: 0,
        high: 2,
        slot: 7,
    })
    .expect("slot 7 is clear on the lower floors");

    // Salvage on the shelves and nothing in the tower that eats it.
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                room.shelve(scrap, 6);
            }
        }
        assert!(
            !state
                .tower
                .floors
                .iter()
                .flat_map(|floor| floor.rooms.iter())
                .any(|room| room.inputs.iter().any(|stack| stack.item == scrap)),
            "this fixture is only meaningful without a forge in the tower"
        );
    }
    let before = total_in_flight(game.state(), scrap);
    assert!(before > 0, "the fixture shelved no scrap");

    game.step(12_000);
    assert_eq!(
        total_in_flight(game.state(), scrap),
        before,
        "a chute threw away salvage the tower had stopped and bled for"
    );
}

#[test]
fn more_salvage_than_anybody_will_buy_is_rubbish() {
    // The other side of the rule above, and the reason it counts a
    // quantity rather than answering yes or no.
    //
    // Protecting scrap without limit breaks the game the other way: a
    // salvaging tower fills every shelf with metal nothing can move, the
    // mill's inbox never clears, and the cutter arm stops. A board's
    // stock is finite, so the amount worth keeping is finite, and past
    // it a chute is exactly right to take the rest.
    let mut game = engine(4105);
    let scrap = item(game.content(), "item.scrap");
    let cap = game.content().settlements_take(scrap);
    assert!(
        cap > 0,
        "the pack's settlements buy no scrap at all, so this test is vacuous"
    );

    crate::tests::stock_for_shaft(&mut game, "shaft.chute", 1);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.chute".into(),
        low: 0,
        high: 2,
        slot: 7,
    })
    .expect("slot 7 is clear on the lower floors");

    // Well past what every settlement in the run could ever take.
    //
    // The shelves are widened to get there, and that is worth noting
    // rather than working around: a *starting* tower holds 60 units all
    // told against a 130-unit appetite across three settlements, so on
    // any small tower scrap is simply never spillable. The cap only
    // starts mattering to a tower that has built enough storage to hold
    // more metal than the world will buy, which is the tower this rule
    // is for.
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                for shelf in &mut room.shelves {
                    if shelf.item.is_none() {
                        shelf.max = 60;
                        shelf.item = Some(scrap);
                        shelf.count = shelf.max;
                    }
                }
            }
        }
    }
    let before = total_in_flight(game.state(), scrap);
    assert!(
        before > cap,
        "the fixture shelved {before}, which is under the {cap} the boards would take"
    );

    game.step(12_000);
    let after = total_in_flight(game.state(), scrap);
    assert!(
        after < before,
        "a tower drowning in scrap threw none of it away"
    );
    // Within one crew load of the cap rather than exactly on it: a spill
    // is a whole armful, decided when the load is picked up, so the last
    // one can carry the total a little under the line. What must not
    // happen is the chute emptying the tower.
    let load = game.content().balance.crew.carry_capacity;
    assert!(
        after >= cap - load,
        "the chute took the salvage well below what the boards will buy: \
         {after} left, {cap} wanted, one load is {load}"
    );
}

#[test]
fn nobody_climbs_down_a_chute() {
    // Crew must never route *through* one, however fast it looks.
    let mut game = engine(4103);
    crate::tests::stock_for_shaft(&mut game, "shaft.chute", 1);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.chute".into(),
        low: 0,
        high: 2,
        slot: 7,
    })
    .expect("slot 7 is clear on the lower floors");
    let chute = game
        .state()
        .tower
        .shafts
        .iter()
        .find(|shaft| shaft.kind == crate::content::ShaftKind::Chute)
        .expect("standing")
        .id;

    for _ in 0..6000 {
        game.step(1);
        for member in &game.state().crew {
            let riding = match member.state {
                CrewState::Boarding { shaft, .. }
                | CrewState::Climbing { shaft, .. }
                | CrewState::Riding { shaft, .. } => shaft == chute,
                _ => false,
            };
            assert!(!riding, "{} tried to travel on a chute", member.name);
        }
    }
}
