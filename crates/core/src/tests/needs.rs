//! Meals, sleep, and the rota.
//!
//! Everything here goes through commands and state, per `tests.rs`. The
//! two needs are tested by the shapes they are supposed to have rather
//! than by their numbers: hunger is a supply problem, tiredness is a
//! scheduling problem, and neither is allowed to make the bottleneck
//! instrument lie.

use crate::command::GameCommand;
use crate::content::Shift;
use crate::ids::CrewId;
use crate::state::{CrewState, Errand};
use crate::tests::{content, engine, item};

/// Ticks in a day, read from the pack rather than written twice.
fn day(game: &crate::engine::GameEngine) -> u32 {
    game.content().balance.clock.ticks_per_day
}

// ---------------------------------------------------------------------------
// The rota
// ---------------------------------------------------------------------------

#[test]
fn a_run_opens_with_its_crew_awake() {
    // The tower sets out in the morning. Before `Clock::new` learned
    // where the day shift starts, a run opened at predawn — the night
    // band — with every crew member on the default day shift asleep for
    // the first 2,592 ticks. Nothing moved, and the only available
    // reading was that the game was broken.
    let game = engine(1);
    assert!(
        game.state().crew.iter().all(|member| !member.is_asleep()),
        "a run opened with its crew in bed"
    );
    assert!(
        game.state()
            .crew
            .iter()
            .all(|member| member.shift == Shift::Day),
        "crew should start on the day shift, so a player who never opens the roster has a tower that works in daylight"
    );
}

#[test]
fn the_day_shift_sleeps_at_night_and_wakes_in_the_morning() {
    let mut game = engine(2);
    let length = day(&game);

    // Somewhere in the middle of the night band.
    game.step(length * 3 / 4);
    assert!(
        game.state()
            .crew
            .iter()
            .any(super::super::state::crew::Crew::is_asleep),
        "nobody was asleep in the middle of the night"
    );

    // And round to the next morning.
    game.step(length / 2);
    assert!(
        game.state().crew.iter().all(|member| !member.is_asleep()),
        "somebody was still asleep well into the morning"
    );
}

#[test]
fn a_sleeper_is_never_stressed() {
    // `wait_ticks` is the only bottleneck instrument in the game and
    // `CrewView.stressed` is driven purely by it. A red-tinted sleeper
    // would make that instrument lie.
    let mut game = engine(3);
    let length = day(&game);
    let mut saw_a_sleeper = false;
    for _ in 0..length {
        game.step(1);
        for member in &game.state().crew {
            if member.is_asleep() {
                saw_a_sleeper = true;
                assert_eq!(
                    member.wait_ticks, 0,
                    "a sleeping crew member accumulated stress"
                );
            }
            if matches!(member.state, CrewState::Eating { .. }) {
                assert_eq!(member.wait_ticks, 0, "eating is not being blocked");
            }
        }
    }
    assert!(saw_a_sleeper, "a whole day passed and nobody slept");
}

#[test]
fn nobody_falls_asleep_on_a_delivery_they_could_have_finished() {
    // Going off shift stops crew taking *new* work; a load already in
    // hand is delivered first. The exception is a carrier the tower has
    // stranded — no free shelf and no hungry room — who has no trip to
    // finish and would otherwise never sleep again. They go to bed
    // holding it, which keeps the invariant that actually matters:
    // nothing a crew member picks up is ever destroyed.
    let mut game = engine(4);
    for _ in 0..day(&game) * 2 {
        game.step(1);
        let stock_full = game.state().tower.floors.iter().all(|floor| {
            floor
                .rooms
                .iter()
                .all(|room| room.shelves.iter().all(|shelf| shelf.count >= shelf.max))
        });
        for member in &game.state().crew {
            if member.is_asleep() && member.is_carrying() {
                assert!(
                    stock_full,
                    "{} went to bed holding a load the tower had room for",
                    member.name
                );
            }
        }
    }
}

