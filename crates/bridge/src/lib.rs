//! wasm-bindgen bridge.
//!
//! Deliberately narrow. The frontend calls `frame` once per animation
//! frame and `view` once per frame after it — everything else is either
//! a command going in or a one-off at startup. v1 made a dozen separate
//! accessor calls at 30hz; each crossing costs a serialisation, and the
//! cost is paid whether or not anything changed.
//!
//! JSON strings are the transport for now: debuggable, and irrelevant
//! at cozy scale. The snapshot shape is already grouped the way a
//! worker-plus-shared-buffer bridge would want it, so replacing this
//! transport later does not require redesigning what crosses.

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::fx::paces_from_int;
use understory_core::state::{Enemy, EnemyState};

thread_local! {
    static ENGINE: RefCell<Option<GameEngine>> = const { RefCell::new(None) };
}

fn with_engine<R>(f: impl FnOnce(&GameEngine) -> R) -> R {
    ENGINE.with(|cell| {
        f(cell
            .borrow()
            .as_ref()
            .expect("call init_game before anything else"))
    })
}

fn with_engine_mut<R>(f: impl FnOnce(&mut GameEngine) -> R) -> R {
    ENGINE.with(|cell| {
        f(cell
            .borrow_mut()
            .as_mut()
            .expect("call init_game before anything else"))
    })
}

/// Start a run. Panics surface as readable JS errors from here on.
#[wasm_bindgen]
pub fn init_game(seed: u64) {
    console_error_panic_hook::set_once();
    ENGINE.with(|cell| {
        *cell.borrow_mut() = Some(GameEngine::new(seed));
    });
}

