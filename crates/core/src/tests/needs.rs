//! Meals, sleep, and the rota.
//!
//! Everything here goes through commands and state, per `tests.rs`. The
//! two needs are tested by the shapes they are supposed to have rather
//! than by their numbers: hunger is a supply problem, tiredness is a
//! scheduling problem, and neither is allowed to make the bottleneck
//! instrument lie.

use crate::command::{CommandError, GameCommand};
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
    // And nobody comes aboard in step with anybody else — the jitter
    // in `add_crew` is what stops need-driven sleep from rebuilding the
    // rota's own failure mode, a whole tower lying down on one tick.
    let rested: Vec<u32> = game
        .state()
        .crew
        .iter()
        .map(|member| member.rested)
        .collect();
    assert!(
        rested.iter().any(|r| *r != rested[0]),
        "every crew member came aboard with identical rest, so they will sleep in lockstep: {rested:?}"
    );
}

#[test]
fn crew_go_to_bed_when_tired_and_get_up_when_rested() {
    // **The whole of the sleep model** (`SYSTEMS.md` §6.32). It used to
    // be the clock: awake meant "the current daypart belongs to my
    // shift". It is now the person.
    let mut game = engine(2);
    let content = game.content().clone();
    let id = game.state().crew[0].id;

    // Drain one person and leave everybody else alone, so what is
    // measured is that *their* meter sent *them* to bed.
    {
        let state = game.state_mut_for_test();
        let member = state
            .crew
            .iter_mut()
            .find(|member| member.id == id)
            .expect("aboard");
        member.rested = 1;
    }
    // Long enough to finish whatever is in their hands and walk to a
    // bunk; the errand is deliberately below a task already under way.
    game.step(600);
    let member = game.state().crew.iter().find(|m| m.id == id).unwrap();
    assert!(
        member.is_asleep(),
        "{} was out of rest and did not go to bed",
        member.name
    );

    // And they get up on their own, without the clock being consulted.
    let full = crate::systems::needs::rested_max(member, &content);
    {
        let state = game.state_mut_for_test();
        let member = state.crew.iter_mut().find(|m| m.id == id).unwrap();
        member.rested = full;
    }
    game.step(2);
    let member = game.state().crew.iter().find(|m| m.id == id).unwrap();
    assert!(
        !member.is_asleep(),
        "{} was fully rested and stayed in bed",
        member.name
    );
}

#[test]
fn a_tower_left_alone_stops_sleeping_in_lockstep() {
    // **The claim the rota was cut for.** Crew loop in
    // `rested_max` + `rested_max / rest_gain` ticks, which is shorter
    // than a day, so they drift round the clock — and they start
    // jittered, so they never begin in step either. If that drift were
    // too weak the tower would still go dark at dusk and cutting the
    // rota would have bought hours without buying cover.
    let mut game = engine(31);
    let mut everybody_asleep = 0u32;
    let mut nobody_asleep = 0u32;
    let mut mixed = 0u32;
    for _ in 0..day(&game) * 3 {
        game.step(1);
        let asleep = game
            .state()
            .crew
            .iter()
            .filter(|member| member.is_asleep())
            .count();
        if asleep == 0 {
            nobody_asleep += 1;
        } else if asleep == game.state().crew.len() {
            everybody_asleep += 1;
        } else {
            mixed += 1;
        }
    }
    assert!(
        mixed > everybody_asleep,
        "the tower spent more time wholly asleep ({everybody_asleep}) than partly ({mixed}),          which is the rota's failure mode arriving by the back door"
    );
    assert!(
        nobody_asleep > 0 && mixed > 0,
        "sleep never staggered at all: {nobody_asleep} awake, {mixed} mixed, {everybody_asleep} out"
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
fn nothing_wakes_a_sleeper_by_itself() {
    // An attack does not rouse anybody. If the simulation woke people
    // when things got bad, the beds would be decorative and the
    // interesting decision — this tower is short-handed tonight, what
    // do I do about it — would be made by the game.
    //
    // 120 ticks is well inside a sleep, so nobody in this window is due
    // to get up of their own accord.
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
            member.is_asleep(),
            "{} woke up because the tower was attacked",
            member.name
        );
    }
}

