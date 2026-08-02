//! Building production.
//!
//! Each tick, every active building advances `production_progress`
//! toward 1.0 at a rate of `dt / seconds_per_craft`. When it crosses
//! 1.0 we consume one unit from every input buffer and push one crate
//! to the output buffer.
//!
//! Raw producers (Lumberyard, Quarry — empty `input_buffers`) advance
//! unconditionally. Crafters (Fletcher needs wood, Forge needs stone,
//! …) stall whenever any input buffer is below the per-craft amount,
//! making the chain visibly bottleneck on missing materials.
//!
//! A full output buffer also stalls production — the building visibly
//! stops producing until a runner clears space, surfacing logistics
//! pressure to the player.

use crate::registry::Registry;
use crate::snapshot::SoundEvent;
use crate::state::*;
use crate::types::Scalar;

/// Each input slot consumes 1 unit per craft for v1. Future cycles can
/// promote this to a per-input amount on the registry.
const INPUT_PER_CRAFT: u32 = 1;

pub fn run(state: &mut GameState, _registry: &Registry, dt: Scalar, sounds: &mut Vec<SoundEvent>) {
    if state.encounter.is_none() && state.drill.is_none() {
        return;
    }

    for floor in &mut state.tower.floors {
        let Some(building) = floor.building.as_mut() else {
            continue;
        };
        if !building.is_active {
            continue;
        }

        // Output buffer full: stall (visible silent building).
        if building.output_buffer.current >= building.output_buffer.max {
            continue;
        }

        // Inputs gate: every input buffer must hold >= INPUT_PER_CRAFT.
        // Raw producers (no inputs) skip this check entirely.
        let has_inputs = !building.input_buffers.is_empty();
        if has_inputs {
            let all_satisfied = building
                .input_buffers
                .iter()
                .all(|inb| inb.current >= INPUT_PER_CRAFT);
            if !all_satisfied {
                // Stall until a runner refills the inbox. Don't reset
                // existing progress — partial work survives the gap.
                continue;
            }
        }

        // Advance the in-progress craft.
        let rate = building.production_rate.max(0.01);
        let seconds_per_craft = 60.0 / rate;
        building.production_progress += dt / seconds_per_craft;

        if building.production_progress >= 1.0 {
            // Consume one unit from each input buffer.
            for inb in &mut building.input_buffers {
                inb.current = inb.current.saturating_sub(INPUT_PER_CRAFT);
            }
            // Push a crate to the outbox.
            building.output_buffer.current += 1;
            building.production_progress = 0.0;
            sounds.push(SoundEvent::BuildingProduce {
                building_type: building.building_type,
            });
        }
    }
}
