//! Intake — the tower strips what it walks past.
//!
//! An intake room accrues at the rate its `IntakeSource` authorises,
//! scaled by the terrain yield underfoot, into a fixed-point
//! accumulator. Every time the
//! accumulator crosses 1.0 it pushes one item into the room's outbox.
//!
//! A full outbox stalls the accumulator rather than discarding the
//! overflow: the arm visibly stops, and nothing vanishes silently. That
//! stall is the feedback — the room goes quiet because nobody is
//! collecting from it.

use crate::content::{Content, IntakeSource};
use crate::fx::{FX_ONE, Fx};
use crate::state::GameState;

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let yield_mul = Fx::ratio(state.world.current_yield_pct(content) as i32, 100);
    let tick = state.tick;
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
            // A damaged arm strips less; a wrecked one strips nothing.
            if !room.is_working(content, tick) {
                continue;
            }

            if room.outputs[slot].is_full() {
                // Stalled: hold the accumulator so partial work
                // survives until a crew member frees space. The arm
                // visibly stops — that silence is the feedback, so
                // there is no event and no warning banner.
                continue;
            }

            // The per-pace accrual `SYSTEMS.md` §3.6 specifies is not
            // wired yet — that needs `paces_last`, written by stride.
            // Until it is, a Terrain source accrues per tick at exactly
            // the equivalent of its authored pace rate, which is the
            // same one-for-one conversion §3.6 uses: at 0.6 paces/tick,
            // 54 paces an item is 90 ticks an item. Behaviour is
            // unchanged for a tower that never stops, which is the
            // whole point of that conversion.
            let Some(source) = rt.intake_source else {
                continue;
            };
            let ticks_per_item = match source {
                IntakeSource::Terrain { paces_per_item } => {
                    paces_per_item * 100 / content.balance.world.stride_paces_per_100_ticks.max(1)
                }
                // Berthing is not built, so a ruin source draws nothing
                // rather than quietly drawing per tick everywhere.
                IntakeSource::Ruin { .. } => continue,
            };

            let base = Fx::ratio(1, ticks_per_item.max(1) as i32);
            room.intake_acc += base * yield_mul;

            while room.intake_acc.0 >= FX_ONE && room.outputs[slot].deposit(1) == 1 {
                room.intake_acc -= Fx::ONE;
                harvested += 1;
                sounds.push(SoundEvent::Harvest);
            }
        }
    }

    state.stats.items_harvested += harvested;

    // Stripping the terrain is noticed. The cost lands next to the act
    // rather than in a separate bookkeeping pass, so it is impossible
    // to add a new way of harvesting and forget to make it provoking.
    let per_100 = content.balance.siege.provocation_per_100_harvested;
    super::siege::provoke_hundredths(state, content, harvested as i64 * per_100);
}
