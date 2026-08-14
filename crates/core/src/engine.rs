//! `GameEngine` — owns the state, applies commands, runs ticks.
//!
//! Deliberately thin. Command validation lives in `engine/commands.rs`
//! and snapshot construction in `snapshot.rs`, so this file stays a
//! readable description of the loop rather than the 1.9k-line drawer
//! that the same file became in v1.

mod commands;

/// How near the tower has to be for a waypoint to be takeable.
/// Re-exported so `snapshot` reads the same window the command
/// enforces — a prompt that offers what the rule refuses is worse than
/// no prompt.
pub(crate) use commands::WAYPOINT_REACH_PACES;

use std::sync::Arc;

use crate::command::{CommandError, CommandResult, GameCommand};
use crate::content::Content;
use crate::replay::{CHECKPOINT_INTERVAL, Divergence, Recorder, Replay, ReplayReport, hash_state};
use crate::snapshot::{CatalogSnapshot, FrameEvents, ViewSnapshot, build_catalog, build_view};
use crate::state::{GameState, SimSpeed};
use crate::systems::{self, CombatEvent, SoundEvent};

/// Microseconds per simulation tick, at 30 Hz.
///
/// 33_333 rather than 33_333.33: the drift is 1 part in 100_000, which
/// nobody will ever perceive, and it keeps the wall-clock accumulator
/// in integers alongside everything else.
pub const TICK_US: u64 = 33_333;

/// Ceiling on ticks per frame, so a backgrounded tab doesn't come back
/// and try to simulate ten minutes in one go.
pub const MAX_TICKS_PER_FRAME: u32 = 12;

pub struct GameEngine {
    content: Arc<Content>,
    state: GameState,
    /// Wall-clock remainder, in microseconds. Engine-local: never part
    /// of state, never affects the content of a tick.
    accumulator_us: u64,
    recorder: Recorder,
}

