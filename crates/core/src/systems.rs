//! Simulation systems, run in a fixed order every tick.
//!
//! The order is load-bearing for determinism and for feel. Reordering
//! it invalidates every golden replay, so don't — extend at the ends,
//! or split a system in place.
//!
//! 1. **stride** — the tower walks; terrain streams in ahead and is
//!    pruned behind.
//! 2. **intake** — intake rooms harvest the band underfoot.
//! 3. **production** — crafting rooms advance, consume, and emit.
//! 4. **haul** — crew advance their legs, then idle crew claim work.
//!
//! Haul runs last so the crew react to the buffers this tick actually
//! produced rather than last tick's.

pub mod haul;
pub mod intake;
pub mod production;
pub mod stride;

use crate::content::Content;
use crate::state::GameState;

/// Things that happened this tick, for the audio layer to voice. Sound
/// is fire-and-forget: emitted here, played or dropped by JS, never
/// read back into the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SoundEvent {
    /// An intake room pulled something out of the terrain.
    Harvest,
    /// A crafting room finished a craft.
    Craft,
    /// A crew member picked up a load.
    Pickup,
    /// A crew member set a load down.
    Deliver,
    /// The tower crossed into a new terrain band.
    BandChange,
}

/// Run exactly one simulation tick.
pub fn tick(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    stride::run(state, content, sounds);
    intake::run(state, content, sounds);
    production::run(state, content, sounds);
    haul::run(state, content, sounds);
    state.tick += 1;
}