// ---------------------------------------------------------------------------
// Beds
// ---------------------------------------------------------------------------

#[test]
fn one_bed_serves_more_than_one_sleeper() {
    // The thing nobody designed, which falls straight out of the model:
    // a bed is only occupied while somebody is in it, and crew who
    // drift apart are not in it at the same time. Under the rota this
    // needed the player to split the roster; it now happens by itself.
    let mut game = engine(9);
    crate::tests::stock_poles(&mut game, 10);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.bunk".into(),
        floor: 3,
        slot: 1,
    })
    .expect("a bunk is three poles and two slots");

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
    //
    // **Beds across the whole tower, not one bunk's worth.** M6 put a
    // bunk in the opening tower (`SYSTEMS.md` §6.11), so this test's
    // own bunk is the second one and three crew sleeping in two rooms
    // is not an over-subscription — it read as one.
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
    ) * game
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .filter(|room| game.content().room(room.def).quarters.is_some())
        .count();

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
    // **The bunk taken out by hand.** M6 put one in the opening tower
    // (`SYSTEMS.md` §6.11) — a crew with nowhere to lie down is a
    // puzzle rather than an opening — so "the starting tower has no
    // bunk" stopped being true and this test stopped testing anything.
    let mut game = engine(11);
    {
        let content = content();
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            floor
                .rooms
                .retain(|room| content.room(room.def).quarters.is_none());
        }
    }
    game.step(day(&game) * 3 / 4);
    let bunkless = game
        .state()
        .crew
        .iter()
        .find(|member| member.is_asleep())
        .expect("a tower with no bunk should have somebody asleep on the deck");
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
fn the_dark_is_one_contiguous_stretch_around_midnight() {
    // Against the shipped sun curve, exposure drops below the lighting
    // threshold and climbs back through it exactly once round the
    // clock. Two dark stretches would mean the lamps came on twice a
    // day, which is a bug in the curve rather than a shape anybody
    // wants — and it used to be caught by `validate_rota`, which went
    // with the rota (`SYSTEMS.md` §6.32).
    let content = content();
    let threshold = content.balance.clock.night_light_threshold;
    let dark: Vec<bool> = (0..1000)
        .step_by(10)
        .map(|permille| content.sun_pct_at(permille) < threshold)
        .collect();
    let flips = dark
        .iter()
        .zip(dark.iter().cycle().skip(1))
        .take(dark.len())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        flips, 2,
        "the lamps change state {flips} times round the day; one dark stretch means two"
    );
    assert!(dark[0], "permille 0 is predawn and should be dark");
}

/// **A hand lamp keeps the dark off one person.**
///
/// `dark_work_pct` slows everybody in a brown-out; a lamp is the tower
/// saying "not this one". It does not end the brown-out — everybody
/// else is still slow — which is the triage a siege asks for.
#[test]
fn a_hand_lamp_answers_the_dark_for_the_one_carrying_it() {
    use crate::systems::needs::work_pct;
    let game = crate::tests::engine(8801);
    let content = game.content().clone();
    let lamp = crate::tests::item(&content, "item.hand_lamp");

    let plain = &game.state().crew[0];
    let mut lit_up = plain.clone();
    lit_up.kit = Some(lamp);

    assert_eq!(
        work_pct(plain, &content, true),
        100,
        "a lit tower is normal"
    );
    assert!(
        work_pct(plain, &content, false) < 100,
        "the dark should slow somebody with no light"
    );
    assert_eq!(
        work_pct(&lit_up, &content, false),
        100,
        "a lamp did not answer the dark"
    );
}

/// Equipping draws the kit off the shelves, and handing it back returns
/// it. Nothing is consumed and nothing is duplicated.
#[test]
fn a_kit_is_lent_from_the_shelves_and_comes_back() {
    use crate::command::GameCommand;
    let mut game = crate::tests::engine(8802);
    crate::tests::stock_item(&mut game, "item.hand_lamp", 1);
    let lamp = crate::tests::item(game.content(), "item.hand_lamp");
    let crew = game.state().crew[0].id;

    assert_eq!(game.state().stock_of(lamp), 1);
    game.try_send(GameCommand::EquipCrew {
        crew,
        kit: Some("item.hand_lamp".into()),
    })
    .expect("one is on the shelves");
    assert_eq!(game.state().stock_of(lamp), 0, "the kit was not taken");
    assert_eq!(game.state().crew[0].kit, Some(lamp));

    game.try_send(GameCommand::EquipCrew { crew, kit: None })
        .expect("handing it in is always legal");
    assert_eq!(game.state().stock_of(lamp), 1, "the kit did not come back");
    assert!(game.state().crew[0].kit.is_none());
}