impl GameEngine {
    /// New run against the embedded content pack.
    ///
    /// # Panics
    /// If the embedded pack fails to load or validate. That is a build
    /// error, not a runtime condition — shipping a broken pack should
    /// fail loudly and immediately.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let content = Content::load_embedded().unwrap_or_else(|errors| {
            let rendered = errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n  ");
            panic!("embedded content pack is invalid:\n  {rendered}");
        });
        Self::with_content(seed, Arc::new(content))
    }

    #[must_use]
    pub fn with_content(seed: u64, content: Arc<Content>) -> Self {
        let state = GameState::new(seed, &content);
        let mut recorder = Recorder::new(seed, content.content_hash);
        recorder.record_checkpoint(0, hash_state(&state));
        Self {
            content,
            state,
            accumulator_us: 0,
            recorder,
        }
    }

    #[must_use]
    pub fn content(&self) -> &Arc<Content> {
        &self.content
    }

    #[must_use]
    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// Direct state access for tests and measurement harnesses that
    /// need to construct a situation no command can produce — an empty
    /// tower, a single terrain band, a provocation level held steady
    /// while the siege is measured against it.
    ///
    /// Deliberately ugly to type and deliberately not `state_mut`.
    /// Nothing that ships to a player may touch this: the command
    /// pattern is what makes the game replayable and debuggable
    /// (`DECISIONS.md` §4), and a second way to mutate state would be a
    /// second way for a replay to diverge from the run it recorded. It
    /// is `pub` only because `examples/` are separate crates and the
    /// instruments live there.
    pub fn state_mut_for_test(&mut self) -> &mut GameState {
        &mut self.state
    }

    /// Apply a command. Accepted commands are recorded into the replay
    /// stamped with the current tick; rejected ones are not, because a
    /// rejected command changed nothing.
    pub fn send(&mut self, cmd: GameCommand) -> CommandResult {
        match commands::apply(&mut self.state, &self.content, &cmd) {
            Ok(()) => {
                self.recorder.record_command(self.state.tick, cmd);
                CommandResult::Ok
            }
            Err(error) => CommandResult::Error(error),
        }
    }

    /// Advance by wall-clock time, honouring the current speed setting.
    /// Returns the sound events produced by every tick that ran.
    pub fn frame(&mut self, elapsed_us: u64) -> Vec<SoundEvent> {
        self.frame_events(elapsed_us).sounds
    }

    /// Advance by wall-clock time and return both payload-free audio cues
    /// and spatial presentation events. Neither is stored in `GameState`.
    pub fn frame_events(&mut self, elapsed_us: u64) -> FrameEvents {
        let multiplier = u64::from(self.state.speed.multiplier());
        if multiplier == 0 {
            // Paused: don't bank real time, or unpausing lurches.
            self.accumulator_us = 0;
            return FrameEvents::from_sim(&self.content, Vec::new(), Vec::new());
        }

        let budget = TICK_US * u64::from(MAX_TICKS_PER_FRAME);
        self.accumulator_us = (self.accumulator_us + elapsed_us * multiplier).min(budget);

        let mut ticks = 0;
        while self.accumulator_us >= TICK_US && ticks < MAX_TICKS_PER_FRAME {
            self.accumulator_us -= TICK_US;
            ticks += 1;
        }
        self.step_events(ticks)
    }

    /// Run exactly `ticks` ticks, ignoring the speed setting. Used by
    /// replay, tests, and anything that wants determinism without a
    /// clock in the way.
    pub fn step(&mut self, ticks: u32) -> Vec<SoundEvent> {
        self.step_events(ticks).sounds
    }

    /// Deterministic stepping with transient presentation output.
    pub fn step_events(&mut self, ticks: u32) -> FrameEvents {
        let mut sounds = Vec::new();
        let mut combat: Vec<CombatEvent> = Vec::new();
        for _ in 0..ticks {
            systems::tick(&mut self.state, &self.content, &mut sounds, &mut combat);
            if self.state.tick.is_multiple_of(CHECKPOINT_INTERVAL) {
                let hash = hash_state(&self.state);
                self.recorder.record_checkpoint(self.state.tick, hash);
            }
        }
        if ticks > 0 {
            self.recorder.set_final_tick(self.state.tick);
        }
        FrameEvents::from_sim(&self.content, sounds, combat)
    }

    /// Fraction of a tick elapsed, for render interpolation.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn alpha(&self) -> f32 {
        self.accumulator_us as f32 / TICK_US as f32
    }

    #[must_use]
    pub fn view(&self) -> ViewSnapshot {
        build_view(&self.state, &self.content, self.alpha())
    }

    #[must_use]
    pub fn catalog(&self) -> CatalogSnapshot {
        build_catalog(&self.content)
    }

    #[must_use]
    pub fn state_hash(&self) -> u64 {
        hash_state(&self.state)
    }

    #[must_use]
    pub fn export_replay(&self) -> Replay {
        let mut replay = self.recorder.snapshot();
        replay.final_tick = self.state.tick;
        replay
    }

    /// Serialise the live state. A save is the state plus, separately,
    /// the replay that produced it.
    pub fn save(&self) -> String {
        serde_json::to_string(&self.state).expect("GameState must serialise")
    }

    pub fn load(&mut self, json: &str) -> Result<(), String> {
        let state: GameState =
            serde_json::from_str(json).map_err(|err| format!("failed to load save: {err}"))?;
        self.state = state;
        self.accumulator_us = 0;
        Ok(())
    }

    /// Convenience for tests and tools: apply a command and assert it
    /// was accepted.
    ///
    /// # Errors
    /// Returns the rejection if the command was not legal.
    pub fn try_send(&mut self, cmd: GameCommand) -> Result<(), CommandError> {
        match self.send(cmd) {
            CommandResult::Ok => Ok(()),
            CommandResult::Error(error) => Err(error),
        }
    }

    /// Set the speed without going through JSON. Used by tests.
    pub fn set_speed(&mut self, speed: SimSpeed) {
        let _ = self.send(GameCommand::SetSpeed { speed });
    }
}

