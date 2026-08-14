//! Charge: the sun, the burner, and what happens when the pool runs
//! dry.
//!
//! M6 cut the sails, so the sun no longer pays in charge at all — it
//! pays the garden in food, and it decides whether the lamps have to
//! come on. Everything else is burnt bamboo.

use crate::command::GameCommand;
use crate::engine::GameEngine;
use crate::tests::{content, engine, item};

fn night_without_walking(game: &mut GameEngine) {
    game.state_mut_for_test().clock.tick_of_day = 0;
    game.state_mut_for_test().walking = false;
}

#[test]
fn the_stairs_are_the_opening_towers_emergency_power_trunk() {
    let mut game = engine(9_101);
    night_without_walking(&mut game);
    game.step(1);
    for floor in 0..game.state().tower.floors.len() as u8 {
        assert!(
            game.state()
                .power
                .served_at(floor, crate::state::power::PowerUse::Lamps)
                > 0,
            "the intact stairs should reach floor {floor}"
        );
    }
}

#[test]
fn severing_the_only_power_trunk_forms_floor_islands() {
    let mut game = engine(9_102);
    night_without_walking(&mut game);
    game.step(1);
    game.state_mut_for_test().tower.shafts[0].health.hp = 0;
    game.step(1);
    assert_eq!(
        game.state()
            .power
            .served_at(2, crate::state::power::PowerUse::Lamps),
        0,
        "a deck with no local source must go dark when the riser is cut"
    );
}

#[test]
fn a_local_bank_keeps_its_island_alive() {
    let mut game = engine(9_103);
    night_without_walking(&mut game);
    game.step(1);
    {
        let state = game.state_mut_for_test();
        state.tower.shafts[0].health.hp = 0;
        state.power.floor_charge.fill(0);
        state.power.floor_charge[4] = 100;
        state.power.charge = 100;
    }
    game.step(1);
    assert_eq!(
        game.state()
            .power
            .served_at(4, crate::state::power::PowerUse::Lamps),
        crate::state::power::FULL,
        "the cell bank on floor four belongs to floor four's island"
    );
    assert_eq!(
        game.state()
            .power
            .served_at(2, crate::state::power::PowerUse::Lamps),
        0
    );
}

#[test]
fn a_local_burner_can_run_an_island_without_a_bank() {
    let mut game = engine(9_104);
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    night_without_walking(&mut game);
    game.step(1);
    {
        let state = game.state_mut_for_test();
        state.tower.shafts[0].health.hp = 0;
        state.power.floor_charge.fill(0);
        state.power.charge = 0;
        let burner = state.tower.floors[3]
            .rooms
            .iter_mut()
            .find(|room| content.room(room.def).burner.is_some())
            .expect("fixture burner");
        burner.active = true;
        let fuel = burner.inputs.first_mut().expect("burner fuel stack");
        fuel.item = bamboo;
        fuel.count = 10;
    }
    game.step(1);
    assert_eq!(
        game.state()
            .power
            .served_at(3, crate::state::power::PowerUse::Lamps),
        crate::state::power::FULL
    );
}

#[test]
fn a_busbar_keeps_floors_connected_after_the_stairs_are_cut() {
    let mut game = engine(9_105);
    let content = content();
    night_without_walking(&mut game);
    game.step(1);
    {
        let state = game.state_mut_for_test();
        state.tower.shafts[0].health.hp = 0;
        state.power.floor_charge.fill(0);
        state.power.floor_charge[4] = 100;
        state.power.charge = 100;
        let mut busbar = state.tower.shafts[0].clone();
        busbar.id = crate::ids::ShaftId(999);
        busbar.def = content.shaft_idx("shaft.busbar").expect("busbar content");
        busbar.kind = crate::content::ShaftKind::Busbar;
        busbar.health.hp = busbar.health.max;
        state.tower.shafts.push(busbar);
    }
    game.step(1);
    assert_eq!(
        game.state()
            .power
            .served_at(2, crate::state::power::PowerUse::Lamps),
        crate::state::power::FULL,
        "the dedicated riser should carry the remote bank across the cut"
    );
}

