//! The tower walks.
//!
//! Distance accumulates in Q8.8 paces at a rate the player will control
//! from M3 onward. Terrain is generated ahead and pruned behind, so the
//! world is unbounded but memory is not.

use crate::content::Content;
use crate::fx::{Fx, paces_from_fx};
use crate::state::GameState;

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let before = state
        .world
        .band_at(state.world.distance)
        .map(|band| band.kind);

    let per_tick = stride_per_tick(content);
    state.world.distance += paces_from_fx(per_tick);

    state.world.generate_ahead(&mut state.rng.world, content);
    state.world.prune_behind(content);

    let after = state
        .world
        .band_at(state.world.distance)
        .map(|band| band.kind);
    if before != after {
        sounds.push(SoundEvent::BandChange);
    }
}

/// Paces per tick, derived from the designer-facing "paces per 100
/// ticks" so the balance file never contains a fixed-point literal.
#[must_use]
pub fn stride_per_tick(content: &Content) -> Fx {
    Fx::ratio(content.balance.world.stride_paces_per_100_ticks as i32, 100)
}
