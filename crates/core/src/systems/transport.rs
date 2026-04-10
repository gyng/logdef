use crate::snapshot::SoundEvent;
use crate::state::*;
use crate::types::Scalar;

/// MVP auto-deliver: transfer building output directly to hero ammo.
/// Skips the full runner/pathfinding system entirely.
#[allow(clippy::ptr_arg)] // Will push to sounds when runner events are added
pub fn run(state: &mut GameState, _dt: Scalar, _sounds: &mut Vec<SoundEvent>) {
    if state.encounter.is_none() {
        return;
    }

    // Collect produced ammo from building output buffers
    let mut arrows_available: u32 = 0;
    let mut bolts_available: u32 = 0;

    for floor in &mut state.tower.floors {
        if let Some(building) = &mut floor.building
            && building.output_buffer.current > 0 {
                match building.output_buffer.resource {
                    ResourceType::Arrows => {
                        arrows_available += building.output_buffer.current;
                        building.output_buffer.current = 0;
                    }
                    ResourceType::Bolts => {
                        bolts_available += building.output_buffer.current;
                        building.output_buffer.current = 0;
                    }
                    _ => {}
                }
            }
    }

    // Deliver to hero's personal ammo (MVP simplification)
    let hero = &mut state.tower.hero;
    let max_ammo = 30u32; // Generous cap for MVP
    match hero.weapon_primary.base_type {
        WeaponBaseType::Bow => {
            hero.personal_ammo = (hero.personal_ammo + arrows_available).min(max_ammo);
        }
        WeaponBaseType::Crossbow => {
            hero.personal_ammo = (hero.personal_ammo + bolts_available).min(max_ammo);
        }
        _ => {}
    }
}
