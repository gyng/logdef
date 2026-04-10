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

/// Run one simulation tick. Returns sound events produced.
pub fn tick(state: &mut GameState, dt: Scalar) -> Vec<SoundEvent> {
    let mut sounds = Vec::new();

    if state.encounter.is_some() {
        production::run(state, dt, &mut sounds);
        transport::run(state, dt, &mut sounds);
        companion_ai::run(state, dt, &mut sounds);
        projectiles::run(state, dt, &mut sounds);
        combat::run(state, dt, &mut sounds);
        economy::run(state, dt, &mut sounds);
    }

    state.tick += 1;
    state.elapsed += dt;

    sounds
}