/// A kit nobody has is refused, and an item that is not a kit is refused
/// as a different thing — the two are separate mistakes.
#[test]
fn equipping_what_the_tower_does_not_have_is_refused() {
    use crate::command::{CommandError, GameCommand};
    let mut game = crate::tests::engine(8803);
    let crew = game.state().crew[0].id;

    let poor = game
        .try_send(GameCommand::EquipCrew {
            crew,
            kit: Some("item.hand_lamp".into()),
        })
        .expect_err("the shelves are bare of lamps");
    assert!(matches!(poor, CommandError::InsufficientStock { .. }));

    crate::tests::stock_item(&mut game, "item.poles", 4);
    let wrong = game
        .try_send(GameCommand::EquipCrew {
            crew,
            kit: Some("item.poles".into()),
        })
        .expect_err("a pole is not something to carry");
    assert!(matches!(wrong, CommandError::NotAKit { .. }));
    assert!(game.state().crew[0].kit.is_none());
}

// ---------------------------------------------------------------------------
// Practice and the work order (`SYSTEMS.md` §6.17)
// ---------------------------------------------------------------------------

#[test]
fn practice_comes_from_the_work_actually_done() {
    // **Earned by doing, and only by doing.** There is no screen where
    // a player assigns this, which is the whole reason it is allowed to
    // exist at all — a tower that has been hauling all morning has
    // people who are better at hauling, and nothing else has moved.
    use crate::state::Job;
    let content = content();
    let mut game = engine(1700);
    game.step(3000);

    let hauled: u32 = game
        .state()
        .crew
        .iter()
        .map(|member| member.practice[Job::Haul.index()])
        .sum();
    assert!(hauled > 0, "an hour of hauling taught nobody anything");

    // And nothing else, because nothing else happened. An undamaged
    // tower with no thief in it teaches no mending and no answering.
    for member in &game.state().crew {
        assert_eq!(
            member.practice[Job::Mend.index()],
            0,
            "{} got better at mending an undamaged tower",
            member.name
        );
        assert_eq!(
            member.practice[Job::Answer.index()],
            0,
            "{} got better at answering a thief that never came",
            member.name
        );
    }
    let _ = content;
}

#[test]
fn practice_stops_at_the_ceiling() {
    // An uncapped counter is a number going up forever with nothing
    // attached to it. The rank stops, so the count stops with it.
    use crate::state::Job;
    let content = content();
    let ceiling = content.balance.crew.practice_per_rank * u32::from(content.balance.crew.max_rank);
    let mut game = engine(1701);

    {
        let state = game.state_mut_for_test();
        for member in &mut state.crew {
            member.practice[Job::Haul.index()] = ceiling;
        }
    }
    game.step(600);

    for member in &game.state().crew {
        assert_eq!(
            member.practice[Job::Haul.index()],
            ceiling,
            "{} went past the ceiling",
            member.name
        );
        assert_eq!(
            member.rank(Job::Haul, &content),
            content.balance.crew.max_rank,
            "the ceiling and the top rank disagree"
        );
    }
}

#[test]
fn a_practised_crew_gets_more_done() {
    // The point of the whole thing. Two identical towers, one seed, one
    // difference: the crew of the second have done this before.
    //
    // Measured on hauls completed rather than on a tick count, because
    // a tick count would be measuring `effective_ticks` — which is a
    // unit test one file over — rather than measuring whether any of it
    // reaches the tower.
    use crate::state::Job;
    let content = content();
    let ceiling = content.balance.crew.practice_per_rank * u32::from(content.balance.crew.max_rank);

    let mut green = engine(1702);
    green.step(5400);

    let mut veteran = engine(1702);
    {
        let state = veteran.state_mut_for_test();
        for member in &mut state.crew {
            member.practice[Job::Haul.index()] = ceiling;
        }
    }
    veteran.step(5400);

    assert!(
        veteran.state().stats.hauls_completed > green.state().stats.hauls_completed,
        "practice bought nothing: {} hauls against {}",
        veteran.state().stats.hauls_completed,
        green.state().stats.hauls_completed
    );
}

