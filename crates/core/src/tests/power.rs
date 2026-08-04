//! Charge: the sun, the sails, the burner, and what happens when the
//! pool runs dry.

use crate::command::GameCommand;
use crate::tests::{content, engine, item};

#[test]
fn the_sun_rises_and_sets() {
    let content = content();
    let day = content.balance.clock.ticks_per_day;

    // Midnight is dark, midday is not. Anything else and the day/night
    // cycle the whole charge economy hangs off does not exist.
    assert_eq!(content.sun_pct_at(0), 0);
    assert!(content.sun_pct_at(500) > 80, "midday should be bright");
    assert_eq!(content.sun_pct_at(1000), 0);
    assert!(day > 0);
}

#[test]
fn the_sun_curve_has_no_steps_in_it() {
    // A jump in sun between adjacent ticks would read as a lighting
    // bug rather than as dusk, so the interpolation has to be smooth.
    let content = content();
    let mut previous = content.sun_pct_at(0);
    for permille in 1..=1000 {
        let now = content.sun_pct_at(permille);
        assert!(
            (now - previous).abs() <= 2,
            "sun jumped from {previous} to {now} at {permille}"
        );
        previous = now;
    }
}

#[test]
fn dayparts_cover_the_whole_day_in_order() {
    let content = content();
    assert!(content.dayparts.len() >= 2);
    assert_eq!(content.dayparts[0].start_permille, 0);
    for permille in 0..1000 {
        let part = content.daypart_at(permille);
        let def = content.daypart(part);
        assert!(def.start_permille <= permille);
        if let Some(next) = content.dayparts.get(part.get() + 1) {
            assert!(next.start_permille > permille);
        }
    }
}

#[test]
fn terrain_opposes_sun_and_yield() {
    // The whole "your route is your power mix" argument collapses if a
    // band is generous with both.
    let content = content();
    let mut pairs: Vec<(i64, i64)> = content
        .terrain
        .iter()
        .map(|band| (band.yield_pct, band.sun_pct))
        .collect();
    pairs.sort_by_key(|(yield_pct, _)| *yield_pct);

    let best_yield = pairs.last().expect("terrain exists");
    let worst_yield = pairs.first().expect("terrain exists");
    assert!(
        best_yield.1 < worst_yield.1,
        "the richest band for biomass should be the poorest for sun: {pairs:?}"
    );
}

#[test]
fn exposure_follows_both_the_clock_and_the_ground() {
    let content = content();
    let canopy = content.terrain_idx("terrain.canopy").unwrap();
    let ruins = content.terrain_idx("terrain.ruin_field").unwrap();

    let mut shaded = engine(700);
    let mut open = engine(700);
    for band in &mut shaded.state_mut_for_test().world.bands {
        band.kind = canopy;
    }
    for band in &mut open.state_mut_for_test().world.bands {
        band.kind = ruins;
    }

    // Wind both to the same point in the day, at midday.
    let noon = content.balance.clock.ticks_per_day / 2;
    shaded.state_mut_for_test().clock.tick_of_day = noon;
    open.state_mut_for_test().clock.tick_of_day = noon;

    let shaded_exposure = crate::systems::power::exposure_pct(shaded.state(), &content);
    let open_exposure = crate::systems::power::exposure_pct(open.state(), &content);
    assert!(
        open_exposure > shaded_exposure,
        "ruin field {open_exposure} should out-light canopy {shaded_exposure}"
    );
}

#[test]
fn sails_fill_the_banks_during_the_day() {
    let mut game = engine(701);
    // Start at dawn so the run begins with sun rather than darkness.
    let noon = content().balance.clock.ticks_per_day / 3;
    game.state_mut_for_test().clock.tick_of_day = noon;
    game.state_mut_for_test().power.charge = 0;

    game.step(1800);
    assert!(
        game.state().power.charge > 0,
        "a sunlit tower with sails banked nothing"
    );
}

#[test]
fn charge_never_exceeds_the_banks() {
    let mut game = engine(702);
    for _ in 0..200 {
        game.step(60);
        let power = &game.state().power;
        assert!(
            power.charge <= power.capacity,
            "charge {} over capacity {}",
            power.charge,
            power.capacity
        );
        assert!(power.charge >= 0);
    }
}

#[test]
fn a_cell_bank_raises_capacity() {
    let mut game = engine(703);
    game.step(1);
    let before = game.state().power.capacity;

    // Bank enough poles to afford one, then build it.
    game.step(6000);
    // A cell bank costs charge cells from M5, not poles. This test is
    // about capacity, not about the cellwright chain, so it buys them.
    crate::tests::stock_for(&mut game, "room.cell_bank", 2);
    let placed = game.try_send(GameCommand::PlaceRoom {
        room: "room.cell_bank".into(),
        floor: 3,
        slot: 1,
    });
    assert!(placed.is_ok(), "{placed:?}");
    game.step(1);

    assert!(
        game.state().power.capacity > before,
        "a cell bank did not add storage"
    );
}

