//! Production — recipes turning inputs into outputs on a timer.
//!
//! A room advances one tick of `progress` when, and only when, every
//! input stack holds at least its per-craft amount and every output
//! stack has room for what the craft will emit. At `craft_ticks` the
//! inputs are consumed and the outputs appear.
//!
//! A stall does not reset progress. Partial work survives the gap, so a
//! chain that hiccups doesn't throw away a half-finished craft — it
//! just goes quiet until the crate arrives.

use crate::content::Content;
use crate::state::GameState;

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let mut crafts = 0u64;

    // Charge is drawn as rooms advance, and production sits second in
    // the priority order — after the cars, before the lamps.
    let mut power = std::mem::replace(&mut state.power, crate::state::Power::new(0));
    let tick = state.tick;
    // Who is standing in which room, gathered once. A `Vec` rather than
    // a set, per `DECISIONS.md` §2 — at single-digit crew a linear scan
    // is cheaper than a hash and, more to the point, ordered.
    let manned: Vec<crate::ids::RoomId> = state
        .crew
        .iter()
        .filter_map(|member| match member.state {
            crate::state::CrewState::Manning { room } => Some(room),
            _ => None,
        })
        .collect();

    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            let rt = content.room_rt(room.def);
            if rt.craft_ticks == 0 || !room.is_working(content, tick) {
                continue;
            }

            let inputs_ready = rt
                .recipe_inputs
                .iter()
                .enumerate()
                .all(|(i, (_, amount, _))| room.inputs.get(i).is_some_and(|s| s.count >= *amount));
            let outputs_ready = rt
                .recipe_outputs
                .iter()
                .enumerate()
                .all(|(i, (_, amount, _))| {
                    room.outputs.get(i).is_some_and(|s| s.space() >= *amount)
                });

            // Missing an input or backed up on the output: hold
            // progress where it is. Partial work survives the gap, and
            // the room simply goes quiet.
            if !inputs_ready || !outputs_ready {
                continue;
            }

            // A powered room that cannot buy its charge this tick holds
            // progress too. Indistinguishable from starving, from the
            // outside — which is correct: it is starving, for power.
            let draw = content.room(room.def).power_draw;
            if !power.draw(crate::state::power::PowerUse::Works, draw) {
                continue;
            }

            room.progress += 1;
            // **Somebody standing in the room shortens the craft, and
            // the shortening is on the target rather than on the step.**
            // Scaling progress instead was the obvious version and it
            // did nothing at all: `manned_work_pct / 100` is integer
            // division, so 150% advanced by exactly the same 1 as
            // nobody, and the test caught it at 29 crafts against 29.
            //
            // `needs::effective_ticks` is the same idea in the other
            // direction but clamps to 100, because being hungry can only
            // slow you down. This is the mirror of it.
            //
            // The recipe is untouched: the room still eats one lot of
            // inputs per craft, so a posting moves the bottleneck onto
            // the chain feeding the room rather than removing it.
            let target = if manned.contains(&room.id) {
                let pct = content.balance.crew.manned_work_pct.max(100);
                u32::try_from(i64::from(rt.craft_ticks) * 100 / pct).unwrap_or(rt.craft_ticks)
            } else {
                rt.craft_ticks
            };
            if room.progress < target {
                continue;
            }

            for (i, (_, amount, _)) in rt.recipe_inputs.iter().enumerate() {
                if let Some(stack) = room.inputs.get_mut(i) {
                    stack.withdraw(*amount);
                }
            }
            for (i, (_, amount, _)) in rt.recipe_outputs.iter().enumerate() {
                if let Some(stack) = room.outputs.get_mut(i) {
                    stack.deposit(*amount);
                }
            }
            room.progress = 0;
            crafts += 1;
            sounds.push(SoundEvent::Craft);
        }
    }

    std::mem::swap(&mut state.power, &mut power);
    state.stats.crafts_completed += crafts;
}
