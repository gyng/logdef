use crate::snapshot::SoundEvent;
use crate::state::GameState;
use crate::types::Scalar;

#[allow(clippy::ptr_arg)] // Will push to sounds when implemented
pub fn run(_state: &mut GameState, _dt: Scalar, _sounds: &mut Vec<SoundEvent>) {
    // TODO: companion target selection, firing, ability usage
}