#[test]
fn a_stranded_carrier_still_gets_to_sleep() {
    // The failure this guards against is somebody who never rests
    // again: awake every night, sliding into permanent tiredness, and
    // working at 60% for the rest of the run because one crate had
    // nowhere to go.
    let mut game = engine(16);
    let bamboo = item(game.content(), "item.bamboo");
    {
        // Fill every shelf and every inbox, and switch every room off so
        // nothing consumes its way back to having space. There is then
        // genuinely nowhere in the tower to put anything down.
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                room.active = false;
                for shelf in &mut room.shelves {
                    shelf.item = Some(bamboo);
                    shelf.count = shelf.max;
                }
                for stack in &mut room.inputs {
                    stack.count = stack.max;
                }
            }
        }
        for member in &mut state.crew {
            member.carrying = Some((bamboo, 1));
        }
    }

    let mut slept = false;
    for _ in 0..day(&game) {
        game.step(1);
        if game.state().crew.iter().any(crate::state::Crew::is_asleep) {
            slept = true;
            break;
        }
    }
    assert!(slept, "a stranded carrier never got to bed");
    assert!(
        game.state()
            .crew
            .iter()
            .all(|member| member.carrying.is_some()),
        "somebody dropped their load to go to sleep"
    );
}

#[test]
fn a_sleeper_is_never_handed_work() {
    let mut game = engine(5);
    for _ in 0..day(&game) {
        game.step(1);
        for member in &game.state().crew {
            if member.is_asleep() {
                assert!(member.task.is_none(), "a sleeper was handed a haul");
                assert!(
                    member.repair_target().is_none(),
                    "a sleeper was handed a repair"
                );
            }
        }
    }
}

#[test]
fn setting_a_sleeper_to_the_other_shift_wakes_them_at_once() {
    // The rota's one emergency verb, built out of nothing but the
    // definition of awake. It costs what it should: they wake unrested
    // and on the slow multiplier, and come morning they are off shift.
    let mut game = engine(6);
    game.step(day(&game) * 3 / 4);

    let sleeper = game
        .state()
        .crew
        .iter()
        .find(|member| member.is_asleep())
        .map(|member| member.id)
        .expect("somebody is asleep in the middle of the night");

    game.try_send(GameCommand::SetShift {
        crew: sleeper,
        shift: Shift::Night,
    })
    .expect("the roster may reshift anybody");
    game.step(1);

    let woken = game
        .state()
        .crew
        .iter()
        .find(|member| member.id == sleeper)
        .expect("they are still aboard");
    assert!(!woken.is_asleep(), "the surge lever did not wake anybody");
}

#[test]
fn nothing_wakes_a_sleeper_by_itself() {
    // An attack does not rouse anybody. If the simulation woke people
    // when things got bad, the rota would be decorative and the
    // interesting decision would be made by the game.
    let mut game = engine(7);
    game.step(day(&game) * 3 / 4);
    let asleep: Vec<CrewId> = game
        .state()
        .crew
        .iter()
        .filter(|member| member.is_asleep())
        .map(|member| member.id)
        .collect();
    assert!(!asleep.is_empty(), "nobody was asleep to begin with");

    // Break the tower open while they sleep.
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            floor.panel.hp = 1;
        }
    }
    game.step(120);

    for id in asleep {
        let member = game
            .state()
            .crew
            .iter()
            .find(|member| member.id == id)
            .expect("still aboard");
        assert!(
            member.is_asleep() || member.shift != Shift::Day,
            "{} woke up because the tower was attacked",
            member.name
        );
    }
}

#[test]
fn reshifting_nobody_is_refused() {
    let mut game = engine(8);
    let err = game
        .try_send(GameCommand::SetShift {
            crew: CrewId(9999),
            shift: Shift::Night,
        })
        .expect_err("there is no crew member 9999");
    assert!(matches!(
        err,
        crate::command::CommandError::NoSuchCrew { .. }
    ));
}

// ---------------------------------------------------------------------------
// Beds
// ---------------------------------------------------------------------------

