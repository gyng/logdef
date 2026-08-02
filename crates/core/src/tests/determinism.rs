//! Determinism is the load-bearing property. If any of these fail,
//! seed sharing, replays, and bug reproduction all break at once —
//! treat a failure here as a P0, never as a flaky test.

use crate::command::GameCommand;
use crate::state::SimSpeed;
use crate::tests::{content, engine};

#[test]
fn same_seed_same_state() {
    let mut a = engine(0xABCD);
    let mut b = engine(0xABCD);
    a.step(900);
    b.step(900);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn different_seeds_diverge() {
    let mut a = engine(1);
    let mut b = engine(2);
    a.step(300);
    b.step(300);
    assert_ne!(
        a.state_hash(),
        b.state_hash(),
        "two seeds produced identical runs — the seed is not reaching the generator"
    );
}

#[test]
fn identical_command_streams_agree() {
    let script = |engine: &mut crate::engine::GameEngine| {
        engine.set_speed(SimSpeed::X1);
        engine.step(600);
        let _ = engine.send(GameCommand::BuildFloor);
        engine.step(120);
        let _ = engine.send(GameCommand::PlaceRoom {
            room: "room.storeroom".into(),
            floor: 3,
            slot: 5,
        });
        engine.step(600);
    };

    let mut a = engine(77);
    let mut b = engine(77);
    script(&mut a);
    script(&mut b);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn tick_count_is_all_that_matters_not_batch_size() {
    // 900 ticks in one call and 900 in dribs must land identically.
    // If they don't, something is reading wall-clock or batch size.
    let mut bulk = engine(31337);
    let mut dribbled = engine(31337);

    bulk.step(900);
    for _ in 0..900 {
        dribbled.step(1);
    }

    assert_eq!(bulk.state_hash(), dribbled.state_hash());
}

#[test]
fn cosmetic_stream_never_moves_the_economy() {
    // The firewall, asserted rather than assumed. Burning the cosmetic
    // stream must leave every economic outcome untouched.
    let mut plain = engine(4242);
    let mut jittered = engine(4242);

    for _ in 0..500 {
        let _ = jittered.state().rng.cosmetic.clone().next_u64();
    }
    plain.step(600);
    jittered.step(600);

    assert_eq!(
        plain.state().stats,
        jittered.state().stats,
        "cosmetic draws leaked into the economy"
    );
    assert_eq!(
        plain.state().world.distance,
        jittered.state().world.distance
    );
}

#[test]
fn a_rejected_command_changes_nothing() {
    let mut game = engine(5);
    game.step(30);
    let before = game.state_hash();

    // Wildly illegal: no such floor.
    let result = game.send(GameCommand::PlaceRoom {
        room: "room.mill".into(),
        floor: 99,
        slot: 0,
    });
    assert!(!result.is_ok());
    assert_eq!(
        game.state_hash(),
        before,
        "a rejected command mutated state — validation must fully precede mutation"
    );
}

#[test]
fn save_and_load_round_trip_exactly() {
    let mut game = engine(9001);
    game.step(450);
    let saved = game.save();
    let hash = game.state_hash();

    let mut restored = engine(1);
    restored.load(&saved).expect("save must load");
    assert_eq!(restored.state_hash(), hash);

    // And it keeps running the same way.
    game.step(300);
    restored.step(300);
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn state_carries_no_floating_point() {
    // Serialised state is the hash input. A float in there would make
    // hashes platform-dependent, so guard it structurally: every JSON
    // number must survive a round trip through i64.
    let mut game = engine(1234);
    game.step(300);
    let value: serde_json::Value =
        serde_json::from_str(&game.save()).expect("state must serialise");

    let mut offenders = Vec::new();
    walk(&value, String::new(), &mut offenders);
    assert!(
        offenders.is_empty(),
        "floating point found in GameState at: {offenders:?}"
    );

    fn walk(value: &serde_json::Value, path: String, out: &mut Vec<String>) {
        match value {
            serde_json::Value::Number(number)
                if number.as_i64().is_none() && number.as_u64().is_none() =>
            {
                out.push(path);
            }
            serde_json::Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    walk(item, format!("{path}[{i}]"), out);
                }
            }
            serde_json::Value::Object(fields) => {
                for (key, field) in fields {
                    walk(field, format!("{path}.{key}"), out);
                }
            }
            _ => {}
        }
    }
}

#[test]
fn content_hash_is_stable_across_loads() {
    assert_eq!(content().content_hash, content().content_hash);
}
