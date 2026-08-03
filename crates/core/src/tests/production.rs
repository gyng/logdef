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