#[test]
fn the_work_order_decides_what_an_idle_person_starts() {
    // A tower with damage *and* a haul available. Under the default
    // order somebody goes to mend; put hauling first and the same tower
    // on the same tick leaves the wall alone.
    use crate::state::{CrewState, Job};

    let mends_under = |order: Vec<Job>| {
        let mut game = engine(1703);
        game.try_send(GameCommand::SetWorkOrder { order })
            .expect("a full permutation");
        // Break every panel, so a mend is always the nearest thing to do.
        {
            let state = game.state_mut_for_test();
            for floor in &mut state.tower.floors {
                floor.panel.hp = 1;
            }
        }
        crate::tests::stock_poles(&mut game, 200);
        game.step(900);
        game.state()
            .crew
            .iter()
            .filter(|member| {
                matches!(member.state, CrewState::Repairing { .. })
                    || member.repair_target().is_some()
            })
            .count()
    };

    let by_default = mends_under(Job::ALL.to_vec());
    let hauling_first = mends_under(vec![Job::Answer, Job::Haul, Job::Mend, Job::Man]);

    assert!(
        by_default > 0,
        "nobody mended a tower with every panel at one hit point"
    );
    assert!(
        hauling_first < by_default,
        "putting hauling first changed nothing: {hauling_first} mending against {by_default}"
    );
}

#[test]
fn a_work_order_with_a_job_missing_is_refused() {
    // A list with a job left out is a list that has quietly made that
    // job unreachable — nobody would ever mend again and nothing would
    // say so.
    use crate::state::Job;
    let mut game = engine(1704);
    let before = game.state().work.clone();

    for bad in [
        vec![Job::Haul, Job::Mend, Job::Man],
        vec![Job::Haul, Job::Haul, Job::Mend, Job::Man],
        Vec::new(),
    ] {
        let err = game
            .try_send(GameCommand::SetWorkOrder { order: bad })
            .expect_err("not a permutation");
        assert!(matches!(err, CommandError::NotAWorkOrder), "{err:?}");
    }
    assert_eq!(
        game.state().work,
        before,
        "a refused order changed the tower"
    );
}

#[test]
fn no_work_order_lets_anybody_skip_dinner() {
    // **Needs are not jobs and are not offered as settings.** Whatever
    // the player ranks first, a hungry crew member still goes and eats
    // — a game that let you turn that off would be offering a mistake
    // as a strategy.
    use crate::state::{CrewState, Job};
    let content = content();
    let mut game = engine(1705);
    game.try_send(GameCommand::SetWorkOrder {
        order: vec![Job::Haul, Job::Man, Job::Mend, Job::Answer],
    })
    .expect("a full permutation");

    // Something to eat. The fixture has no canteen, and a shelf with
    // meals on it is a meal as far as `find_meal` is concerned — which
    // is the point of this test rather than a shortcut around it: what
    // is under test is that the *ladder* still stops for dinner, not
    // that the canteen works.
    crate::tests::stock_item(&mut game, "item.meals", 6);
    {
        let state = game.state_mut_for_test();
        for member in &mut state.crew {
            member.hunger = content.balance.crew.hungry_ticks + 1;
        }
    }
    let before = game.state().stats.meals_eaten;
    game.step(3600);
    assert!(
        game.state().stats.meals_eaten > before
            || game
                .state()
                .crew
                .iter()
                .any(|member| matches!(member.state, CrewState::Eating { .. })),
        "everybody worked through dinner because hauling was ranked first"
    );
}

// ---------------------------------------------------------------------------
// Traits (`SYSTEMS.md` §6.25)
// ---------------------------------------------------------------------------