/// Apply a command. Takes the JSON encoding of a `GameCommand` and
/// returns the JSON encoding of a `CommandResult`.
#[wasm_bindgen]
pub fn send_command(json: &str) -> String {
    let cmd: GameCommand = match serde_json::from_str(json) {
        Ok(cmd) => cmd,
        Err(err) => return format!(r#"{{"Error":{{"Malformed":"{err}"}}}}"#),
    };
    let result = with_engine_mut(|engine| engine.send(cmd));
    serde_json::to_string(&result).unwrap_or_else(|err| format!(r#"{{"Error":"{err}"}}"#))
}

/// Advance the simulation by `elapsed_us` microseconds of wall clock,
/// scaled by the current speed. Returns payload-free sound events beside
/// spatial combat events. Call once per animation frame.
#[wasm_bindgen]
pub fn frame(elapsed_us: u32) -> String {
    let events = with_engine_mut(|engine| engine.frame_events(u64::from(elapsed_us)));
    serde_json::to_string(&events).unwrap_or_else(|_| r#"{"sounds":[],"combat":[]}"#.into())
}

/// Everything the renderer needs for one frame, as one JSON document.
#[wasm_bindgen]
pub fn view() -> String {
    with_engine(|engine| serde_json::to_string(&engine.view()).unwrap_or_default())
}

/// Item, room, and terrain definitions. Static for the lifetime of a
/// content pack — fetch once at startup, never poll.
#[wasm_bindgen]
pub fn catalog() -> String {
    with_engine(|engine| serde_json::to_string(&engine.catalog()).unwrap_or_default())
}

/// Hex XXH3 of the current state. Diagnostic, and the thing replay
/// verification compares.
#[wasm_bindgen]
pub fn state_hash() -> String {
    with_engine(|engine| format!("{:016x}", engine.state_hash()))
}

/// Serialised `GameState`.
#[wasm_bindgen]
pub fn save() -> String {
    with_engine(GameEngine::save)
}

#[wasm_bindgen]
pub fn load(json: &str) -> Result<(), JsValue> {
    with_engine_mut(|engine| engine.load(json).map_err(|err| JsValue::from_str(&err)))
}

/// The replay recorded so far: seed, content hash, commands, and
/// periodic state hashes.
#[wasm_bindgen]
pub fn export_replay() -> String {
    with_engine(|engine| engine.export_replay().to_json())
}

/// Replay a recording and report whether every checkpoint matched.
#[wasm_bindgen]
pub fn verify_replay(json: &str) -> String {
    let content = with_engine(|engine| engine.content().clone());
    let report = match understory_core::replay::Replay::from_json(json) {
        Ok(replay) => understory_core::verify_replay(&replay, content),
        Err(message) => {
            return format!(
                r#"{{"ok":false,"message":"{}"}}"#,
                message.replace('"', "'")
            );
        }
    };
    serde_json::to_string(&report).unwrap_or_default()
}

/// Verify the golden fixture embedded in this binary.
///
/// The native test suite verifies the same bytes. If this passes in the
/// browser and there, wasm and native agree on every state hash — which
/// is the determinism guarantee the whole project rests on.
#[wasm_bindgen]
pub fn verify_golden_replay() -> String {
    let report = understory_core::verify_golden_replay();
    serde_json::to_string(&report).unwrap_or_default()
}

/// Put items straight onto the tower's shelves. **Tests only.**
///
/// The browser harness has had `view`, `catalog`, `send` and `step`
/// since M0 and no way to hand the tower anything, so every spec that
/// wanted a room had to *earn* it — which quietly made each of them an
/// economy test wearing a UI test's clothes. Measured at M6: the roster
/// spec, whose subject is two schedule widgets, spent 194,700 ticks
/// building a rope chain and browned out at 2 charge without ever
/// reaching the elevator it exists to schedule.
///
/// The native side has had `GameEngine::state_mut_for_test` for exactly
/// this reason (`DECISIONS.md` §4 explains why it is walled off), and
/// the same rule applies here: nothing that ships to a player may call
/// this. It bypasses the command pattern, so anything it touches is
/// invisible to a replay.
///
/// Returns how many were actually shelved — a shelf holds one kind, so
/// asking for more than there is room for is a real outcome and not an
/// error.
#[wasm_bindgen]
pub fn debug_grant(item: &str, amount: i64) -> i64 {
    with_engine_mut(|engine| {
        let Some(idx) = engine.content().item_idx(item) else {
            return 0;
        };
        let state = engine.state_mut_for_test();
        let mut left = amount;
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                left -= room.shelve(idx, left);
                if left <= 0 {
                    return amount;
                }
            }
        }
        amount - left
    })
}

/// Wreck the room covering `slot` on `floor`. **Tests only.**
///
/// Visual harnesses need to prove the destroyed presentation without
/// waiting for a seeded wave to choose one particular room. Like
/// `debug_grant`, this bypasses commands and replays and must never be
/// called by shipping UI. Returns whether a room was found.
#[wasm_bindgen]
pub fn debug_wreck_room(floor: u8, slot: u8) -> bool {
    with_engine_mut(|engine| {
        let state = engine.state_mut_for_test();
        let Some(room) = state
            .tower
            .floor_mut(floor)
            .and_then(|level| level.rooms.iter_mut().find(|room| room.covers(slot)))
        else {
            return false;
        };
        room.health.hp = 0;
        true
    })
}

/// Stage one of every authored creature ahead of the tower. **Tests only.**
///
/// Art checkpoints need the real renderer's approach placement, depth,
/// tower overlap and day/night treatment. Waiting for seeded waves cannot
/// guarantee all eight definitions, especially the berth-only warden, so
/// this hook constructs a gallery in simulation state without touching the
/// command/replay surface used by the shipped game.
#[wasm_bindgen]
pub fn debug_stage_enemies() -> usize {
    with_engine_mut(|engine| {
        let defs: Vec<_> = engine
            .content()
            .enemies
            .iter()
            .enumerate()
            .map(|(index, def)| {
                (
                    understory_core::ids::EnemyIdx(index as u16),
                    def.hp,
                    def.cling_ticks,
                )
            })
            .collect();
        let state = engine.state_mut_for_test();
        state.siege.enemies.clear();
        let tower_at = state.world.distance;
        for (index, (def, hp, cling_left)) in defs.into_iter().enumerate() {
            let id = state.alloc_enemy_id();
            // Stagger the real world positions so logarithmic approach
            // compression and size-at-depth are both exercised.
            let ahead = 18 + index as i64 * 13;
            state.siege.enemies.push(Enemy {
                id,
                def,
                at: tower_at + paces_from_int(ahead),
                hp,
                state: EnemyState::Approaching,
                attack_cooldown: 0,
                cling_left,
                fade_left: 0,
            });
        }
        state.siege.enemies.len()
    })
}

/// Run exactly `ticks` ticks regardless of the speed setting. For tests
/// that need to reach a state quickly without waiting in real time.
#[wasm_bindgen]
pub fn debug_step(ticks: u32) -> String {
    let events = with_engine_mut(|engine| engine.step_events(ticks));
    serde_json::to_string(&events).unwrap_or_else(|_| r#"{"sounds":[],"combat":[]}"#.into())
}
