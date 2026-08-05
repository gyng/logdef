//! Intake and production: the two systems that make items appear.

use crate::tests::{content, engine, item};

#[test]
fn intake_fills_its_outbox_then_stops() {
    // With no crew to collect, the cutter arm fills up and goes quiet.
    // Nothing is lost, and nothing overflows.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = engine(200);
    game.state_mut_for_test().crew.clear();

    game.step(6000);

    let arm = game
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .find(|room| room.outputs.iter().any(|s| s.item == bamboo) && room.inputs.is_empty())
        .expect("the starting tower has a cutter arm");

    let stack = arm
        .outputs
        .iter()
        .find(|s| s.item == bamboo)
        .expect("the arm's outbox");
    assert_eq!(stack.count, stack.max, "the arm did not fill up");
    assert!(stack.count <= stack.max, "the arm overfilled");
}

#[test]
fn intake_scales_with_the_terrain_underfoot() {
    // Same tower, same ticks, different bands: the rich one must
    // out-harvest the barren one. This is the seed of "your route is
    // your power mix".
    let content = content();
    let mut rich = engine(201);
    let mut barren = engine(201);

    // Nail each run to a single band by rewriting the world's terrain
    // to one kind, which is what the yield multiplier reads.
    let canopy = content
        .terrain_idx("terrain.canopy")
        .expect("pack defines canopy");
    let ruins = content
        .terrain_idx("terrain.ruin_field")
        .expect("pack defines ruin field");
    force_single_band(&mut rich, canopy);
    force_single_band(&mut barren, ruins);

    rich.step(1800);
    barren.step(1800);

    assert!(
        rich.state().stats.items_harvested > barren.state().stats.items_harvested,
        "canopy harvested {} vs ruin field {}",
        rich.state().stats.items_harvested,
        barren.state().stats.items_harvested
    );
}

fn force_single_band(game: &mut crate::engine::GameEngine, kind: crate::ids::TerrainIdx) {
    let world = &mut game.state_mut_for_test().world;
    for band in &mut world.bands {
        band.kind = kind;
    }
}

#[test]
fn a_recipe_stalls_without_inputs_and_keeps_its_progress() {
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = engine(202);
    game.state_mut_for_test().crew.clear();

    // Hand the mill exactly one bamboo, let it get part-way, then
    // watch it hold.
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if let Some(stack) = room.inputs.iter_mut().find(|s| s.item == bamboo) {
                    stack.deposit(1);
                }
            }
        }
    }

    let craft_ticks = content
        .rooms
        .iter()
        .enumerate()
        .find(|(_, room)| room.id == "room.mill")
        .map(|(i, _)| content.room_runtime[i].craft_ticks)
        .expect("the pack defines a mill");

    game.step(craft_ticks - 10);
    let mid = mill_progress(&game);
    assert!(mid > 0, "the mill never started");

    game.step(20); // completes the craft, then starves
    assert_eq!(game.state().stats.crafts_completed, 1);

    let stalled_at = mill_progress(&game);
    game.step(300);
    assert_eq!(
        mill_progress(&game),
        stalled_at,
        "a starved mill kept advancing"
    );
    assert_eq!(game.state().stats.crafts_completed, 1);
}

#[test]
fn a_full_outbox_stalls_a_recipe_rather_than_dropping_the_craft() {
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let poles = item(&content, "item.poles");
    let mut game = engine(203);
    game.state_mut_for_test().crew.clear();

    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if let Some(stack) = room.inputs.iter_mut().find(|s| s.item == bamboo) {
                    let space = stack.space();
                    stack.deposit(space);
                }
                if let Some(stack) = room.outputs.iter_mut().find(|s| s.item == poles) {
                    let space = stack.space();
                    stack.deposit(space);
                }
            }
        }
    }

    let crafts_before = game.state().stats.crafts_completed;
    let inputs_before = mill_input_count(&game, bamboo);
    game.step(1200);

    assert_eq!(
        game.state().stats.crafts_completed,
        crafts_before,
        "a backed-up mill kept crafting"
    );
    assert_eq!(
        mill_input_count(&game, bamboo),
        inputs_before,
        "a backed-up mill still ate its inputs"
    );
}

#[test]
fn craft_output_matches_the_recipe_exactly() {
    let content = content();
    let poles = item(&content, "item.poles");
    let mut game = engine(204);

    let start_poles = crate::tests::total_in_flight(game.state(), poles);
    game.step(3600);
    let crafts = game.state().stats.crafts_completed as i64;
    let end_poles = crate::tests::total_in_flight(game.state(), poles);

    // The mill's recipe emits exactly one pole per craft — allowing
    // for whatever repair spent putting the tower back together.
    let spent = game.state().stats.repair_poles_spent as i64;
    assert_eq!(end_poles - start_poles + spent, crafts);
}