#[test]
fn a_constrained_trunk_shares_one_circuit_between_floors() {
    let mut game = engine(9_106);
    let content = content();
    let scrap = item(&content, "item.scrap");
    crate::tests::stock_for(&mut game, "room.salvage_rig", 1);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.salvage_rig".into(),
        floor: 1,
        slot: 5,
    })
    .expect("the forge's physical prerequisite should stand");
    for (floor, slot) in [(1, 3), (3, 1)] {
        crate::tests::stock_for(&mut game, "room.sun_forge", 1);
        game.try_send(GameCommand::PlaceRoom {
            room: "room.sun_forge".into(),
            floor,
            slot,
        })
        .expect("forge placement");
    }
    {
        let state = game.state_mut_for_test();
        // Put the fixture burner between the two equal loads. The
        // emergency stairs can carry only half of either forge's draw
        // across each boundary, so both must brown out equally.
        let position = state.tower.floors[3]
            .rooms
            .iter()
            .position(|room| content.room(room.def).burner.is_some())
            .expect("fixture burner");
        let mut burner = state.tower.floors[3].rooms.remove(position);
        burner.slot = 5;
        let fuel = burner.inputs.first_mut().expect("burner fuel");
        fuel.count = fuel.max;
        state.tower.floors[2].rooms.push(burner);
        state.tower.floors[2].rooms.sort_by_key(|room| room.slot);
        for floor in [1usize, 3] {
            let forge = state.tower.floors[floor]
                .rooms
                .iter_mut()
                .find(|room| content.room(room.def).id == "room.sun_forge")
                .expect("placed forge");
            let input = forge.inputs.first_mut().expect("forge input");
            input.item = scrap;
            input.count = input.max;
        }
        state.clock.tick_of_day = content.balance.clock.ticks_per_day / 2;
        state.walking = false;
        state.power.charge = 0;
        state.power.floor_charge.fill(0);
    }
    game.step(1);

    let lower = game
        .state()
        .power
        .served_at(1, crate::state::power::PowerUse::Works);
    let upper = game
        .state()
        .power
        .served_at(3, crate::state::power::PowerUse::Works);
    assert!(lower > 0 && upper > 0 && lower < crate::state::power::FULL);
    assert!(
        lower.abs_diff(upper) <= 10,
        "one circuit was allocated by floor order instead of proportionally: {lower} vs {upper}"
    );
}

#[test]
fn burner_fuel_and_smoke_are_billed_on_the_source_floor() {
    let mut game = engine(9_107);
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let scrap = item(&content, "item.scrap");
    crate::tests::stock_for(&mut game, "room.salvage_rig", 1);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.salvage_rig".into(),
        floor: 1,
        slot: 5,
    })
    .expect("the forge's physical prerequisite should stand");
    crate::tests::stock_for(&mut game, "room.burner", 1);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.burner".into(),
        floor: 1,
        slot: 3,
    })
    .expect("lower burner placement");
    crate::tests::stock_for(&mut game, "room.sun_forge", 1);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.sun_forge".into(),
        floor: 3,
        slot: 1,
    })
    .expect("local forge placement");
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if content.room(room.def).burner.is_some() {
                    let fuel = room.inputs.first_mut().expect("burner fuel");
                    fuel.item = bamboo;
                    fuel.count = fuel.max;
                }
                if content.room(room.def).id == "room.sun_forge" {
                    let input = room.inputs.first_mut().expect("forge input");
                    input.item = scrap;
                    input.count = input.max;
                }
            }
        }
        state.clock.tick_of_day = content.balance.clock.ticks_per_day / 2;
        state.walking = false;
        // Full banks leave no spare-generation bill. The only fuel owed
        // this tick is the floor-three forge's live draw.
        state.power.charge = state.power.capacity;
    }
    game.step(1);

    let burning = |floor: usize| {
        game.state().tower.floors[floor]
            .rooms
            .iter()
            .find(|room| content.room(room.def).burner.is_some())
            .map(|room| (room.burning, room.burn_acc))
            .expect("burner on expected floor")
    };
    assert_eq!(burning(1), (false, 0), "remote burner was billed first");
    let local = burning(3);
    assert!(
        local.0 && local.1 > 0,
        "local source was not billed: {local:?}"
    );
}

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
fn the_burner_is_the_only_income_a_tower_builds() {
    // M6 cut the sails, and this is the test that says so: the same
    // tower at the same hour banks real charge with a burner and only
    // the Heartseed's trickle without one. Nothing arrives from the
    // sky any more.
    //
    // The trickle is why this compares against a threshold rather
    // than against zero — six per 100 ticks is a floor that keeps a
    // stalled tower recoverable, not an income, and one burn is worth
    // more than a thousand ticks of it.
    let content = content();
    let noon = content.balance.clock.ticks_per_day / 2;

    let bamboo = item(&content, "item.bamboo");
    let fuel_and_settle = |game: &mut GameEngine| {
        let state = game.state_mut_for_test();
        // Fuel the burner directly: whether the crew can keep one fed
        // is haul's question, not this one.
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if let Some(stack) = room.inputs.iter_mut().find(|s| s.item == bamboo) {
                    let space = stack.space();
                    stack.deposit(space);
                }
            }
        }
        state.clock.tick_of_day = noon;
        state.power.charge = 0;
        state.walking = false;
    };

    let mut with = engine(701);
    fuel_and_settle(&mut with);
    with.step(600);
    assert!(
        with.state().power.charge > 0,
        "a fuelled burner banked nothing at noon"
    );

    let mut without = engine(701);
    without
        .state_mut_for_test()
        .tower
        .floors
        .iter_mut()
        .for_each(|floor| {
            floor
                .rooms
                .retain(|room| content.room(room.def).burner.is_none());
        });
    fuel_and_settle(&mut without);
    without.step(600);

    let trickle = content.balance.power.heartseed_charge_per_100_ticks * 6;
    assert!(
        without.state().power.charge <= trickle,
        "a tower with no burner banked {} at full noon, above the          Heartseed's {trickle} — something still pays for sunlight",
        without.state().power.charge
    );
    assert!(
        with.state().power.charge > without.state().power.charge,
        "the burner added nothing over the bare trickle"
    );
}

