//! Snapshot shape. These are the contract with the frontend: if one of
//! these changes, a TypeScript type changes with it in the same commit.

use crate::content::RoomCategory;
use crate::tests::{engine, item};

#[test]
fn the_catalog_describes_the_whole_pack() {
    let game = engine(400);
    let catalog = game.catalog();

    assert_eq!(catalog.items.len(), game.content().items.len());
    assert_eq!(catalog.rooms.len(), game.content().rooms.len());
    assert_eq!(catalog.terrain.len(), game.content().terrain.len());
    assert_eq!(catalog.content_hash.len(), 16, "hash must be hex u64");
    assert!(catalog.max_floors >= catalog.floor_slots / 4);

    // Indices in the catalog line up with the indices in the view, or
    // the frontend renders the wrong glyph on everything.
    for (i, item) in catalog.items.iter().enumerate() {
        assert_eq!(item.id, game.content().items[i].id);
    }
    for (i, room) in catalog.rooms.iter().enumerate() {
        assert_eq!(room.id, game.content().rooms[i].id);
    }
}

#[test]
fn every_room_category_is_represented() {
    let catalog = engine(401).catalog();
    for category in [
        RoomCategory::Intake,
        RoomCategory::Production,
        RoomCategory::Storage,
        RoomCategory::Heart,
    ] {
        assert!(
            catalog.rooms.iter().any(|room| room.category == category),
            "the pack has no {category:?} room"
        );
    }
}

#[test]
fn the_view_mirrors_the_tower() {
    let mut game = engine(402);
    game.step(300);
    let view = game.view();
    let state = game.state();

    assert_eq!(view.tick, state.tick);
    assert_eq!(view.tower.floors.len(), state.tower.floors.len());
    assert_eq!(view.tower.shafts.len(), state.tower.shafts.len());

    for (rendered, actual) in view.tower.floors.iter().zip(&state.tower.floors) {
        assert_eq!(rendered.index, actual.index);
        assert_eq!(rendered.rooms.len(), actual.rooms.len());
        for (room_view, room) in rendered.rooms.iter().zip(&actual.rooms) {
            assert_eq!(room_view.id, room.id.0);
            assert_eq!(room_view.slot, room.slot);
            assert_eq!(room_view.width, room.width);
        }
    }
}

#[test]
fn reported_stock_matches_what_construction_can_spend() {
    let mut game = engine(403);
    game.step(1800);
    let poles = item(game.content(), "item.poles");

    let reported = game
        .view()
        .stock
        .iter()
        .find(|entry| entry.item == poles.0)
        .map_or(0, |entry| entry.count);
    assert_eq!(reported, game.state().stock_of(poles));
}

#[test]
fn a_starved_room_is_flagged_stalled() {
    let mut game = engine(404);
    // No crew means the mill never gets fed.
    game.state_mut_for_test().crew.clear();
    game.step(600);

    let mill_stalled = game
        .view()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .any(|room| room.stalled);
    assert!(mill_stalled, "a starved chain reported itself healthy");
}

#[test]
fn alpha_stays_in_range() {
    let mut game = engine(405);
    game.set_speed(crate::state::SimSpeed::X1);
    for _ in 0..200 {
        game.frame(7_000);
        let alpha = game.alpha();
        assert!(
            (0.0..1.0).contains(&alpha),
            "interpolation alpha escaped its range: {alpha}"
        );
    }
}

#[test]
fn every_feature_names_a_real_glyph() {
    let mut game = engine(406);
    game.step(900);
    let catalog = game.catalog();
    let view = game.view();

    assert!(!view.world.features.is_empty(), "the world is bare");
    for feature in &view.world.features {
        let terrain = catalog
            .terrain
            .get(feature.band as usize)
            .expect("feature names a real band");
        assert!(
            (feature.kind as usize) < terrain.feature_kinds.len(),
            "feature kind {} out of range for {}",
            feature.kind,
            terrain.id
        );
    }
}

#[test]
fn the_view_serialises_to_json() {
    let mut game = engine(407);
    game.step(300);
    let json = serde_json::to_string(&game.view()).expect("view must serialise");
    assert!(json.contains("\"crew\""));
    assert!(json.contains("\"world\""));
    assert!(json.contains("\"tower\""));
}
