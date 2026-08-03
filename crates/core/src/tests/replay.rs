//! Replay: record, verify, and detect a break.
//!
//! The golden fixture test here and the browser check in the Playwright
//! smoke test run the **same embedded bytes**. Together they are the
//! native/wasm hash-parity gate.

use crate::command::GameCommand;
use crate::engine::{verify_golden_replay, verify_replay};
use crate::replay::{CHECKPOINT_INTERVAL, Replay};
use crate::state::SimSpeed;
use crate::tests::{content, engine};

#[test]
fn a_recorded_session_replays_exactly() {
    let mut game = engine(555);
    game.set_speed(SimSpeed::X1);
    game.step(300);
    let _ = game.send(GameCommand::BuildFloor);
    game.step(300);
    let _ = game.send(GameCommand::PlaceRoom {
        room: "room.storeroom".into(),
        floor: 3,
        slot: 5,
    });
    game.step(600);

    let replay = game.export_replay();
    let report = verify_replay(&replay, content());

    assert!(report.ok, "{}", report.message);
    assert!(report.checked > 1, "no checkpoints were compared");
    assert_eq!(report.final_tick, game.state().tick);
}

#[test]
fn replaying_reproduces_the_final_hash() {
    let mut game = engine(556);
    game.step(900);
    let expected = game.state_hash();
    let replay = game.export_replay();

    let mut rerun = engine(556);
    let mut commands = replay.commands.iter().peekable();
    for tick in 0..replay.final_tick {
        while commands.peek().is_some_and(|entry| entry.tick == tick) {
            let entry = commands.next().expect("peeked");
            let _ = rerun.send(entry.cmd.clone());
        }
        rerun.step(1);
    }
    assert_eq!(rerun.state_hash(), expected);
}

#[test]
fn a_tampered_checkpoint_is_caught_at_its_tick() {
    let mut game = engine(557);
    game.step(300);
    let mut replay = game.export_replay();

    let target = replay
        .checkpoints
        .iter_mut()
        .find(|cp| cp.tick == CHECKPOINT_INTERVAL * 3)
        .expect("a checkpoint three seconds in");
    let tampered_tick = target.tick;
    target.hash = "deadbeefdeadbeef".into();

    let report = verify_replay(&replay, content());
    assert!(!report.ok);
    let divergence = report.divergence.expect("should name the tick");
    assert_eq!(divergence.tick, tampered_tick);
}

#[test]
fn a_dropped_command_shows_up_as_divergence() {
    // Removing a command from a recording must break the hashes —
    // which is exactly what proves the hashes are watching state that
    // commands actually move.
    let mut game = engine(558);
    game.step(600);
    game.try_send(GameCommand::BuildFloor)
        .expect("affordable after 20 seconds");
    game.step(300);

    let mut replay = game.export_replay();
    let before = replay.commands.len();
    replay
        .commands
        .retain(|entry| !matches!(entry.cmd, GameCommand::BuildFloor));
    assert!(replay.commands.len() < before, "nothing was dropped");

    let report = verify_replay(&replay, content());
    assert!(!report.ok, "dropping a build went unnoticed");
}

#[test]
fn a_foreign_content_pack_is_refused_outright() {
    let mut game = engine(559);
    game.step(60);
    let mut replay = game.export_replay();
    replay.content_hash = "0123456789abcdef".into();

    let report = verify_replay(&replay, content());
    assert!(!report.ok);
    assert!(
        report.message.contains("content pack mismatch"),
        "{}",
        report.message
    );
    assert!(
        report.divergence.is_none(),
        "a pack mismatch should fail before replaying, not produce divergences"
    );
}

#[test]
fn replays_round_trip_through_json() {
    let mut game = engine(560);
    game.set_speed(SimSpeed::X2);
    game.step(120);
    let replay = game.export_replay();

    let json = replay.to_json();
    let parsed = Replay::from_json(&json).expect("round trip");
    assert_eq!(parsed, replay);
}