#[test]
fn beds_are_shared_between_shifts() {
    // The thing nobody designed, which falls straight out of the model:
    // a bed is only occupied while its sleeper is off shift, so two
    // crew on opposite shifts need one bed between them.
    let mut game = engine(9);
    crate::tests::stock_poles(&mut game, 10);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.bunk".into(),
        floor: 3,
        slot: 1,
    })
    .expect("a bunk is three poles and two slots");

    let ids: Vec<CrewId> = game.state().crew.iter().map(|member| member.id).collect();
    // Everybody but the first onto the night shift, so at most one
    // person is ever off shift at a time.
    for id in ids.iter().skip(1) {
        game.try_send(GameCommand::SetShift {
            crew: *id,
            shift: Shift::Night,
        })
        .expect("the roster may reshift anybody");
    }

    let mut slept_in_a_bed = 0;
    for _ in 0..day(&game) {
        game.step(1);
        for member in &game.state().crew {
            if member.is_asleep() && member.errand.is_some_and(|e| e.is_bunk()) {
                slept_in_a_bed += 1;
            }
        }
    }
    assert!(
        slept_in_a_bed > 0,
        "one bunk served nobody across a whole day of a split rota"
    );
}

#[test]
fn a_bunk_never_holds_more_sleepers_than_it_has_beds() {
    // Occupancy is derived by scanning the crew whose errand names the
    // room — there is no counter to get out of step with reality, and
    // this is what makes sure the scan is actually consulted.
    let mut game = engine(10);
    crate::tests::stock_poles(&mut game, 10);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.bunk".into(),
        floor: 3,
        slot: 1,
    })
    .expect("a bunk is three poles and two slots");
    let beds = usize::from(
        game.content()
            .rooms
            .iter()
            .find(|room| room.id == "room.bunk")
            .and_then(|room| room.quarters.as_ref())
            .expect("a bunk has quarters")
            .sleepers,
    );

    for _ in 0..day(&game) * 2 {
        game.step(1);
        let claimed = game
            .state()
            .crew
            .iter()
            .filter(|member| member.errand.is_some_and(|e| e.is_bunk()))
            .count();
        assert!(
            claimed <= beds,
            "{claimed} crew claimed a bunk with {beds} beds"
        );
    }
}

#[test]
fn with_no_bed_at_all_they_lie_down_where_they_stand() {
    // Survivable and visibly degrading, which is the right shape for a
    // cost the player can stop paying at any moment for three poles.
    let mut game = engine(11);
    game.step(day(&game) * 3 / 4);
    let bunkless = game
        .state()
        .crew
        .iter()
        .find(|member| member.is_asleep())
        .expect("the starting tower has no bunk, so they sleep on the deck");
    assert!(
        bunkless.errand.is_none(),
        "somebody found a bed in a tower with no quarters in it"
    );
}

#[test]
fn a_bed_rests_you_faster_than_the_deck_does() {
    let content = content();
    let balance = &content.balance.crew;
    assert!(
        balance.rest_gain_per_tick > balance.no_bunk_rest_gain,
        "a bunk has to be worth building"
    );
    // The arithmetic the design rests on: a night off shift in a bed
    // refills more than a day's work spends, and on the deck it does
    // not.
    let night = 6_048u32;
    let day_shift = 8_352u32;
    assert!(
        night * balance.rest_gain_per_tick >= day_shift,
        "a bunked day worker does not wake up full"
    );
    assert!(
        night * balance.no_bunk_rest_gain < day_shift,
        "sleeping on the deck was supposed to be a net loss"
    );
}

// ---------------------------------------------------------------------------
// Meals
// ---------------------------------------------------------------------------

#[test]
fn a_hungry_crew_member_goes_and_eats() {
    let mut game = engine(12);
    crate::tests::stock_poles(&mut game, 10);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.canteen".into(),
        floor: 1,
        slot: 4,
    })
    .expect("a canteen is five poles and two slots");

    // Put a meal in reach and make somebody hungry enough to want it.
    let meals = item(game.content(), "item.meals");
    let hungry = game.content().balance.crew.hungry_ticks;
    {
        let state = game.state_mut_for_test();
        state.shelve(meals, 4);
        for member in &mut state.crew {
            member.hunger = hungry;
        }
    }

    let mut ate = false;
    for _ in 0..2_000 {
        game.step(1);
        if game
            .state()
            .crew
            .iter()
            .any(|member| matches!(member.state, CrewState::Eating { .. }))
        {
            ate = true;
            break;
        }
    }
    assert!(ate, "a hungry crew member never went to a meal");
}

