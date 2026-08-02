//! Replays — the one format.
//!
//! A run is its seed, the content pack it ran against, and the list of
//! commands the player issued with the tick each landed on. That is
//! enough to reconstruct the run exactly, which means one format serves
//! four jobs at once: the save file, the regression fixture, the
//! seed-sharing format, and — if there is ever netcode — the wire.
//!
//! Periodic state hashes turn it into a test. Replaying a recording and
//! comparing hashes tick by tick catches a determinism break at the
//! moment it happens rather than at the end, and reports the tick.
//!
//! `assets/replays/golden.json` is embedded here with `include_str!`
//! so the native test suite and the browser verify **the same bytes**.
//! That is the native/wasm parity gate.

use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

use crate::command::GameCommand;
use crate::state::GameState;

/// Ticks between state hashes. One second of simulation.
pub const CHECKPOINT_INTERVAL: u64 = 30;

/// Bumped whenever the format changes shape. Old replays are rejected
/// rather than misread.
pub const REPLAY_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickCommand {
    pub tick: u64,
    pub cmd: GameCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub tick: u64,
    /// Hex, because JSON numbers cannot hold a `u64` losslessly.
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay {
    pub version: u32,
    pub seed: u64,
    /// Hex XXH3 of the content pack this was recorded against.
    pub content_hash: String,
    pub commands: Vec<TickCommand>,
    pub checkpoints: Vec<Checkpoint>,
    pub final_tick: u64,
}

impl Replay {
    #[must_use]
    pub fn new(seed: u64, content_hash: u64) -> Self {
        Self {
            version: REPLAY_VERSION,
            seed,
            content_hash: format!("{content_hash:016x}"),
            commands: Vec::new(),
            checkpoints: Vec::new(),
            final_tick: 0,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("replay must serialise")
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let replay: Replay =
            serde_json::from_str(json).map_err(|err| format!("malformed replay: {err}"))?;
        if replay.version != REPLAY_VERSION {
            return Err(format!(
                "replay version {} but this build reads version {REPLAY_VERSION}",
                replay.version
            ));
        }
        Ok(replay)
    }
}

/// Accumulates a replay as a session is played.
#[derive(Debug, Clone)]
pub struct Recorder {
    replay: Replay,
    enabled: bool,
}

impl Recorder {
    #[must_use]
    pub fn new(seed: u64, content_hash: u64) -> Self {
        Self {
            replay: Replay::new(seed, content_hash),
            enabled: true,
        }
    }

    /// Takes ownership: the engine has finished with the command by the
    /// time it lands here, so there is nothing to clone.
    pub fn record_command(&mut self, tick: u64, cmd: GameCommand) {
        if self.enabled {
            self.replay.commands.push(TickCommand { tick, cmd });
        }
    }

    pub fn record_checkpoint(&mut self, tick: u64, hash: u64) {
        if self.enabled {
            self.replay.checkpoints.push(Checkpoint {
                tick,
                hash: format!("{hash:016x}"),
            });
        }
    }

    pub fn set_final_tick(&mut self, tick: u64) {
        self.replay.final_tick = tick;
    }

    /// Stop recording — used while *replaying* so verification does not
    /// append to the recording it is checking.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub fn snapshot(&self) -> Replay {
        self.replay.clone()
    }
}

/// XXH3 of the serialised state.
///
/// JSON is exact here precisely because there is no floating point in
/// `GameState` — every field is an integer, every map is ordered, and
/// serde emits struct fields in declaration order. If a float ever
/// sneaks into state, this silently becomes platform-dependent, which
/// is one more reason there are none.
#[must_use]
pub fn hash_state(state: &GameState) -> u64 {
    let bytes = serde_json::to_vec(state).expect("GameState must serialise");
    xxh3_64(&bytes)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Divergence {
    pub tick: u64,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayReport {
    pub ok: bool,
    /// Number of checkpoints compared.
    pub checked: usize,
    pub final_tick: u64,
    pub divergence: Option<Divergence>,
    pub message: String,
}

impl ReplayReport {
    pub(crate) fn failure(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            checked: 0,
            final_tick: 0,
            divergence: None,
            message: message.into(),
        }
    }
}

/// The golden fixture, embedded so native and wasm verify identical
/// bytes. Regenerate with
/// `cargo run -p understory-core --example record_golden`.
pub const GOLDEN_REPLAY: &str = include_str!("../../../assets/replays/golden.json");