#[test]
fn a_future_format_version_is_rejected_rather_than_misread() {
    let mut game = engine(561);
    game.step(30);
    let mut replay = game.export_replay();
    replay.version = 999;
    let error = Replay::from_json(&replay.to_json()).expect_err("should refuse");
    assert!(error.contains("version"), "{error}");
}

#[test]
fn rejected_commands_are_not_recorded() {
    let mut game = engine(562);
    let before = game.export_replay().commands.len();
    let result = game.send(GameCommand::PlaceRoom {
        room: "room.nope".into(),
        floor: 0,
        slot: 0,
    });
    assert!(!result.is_ok());
    assert_eq!(
        game.export_replay().commands.len(),
        before,
        "a rejected command was written into the replay"
    );
}

#[test]
fn the_golden_fixture_verifies() {
    // The parity gate. The Playwright smoke test runs this same call
    // through wasm; if the two ever disagree, the platforms have
    // diverged and every recorded run is suspect.
    let report = verify_golden_replay();
    assert!(
        report.ok,
        "golden replay failed: {}\n\
         If this is an intended simulation change, regenerate with:\n  \
         cargo run -p understory-core --example record_golden",
        report.message
    );
    assert!(report.checked > 10, "the fixture is too short to be useful");
    assert!(report.final_tick >= 1800, "the fixture is too short");
}

#[test]
fn every_command_survives_the_replay_format() {
    // A replay is a seed and a command stream, so a command that does
    // not survive JSON is a run that cannot be shared, and it fails
    // silently — the replay loads, it just does something else.
    //
    // The golden fixture cannot cover every command: some of them are
    // an hour of walking apart, and the two at the enclave would have
    // tripled the fixture to exercise arithmetic the unit tests already
    // pin (see `record_golden.rs`). This covers the format instead, and
    // the match below is exhaustive on purpose — adding a command
    // without adding it here will not compile.
    use crate::command::GameCommand as C;
    use crate::state::{ShaftPriority, SimSpeed};

    let every = vec![
        C::SetSpeed {
            speed: SimSpeed::X4,
        },
        C::BuildFloor,
        C::PlaceRoom {
            room: "room.mill".into(),
            floor: 1,
            slot: 2,
        },
        C::RemoveRoom { floor: 1, slot: 2 },
        C::SetRoomActive {
            floor: 1,
            slot: 2,
            active: false,
        },
        C::BuildShaft {
            shaft: "shaft.elevator".into(),
            low: 0,
            high: 3,
            slot: 7,
        },
        C::RemoveShaft {
            id: crate::ids::ShaftId(1),
        },
        C::SetShaftProgram {
            id: crate::ids::ShaftId(1),
            daypart: 0,
            served: vec![true, false, true],
            priority: ShaftPriority::FreightFirst,
        },
        C::SetStriding { walking: false },
        C::TakeFork { branch: 1 },
        C::Trade { offer: 2 },
        C::Recruit,
        C::Reinforce,
        C::SetShift {
            crew: crate::ids::CrewId(1),
            shift: crate::content::Shift::Night,
        },
    ];

    // Exhaustiveness: if a variant is added and not listed above, this
    // match stops compiling and whoever added it has to decide.
    for command in &every {
        match command {
            C::SetSpeed { .. }
            | C::BuildFloor
            | C::PlaceRoom { .. }
            | C::RemoveRoom { .. }
            | C::SetRoomActive { .. }
            | C::BuildShaft { .. }
            | C::RemoveShaft { .. }
            | C::SetShaftProgram { .. }
            | C::SetStriding { .. }
            | C::TakeFork { .. }
            | C::Trade { .. }
            | C::Recruit
            | C::Reinforce
            | C::SetShift { .. } => {}
        }
    }

    for command in every {
        let json = serde_json::to_string(&command).expect("a command serialises");
        let back: C = serde_json::from_str(&json).expect("and comes back");
        assert_eq!(back, command, "{json} did not survive the round trip");
    }
}
