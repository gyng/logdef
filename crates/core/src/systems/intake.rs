//! Intake — the tower strips what it walks past.
//!
//! An intake room accrues `1 / ticks_per_item`, scaled by the terrain
//! yield underfoot, into a fixed-point accumulator. Every time the
//! accumulator crosses 1.0 it pushes one item into the room's outbox.
//!
//! A full outbox stalls the accumulator rather than discarding the
//! overflow: the arm visibly stops, and nothing vanishes silently. That
//! stall is the feedback — the room goes quiet because nobody is
//! collecting from it.

use crate::content::Content;
use crate::fx::{FX_ONE, Fx};
use crate::state::GameState;

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let yield_mul = Fx::ratio(state.world.current_yield_pct(content) as i32, 100);
    let mut harvested = 0u64;

    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            let rt = content.room_rt(room.def);
            let Some(item) = rt.intake_item else {
                continue;
            };
            let Some(slot) = room.outputs.iter().position(|s| s.item == item) else {
                continue;
            };

            if room.outputs[slot].is_full() {
                // Stalled: hold the accumulator so partial work
                // survives until a crew member frees space. The arm
                // visibly stops — that silence is the feedback, so
                // there is no event and no warning banner.
                continue;
            }

            let base = Fx::ratio(1, rt.intake_ticks_per_item.max(1) as i32);
            room.intake_acc += base * yield_mul;

            while room.intake_acc.0 >= FX_ONE && room.outputs[slot].deposit(1) == 1 {
                room.intake_acc -= Fx::ONE;
                harvested += 1;
                sounds.push(SoundEvent::Harvest);
            }
        }
    }

    state.stats.items_harvested += harvested;
}