#[test]
fn eating_resets_hunger_and_consumes_exactly_one_meal() {
    let mut game = engine(13);
    let meals = item(game.content(), "item.meals");
    let hungry = game.content().balance.crew.hungry_ticks;
    {
        let state = game.state_mut_for_test();
        state.shelve(meals, 6);
        for member in &mut state.crew {
            member.hunger = hungry;
        }
    }
    let before = game.state().stock_of(meals);

    let mut fed = 0;
    for _ in 0..6_000 {
        game.step(1);
        fed = game
            .state()
            .crew
            .iter()
            .filter(|member| member.hunger < hungry)
            .count();
        if fed > 0 {
            break;
        }
    }
    assert!(fed > 0, "nobody's hunger was ever reset by a meal");
    let after = game.state().stock_of(meals);
    assert_eq!(
        before - after,
        fed as i64,
        "meals consumed did not match mouths fed"
    );
}

#[test]
fn a_fed_tower_never_sees_the_penalty() {
    // The whole point of two thresholds rather than one: with a third
    // of a day between going-to-eat and slowing-down, the penalty
    // appears only when the kitchen has actually failed.
    let content = content();
    let balance = &content.balance.crew;
    assert!(
        balance.starving_ticks > balance.hungry_ticks,
        "the gap between the thresholds is the design"
    );
    assert!(
        balance.starving_ticks - balance.hungry_ticks >= content.balance.clock.ticks_per_day / 6,
        "the gap has to be wide enough that a working kitchen never reaches the penalty"
    );
}

#[test]
fn a_meal_errand_that_loses_its_room_clears_rather_than_spinning() {
    let mut game = engine(14);
    crate::tests::stock_poles(&mut game, 10);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.canteen".into(),
        floor: 1,
        slot: 4,
    })
    .expect("a canteen is five poles and two slots");

    let meals = item(game.content(), "item.meals");
    let hungry = game.content().balance.crew.hungry_ticks;
    {
        let state = game.state_mut_for_test();
        state.shelve(meals, 4);
        for member in &mut state.crew {
            member.hunger = hungry;
        }
    }
    // Let somebody commit to a meal, then take the whole tower's meals
    // away underneath them.
    for _ in 0..600 {
        game.step(1);
        if game
            .state()
            .crew
            .iter()
            .any(|member| matches!(member.errand, Some(Errand::Meal { .. })))
        {
            break;
        }
    }
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                for shelf in &mut room.shelves {
                    if shelf.item == Some(meals) {
                        shelf.item = None;
                        shelf.count = 0;
                    }
                }
                for stack in &mut room.outputs {
                    if stack.item == meals {
                        stack.count = 0;
                    }
                }
            }
        }
    }
    game.step(600);
    assert!(
        game.state()
            .crew
            .iter()
            .all(|member| !matches!(member.errand, Some(Errand::Meal { .. }))),
        "somebody is still walking to a meal that no longer exists"
    );
}

// ---------------------------------------------------------------------------
// The multiplier
// ---------------------------------------------------------------------------

#[test]
fn being_cared_for_is_the_baseline_and_never_a_buff() {
    // A food that made people *faster* would turn the crew into a
    // throughput stat to optimise, which is the one thing the fourth
    // structural call exists to prevent.
    let content = content();
    let mut member = crate::state::Crew::new(
        CrewId(1),
        "Test".into(),
        0,
        content.balance.crew.rested_max_ticks,
    );
    assert_eq!(
        crate::systems::needs::work_pct(&member, &content, true),
        100,
        "a fed, rested crew member in a lit tower is the baseline"
    );

    // And every penalty only ever takes away.
    member.hunger = content.balance.crew.starving_ticks;
    let starving = crate::systems::needs::work_pct(&member, &content, true);
    assert!(starving < 100);
    member.rested = 0;
    let also_tired = crate::systems::needs::work_pct(&member, &content, true);
    assert!(also_tired < starving);
    let and_dark = crate::systems::needs::work_pct(&member, &content, false);
    assert!(and_dark < also_tired);
    assert!(
        and_dark >= 1,
        "a need that halts the tower is a death spiral"
    );
}