#[test]
fn everybody_who_joins_is_somebody_in_particular() {
    // **The three you set out with have none, and everybody who joins
    // has one** (`SYSTEMS.md` §6.25). §6.11 rebuilt the opening so a new
    // player is not handed a roll they cannot read; variety arrives with
    // the people you *choose* to bring aboard.
    let content = content();
    assert!(!content.traits.is_empty(), "the pack defines no traits");
    let mut game = engine(1950);
    let started = game.state().crew.len();
    assert!(
        game.state()
            .crew
            .iter()
            .all(|member| member.traits.is_empty()),
        "the opening crew were handed a roll"
    );

    {
        let state = game.state_mut_for_test();
        for _ in 0..6 {
            state.add_crew(&content);
        }
    }
    for member in game.state().crew.iter().skip(started) {
        assert_eq!(
            member.traits.len(),
            1,
            "{} joined with {} trait(s)",
            member.name,
            member.traits.len()
        );
        assert!(
            member
                .traits
                .iter()
                .all(|idx| idx.get() < content.traits.len()),
            "{} has a trait the pack does not define",
            member.name
        );
    }
}

#[test]
fn a_trait_rolls_on_the_sim_stream_not_the_cosmetic_one() {
    // **The firewall, from the other side** (`DECISIONS.md` §2). A name
    // and a `fidget` are cosmetic because they must never move an
    // economic roll. A trait changes how fast somebody gets hungry and
    // how much they carry, so it is *not* cosmetic — and the property
    // that proves it is on the right stream is that different seeds
    // give different traits while the same seed gives the same ones.
    let content = content();
    let traits_for = |seed: u64| -> Vec<usize> {
        let mut game = engine(seed);
        {
            let state = game.state_mut_for_test();
            for _ in 0..6 {
                state.add_crew(&content);
            }
        }
        game.state()
            .crew
            .iter()
            .flat_map(|member| member.traits.iter().map(|idx| idx.get()))
            .collect()
    };
    assert_eq!(traits_for(1951), traits_for(1951), "the same seed differed");
    let mut differed = false;
    for seed in 1952..1962 {
        if traits_for(seed) != traits_for(1951) {
            differed = true;
            break;
        }
    }
    assert!(differed, "ten seeds all produced the same crew");
}

#[test]
fn a_big_appetite_eats_sooner() {
    // Traits are shaped like needs rather than like bonuses, and this
    // is the one that only costs. The tower feels it as the larder
    // emptying, not as a number.
    let content = content();
    let idx = content
        .traits
        .iter()
        .position(|def| def.id == "trait.big_appetite")
        .expect("the pack defines a big appetite");

    let mut game = engine(1953);
    {
        let state = game.state_mut_for_test();
        for member in &mut state.crew {
            member.traits.clear();
        }
        state.crew[0]
            .traits
            .push(crate::ids::TraitIdx(u16::try_from(idx).expect("small")));
    }

    let hungry = crate::systems::needs::hungry_ticks(&game.state().crew[0], &content);
    let plain = crate::systems::needs::hungry_ticks(&game.state().crew[1], &content);
    assert!(
        hungry < plain,
        "a big appetite waited as long as anybody else: {hungry} against {plain}"
    );
}

#[test]
fn somebody_who_sleeps_rough_well_frees_a_bed() {
    // The rota is the system traits exist to make interesting. A
    // nocturnal crew member rests on bare deck about as well as most
    // people do in a bunk, so the tower gets a bed back.
    let content = content();
    let nocturnal = content
        .traits
        .iter()
        .find(|def| def.id == "trait.nocturnal")
        .expect("the pack defines a nocturnal");
    let light = content
        .traits
        .iter()
        .find(|def| def.id == "trait.light_sleeper")
        .expect("the pack defines a light sleeper");
    assert!(
        nocturnal.deck_rest_pct > 100 && light.deck_rest_pct < 100,
        "the two sleep traits should point in opposite directions"
    );
}

#[test]
fn a_trait_that_says_it_is_practised_arrives_practised() {
    use crate::state::Job;
    let content = content();
    let mut game = engine(1954);
    {
        let state = game.state_mut_for_test();
        for _ in 0..12 {
            state.add_crew(&content);
        }
    }
    for member in &game.state().crew {
        for idx in &member.traits {
            let Some(job) = content.traits[idx.get()].practised_at else {
                continue;
            };
            assert!(
                member.rank(job, &content) > 0,
                "{} is a {} and knows nothing about it",
                member.name,
                content.traits[idx.get()].name
            );
            // And only that job — a trait is a head start, not a
            // finished veteran.
            for other in Job::ALL {
                if other != job {
                    assert_eq!(
                        member.rank(other, &content),
                        0,
                        "{} arrived practised at something their trait never mentioned",
                        member.name
                    );
                }
            }
        }
    }
}

