//! The tower walks.
//!
//! Distance accumulates in Q8.8 paces, and terrain is generated ahead
//! and pruned behind so the world is unbounded at constant memory.
//!
//! Runs last in the tick, which is also its place in the charge
//! priority order: walking is the first thing a tower short of power
//! gives up. A tower that has stopped is still a working tower — it is
//! just not getting anywhere, which is the bank-or-burn tension made
//! literal.

use crate::content::Content;
use crate::fx::{Fx, paces_from_fx};
use crate::state::GameState;

use super::{SoundEvent, power};

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let before = state
        .world
        .band_at(state.world.distance)
        .map(|band| band.kind);

    state.strode = power::pay_for_stride(state, content);
    if state.strode {
        state.world.distance += paces_from_fx(stride_per_tick(content));
    }

    // Terrain keeps streaming whether or not the legs are running: the
    // horizon has to already exist when the tower starts moving again.
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
