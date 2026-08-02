//! The streaming world. Properties rather than examples: terrain is
//! generated, so the interesting assertions are invariants that must
//! hold for every seed.

use crate::fx::{FX_SHIFT, paces_from_int};
use crate::tests::{content, engine};

#[test]
fn the_tower_actually_walks() {
    let mut game = engine(300);
    let start = game.state().world.distance;
    game.step(900);
    assert!(
        game.state().world.distance > start,
        "the tower did not move"
    );
}

#[test]
fn distance_advances_by_the_same_amount_every_tick() {
    let mut game = engine(301);
    game.step(1);
    let first = game.state().world.distance;
    game.step(1);
    let second = game.state().world.distance;
    game.step(1);
    let third = game.state().world.distance;
    assert_eq!(second - first, third - second, "the stride is not uniform");
}

#[test]
fn bands_tile_the_world_without_gaps_or_overlaps() {
    for seed in 0..40u64 {
        let mut game = engine(seed);
        game.step(600);
        let bands = &game.state().world.bands;
        for pair in bands.windows(2) {
            assert_eq!(
                pair[0].end(),
                pair[1].start,
                "seed {seed}: gap or overlap between bands"
            );
        }
        assert!(
            bands.iter().all(|band| band.length > 0),
            "seed {seed}: zero-length band"
        );
    }
}

#[test]
fn the_tower_is_always_standing_somewhere() {
    for seed in 0..40u64 {
        let mut game = engine(seed);
        for _ in 0..30 {
            game.step(60);
            let world = &game.state().world;
            assert!(
                world.band_at(world.distance).is_some(),
                "seed {seed}: no band under the tower at distance {}",
                world.distance
            );
        }
    }
}

#[test]
fn terrain_streams_ahead_and_is_pruned_behind() {
    let content = content();
    let ahead = paces_from_int(content.balance.world.stream_ahead_paces);
    let behind = paces_from_int(content.balance.world.stream_behind_paces);

    let mut game = engine(302);
    for _ in 0..60 {
        game.step(60);
        let world = &game.state().world;
        assert!(
            world.generated_to >= world.distance + ahead,
            "the horizon fell behind the tower"
        );
        let oldest = world.bands.first().map_or(0, |band| band.start);
        assert!(
            oldest
                >= world.distance - behind - paces_from_int(content.balance.world.band_max_paces),
            "stale terrain was never pruned"
        );
    }
}

#[test]
fn memory_stays_bounded_over_a_long_walk() {
    let mut game = engine(303);
    game.step(600);
    let early = game.state().world.bands.len() + game.state().world.features.len();
    game.step(30_000); // sixteen minutes
    let late = game.state().world.bands.len() + game.state().world.features.len();
    assert!(
        late <= early * 3,
        "the world grew without bound: {early} then {late}"
    );
}

#[test]
fn consecutive_bands_never_repeat_a_kind() {
    for seed in 0..40u64 {
        let mut game = engine(seed);
        game.step(3000);
        let bands = &game.state().world.bands;
        for pair in bands.windows(2) {
            assert_ne!(
                pair[0].kind, pair[1].kind,
                "seed {seed}: the horizon repeated itself"
            );
        }
    }
}

#[test]
fn every_feature_stands_in_a_band() {
    for seed in 0..20u64 {
        let mut game = engine(seed);
        game.step(1200);
        let world = &game.state().world;
        for feature in &world.features {
            assert!(
                world.bands.iter().any(|band| band.contains(feature.at)),
                "seed {seed}: an orphaned feature at {}",
                feature.at
            );
        }
    }
}

#[test]
fn features_are_sorted_for_stable_draw_order() {
    let mut game = engine(304);
    game.step(1200);
    let features = &game.state().world.features;
    assert!(features.windows(2).all(|pair| pair[0].at <= pair[1].at));
}

#[test]
fn the_yield_underfoot_matches_the_band_underfoot() {
    let content = content();
    let mut game = engine(305);
    for _ in 0..50 {
        game.step(60);
        let world = &game.state().world;
        let band = world.band_at(world.distance).expect("standing somewhere");
        assert_eq!(
            world.current_yield_pct(&content),
            content.terrain(band.kind).yield_pct
        );
    }
}

#[test]
fn different_seeds_produce_different_terrain() {
    let mut a = engine(1);
    let mut b = engine(2);
    a.step(1200);
    b.step(1200);
    let kinds = |game: &crate::engine::GameEngine| {
        game.state()
            .world
            .bands
            .iter()
            .map(|band| band.kind.0)
            .collect::<Vec<_>>()
    };
    assert_ne!(kinds(&a), kinds(&b));
}

#[test]
fn distance_is_reported_in_whole_paces_to_the_renderer() {
    let mut game = engine(306);
    game.step(300);
    let view = game.view();
    let expected = game.state().world.distance >> FX_SHIFT;
    assert!(
        (view.world.distance - expected as f32).abs() < 1.0,
        "snapshot distance {} does not match state {expected}",
        view.world.distance
    );
}
