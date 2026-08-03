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
use crate::fx::{Fx, paces_from_fx, paces_from_int};
use crate::state::GameState;

use super::{SoundEvent, power};

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let before = state
        .world
        .band_at(state.world.distance)
        .map(|band| band.kind);

    let from = state.world.distance;
    state.strode = power::pay_for_stride(state, content);
    if state.strode {
        // **Slowed by whatever is holding on.** A mire-hulk takes a leg
        // and the tower walks at a fraction of its pace while it does —
        // which means it also sheds the hulk later, because `cling_ticks`
        // runs down against a tower that is *moving*. The answer to every
        // other wave since M2 has been "keep walking"; this is the one
        // creature that makes that answer worse, and the whole of the
        // mechanic is this multiplication (`SYSTEMS.md` §5.5).
        //
        // Scales the step rather than the charge: a dragged tower pays
        // the same to walk and gets less for it, which is the right way
        // round. Being slowed should cost you the ground, not the power.
        let drag = super::siege::drag_pct(state, content);
        let step = paces_from_fx(stride_per_tick(content)) * drag / 100;
        // Never step over a block. Landing exactly on it is what makes
        // `is_blocked` true next tick, which is how the halt begins.
        state.world.distance = match state.world.blocked_at() {
            Some(limit) => (state.world.distance + step).min(limit),
            None => state.world.distance + step,
        };
        cross_fork(state, content);
        cross_region(state, content, sounds);
        arrive(state, sounds);
    }

    // Ground actually covered, for intake to accrue against next tick.
    // Measured rather than assumed, so the last stride into a fork line
    // reports the short step it really took. Zero when the legs did not
    // run, which is the whole of "a stopped tower harvests nothing"
    // (`SYSTEMS.md` §3.6) — see `GameState::paces_last` for why intake
    // reads it a tick late instead of stride running earlier.
    state.paces_last = state.world.distance - from;

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

/// Commit the branch once the tower is over the fork line.
///
/// The answer stops being changeable here rather than when it is given,
/// which is what lets a player think again right up until the moment
/// they are standing on the split.
fn cross_fork(state: &mut GameState, content: &Content) {
    let Some(fork) = state.world.fork else {
        return;
    };
    if state.world.distance < fork.at || fork.answer.is_none() {
        return;
    }
    state.world.fork = None;
    state.world.next_fork_at +=
        paces_from_int(content.region(state.world.region).fork_interval_paces);
    state.world.skip_forks_too_near_an_edge(content);
}

/// Crossing a region boundary is an event, not only a fact: it is where
/// the fork schedule resets and where the next region's one-time setup
/// happens.
fn cross_region(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let here = state.world.region_at(state.world.distance);
    if here == state.world.region {
        return;
    }
    state.world.enter_region(content, here);
    sounds.push(SoundEvent::RegionChange);
}

/// The far edge of the last region, reached.
///
/// The generator has nothing past it, so `blocked_at` already stopped
/// the tower there; this is only the moment being noticed. Presented as
/// an arrival rather than a victory — the game reports where the tower
/// got to, it does not grade it (`DECISIONS.md` §8).
fn arrive(state: &mut GameState, sounds: &mut Vec<SoundEvent>) {
    if state.arrived || state.world.distance < state.world.journey_end() {
        return;
    }
    state.arrived = true;
    sounds.push(SoundEvent::Arrived);
}

/// Paces per tick, derived from the designer-facing "paces per 100
/// ticks" so the balance file never contains a fixed-point literal.
#[must_use]
pub fn stride_per_tick(content: &Content) -> Fx {
    Fx::ratio(content.balance.world.stride_paces_per_100_ticks as i32, 100)
}