#[test]
fn a_sail_below_the_roof_is_shaded() {
    // Growing taller costs you your power deck. This is the mechanical
    // form of that promise, and the reason sail placement is a decision
    // you keep re-making.
    let mut game = engine(704);
    let noon = content().balance.clock.ticks_per_day / 2;
    game.state_mut_for_test().clock.tick_of_day = noon;
    game.state_mut_for_test().power.charge = 0;
    game.step(600);
    let with_roof_sails = game.state().power.income_last;

    // Build over the top of them.
    game.step(6000);
    game.try_send(GameCommand::BuildFloor)
        .expect("affordable by now");
    game.state_mut_for_test().clock.tick_of_day = noon;
    game.step(60);

    assert!(with_roof_sails > 0, "the sails never produced at all");
    assert_eq!(
        game.state().power.income_last,
        0,
        "sails kept working with a floor built above them"
    );

    // And the cross-section says so, without a warning banner.
    let shaded = game
        .view()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .any(|room| room.shaded);
    assert!(shaded, "a shaded sail did not report itself shaded");
}

#[test]
fn a_burner_turns_bamboo_into_charge() {
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = engine(705);

    // Night, so the sails contribute nothing and the burner is the only
    // possible source.
    game.state_mut_for_test().clock.tick_of_day = 0;
    game.step(3000);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.burner".into(),
        // A burner is a chimney: `min_floor` 2 keeps it above the works.
        // Slot 1, because the starting layout already has floor 2's
        // right-hand slots.
        floor: 2,
        slot: 1,
    })
    .expect("affordable");

    // Hand it fuel directly; whether the crew deliver is the haul
    // system's business, not this test's.
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if let Some(stack) = room.inputs.iter_mut().find(|s| s.item == bamboo) {
                    let space = stack.space();
                    stack.deposit(space);
                }
            }
        }
        state.clock.tick_of_day = 0;
        state.power.charge = 0;
        state.walking = false;
    }

    game.step(600);
    assert!(
        game.state().power.charge > 0,
        "a fuelled burner produced no charge in the dark"
    );
}

#[test]
fn switching_a_room_off_stops_it() {
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = engine(706);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.burner".into(),
        // A burner is a chimney: `min_floor` 2 keeps it above the works.
        // Slot 1, because the starting layout already has floor 2's
        // right-hand slots.
        floor: 2,
        slot: 1,
    })
    .expect("affordable");
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if let Some(stack) = room.inputs.iter_mut().find(|s| s.item == bamboo) {
                    let space = stack.space();
                    stack.deposit(space);
                }
            }
        }
        state.clock.tick_of_day = 0;
        state.walking = false;
    }

    game.try_send(GameCommand::SetRoomActive {
        floor: 2,
        slot: 1,
        active: false,
    })
    .expect("the burner is there");

    game.state_mut_for_test().power.charge = 0;
    game.step(600);
    assert_eq!(
        game.state().power.charge,
        0,
        "a burner that was switched off kept burning"
    );
}

#[test]
fn halting_the_legs_banks_the_charge_they_would_have_burned() {
    let mut game = engine(707);
    let noon = content().balance.clock.ticks_per_day / 2;

    let mut walking = engine(707);
    walking.state_mut_for_test().clock.tick_of_day = noon;
    walking.step(900);

    game.state_mut_for_test().clock.tick_of_day = noon;
    game.try_send(GameCommand::SetStriding { walking: false })
        .expect("always legal");
    game.step(900);

    // **Measured on the stride credit, not on the bank.**
    //
    // Comparing the two towers' charge totals looks like the obvious
    // assertion and is not one: a stopped tower stays in whatever band
    // it stopped in while a walking one moves through others, so the
    // comparison is partly about *sunlight* and only partly about the
    // legs. It passed for two milestones and then failed the day M5
    // added a third region — which changed the world roll, which put
    // seed 707's stopped tower in shade. Nothing about halting had
    // changed at all.
    //
    // `buy_block` tops the credit up to 99 only when the legs actually
    // pay for a block, so a credit that never moves is the legs never
    // buying, and that is the whole claim.
    assert!(
        game.state().power.stride_credit >= walking.state().power.stride_credit,
        "a halted tower bought more stride than a walking one"
    );
    assert!(
        !game.state().strode,
        "a tower told to stand still kept walking"
    );
    // And the control: the walking tower did buy stride. `spent_last`
    // is one tick's spend and striding is bought in blocks of a hundred
    // ticks, so ninety-nine ticks in a hundred it is zero — the credit
    // is what carries the fact.
    assert!(
        walking.state().strode,
        "the walking tower was not actually walking, so this proves nothing"
    );
    assert_eq!(
        game.state().world.distance,
        0,
        "a halted tower kept walking"
    );
}