/// **The burst bucket must hold the largest lump the game can ask for.**
///
/// `buy_block` pays for a hundred ticks of striding in one go, lighting
/// does the same, and an emplacement pays for a shot in one go — so a
/// bucket smaller than the biggest of those makes that purchase
/// arithmetically unpayable however healthy the tower's supply. It is
/// not a balance question; the thing simply never happens.
///
/// **This is a rule attached to the data rather than written in a
/// document, deliberately** (`AGENTS.md` on exactly that). The floor was
/// set against the stride block alone once and a lantern mast — 40 a
/// shot, the largest draw in the pack — became unfirable on any tower
/// whose burner had run dry. Nothing about charge caught it; a targeting
/// test did.
#[test]
fn the_burst_floor_covers_the_largest_lump_in_the_pack() {
    let content = content();
    let power = &content.balance.power;

    let stride = power.stride_charge_per_100_ticks;
    let lamps =
        power.light_charge_per_100_ticks_per_floor * i64::from(content.balance.tower.max_floors);
    let shot = content
        .rooms
        .iter()
        .filter_map(|room| room.defence.as_ref())
        .map(|defence| defence.charge_per_shot)
        .max()
        .unwrap_or(0);

    let largest = stride.max(lamps).max(shot);
    assert!(
        power.rail_burst_floor >= largest,
        "rail_burst_floor is {} but the pack can ask for {largest} in one lump          (stride {stride}, lamps at max height {lamps}, biggest shot {shot}) —          whatever asks for the largest can never be paid for",
        power.rail_burst_floor
    );
}