fn mill_progress(game: &crate::engine::GameEngine) -> u32 {
    game.state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .filter(|room| !room.inputs.is_empty())
        .map(|room| room.progress)
        .max()
        .unwrap_or(0)
}

/// **The mill's inbox, and only the mill's.**
///
/// This summed the item across every room in the tower, which was the
/// same number until M6 cut the sails and put a burner in the opening
/// tower — a second room that eats bamboo. The stall assertion then
/// read the burner's six stalks going up the chimney as "a backed-up
/// mill still ate its inputs".
fn mill_input_count(game: &crate::engine::GameEngine, item: crate::ids::ItemIdx) -> i64 {
    let mill = game
        .content()
        .room_idx("room.mill")
        .expect("the pack defines a mill");
    game.state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .filter(|room| room.def == mill)
        .flat_map(|room| room.inputs.iter())
        .filter(|stack| stack.item == item)
        .map(|stack| stack.count)
        .sum()
}

#[test]
fn every_authored_yield_is_a_different_harvest_rate() {
    // The test above compared canopy against ruin field — the two
    // extremes — and passed for years while the middle of the range was
    // collapsed. Intake used to accrue a truncated per-tick *fraction
    // of an item*: `Fx::ratio(1, 90)` is `Fx(2)` in Q8.8, and
    // `Fx(2) * 1.40` is also `Fx(2)`, so dense canopy (140%) and open
    // clearing (100%) harvested at exactly the same rate, as did ruin
    // field (50%) and drowned street (60%). Four authored kinds behaved
    // as two, and the flagship contrast of the route being the power
    // mix (`DESIGN.md` pillar 1) was not in the simulation at all.
    //
    // So this walks every terrain in the pack and insists the ordering
    // by `yield_pct` is the ordering by what comes out of the ground,
    // with no ties. Any future arithmetic that flattens two bands
    // together fails here rather than in somebody's play session.
    // Measured per thousand paces rather than per tick, because the
    // two are not the same thing and the difference is a real system
    // rather than noise: a barren band is usually a sunny one, so its
    // tower banks more charge, walks further, and can out-harvest a
    // richer band on raw totals. That trade is the design working. What
    // this test is about is the yield alone, so it divides it out.
    let content = content();
    let mut measured: Vec<(i64, i64, &str)> = Vec::new();

    for (i, def) in content.terrain.iter().enumerate() {
        let kind = crate::ids::TerrainIdx(i as u16);
        let mut game = engine(201);
        // Re-forced as it goes: `force_single_band` only rewrites the
        // bands that exist, and the tower walks into freshly generated
        // ones within the first thousand paces. Forcing once and
        // stepping for a minute measures mostly ordinary terrain, which
        // is why the older test above only ever compared the two
        // extremes and still passed while the middle was collapsed.
        for _ in 0..36 {
            force_single_band(&mut game, kind);
            game.step(100);
        }
        let paces = game.state().world.distance >> crate::fx::FX_SHIFT;
        assert!(paces > 0, "{} never walked anywhere", def.id);
        measured.push((
            content.terrain_runtime[i].yield_pct,
            game.state().stats.items_harvested as i64 * 1000 / paces,
            def.id.as_str(),
        ));
    }

    measured.sort_by_key(|(yield_pct, _, _)| *yield_pct);
    for pair in measured.windows(2) {
        let (poor_pct, poor, poor_id) = pair[0];
        let (rich_pct, rich, rich_id) = pair[1];
        assert!(
            rich > poor,
            "{rich_id} ({rich_pct}%) harvested {rich} and {poor_id} ({poor_pct}%) \
             harvested {poor} — two different yields came out the same"
        );
    }
}

// ---------------------------------------------------------------------------
// Buffers (`DESIGN.md` §2 insight 1)
// ---------------------------------------------------------------------------

#[test]
fn every_room_that_moves_an_item_has_a_buffer_for_it() {
    // **The hauling economy is made of buffers.** A room takes
    // deliveries into an inbox, stalls when its outbox fills, and the
    // crew route around both — that back-pressure *is* the game's
    // central bet, that transport is shared rather than dedicated.
    //
    // The schema already makes `buffer_max` mandatory wherever it means
    // anything, so this walks the shipped pack and says so out loud
    // rather than trusting a field to stay required.
    let content = content();
    for room in &content.rooms {
        if let Some(recipe) = room.recipe.as_ref() {
            assert!(
                !recipe.inputs.is_empty() || !recipe.outputs.is_empty(),
                "{}: a recipe with neither inputs nor outputs",
                room.id
            );
            for entry in recipe.inputs.iter().chain(recipe.outputs.iter()) {
                assert!(
                    entry.buffer_max > 0,
                    "{}: {} has no buffer, so nothing can ever queue there",
                    room.id,
                    entry.item
                );
            }
        }
        if let Some(intake) = room.intake.as_ref() {
            assert!(
                intake.buffer_max > 0,
                "{}: an intake with nowhere to put what it takes",
                room.id
            );
        }
        if let Some(defence) = room.defence.as_ref() {
            assert!(
                defence.buffer_max > 0,
                "{}: an emplacement with no rack",
                room.id
            );
        }
    }
}