#[test]
fn the_penalty_stretches_the_leg_rather_than_shrinking_the_step() {
    // The trap: applying a percentage to `Fx::ratio(1, ticks)`
    // reproduces the truncation that made four terrain yields behave as
    // two before M3. Scaling the tick count keeps a single integer
    // division and leaves the Fx precision where it already is.
    use crate::systems::needs::effective_ticks;
    assert_eq!(effective_ticks(12, 100), 12);
    assert_eq!(effective_ticks(12, 50), 24);
    assert_eq!(effective_ticks(30, 60), 50);
    // And it is monotone: slower is never faster.
    let mut last = 0;
    for pct in (1..=100).rev() {
        let ticks = effective_ticks(30, pct);
        assert!(ticks >= last, "work_pct {pct} was not monotone");
        last = ticks;
    }
}

#[test]
fn a_slow_crew_member_still_arrives() {
    // The worst case is crawling, never stopped. A leg that never
    // completes would strand somebody for the rest of the run.
    let mut game = engine(15);
    {
        let state = game.state_mut_for_test();
        for member in &mut state.crew {
            member.hunger = u32::MAX / 2;
            member.rested = 0;
        }
    }
    let before = game.state().stats.hauls_completed;
    game.step(day(&game));
    assert!(
        game.state().stats.hauls_completed > before,
        "a starving, exhausted crew delivered nothing at all in a whole day"
    );
}

// ---------------------------------------------------------------------------
// The pack
// ---------------------------------------------------------------------------

#[test]
fn the_rota_is_two_contiguous_bands() {
    // A rota with two separate night stretches is not a rota; it is a
    // bug in the content pack, and a broken pack is a load error.
    let content = content();
    let parts = &content.dayparts;
    let handovers = parts
        .iter()
        .zip(parts.iter().cycle().skip(1))
        .take(parts.len())
        .filter(|(a, b)| a.shift != b.shift)
        .count();
    assert_eq!(handovers, 2, "the shipped rota is not two contiguous bands");
    assert!(parts.iter().any(|part| part.shift == Shift::Day));
    assert!(parts.iter().any(|part| part.shift == Shift::Night));
}

#[test]
fn the_night_shift_is_the_dark_shift() {
    // Against the shipped sun curve, exposure drops below the lighting
    // threshold and climbs back through it entirely inside the night
    // band. That is what makes "the night shift is the dark shift"
    // true rather than merely intended.
    let content = content();
    let threshold = content.balance.clock.night_light_threshold;
    for permille in (0..1000).step_by(10) {
        let dark = content.sun_pct_at(permille) < threshold;
        if dark {
            let part = &content.dayparts[content.daypart_at(permille).0 as usize];
            assert_eq!(
                part.shift,
                Shift::Night,
                "permille {permille} is dark but belongs to the day shift"
            );
        }
    }
}

#[test]
fn the_night_band_is_the_shorter_one() {
    // Staffing the night costs more hands than it returns, and that
    // asymmetry is the night shift's compensation for being the
    // dangerous one.
    let content = content();
    let parts = &content.dayparts;
    let mut day_span = 0i64;
    for (i, part) in parts.iter().enumerate() {
        let next = parts[(i + 1) % parts.len()].start_permille;
        let span = if next > part.start_permille {
            next - part.start_permille
        } else {
            1000 - part.start_permille + next
        };
        if part.shift == Shift::Day {
            day_span += span;
        }
    }
    assert!(
        day_span > 500,
        "the day band should be the longer of the two; it is {day_span} per-mille"
    );
}