#[test]
fn the_pack_has_a_lot_of_traits_and_a_rare_tail() {
    // **Rarity is the whole reason there are forty.** A pack where
    // every trait is equally likely has no rare ones by definition, and
    // somebody merely *unusual* is worth more than somebody strong: the
    // common ones are quirks you plan around and the rare ones are why
    // you remember a particular run's roster.
    let content = content();
    assert!(
        content.traits.len() >= 30,
        "only {} traits; the point of them is that a run shows you a few of many",
        content.traits.len()
    );
    let heaviest = content
        .traits
        .iter()
        .map(|def| def.weight)
        .max()
        .unwrap_or(0);
    let lightest = content
        .traits
        .iter()
        .map(|def| def.weight)
        .min()
        .unwrap_or(0);
    assert!(
        heaviest >= lightest * 4,
        "every trait is about as likely as every other ({lightest}..{heaviest}); nothing is rare"
    );
}

#[test]
fn every_trait_does_something_and_can_be_drawn() {
    // Held at load too (`content::validate`), and here so a failure
    // names the trait rather than a load error list.
    let content = content();
    for def in &content.traits {
        assert!(def.does_something(), "{} changes nothing", def.id);
        assert!(def.weight > 0, "{} can never be drawn", def.id);
        assert!(
            !def.blurb.is_empty(),
            "{} has nothing to say for itself",
            def.id
        );
    }
}

#[test]
fn a_rare_trait_is_actually_rare() {
    // Measured through the draw rather than asserted about the weights:
    // a weighted table with a bug in it still has the right weights in
    // it. Two hundred crew, and the rare tail should be a small share.
    let content = content();
    let rare: Vec<usize> = content
        .traits
        .iter()
        .enumerate()
        .filter(|(_, def)| def.weight <= 10)
        .map(|(at, _)| at)
        .collect();
    assert!(!rare.is_empty(), "the pack has no rare traits");

    let mut game = engine(1960);
    let mut drawn = 0u32;
    let mut rare_drawn = 0u32;
    {
        let state = game.state_mut_for_test();
        for _ in 0..200 {
            state.add_crew(&content);
        }
        for member in state.crew.iter().skip(3) {
            for idx in &member.traits {
                drawn += 1;
                if rare.contains(&idx.get()) {
                    rare_drawn += 1;
                }
            }
        }
    }
    assert!(drawn > 100, "only {drawn} draws to judge by");
    // Seven rare traits at weight 10 against a table summing to ~2,590
    // is about 2.7%. Anything up to a fifth is still a tail; a third is
    // not, and would mean the weights are being ignored.
    assert!(
        rare_drawn * 5 < drawn,
        "rare traits came up {rare_drawn} times in {drawn}; the weights are not binding"
    );
}

#[test]
fn a_trait_can_answer_the_dark_the_way_a_lamp_does() {
    let content = content();
    let owl = content
        .traits
        .iter()
        .position(|def| def.sees_in_the_dark)
        .expect("the pack has somebody who works unlit");

    let mut game = engine(1961);
    {
        let state = game.state_mut_for_test();
        for member in &mut state.crew {
            member.traits.clear();
            member.kit = None;
        }
        state.crew[0]
            .traits
            .push(crate::ids::TraitIdx(u16::try_from(owl).expect("small")));
    }
    let crew = &game.state().crew;
    assert_eq!(
        crate::systems::needs::work_pct(&crew[0], &content, false),
        crate::systems::needs::work_pct(&crew[0], &content, true),
        "the dark still slowed down somebody who sees in it"
    );
    assert!(
        crate::systems::needs::work_pct(&crew[1], &content, false)
            < crate::systems::needs::work_pct(&crew[1], &content, true),
        "the dark stopped costing everybody else anything"
    );
}
