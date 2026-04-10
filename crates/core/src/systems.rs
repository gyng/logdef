use std::time::Duration;

#[cfg(debug_assertions)]
use std::time::Instant;

pub mod combat;
pub mod companion_ai;
pub mod economy;
/// Simulation systems, executed in fixed order each tick.
/// Order matters for determinism — do not reorder.
///
/// 1. production  — buildings produce crates
/// 2. transport   — runners move crates, lifts/chutes operate
/// 3. companion_ai — companions pick targets, fire
/// 4. projectiles — move projectiles, sweep collision
/// 5. combat      — enemy AI, damage, deaths
/// 6. economy     — gold collection, operating costs
pub mod production;
pub mod projectiles;
pub mod transport;

use crate::snapshot::SoundEvent;
use crate::state::GameState;
use crate::types::Scalar;

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy)]
struct Timer(Instant);

#[cfg(not(debug_assertions))]
#[derive(Debug, Clone, Copy, Default)]
struct Timer;

#[cfg(debug_assertions)]
fn start_timer() -> Timer {
    Timer(Instant::now())
}

#[cfg(not(debug_assertions))]
fn start_timer() -> Timer {
    Timer
}

#[cfg(debug_assertions)]
fn finish_timer(timer: Timer) -> Duration {
    timer.0.elapsed()
}

#[cfg(not(debug_assertions))]
fn finish_timer(_timer: Timer) -> Duration {
    Duration::ZERO
}

#[derive(Debug, Clone, Default)]
pub struct TickProfile {
    pub total: Duration,
    pub production: Duration,
    pub transport: Duration,
    pub companion_ai: Duration,
    pub projectiles: Duration,
    pub combat: Duration,
    pub economy: Duration,
}

#[derive(Debug, Clone)]
pub struct TickResult {
    pub sounds: Vec<SoundEvent>,
    pub profile: TickProfile,
}

/// Run one simulation tick. Returns sound events produced.
pub fn tick(state: &mut GameState, dt: Scalar) -> TickResult {
    let tick_start = start_timer();
    let mut sounds = Vec::new();
    let mut profile = TickProfile::default();

    if state.encounter.is_some() {
        let start = start_timer();
        production::run(state, dt, &mut sounds);
        profile.production = finish_timer(start);

        let start = start_timer();
        transport::run(state, dt, &mut sounds);
        profile.transport = finish_timer(start);

        let start = start_timer();
        companion_ai::run(state, dt, &mut sounds);
        profile.companion_ai = finish_timer(start);

        let start = start_timer();
        projectiles::run(state, dt, &mut sounds);
        profile.projectiles = finish_timer(start);

        let start = start_timer();
        combat::run(state, dt, &mut sounds);
        profile.combat = finish_timer(start);

        let start = start_timer();
        economy::run(state, dt, &mut sounds);
        profile.economy = finish_timer(start);
    }

    state.tick += 1;
    state.elapsed += dt;
    profile.total = finish_timer(tick_start);

    TickResult { sounds, profile }
}
