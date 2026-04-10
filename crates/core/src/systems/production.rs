use crate::snapshot::SoundEvent;
use crate::state::GameState;
use crate::types::Scalar;

pub fn run(state: &mut GameState, _dt: Scalar, sounds: &mut Vec<SoundEvent>) {
    if state.encounter.is_none() {
        return;
    }

    for floor in &mut state.tower.floors {
        let building = match &mut floor.building {
            Some(b) if b.is_active => b,
            _ => continue,
        };

        // Accumulate fractional production in a simple way:
        // Add to a "progress" counter using the existing buffer current as integer.
        // We track fractional progress by checking if accumulated production crosses 1.0.
        // Use output_buffer.max as threshold — produce only if buffer not full.
        if building.output_buffer.current < building.output_buffer.max {
            // We need a fractional accumulator. Since Building doesn't have one,
            // we'll use a trick: check if (rate * elapsed_time) would have produced
            // by now. Simpler: just probabilistically produce based on dt.
            // Actually, let's use a deterministic approach:
            // At 6 crates/min = 0.1 crates/sec = 0.00333 crates/tick at 30hz.
            // After 300 ticks (10 sec), we'd have accumulated 1.0 crate.
            // We'll track this via a static counter approximation:
            // production_progress += production_this_tick, and when >= 1.0, produce.
            //
            // Problem: Building struct has no progress field. Let's repurpose
            // production_rate as both rate config AND accumulate progress in a
            // separate field. For MVP, add it to output_buffer.current using
            // fractional math: if we reach the next integer, emit a crate.

            // MVP simplified: check if enough time has passed based on tick count
            // production_rate crates/min → one crate every (60/rate) seconds
            // At 30hz: one crate every (60/rate * 30) ticks
            let ticks_per_crate = (60.0 / building.production_rate * 30.0) as u64;
            if ticks_per_crate > 0 && state.tick % ticks_per_crate == 0 {
                building.output_buffer.current += 1;
                sounds.push(SoundEvent::BuildingProduce {
                    building_type: building.building_type,
                });
            }
        }
    }
}