/// **A tower stripped of everything still has a pipe.**
///
/// The rail is summed from burners and cell banks, so it is a brand new
/// way to rebuild the M6 deadlock: a tower out of fuel with wrecked
/// banks would supply nothing per tick and be unable to spend the
/// trickle it is still accruing. `heartseed_rail_per_tick` is the floor
/// that prevents it, and this is that floor stated directly — the
/// sibling of `a_tower_that_runs_completely_dry_can_still_crawl_out`
/// one layer down (`SYSTEMS.md` §6.36).
#[test]
fn the_pipe_never_closes_completely() {
    let content = content();
    let mut game = engine(909);
    game.step(600);

    // Strip the fuel but leave charge in the bank: the pipe's width is
    // what is being measured, and a bank with nothing in it releases
    // nothing however wide its conduit.
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                for stack in room.inputs.iter_mut().chain(room.outputs.iter_mut()) {
                    stack.count = 0;
                }
            }
        }
        state.power.charge = state.power.capacity;
    }
    game.step(1);

    assert_eq!(
        game.state().power.rail,
        content.balance.power.heartseed_rail_per_tick
            + content
                .rooms
                .iter()
                .find(|room| room.id == "room.cell_bank")
                .and_then(|room| room.bank.as_ref())
                .map_or(0, |bank| bank.discharge_per_tick),
        "a fuelless tower should be down to the Heartseed and its banks"
    );
    assert!(
        game.state().power.rail > 0,
        "the pipe closed completely and nothing can ever spend again"
    );
}

/// **The rail is what can supply *right now*, not what was built.**
///
/// A burner with an empty inbox is not a source. If it counted, a tower
/// would report supply it cannot deliver and the ranking would shed the
/// wrong circuit.
#[test]
fn an_unfuelled_burner_supplies_nothing() {
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = engine(910);
    game.step(2);

    let starved = game.state().power.rail;

    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if content.room(room.def).burner.is_some()
                    && let Some(fuel) = room.inputs.first_mut()
                {
                    fuel.item = bamboo;
                    let space = fuel.space();
                    fuel.deposit(space);
                }
            }
        }
    }
    game.step(1);
    let fed = game.state().power.rail;

    assert!(
        fed > starved,
        "a fuelled burner ({fed}) supplied no more than a starved one ({starved})"
    );
}

#[test]
fn a_tower_that_runs_completely_dry_can_still_crawl_out() {
    // **The deadlock M6 created, and the reason the Heartseed
    // trickles at all.**
    //
    // Cutting the sails left the burner as the only income. A burner
    // eats bamboo, bamboo is harvested from ground covered, and a
    // tower with no charge cannot walk — so a tower that ran dry
    // harvested nothing, and having harvested nothing stayed dry.
    // Measured on `journey.rs`'s seed 1 before the fix: charge 4/2300
    // and 160,000 ticks parked at one pace, with no path back for any
    // player at any skill.
    //
    // This is the anti-deadlock property stated directly: strip the
    // tower of every joule and every stalk, and it must still be
    // walking and cutting some time later.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = engine(707);
    game.step(600);

    {
        let state = game.state_mut_for_test();
        state.power.charge = 0;
        state.walking = true;
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                for stack in room.inputs.iter_mut().chain(room.outputs.iter_mut()) {
                    stack.count = 0;
                }
                for shelf in &mut room.shelves {
                    shelf.count = 0;
                }
            }
        }
    }

    let distance_before = game.state().world.distance;
    let harvested_before = game.state().stats.harvested_by_item[bamboo.0 as usize];

    // A full day, which is generous on purpose: the claim is that the
    // tower recovers, not that it recovers quickly.
    game.step(content.balance.clock.ticks_per_day);

    assert!(
        game.state().world.distance > distance_before,
        "a tower stripped of charge never moved again"
    );
    assert!(
        game.state().stats.harvested_by_item[bamboo.0 as usize] > harvested_before,
        "a tower stripped of charge never cut anything again"
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
fn a_wrecked_cell_bank_stops_providing_capacity() {
    let mut game = engine(1703);
    game.step(6000);
    crate::tests::stock_for(&mut game, "room.cell_bank", 2);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.cell_bank".into(),
        floor: 3,
        slot: 1,
    })
    .expect("bank placement");
    game.step(1);
    let with_bank = game.state().power.capacity;

    let bank_def = game
        .content()
        .room_idx("room.cell_bank")
        .expect("cell bank definition");
    let bank_capacity = game
        .content()
        .room(bank_def)
        .bank
        .as_ref()
        .expect("bank capacity")
        .capacity;
    game.state_mut_for_test()
        .tower
        .floors
        .iter_mut()
        .flat_map(|floor| floor.rooms.iter_mut())
        .find(|room| room.def == bank_def)
        .expect("placed cell bank")
        .health
        .hp = 0;
    game.step(1);

    assert_eq!(
        game.state().power.capacity,
        with_bank - bank_capacity,
        "destroyed storage still held charge"
    );
    assert!(game.state().power.charge <= game.state().power.capacity);
}