#[test]
fn a_tower_out_of_charge_stops_walking_before_it_stops_working() {
    // Charge priority in one assertion: striding is last in line, so it
    // is the first thing to fail.
    let mut game = engine(708);
    {
        let state = game.state_mut_for_test();
        state.clock.tick_of_day = 0; // night: no solar income
        state.power.charge = 0;
    }
    let before = game.state().world.distance;
    game.step(300);

    assert_eq!(
        game.state().world.distance,
        before,
        "the tower walked on an empty bank"
    );
    assert!(
        game.state().power.brownout,
        "a failed draw was not reported"
    );
}

#[test]
fn a_banked_night_is_survivable_and_an_empty_one_is_not() {
    let ticks_per_day = content().balance.clock.ticks_per_day;

    let mut banked = engine(709);
    let mut empty = engine(709);
    for game in [&mut banked, &mut empty] {
        game.state_mut_for_test().clock.tick_of_day = ticks_per_day * 9 / 10;
    }
    banked.state_mut_for_test().power.charge = banked.state().power.capacity;
    empty.state_mut_for_test().power.charge = 0;

    banked.step(600);
    empty.step(600);

    assert!(
        !banked.state().power.brownout,
        "a full bank still browned out overnight"
    );
    assert!(
        empty.state().power.brownout,
        "an empty bank sailed through the night"
    );
}

#[test]
fn a_powered_room_stalls_without_charge() {
    let content = content();
    let poles = item(&content, "item.poles");
    let darts = item(&content, "item.darts");
    let mut game = engine(710);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.thornwright".into(),
        floor: 1,
        slot: 4,
    })
    .expect("affordable");

    // Fuel it with poles but starve it of charge. The mill draws no
    // power and will keep working — only the thornwright should stop,
    // which is exactly the distinction being tested.
    let stock_it = |game: &mut crate::engine::GameEngine| {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if let Some(stack) = room.inputs.iter_mut().find(|s| s.item == poles) {
                    let space = stack.space();
                    stack.deposit(space);
                }
            }
        }
    };

    stock_it(&mut game);
    {
        let state = game.state_mut_for_test();
        state.clock.tick_of_day = 0;
        state.power.charge = 0;
        state.walking = false;
    }
    game.step(900);
    assert_eq!(
        crate::tests::total_in_flight(game.state(), darts),
        0,
        "the thornwright crafted with an empty bank"
    );

    // Give it charge and it comes back to life.
    stock_it(&mut game);
    game.state_mut_for_test().power.charge = game.state().power.capacity;
    game.step(900);
    assert!(
        crate::tests::total_in_flight(game.state(), darts) > 0,
        "the thornwright stayed dead after the power came back"
    );
}

#[test]
fn a_burn_consumes_exactly_its_fuel_and_yields_exactly_its_charge() {
    // The end-to-end burner test only asks "did charge go up", which
    // leaves every number in the burn loop free to drift. This pins
    // them: one burn, on the tick it lands, costs exactly
    // `fuel_per_burn` and pays exactly `charge_per_burn`.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let burner = content
        .rooms
        .iter()
        .find(|room| room.id == "room.burner")
        .and_then(|room| room.burner.as_ref())
        .expect("the pack defines a burner");

    let mut game = engine(711);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.burner".into(),
        // A burner is a chimney: `min_floor` 2 keeps it above the works.
        // Slot 1, because the starting layout already has floor 2's
        // right-hand slots.
        floor: 2,
        slot: 1,
    })
    .expect("affordable");

    // Isolate it: no crew to move fuel around, night so the sails add
    // nothing, halted so the legs draw nothing, and plenty of headroom
    // in the banks so nothing is clipped.
    {
        let state = game.state_mut_for_test();
        state.crew.clear();
        state.clock.tick_of_day = 0;
        state.walking = false;
        state.power.charge = 0;
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if let Some(fuel) = room.inputs.iter_mut().find(|s| s.item == bamboo) {
                    let space = fuel.space();
                    fuel.deposit(space);
                }
            }
        }
    }

    let fuel_before = burner_fuel(&game, bamboo);
    let mut burns = 0;
    let mut saw_exact_income = false;

    // Two burns' worth of ticks, plus slack for the tick the burn
    // lands on.
    for _ in 0..(burner.burn_ticks * 2 + 2) {
        game.step(1);
        let income = game.state().power.income_last;
        if income > 0 {
            burns += 1;
            assert_eq!(
                income, burner.charge_per_burn,
                "a burn paid {income} rather than {}",
                burner.charge_per_burn
            );
            saw_exact_income = true;
        }
    }

    assert!(saw_exact_income, "the burner never produced anything");
    assert_eq!(burns, 2, "expected exactly two burns, saw {burns}");
    assert_eq!(
        fuel_before - burner_fuel(&game, bamboo),
        burner.fuel_per_burn * 2,
        "two burns did not eat exactly two burns' worth of fuel"
    );
}