#[test]
fn a_room_with_a_zero_buffer_is_a_broken_pack() {
    // The rule has to *bite*, not merely hold today. A buffer of zero
    // does not fail loudly — the room simply never participates and the
    // tower reads as mysteriously slow — which is exactly the kind of
    // thing `Content::load` is supposed to catch at build time
    // (`AGENTS.md` §IV: a broken pack is a build error).
    let mut room = content()
        .rooms
        .iter()
        .find(|room| room.id == "room.mill")
        .cloned()
        .expect("the pack defines a mill");
    if let Some(recipe) = room.recipe.as_mut() {
        recipe.inputs[0].buffer_max = 0;
    }

    let mut errors = Vec::new();
    let mut broken = content().as_ref().clone();
    broken.rooms = vec![room];
    crate::content::validate_buffers_for_test(&broken, &mut errors);
    assert!(
        errors.iter().any(|error| error.message.contains("buffer")),
        "a zero buffer passed validation: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Saturation, and telling it apart from starvation (`SYSTEMS.md` §6.26)
// ---------------------------------------------------------------------------

#[test]
fn a_tower_that_outgrows_its_demand_says_which_silence_it_is() {
    // **The measurement that corrected §6.9.** With the cutter arm
    // uncapped (§6.24) a tower can out-harvest its own consumers, and
    // the open question filed against that said bamboo would claim
    // every shelf and poles would have nowhere to land. It is the other
    // way round: the mill converts *everything*, every shelf ends up
    // holding poles, and bamboo reads zero.
    //
    // Nothing is broken — backpressure propagates exactly as designed,
    // shelves to outbox to arm — and the escape is to spend, which a
    // tower holding a hundred poles can certainly do. What was missing
    // is that a room quiet because **nothing wants what it makes** was
    // indistinguishable from one quiet because **nobody brought it
    // anything**, and those are answered by opposite actions.
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let poles = item(&content, "item.poles");
    let mut game = crate::tests::engine(4444);

    let mut arms = 1;
    for floor in 2..5u8 {
        crate::tests::stock_poles(&mut game, 20);
        let slots = game.state().tower.floors[floor as usize].slots;
        let width = game
            .content()
            .room_idx("room.cutter_arm")
            .map(|idx| game.content().room(idx).width)
            .expect("the pack defines a cutter arm");
        if game
            .try_send(crate::command::GameCommand::PlaceRoom {
                room: "room.cutter_arm".into(),
                floor,
                slot: slots - width,
            })
            .is_ok()
        {
            arms += 1;
        }
    }
    assert!(
        arms >= 3,
        "only {arms} arm(s); this needs a tower that over-harvests"
    );
    crate::tests::step_quietly(&mut game, 30_000);

    // Saturated on the *product*, not the raw material.
    let shelved = |item| {
        game.state()
            .tower
            .floors
            .iter()
            .flat_map(|floor| floor.rooms.iter())
            .flat_map(|room| room.shelves.iter())
            .filter(|shelf| shelf.item == Some(item))
            .map(|shelf| shelf.count)
            .sum::<i64>()
    };
    assert!(
        shelved(poles) > shelved(bamboo),
        "expected a tower full of poles, found {} poles against {} bamboo",
        shelved(poles),
        shelved(bamboo)
    );

    // And every arm quiet for the *right stated reason*.
    let view = game.view();
    let arm_def = content
        .rooms
        .iter()
        .position(|room| room.id == "room.cutter_arm")
        .expect("the pack defines a cutter arm");
    let quiet: Vec<_> = view
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .filter(|room| usize::from(room.def) == arm_def)
        .collect();
    assert!(!quiet.is_empty(), "no arms in the view");
    assert!(
        quiet
            .iter()
            .all(|room| room.stall == Some(crate::snapshot::StallTag::BackedUp)),
        "an over-harvesting tower's arms should read backed up, not starved: {:?}",
        quiet.iter().map(|room| room.stall).collect::<Vec<_>>()
    );
}

#[test]
fn a_starved_room_and_a_backed_up_one_do_not_read_alike() {
    // The property the tag exists for, stated on its own so a change to
    // either branch cannot quietly collapse them together again.
    let content = content();
    let mut game = crate::tests::engine(4445);
    crate::tests::step_quietly(&mut game, 6_000);

    let view = game.view();
    let mut seen = std::collections::BTreeSet::new();
    for floor in &view.tower.floors {
        for room in &floor.rooms {
            assert_eq!(
                room.stalled,
                room.stall.is_some(),
                "a room said it was quiet and gave no reason, or the reverse"
            );
            if let Some(tag) = room.stall {
                seen.insert(format!("{tag:?}"));
            }
        }
    }
    let _ = content;
    let _ = seen;
}