#[test]
fn a_garden_built_over_is_a_garden_in_the_dark() {
    // Growing taller costs you your roof. This is the mechanical form
    // of that promise, and the reason garden placement is a decision
    // you keep re-making.
    //
    // **It is also the rule that nearly went out with the sails.**
    // `top_floor_only` was enforced in exactly one place — the roof
    // filter inside `collect_solar` — so cutting the solar economy cut
    // the rule with it, and left the garden dimmed by the snapshot
    // while it grew at full rate. `intake.rs` enforces it now.
    let content = content();
    let noon = content.balance.clock.ticks_per_day / 2;
    let produce = item(&content, "item.resin_feedstock");

    let grown = |game: &GameEngine| -> i64 {
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

    let mut game = engine(704);
    let top = game.state().tower.top_floor();
    crate::tests::stock_poles(&mut game, 30);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.garden".into(),
        floor: top,
        slot: 6,
    })
    .expect("the roof has room at slot 6");
    game.state_mut_for_test().clock.tick_of_day = noon;
    game.step(2400);
    let on_the_roof = grown(&game);
    assert!(on_the_roof > 0, "a garden in full noon grew nothing");

    // Build over the top of it.
    crate::tests::stock_poles(&mut game, 30);
    game.try_send(GameCommand::BuildFloor)
        .expect("affordable, and the tower is not at max_floors");
    game.state_mut_for_test().clock.tick_of_day = noon;
    let before = grown(&game);
    game.step(900);
    assert_eq!(
        grown(&game),
        before,
        "a garden kept growing with a floor built above it"
    );

    // And the cross-section says so, without a warning banner.
    let shaded = game
        .view()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .any(|room| room.shaded);
    assert!(shaded, "a shaded garden did not report itself shaded");
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
        // A burner is a chimney: `min_floor` keeps it above the works.
        // The roof, because the fixture tower's floor 2 is full and
        // this test only needs a second burner somewhere.
        floor: 3,
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
    // The fixture tower's own burner, at floor 3 slot 5. Placing a
    // second one would leave the first burning through the "switched
    // off" half of this test.
    let mut game = engine(706);
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
        floor: 3,
        slot: 5,
        active: false,
    })
    .expect("the burner is there");

    // **Fuel, not charge.** Since M6 the Heartseed trickles, so a
    // tower banks a little whatever its rooms are doing; what a
    // switched-off burner must not do is eat.
    let fuel_before = burner_fuel(&game, bamboo);
    game.step(600);
    assert_eq!(
        burner_fuel(&game, bamboo),
        fuel_before,
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
    // It read the stride credit until §6.39 retired block purchases;
    // what the legs *ask for* is the same fact stated per tick, and a
    // halted tower asks for nothing.
    use crate::state::power::PowerUse;
    assert_eq!(
        game.state().power.demand[PowerUse::Legs.index()],
        0,
        "a halted tower still asked for charge to walk"
    );
    assert!(
        !game.state().strode,
        "a tower told to stand still kept walking"
    );
    // And the control: the walking tower did ask for stride.
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
    let starved = game.state().world.distance - before;

    // **It crawls rather than stopping dead** (`SYSTEMS.md` §6.39). The
    // legs are last in line, so they are the first thing to go short —
    // and going short is now a *rate*: the tower keeps walking on the
    // Heartseed's trickle at a fraction of its pace. Asserting it stops
    // entirely was asserting the discrete model.
    let mut fed = engine(708);
    fed.state_mut_for_test().clock.tick_of_day = 0;
    let fed_before = fed.state().world.distance;
    fed.step(300);
    let full = fed.state().world.distance - fed_before;
    assert!(
        starved < full,
        "an empty bank covered {starved} against a full one's {full}"
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

    let banked_from = banked.state().world.distance;
    let empty_from = empty.state().world.distance;
    banked.step(600);
    empty.step(600);
    let banked_paces = banked.state().world.distance - banked_from;
    let empty_paces = empty.state().world.distance - empty_from;

    assert!(
        !banked.state().power.brownout,
        "a full bank still browned out overnight"
    );
    assert!(
        empty_paces * 2 < banked_paces,
        "an empty bank covered {empty_paces} against a banked tower's {banked_paces} —          the bank is supposed to be what carries a tower through the night"
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
    game.step(1);
    let view = game.view();
    let thornwright = view
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .find(|room| content.room(crate::ids::RoomIdx(room.def)).id == "room.thornwright")
        .expect("thornwright view");
    assert!(!thornwright.powered);
    assert_eq!(
        thornwright.stall,
        Some(crate::snapshot::StallTag::Unpowered)
    );
    assert!(view.power.refused[crate::state::power::PowerUse::Works.index()]);
    game.step(899);
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
fn fuel_buys_exactly_the_charge_it_is_worth() {
    // **A stalk is still worth `charge_per_burn`, spread out.** M6's
    // burner produced in discrete 800-charge burns; §6.39 makes it a
    // ceiling billed for what is drawn, so the property is no longer
    // "one burn moved the bank by 800" but "N stalks bought N x 800".
    //
    // Measured over a window with the tower parked and the bank drained
    // each tick, so every joule generated has somewhere to go and none
    // is lost to a full bank.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = engine(717);
    game.try_send(GameCommand::SetStriding { walking: false })
        .expect("always legal");
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if content.room(room.def).burner.is_some()
                    && let Some(fuel) = room.inputs.first_mut()
                {
                    fuel.item = bamboo;
                    let space = fuel.space();
                    fuel.deposit(space);
                }
            }
        }
    }

    let fuel_before = game.state().stats.fuel_burned;
    let mut generated = 0i64;
    for _ in 0..4_000 {
        game.step(1);
        generated += game.state().power.income_last;
        // Keep the bank hungry so generation is never wasted.
        game.state_mut_for_test().power.charge = 0;
    }
    let burned = i64::try_from(game.state().stats.fuel_burned - fuel_before).unwrap_or(0);
    assert!(burned > 0, "no fuel was consumed, so this measures nothing");

    let worth = content
        .rooms
        .iter()
        .find_map(|room| room.burner.as_ref())
        .map(|burner| burner.charge_per_burn)
        .expect("the pack defines a burner");
    // **The property is that charge is never free**, which is the one
    // an equality cannot state here: `income_last` also carries the
    // Heartseed's trickle, and generation spent the same tick it is made
    // never reaches the bank at all. What must hold is that the bank
    // never receives more than the fuel paid for, plus that trickle.
    let trickle = content.balance.power.heartseed_charge_per_100_ticks * 4_000 / 100;
    let ceiling = burned * worth + trickle;
    assert!(
        generated <= ceiling,
        "{burned} stalks and a {trickle} trickle banked {generated}, over the {ceiling} paid for"
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
fn the_sun_scales_a_garden_rather_than_switching_it_on_and_off() {
    // Two identical towers at the same moment of the day, one under
    // canopy and one in a ruin-field, must grow measurably different
    // amounts. A garden that ignored terrain would still pass the
    // "gardens produce something" test.
    //
    // This used to be the sails' test, and it is the garden's now:
    // since M6 the garden is the only thing in the game the sky pays,
    // so it is the only place terrain's `sun_pct` can be caught
    // failing to arrive.
    let content = content();
    let canopy = content.terrain_idx("terrain.canopy").unwrap();
    let ruins = content.terrain_idx("terrain.ruin_field").unwrap();
    let noon = content.balance.clock.ticks_per_day / 2;
    let produce = item(&content, "item.resin_feedstock");

    let harvest = |kind| {
        let mut game = engine(712);
        let top = game.state().tower.top_floor();
        crate::tests::stock_poles(&mut game, 30);
        game.try_send(GameCommand::PlaceRoom {
            room: "room.garden".into(),
            floor: top,
            slot: 6,
        })
        .expect("the roof has room at slot 6");
        // One person in it, because a farm without them grows nothing.
        // The other two remain the same in both fixtures, so terrain is
        // the only difference under measurement.
        assert_eq!(crate::tests::staff(&mut game, top, 6, 1), 1);
        {
            let state = game.state_mut_for_test();
            for band in &mut state.world.bands {
                band.kind = kind;
            }
            state.clock.tick_of_day = noon;
            state.walking = false;
        }
        // Long enough for the crew to reach the roof and then farm.
        game.step(2400);
        game.state()
            .tower
            .floors
            .iter()
            .flat_map(|floor| floor.rooms.iter())
            .flat_map(|room| room.outputs.iter())
            .filter(|stack| stack.item == produce)
            .map(|stack| stack.count)
            .sum::<i64>()
    };

    let shaded = harvest(canopy);
    let open = harvest(ruins);
    assert!(
        open > shaded,
        "a garden grew {open} in the open against {shaded} under canopy — terrain is not reaching it"
    );
}

/// **The default ranking is the old tick order, so it changes nothing.**
///
/// A tower whose player has never touched the ranking has to behave
/// exactly as it did before the ranking existed, or every balance row
/// measured against the old behaviour silently stopped being true.
#[test]
fn the_default_charge_ranking_is_the_tick_order() {
    use crate::state::power::Power;
    let power = Power::new(100);
    let ticks: Vec<usize> = power
        .priority
        .iter()
        .map(|use_| use_.tick_position())
        .collect();
    let mut sorted = ticks.clone();
    sorted.sort_unstable();
    assert_eq!(ticks, sorted, "the default order is not the tick order");
}

/// Ranking the legs first makes an earlier circuit yield to them.
///
/// **This is the whole feature, and under §6.39 it is continuous.** The
/// tick order used to decide who got the last of the bank whatever the
/// player asked for; now the supply is divided once, in the player's
/// order, and what changes is the *rate* each circuit runs at rather
/// than whether it runs.
#[test]
fn ranking_the_legs_first_makes_the_lifts_yield() {
    use crate::state::power::{FULL, Power, PowerUse};

    let want = |power: &mut Power| {
        power.demand[PowerUse::Lifts.index()] = 60;
        power.demand[PowerUse::Legs.index()] = 60;
    };

    // Default order: lifts are served first and the legs take what is
    // left of a supply that covers only one of them.
    let mut default = Power::new(1000);
    want(&mut default);
    default.allocate(60);
    assert_eq!(default.served(PowerUse::Lifts), FULL);
    assert_eq!(default.served(PowerUse::Legs), 0);

    // Legs first: exactly the reverse, on the same tower and the same
    // supply. Nothing but the order moved.
    let mut legs_first = Power::new(1000);
    want(&mut legs_first);
    legs_first.priority = vec![
        PowerUse::Legs,
        PowerUse::Lifts,
        PowerUse::Works,
        PowerUse::Guns,
        PowerUse::Lamps,
    ];
    legs_first.allocate(60);
    assert_eq!(legs_first.served(PowerUse::Legs), FULL);
    assert_eq!(legs_first.served(PowerUse::Lifts), 0);
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

#[test]
fn a_paused_game_is_not_a_brown_out() {
    // **Found by playing the game through its own agent tools**
    // (`SYSTEMS.md` §6.31). The first `look` of a fresh run printed
    // `Day 1, Morning · 0 paces · brownout` directly above
    // `Charge 800/800`, and the two facts the snapshot ships disagreed:
    // `power.brownout` was false and `journey.halt` said otherwise.
    //
    // The cause was that `halt_reason` read "the player wants to walk
    // and the tower did not move" as a brown-out, and at tick 0 nothing
    // has moved because time is not running.
    let game = crate::tests::opening(2300);
    let view = game.view();
    assert!(!view.power.brownout, "a fresh tower is not browned out");
    assert_ne!(
        view.journey.halt,
        crate::snapshot::HaltView::Brownout,
        "a paused game announced a brown-out with a full bank"
    );
}

#[test]
fn the_two_brown_out_facts_agree() {
    // The property behind it: a tower that says it is halted *for* a
    // brown-out had better be in one. The legs read `journey.halt` and
    // the audio reads it too (§4.6), so a disagreement here is two
    // subsystems telling the player different things.
    let mut game = crate::tests::engine(2301);
    for _ in 0..600 {
        game.step(30);
        let view = game.view();
        if view.journey.halt == crate::snapshot::HaltView::Brownout {
            assert!(
                view.power.brownout,
                "halt said brown-out at tick {} while power did not",
                view.tick
            );
        }
    }
}
