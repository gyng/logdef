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

fn mill_input_count(game: &crate::engine::GameEngine, item: crate::ids::ItemIdx) -> i64 {
    game.state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
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