fn burner_fuel(game: &crate::engine::GameEngine, fuel: crate::ids::ItemIdx) -> i64 {
    game.state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .filter(|room| game.content().room(room.def).burner.is_some())
        .flat_map(|room| room.inputs.iter())
        .filter(|stack| stack.item == fuel)
        .map(|stack| stack.count)
        .sum()
}

#[test]
fn sail_income_scales_with_exposure_rather_than_being_on_or_off() {
    // Two identical towers at the same moment of the day, one under
    // canopy and one in a ruin-field, must bank measurably different
    // amounts. A sail that ignored terrain would still pass the
    // "sails produce something" test.
    let content = content();
    let canopy = content.terrain_idx("terrain.canopy").unwrap();
    let ruins = content.terrain_idx("terrain.ruin_field").unwrap();
    let noon = content.balance.clock.ticks_per_day / 2;

    let bank = |kind| {
        let mut game = engine(712);
        {
            let state = game.state_mut_for_test();
            for band in &mut state.world.bands {
                band.kind = kind;
            }
            state.clock.tick_of_day = noon;
            state.power.charge = 0;
            state.walking = false;
            state.crew.clear();
        }
        game.step(900);
        game.state().power.charge
    };

    let shaded = bank(canopy);
    let open = bank(ruins);
    assert!(
        open > shaded,
        "sails banked {open} in the open against {shaded} under canopy — terrain is not reaching them"
    );
}

/// **The default ranking is the old tick order, so it changes nothing.**
///
/// Charge priority used to *be* the order the tick spent in. A tower
/// whose player has never touched the ranking has to behave exactly as
/// it did before the ranking existed, or every balance row measured
/// against the old behaviour silently stopped being true.
#[test]
fn the_default_charge_ranking_reserves_nothing() {
    use crate::state::power::{Power, PowerUse};
    let mut power = Power::new(100);
    power.demand = vec![50, 50, 50, 50];
    for use_ in PowerUse::ALL {
        assert_eq!(
            power.reserved_against(use_),
            0,
            "{use_:?} held charge back under the default ranking"
        );
    }
}

/// Ranking the legs first makes an earlier draw yield to them.
///
/// This is the whole feature: lifts spend at the top of the tick and
/// legs at the bottom, so without a reserve the lifts always win the
/// last of the bank whatever the player asked for.
#[test]
fn ranking_the_legs_first_makes_the_lifts_yield() {
    use crate::state::power::{Power, PowerUse};
    let mut power = Power::new(100);
    power.demand = vec![0, 0, 0, 60];
    power.priority = vec![
        PowerUse::Legs,
        PowerUse::Lifts,
        PowerUse::Works,
        PowerUse::Lamps,
    ];

    // 60 is held for the legs, so the lifts may spend only 40.
    assert_eq!(power.reserved_against(PowerUse::Lifts), 60);
    assert!(
        !power.draw(PowerUse::Lifts, 41),
        "the lifts took the legs' charge"
    );
    assert!(power.brownout, "a refusal is a brown-out");
    assert!(power.draw(PowerUse::Lifts, 40));
    assert_eq!(power.charge, 60);

    // And the legs, drawing last, still get theirs.
    assert!(power.draw(PowerUse::Legs, 60));
    assert_eq!(power.charge, 0);
}

/// A ranking that is not all four uses is refused whole.
#[test]
fn a_partial_charge_ranking_is_rejected() {
    use crate::command::GameCommand;
    use crate::state::power::PowerUse;
    let mut game = crate::tests::engine(4242);
    let before = game.state().power.priority.clone();
    let err = game
        .try_send(GameCommand::SetPowerPriority {
            order: vec![
                PowerUse::Legs,
                PowerUse::Legs,
                PowerUse::Legs,
                PowerUse::Legs,
            ],
        })
        .expect_err("four of the same is not a ranking");
    assert!(matches!(
        err,
        crate::command::CommandError::BadPowerPriority { .. }
    ));
    assert_eq!(
        game.state().power.priority,
        before,
        "a rejected ranking still changed the order"
    );
}
