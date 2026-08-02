use std::time::Duration;

// Perf timers use std::time::Instant, which panics on wasm32-unknown-unknown.
// Gate on non-wasm + debug_assertions.
#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
use std::time::Instant;

pub mod combat;
pub mod companion_ai;
pub mod economy;

/// Vertical pixel height of one tower floor. Shared between the
/// transport AI (runner travel time) and engine (hero spawn y for
/// projectiles) so the visible floor height matches the simulated one.
pub const FLOOR_HEIGHT_PX: crate::types::Scalar = 80.0;
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

use crate::registry::Registry;
use crate::snapshot::SoundEvent;
use crate::state::GameState;
use crate::types::Scalar;

#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
#[derive(Debug, Clone, Copy)]
struct Timer(Instant);

#[cfg(any(not(debug_assertions), target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, Default)]
struct Timer;

#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
fn start_timer() -> Timer {
    Timer(Instant::now())
}

#[cfg(any(not(debug_assertions), target_arch = "wasm32"))]
fn start_timer() -> Timer {
    Timer
}

#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
fn finish_timer(timer: Timer) -> Duration {
    timer.0.elapsed()
}

#[cfg(any(not(debug_assertions), target_arch = "wasm32"))]
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
pub fn tick(state: &mut GameState, registry: &Registry, dt: Scalar) -> TickResult {
    let tick_start = start_timer();
    let mut sounds = Vec::new();
    let mut profile = TickProfile::default();

    let in_encounter = state.encounter.is_some();
    let in_drill = state.drill.is_some();

    if in_encounter || in_drill {
        let start = start_timer();
        production::run(state, registry, dt, &mut sounds);
        profile.production = finish_timer(start);

        let start = start_timer();
        transport::run(state, registry, dt, &mut sounds);
        profile.transport = finish_timer(start);
    }

    if in_encounter {
        let start = start_timer();
        companion_ai::run(state, registry, dt, &mut sounds);
        profile.companion_ai = finish_timer(start);

        let start = start_timer();
        projectiles::run(state, registry, dt, &mut sounds);
        profile.projectiles = finish_timer(start);

        let start = start_timer();
        combat::run(state, registry, dt, &mut sounds);
        profile.combat = finish_timer(start);

        let start = start_timer();
        economy::run(state, registry, dt, &mut sounds);
        profile.economy = finish_timer(start);
    }

    if in_drill {
        advance_drill(state, dt, &mut sounds);
    }

    state.tick += 1;
    state.elapsed += dt;
    profile.total = finish_timer(tick_start);

    TickResult { sounds, profile }
}

/// Drive synthetic demand for the drill: every full second drain one
/// crate from the hero balcony rack to model continuous combat use.
/// When the timer hits zero, end the drill and pay XP based on how
/// many crates the chain delivered.
fn advance_drill(state: &mut GameState, dt: Scalar, _sounds: &mut [SoundEvent]) {
    let Some(drill) = state.drill.as_mut() else {
        return;
    };
    drill.seconds_remaining -= dt;
    drill.demand_accumulator += dt;
    while drill.demand_accumulator >= 1.0 {
        drill.demand_accumulator -= 1.0;
        let hero_balcony = state.tower.hero.position;
        if let Some(b) = state
            .tower
            .balconies
            .iter_mut()
            .find(|b| b.id == hero_balcony)
            && b.rack.current > 0
        {
            b.rack.current -= 1;
        }
    }

    if drill.seconds_remaining <= 0.0 {
        let deliveries = state
            .deliveries_completed
            .saturating_sub(drill.deliveries_at_start);
        // Award 5 xp per crate delivered during the drill — small but
        // makes optimisation pay off across multiple drills.
        let xp = deliveries * 5;
        state.tower.hero.xp += xp;
        state.drill = None;
    }
}