impl std::fmt::Debug for GameEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameEngine")
            .field("tick", &self.state.tick)
            .field("seed", &self.state.seed)
            .field("speed", &self.state.speed)
            .finish_non_exhaustive()
    }
}

/// Replay a recording into a fresh engine and compare every checkpoint.
///
/// Reports the **first** divergence with its tick, because the tick a
/// determinism break appears on is nearly always the tick that caused
/// it. A content-hash mismatch fails immediately rather than producing
/// a confusing wall of divergences.
#[must_use]
pub fn verify_replay(replay: &Replay, content: Arc<Content>) -> ReplayReport {
    let expected_hash = format!("{:016x}", content.content_hash);
    if replay.content_hash != expected_hash {
        return ReplayReport::failure(format!(
            "content pack mismatch: replay recorded against {}, this build has {expected_hash}",
            replay.content_hash
        ));
    }

    let mut engine = GameEngine::with_content(replay.seed, content);
    engine.recorder.disable();

    let mut checkpoints = replay.checkpoints.iter().peekable();
    let mut checked = 0usize;

    // Checkpoint 0 is the freshly seeded state, before any tick.
    if let Some(cp) = checkpoints.peek()
        && cp.tick == 0
    {
        let actual = format!("{:016x}", engine.state_hash());
        if actual != cp.hash {
            return ReplayReport {
                ok: false,
                checked,
                final_tick: 0,
                divergence: Some(Divergence {
                    tick: 0,
                    expected: cp.hash.clone(),
                    actual,
                }),
                message: "initial state differs — seed or content pack changed".into(),
            };
        }
        checked += 1;
        checkpoints.next();
    }

    let mut commands = replay.commands.iter().peekable();

    for tick in 0..replay.final_tick {
        // Commands stamped tick T apply before tick T runs.
        while let Some(entry) = commands.peek() {
            if entry.tick != tick {
                break;
            }
            let _ = commands::apply(&mut engine.state, &engine.content, &entry.cmd);
            commands.next();
        }

        engine.step(1);

        if let Some(cp) = checkpoints.peek()
            && cp.tick == engine.state.tick
        {
            let actual = format!("{:016x}", engine.state_hash());
            if actual != cp.hash {
                return ReplayReport {
                    ok: false,
                    checked,
                    final_tick: engine.state.tick,
                    divergence: Some(Divergence {
                        tick: engine.state.tick,
                        expected: cp.hash.clone(),
                        actual,
                    }),
                    message: format!("state diverged at tick {}", engine.state.tick),
                };
            }
            checked += 1;
            checkpoints.next();
        }
    }

    let leftover = checkpoints.count();
    if leftover > 0 {
        return ReplayReport {
            ok: false,
            checked,
            final_tick: engine.state.tick,
            divergence: None,
            message: format!("{leftover} checkpoint(s) past the recorded final tick"),
        };
    }

    ReplayReport {
        ok: true,
        checked,
        final_tick: engine.state.tick,
        divergence: None,
        message: format!(
            "{checked} checkpoint(s) matched across {} tick(s)",
            engine.state.tick
        ),
    }
}

/// Verify the embedded golden fixture against this build. The same
/// bytes run natively in `cargo test` and in the browser through the
/// wasm bridge — that pairing is the hash-parity gate.
#[must_use]
pub fn verify_golden_replay() -> ReplayReport {
    let content = match Content::load_embedded() {
        Ok(content) => Arc::new(content),
        Err(errors) => {
            return ReplayReport::failure(format!(
                "content pack failed to load: {}",
                errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
    };
    match Replay::from_json(crate::replay::GOLDEN_REPLAY) {
        Ok(replay) => verify_replay(&replay, content),
        Err(message) => ReplayReport::failure(message),
    }
}
