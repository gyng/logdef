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
use crate::ids::ItemIdx;
use crate::state::GameState;

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let mut crafts = 0u64;

    // A finite-output recipe may carry an authored tower-wide kanban.
    // Count before borrowing the tower mutably, then update the count
    // as this tick completes crafts. Hauling only moves these items and
    // consumers run after production, so the next tick observes every
    // other change without putting policy into the porter.
    let mut targeted_stock: Vec<(ItemIdx, i64)> = content
        .room_runtime
        .iter()
        .filter_map(|room| room.output_stock_target.map(|(item, _)| item))
        .map(|item| (item, total_in_flight(state, item)))
        .collect();
    targeted_stock.sort_by_key(|(item, _)| *item);
    targeted_stock.dedup_by_key(|(item, _)| *item);

    // Charge is drawn as rooms advance, and production sits second in
    // the priority order — after the cars, before the lamps.
    let mut power = std::mem::replace(&mut state.power, crate::state::Power::new(0));
    let tick = state.tick;
    let manned = super::manned_rooms(state, content);

    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            let rt = content.room_rt(room.def);
            if rt.craft_ticks == 0 || !room.is_working(content, tick) {
                continue;
            }
            if room.exhaust_refused {
                continue;
            }
            if !super::staffed(content, room, &manned) {
                // Understaffed. Stalls in place like a starved room
                // rather than resetting, so the work already done
                // survives somebody being called away.
                continue;
            }

            if let Some((item, target)) = rt.output_stock_target {
                let held = targeted_stock
                    .iter()
                    .find(|(candidate, _)| *candidate == item)
                    .map_or(0, |(_, held)| *held);
                let batch = rt
                    .recipe_outputs
                    .first()
                    .map_or(0, |(_, amount, _)| *amount);
                if held + batch > target {
                    continue;
                }
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

            // **A half-powered room turns at half speed** (`SYSTEMS.md`
            // §6.39). It used to be paid in full or hold its progress
            // entirely, which made the smallest expressible `power_draw`
            // an all-or-nothing commitment; now the Works circuit's
            // share applies to every room on it equally.
            //
            // A room that draws nothing is never slowed, so an unpowered
            // chain is untouched by a brown-out — which is what keeps the
            // charge loop (cutter arm to bamboo to burner) free of any
            // dependence on charge.
            let draw = content.room(room.def).power_draw;
            let pace = if draw > 0 {
                power.served_at(floor.index, crate::state::power::PowerUse::Works)
            } else {
                crate::state::power::FULL
            };
            if pace <= 0 {
                room.power_refused = true;
                continue;
            }
            room.power_refused = pace < crate::state::power::FULL;

            // Whole ticks of work, plus whatever fraction carries over.
            room.work_acc += pace;
            let ticks = room.work_acc / crate::state::power::FULL;
            room.work_acc -= ticks * crate::state::power::FULL;
            if ticks <= 0 {
                continue;
            }
            room.progress += u32::try_from(ticks).unwrap_or(1);
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
            //
            // **And a practised hand on the machine shortens it
            // further.** `post_pct` is 100 for an empty room, the
            // posting bonus for a room with somebody in it, and that
            // plus their rank for a room with somebody in it who has
            // been doing this a while.
            let pct = super::post_pct(content, room.id, &manned);
            let target = if pct > 100 {
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
            if let Some((item, _)) = rt.output_stock_target
                && let Some((_, held)) = targeted_stock
                    .iter_mut()
                    .find(|(candidate, _)| *candidate == item)
            {
                *held += rt
                    .recipe_outputs
                    .first()
                    .map_or(0, |(_, amount, _)| *amount);
            }
            room.progress = 0;
            crafts += 1;
            sounds.push(SoundEvent::Craft);
        }
    }

    std::mem::swap(&mut state.power, &mut power);
    state.stats.crafts_completed += crafts;
}

fn total_in_flight(state: &GameState, target: ItemIdx) -> i64 {
    let rooms: i64 = state
        .tower
        .floors
        .iter()
        .flat_map(|floor| &floor.rooms)
        .map(|room| {
            room.inputs
                .iter()
                .chain(&room.outputs)
                .filter(|stack| stack.item == target)
                .map(|stack| stack.count)
                .sum::<i64>()
                + room
                    .shelves
                    .iter()
                    .filter(|shelf| shelf.item == Some(target))
                    .map(|shelf| shelf.count)
                    .sum::<i64>()
        })
        .sum();
    let carried: i64 = state
        .crew
        .iter()
        .filter_map(|crew| crew.carrying)
        .filter(|(item, _)| *item == target)
        .map(|(_, count)| count)
        .sum();
    let cars: i64 = state
        .tower
        .shafts
        .iter()
        .flat_map(|shaft| &shaft.cars)
        .flat_map(|car| &car.freight)
        .filter(|stack| stack.item == target)
        .map(|stack| stack.count)
        .sum();
    rooms + carried + cars
}
