//! Cars: the elevator, and the dumbwaiter.
//!
//! The dispatch model is the SimTower one, ported rather than invented:
//! bidirectional sweep, a dispatch threshold so cars batch instead of
//! yo-yoing after single callers, dwell proportional to how many people
//! move, and fixed queues that belong to floors rather than to cars.
//! That last detail is what lets a second car share a shaft later
//! without any of this being rewritten.
//!
//! Getting this feel right is the point of the whole milestone. An
//! elevator that teleports the nearest caller instantly is not an
//! elevator, and a tower served by one would have no contention worth
//! playing against.
//!
//! Runs before `haul`, so crew see cars where they actually are this
//! tick, and before `production`'s charge draw so a moving car is
//! served first when the pool is thin.

use crate::content::{Content, ShaftKind};
use crate::fx::Fx;
use crate::ids::{CrewId, DaypartIdx, FloorIdx, ItemIdx, ShaftId};
use crate::state::{Car, CarDir, CarState, Crew, CrewState, GameState, ShaftPriority, Stack};

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let daypart = state.clock.daypart(content);

    for index in 0..state.tower.shafts.len() {
        match state.tower.shafts[index].kind {
            // Neither of these is driven. Stairs are climbed under a
            // crew member's own power, and a chute is gravity: things
            // fall down it the moment they are let go, so there is
            // nothing to advance between ticks.
            ShaftKind::Stairs | ShaftKind::Chute => {}
            ShaftKind::Elevator => run_elevator(state, content, index, daypart, sounds),
        }
    }
}

// ---------------------------------------------------------------------------
// Elevator
// ---------------------------------------------------------------------------

/// One waiting crew member, from the car's point of view. Who they are
/// only matters once the doors open, so a call is just a floor, a
/// direction, and how long it has been standing there.
struct Call {
    from: FloorIdx,
    dir: CarDir,
    waited: u32,
}

fn run_elevator(
    state: &mut GameState,
    content: &Content,
    shaft_index: usize,
    daypart: DaypartIdx,
    sounds: &mut Vec<SoundEvent>,
) {
    let shaft_id = state.tower.shafts[shaft_index].id;
    let def_idx = state.tower.shafts[shaft_index].def;
    let ticks_per_floor = content.shaft(def_idx).ticks_per_floor.max(1);
    let charge_per_floor = content.shaft(def_idx).charge_per_floor;
    let capacity = state.tower.shafts[shaft_index].capacity;
    let batch = content.shaft(def_idx).batch;
    let balance = &content.balance.transport;

    let calls = collect_calls(&state.crew, shaft_id);

    for car_index in 0..state.tower.shafts[shaft_index].cars.len() {
        let car = state.tower.shafts[shaft_index].cars[car_index].clone();
        let state_before = car.state;
        let next = match car.state {
            CarState::Dwelling { ticks_left } => {
                if ticks_left > 0 {
                    let mut car = car;
                    car.state = CarState::Dwelling {
                        ticks_left: ticks_left - 1,
                    };
                    car
                } else {
                    depart(car, &state.tower.shafts[shaft_index], daypart, &calls)
                }
            }
            CarState::Idle => {
                let mut car = car;
                car.idle_ticks = car.idle_ticks.saturating_add(1);
                dispatch(
                    car,
                    &state.tower.shafts[shaft_index],
                    daypart,
                    &calls,
                    balance,
                )
            }
            CarState::Moving => {
                // Charge buys the next step. A car that cannot pay
                // simply holds position — the brown-out made physical.
                let step = Fx::ratio(1, ticks_per_floor as i32);
                let cost = charge_per_floor / i64::from(ticks_per_floor).max(1);
                if !state.power.draw(crate::state::power::PowerUse::Lifts, cost) {
                    car
                } else {
                    advance(car, step, &state.tower.shafts[shaft_index], daypart, &calls)
                }
            }
        };

        state.tower.shafts[shaft_index].cars[car_index] = next;

        // **Nobody is calling, so fetch something.** This is the whole
        // of what used to be a second kind of shaft (`SYSTEMS.md`
        // §6.18). A lift standing still is a lift that could be moving
        // stock, and moving stock is the half of vertical transport
        // that costs *nobody a walk* — which `examples/lift.rs`
        // measured as the dumbwaiter's real advantage, and the one that
        // grows rather than shrinks as a hull widens.
        //
        // Riders come first and always: this only runs on a car that
        // `dispatch` left idle with nothing to serve, no stops and
        // nobody aboard. A caller appearing next tick finds a car with
        // a load in it and a stop on its list, which is exactly the
        // contention the design wants — one shaft, two demands, and the
        // player watching a queue form behind a crate.
        let idle_and_free = {
            let car = &state.tower.shafts[shaft_index].cars[car_index];
            matches!(car.state, CarState::Idle)
                && car.riders.is_empty()
                && car.stops.is_empty()
                && car.freight.is_empty()
        };
        if idle_and_free {
            seek_freight(state, shaft_index, car_index, batch, daypart);
        }

        // Doors just opened: let people off, then on. Only on the
        // transition, so a long dwell does not re-board every tick.
        //
        // **And on a re-opening, which is the half that was missing.**
        // `depart` ends with "someone is calling from this very floor
        // and could not board" and answers it by returning
        // `Dwelling { ticks_left: 0 }` — the doors staying open. That
        // is a Dwelling-to-Dwelling transition, so the guard above
        // suppressed the one call it exists to make, and the car sat
        // at the floor its callers were standing on for ever.
        //
        // Measured on `a_severed_shaft_forces_a_live_reroute` after M6
        // grew the fixture tower: all three crew on floor 0, waiting
        // **18,857 ticks**, boarding a car parked on floor 0 with no
        // riders. It needed a car to be dwelling *before* the callers
        // appeared, which is what cutting the stairs out from under
        // them produces and very little else does.
        //
        // A zero-length dwell only ever comes from that branch of
        // `depart`, so this cannot re-board on an ordinary stop.
        let now = state.tower.shafts[shaft_index].cars[car_index].state;
        let opened = matches!(now, CarState::Dwelling { .. })
            && (!matches!(state_before, CarState::Dwelling { .. })
                || matches!(state_before, CarState::Dwelling { ticks_left: 0 }));
        if opened {
            service_stop(
                state,
                content,
                shaft_index,
                car_index,
                shaft_id,
                capacity,
                daypart,
                sounds,
            );
        }

        sync_riders(state, shaft_index, car_index);
    }
}

fn collect_calls(crew: &[Crew], shaft: ShaftId) -> Vec<Call> {
    crew.iter()
        .filter_map(|member| match member.state {
            CrewState::Boarding {
                shaft: at,
                to_floor,
            } if at == shaft => Some(Call {
                from: member.floor(),
                dir: CarDir::toward(member.floor(), to_floor),
                waited: member.wait_ticks,
            }),
            _ => None,
        })
        .collect()
}

/// An idle car departs when demand justifies it: either enough callers
/// have gathered, or the oldest has waited long enough. Without this a
/// car chases every single caller and the queue never forms — and the
/// queue is the thing this milestone exists to test.
fn dispatch(
    mut car: Car,
    shaft: &crate::state::Shaft,
    daypart: DaypartIdx,
    calls: &[Call],
    balance: &crate::content::TransportBalance,
) -> Car {
    let program = shaft.program(daypart);
    let served: Vec<&Call> = calls
        .iter()
        .filter(|call| program.serves(call.from))
        .collect();

    if served.is_empty() && car.stops.is_empty() {
        car.dir = CarDir::Idle;
        return car;
    }

    let longest_wait = served.iter().map(|call| call.waited).max().unwrap_or(0);
    let enough = served.len() >= balance.dispatch_threshold as usize
        || longest_wait >= balance.dispatch_max_wait_ticks
        || !car.stops.is_empty();
    if !enough {
        return car;
    }

    // Head for whichever outstanding floor is nearest. Ties go to the
    // lower floor so the choice is a pure function of state.
    let here = car.floor();
    let target = served
        .iter()
        .map(|call| call.from)
        .chain(car.stops.iter().copied())
        .min_by_key(|floor| (floor.abs_diff(here), *floor));

    let Some(target) = target else {
        car.dir = CarDir::Idle;
        return car;
    };

    car.idle_ticks = 0;
    if target == here {
        // Already here — open up rather than travelling zero floors.
        car.state = CarState::Dwelling {
            ticks_left: balance.dwell_base_ticks,
        };
        car.dir = served
            .iter()
            .find(|call| call.from == here)
            .map_or(CarDir::Up, |call| call.dir);
    } else {
        car.dir = CarDir::toward(here, target);
        car.state = CarState::Moving;
    }
    car
}

/// Move a car one step, stopping when it reaches a floor somebody wants.
fn advance(
    mut car: Car,
    step: Fx,
    shaft: &crate::state::Shaft,
    daypart: DaypartIdx,
    calls: &[Call],
) -> Car {
    let low = Fx::from_int(i32::from(shaft.low));
    let high = Fx::from_int(i32::from(shaft.high));

    match car.dir {
        CarDir::Up => car.pos = (car.pos + step).min(high),
        CarDir::Down => car.pos = (car.pos - step).max(low),
        CarDir::Idle => {
            car.state = CarState::Idle;
            return car;
        }
    }

    if !car.at_floor() {
        return car;
    }

    let here = car.floor();
    let program = shaft.program(daypart);
    let wants_stop = car.stops.contains(&here)
        || (program.serves(here)
            && calls
                .iter()
                .any(|call| call.from == here && (call.dir == car.dir || car.riders.is_empty())));

    if wants_stop {
        // Dwell length is set when the doors open, once we know how
        // many people actually move.
        car.state = CarState::Dwelling { ticks_left: 0 };
        return car;
    }

    // Nothing here. Keep going, reverse, or park.
    let ends = (car.dir == CarDir::Up && here >= shaft.high)
        || (car.dir == CarDir::Down && here <= shaft.low);
    if ends && !car.has_stop_beyond(here, car.dir) {
        car.dir = car.dir.reversed();
        if !car.has_stop_beyond(here, car.dir) && calls.is_empty() {
            car.state = CarState::Idle;
            car.dir = CarDir::Idle;
        }
    }
    car
}

/// After a dwell finishes, pick the next direction by sweep: carry on
/// if anything remains ahead, otherwise reverse, otherwise park.
fn depart(mut car: Car, shaft: &crate::state::Shaft, daypart: DaypartIdx, calls: &[Call]) -> Car {
    let here = car.floor();
    let program = shaft.program(daypart);
    let call_beyond = |dir: CarDir| {
        calls.iter().any(|call| {
            program.serves(call.from)
                && match dir {
                    CarDir::Up => call.from > here,
                    CarDir::Down => call.from < here,
                    CarDir::Idle => false,
                }
        })
    };

    if car.has_stop_beyond(here, car.dir) || call_beyond(car.dir) {
        car.state = CarState::Moving;
        return car;
    }

    let back = car.dir.reversed();
    if car.has_stop_beyond(here, back) || call_beyond(back) {
        car.dir = back;
        car.state = CarState::Moving;
        return car;
    }

    // Someone is calling from this very floor and could not board.
    if calls.iter().any(|call| call.from == here) {
        car.state = CarState::Dwelling { ticks_left: 0 };
        return car;
    }

    car.dir = CarDir::Idle;
    car.state = CarState::Idle;
    car.idle_ticks = 0;
    car
}

/// The doors are open. Alight anyone who has arrived, then board whoever
/// fits, in priority order, and set the dwell from how many moved.
#[allow(clippy::too_many_arguments)]
fn service_stop(
    state: &mut GameState,
    content: &Content,
    shaft_index: usize,
    car_index: usize,
    shaft_id: ShaftId,
    capacity: u8,
    daypart: DaypartIdx,
    sounds: &mut Vec<SoundEvent>,
) {
    let here = state.tower.shafts[shaft_index].cars[car_index].floor();
    let mut moved = 0u32;

    // Alight. Riders whose destination is this floor step out and pick
    // up their journey from here.
    let arriving: Vec<CrewId> = state.tower.shafts[shaft_index].cars[car_index]
        .riders
        .iter()
        .copied()
        .filter(|id| {
            state.crew.iter().any(|member| {
                member.id == *id
                    && matches!(member.state, CrewState::Riding { to_floor, .. } if to_floor == here)
            })
        })
        .collect();

    for id in arriving {
        state.tower.shafts[shaft_index].cars[car_index]
            .riders
            .retain(|rider| *rider != id);
        if let Some(member) = state.crew.iter_mut().find(|member| member.id == id) {
            member.snap_to(here, state.tower.shafts[shaft_index].slot);
            member.state = CrewState::Idle;
            member.wait_ticks = 0;
        }
        moved += 1;
    }
    // **Freight gets off with the riders.** A lift that has stopped to
    // open its doors is a lift that can put the crate down, which is
    // why the merge costs a call here rather than a second state
    // machine. Ahead of `clear_stop`, so a load that did not fit leaves
    // the stop standing and the car comes back for it.
    if !state.tower.shafts[shaft_index].cars[car_index]
        .freight
        .is_empty()
    {
        unload_freight(state, shaft_index, car_index, sounds);
        moved += 1;
    }
    if state.tower.shafts[shaft_index].cars[car_index]
        .freight
        .is_empty()
    {
        state.tower.shafts[shaft_index].cars[car_index].clear_stop(here);
    }

    // An empty car with nothing further along its sweep takes its
    // direction from whoever has waited longest here. Without this a
    // car that travelled up to answer a call arrives pointing up and
    // then refuses to board the person who wanted to go down — it
    // would ferry an empty box back and forth forever.
    let turn_around = {
        let car = &state.tower.shafts[shaft_index].cars[car_index];
        car.riders.is_empty() && !car.has_stop_beyond(here, car.dir)
    };
    if turn_around {
        let adopt = state
            .crew
            .iter()
            .filter(|member| {
                matches!(member.state, CrewState::Boarding { shaft, .. }
                    if shaft == shaft_id && member.floor() == here)
            })
            .max_by_key(|member| (member.wait_ticks, member.id.0))
            .and_then(|member| match member.state {
                CrewState::Boarding { to_floor, .. } => Some(CarDir::toward(here, to_floor)),
                _ => None,
            });
        if let Some(dir) = adopt {
            state.tower.shafts[shaft_index].cars[car_index].dir = dir;
        }
    }

    // Board. Direction has to match, or the car would carry people the
    // wrong way — the classic elevator mistake.
    let dir = state.tower.shafts[shaft_index].cars[car_index].dir;
    let priority = state.tower.shafts[shaft_index].program(daypart).priority;

    let mut candidates: Vec<usize> = state
        .crew
        .iter()
        .enumerate()
        .filter(|(_, member)| {
            matches!(member.state, CrewState::Boarding { shaft, to_floor }
                if shaft == shaft_id
                    && member.floor() == here
                    && (dir == CarDir::Idle || CarDir::toward(here, to_floor) == dir))
        })
        .map(|(index, _)| index)
        .collect();

    candidates.sort_by_key(|index| {
        let member = &state.crew[*index];
        let laden = member.is_carrying();
        let rank = match priority {
            ShaftPriority::Balanced => 0,
            ShaftPriority::FreightFirst => i32::from(!laden),
            ShaftPriority::CrewFirst => i32::from(laden),
        };
        // Longest wait first within a rank, then by id so the order is
        // never down to vector position.
        (rank, -(member.wait_ticks as i32), member.id.0)
    });

    for index in candidates {
        let load = state.tower.shafts[shaft_index].car_load(car_index, &state.crew);
        let weight = if state.crew[index].is_carrying() {
            2
        } else {
            1
        };
        if load + weight > capacity {
            continue;
        }
        let CrewState::Boarding { to_floor, .. } = state.crew[index].state else {
            continue;
        };
        state.tower.shafts[shaft_index].cars[car_index]
            .riders
            .push(state.crew[index].id);
        state.tower.shafts[shaft_index].cars[car_index].add_stop(to_floor);
        state.crew[index].state = CrewState::Riding {
            shaft: shaft_id,
            car: car_index as u8,
            to_floor,
        };
        state.crew[index].wait_ticks = 0;
        moved += 1;
    }

    let balance = &content.balance.transport;
    let dwell = balance.dwell_base_ticks + balance.dwell_per_unit_ticks * moved;
    state.tower.shafts[shaft_index].cars[car_index].state =
        CarState::Dwelling { ticks_left: dwell };
    if moved > 0 {
        sounds.push(SoundEvent::CarStop);
    }
}

/// Riders are cargo: their position is the car's.
fn sync_riders(state: &mut GameState, shaft_index: usize, car_index: usize) {
    let pos = state.tower.shafts[shaft_index].cars[car_index].pos;
    let slot = state.tower.shafts[shaft_index].slot;
    let riders = state.tower.shafts[shaft_index].cars[car_index]
        .riders
        .clone();
    for id in riders {
        if let Some(member) = state.crew.iter_mut().find(|member| member.id == id) {
            member.floor_fx = pos;
            member.slot_fx = Fx::from_int(i32::from(slot));
        }
    }
}

// ---------------------------------------------------------------------------
// Dumbwaiter
// ---------------------------------------------------------------------------

/// Find the best load to move, using the same priority the crew use: a
/// hungry recipe outranks a shelf.
///
/// **This is the dumbwaiter, and it is now something a lift does when
/// nobody is calling** (`SYSTEMS.md` §6.18). The scoring is unchanged
/// from the shaft it used to be, deliberately: `examples/lift.rs`
/// measured that behaviour as the best vertical transport in the game
/// at eight floors, and a merge that rewrote it would have thrown away
/// the thing worth keeping.
///
/// What changed is where the load's destination goes. A dumbwaiter had
/// a `target` and drove itself to it; a lift has `stops`, and putting
/// the destination there means the car's own routing carries the crate
/// — so a rider calling mid-trip is served on the way rather than
/// waiting for the freight to finish. One shaft, two demands, and the
/// contention between them visible on the cross-section.
///
/// **The program binds freight too**, and forgetting that was the one
/// bug the merge introduced: a lift told not to serve a floor went and
/// fetched a crate from it anyway, because the freight finder scanned
/// the shaft's whole span and knew nothing about the schedule.
/// `an_unserved_floor_is_not_stopped_at` caught it. A daypart program is
/// the player saying *this shaft is not for that floor right now*, and a
/// statement that only bound half the shaft's traffic would be worse
/// than no statement at all.
fn seek_freight(
    state: &mut GameState,
    shaft_index: usize,
    car_index: usize,
    batch: i64,
    daypart: DaypartIdx,
) {
    let batch = batch.max(1);
    let (low, high) = (
        state.tower.shafts[shaft_index].low,
        state.tower.shafts[shaft_index].high,
    );
    let program = state.tower.shafts[shaft_index].program(daypart);
    let here = state.tower.shafts[shaft_index].cars[car_index].floor();

    let mut best: Option<(i64, FloorIdx, FloorIdx, ItemIdx, i64)> = None;

    for floor in low..=high {
        if !program.serves(floor) {
            continue;
        }
        let Some(source_floor) = state.tower.floor(floor) else {
            continue;
        };
        for room in &source_floor.rooms {
            for stack in &room.outputs {
                if stack.count <= 0 {
                    continue;
                }
                let Some((dest_floor, priority)) =
                    best_freight_destination(state, low, high, floor, stack.item, program)
                else {
                    continue;
                };
                let amount = stack.count.min(batch);
                // Prefer priority, then the shorter round trip.
                let score = priority * 1000
                    - i64::from(here.abs_diff(floor))
                    - i64::from(floor.abs_diff(dest_floor));
                if best.as_ref().is_none_or(|(s, ..)| score > *s) {
                    best = Some((score, floor, dest_floor, stack.item, amount));
                }
            }
        }
    }

    let Some((_, from, to, item, amount)) = best else {
        return;
    };

    // Not at the pickup floor yet: put it on the list and let the car's
    // own routing take it there. `dispatch` treats a non-empty `stops`
    // as reason enough to depart, so this is a call the car makes to
    // itself.
    if here != from {
        let car = &mut state.tower.shafts[shaft_index].cars[car_index];
        car.add_stop(from);
        return;
    }

    let taken = withdraw_from_floor(state, from, item, amount);
    if taken == 0 {
        return;
    }
    let car = &mut state.tower.shafts[shaft_index].cars[car_index];
    car.freight.push(Stack {
        item,
        count: taken,
        max: taken,
    });
    car.add_stop(to);
}

fn best_freight_destination(
    state: &GameState,
    low: FloorIdx,
    high: FloorIdx,
    from: FloorIdx,
    item: ItemIdx,
    program: &crate::state::ShaftProgram,
) -> Option<(FloorIdx, i64)> {
    let mut best: Option<(i64, i64, FloorIdx)> = None;
    for floor in low..=high {
        if floor == from || !program.serves(floor) {
            continue;
        }
        let Some(target) = state.tower.floor(floor) else {
            continue;
        };
        for room in &target.rooms {
            for stack in &room.inputs {
                if stack.item == item && stack.space() > 0 {
                    consider_dest(&mut best, 3, stack.space(), floor);
                }
            }
            let shelf_space = room.shelf_space_for(item);
            if shelf_space > 0 {
                consider_dest(&mut best, 2, shelf_space, floor);
            }
        }
    }
    best.map(|(priority, _, floor)| (floor, priority))
}

fn consider_dest(
    best: &mut Option<(i64, i64, FloorIdx)>,
    priority: i64,
    space: i64,
    floor: FloorIdx,
) {
    let candidate = (priority, space, floor);
    let better = match best {
        None => true,
        Some((bp, bs, bf)) => {
            (priority, space) > (*bp, *bs) || ((priority, space) == (*bp, *bs) && floor < *bf)
        }
    };
    if better {
        *best = Some(candidate);
    }
}

fn withdraw_from_floor(state: &mut GameState, floor: FloorIdx, item: ItemIdx, amount: i64) -> i64 {
    let Some(target) = state.tower.floor_mut(floor) else {
        return 0;
    };
    for room in &mut target.rooms {
        for stack in &mut room.outputs {
            if stack.item == item {
                let taken = stack.withdraw(amount);
                if taken > 0 {
                    return taken;
                }
            }
        }
    }
    0
}

/// Put down whatever the car was carrying for this floor.
///
/// Called from `service_stop`, so freight and riders get off at the
/// same stop — which is the merge (`SYSTEMS.md` §6.18) in one line: a
/// lift that is already stopping to open its doors puts the crate down
/// while it is there.
fn unload_freight(
    state: &mut GameState,
    shaft_index: usize,
    car_index: usize,
    sounds: &mut Vec<SoundEvent>,
) {
    let here = state.tower.shafts[shaft_index].cars[car_index].floor();
    let freight = std::mem::take(&mut state.tower.shafts[shaft_index].cars[car_index].freight);
    let mut delivered = 0u64;

    for load in freight {
        let mut remaining = load.count;
        if let Some(target) = state.tower.floor_mut(here) {
            // **Every inbox on the floor first, then the shelves.**
            //
            // This used to walk the rooms once and offer each room its
            // inbox *and* its shelves before moving on, so a storeroom
            // at a lower slot number swallowed the whole load before
            // the mill three slots along was ever asked. The car had
            // chosen this floor precisely because a hungry recipe was
            // on it — `best_freight_destination` scores an inbox 3
            // against a shelf's 2 — and then unloaded as though it had
            // not.
            //
            // Invisible until M6, because no floor in any fixture held
            // both a storeroom and a consumer; the moment one did, the
            // mill sat at 0 while 45 stalks went past it onto shelves.
            // Two passes cost nothing at this room count and make the
            // unload agree with the choice.
            for room in &mut target.rooms {
                for stack in &mut room.inputs {
                    if stack.item == load.item {
                        remaining -= stack.deposit(remaining);
                    }
                }
                if remaining == 0 {
                    break;
                }
            }
            if remaining > 0 {
                for room in &mut target.rooms {
                    remaining -= room.shelve(load.item, remaining);
                    if remaining == 0 {
                        break;
                    }
                }
            }
        }
        // Anything that did not fit stays aboard rather than vanishing;
        // the car will look for somewhere else to put it.
        if remaining > 0 {
            state.tower.shafts[shaft_index].cars[car_index]
                .freight
                .push(Stack {
                    item: load.item,
                    count: remaining,
                    max: remaining,
                });
        }
        if load.count > remaining {
            delivered += 1;
        }
    }

    if delivered > 0 {
        state.stats.hauls_completed += delivered;
        sounds.push(SoundEvent::Deliver);
    }
}

/// How long a trip up this shaft is likely to take, including the wait.
/// Used by `haul` to choose between shafts.
///
/// `queued` is how many crew are already waiting at this shaft. It
/// matters more than anything else in the estimate: a flat penalty for
/// "the stairs are occupied" is not enough, because five people queued
/// on a one-body staircase is five times the wait of one, and a crew
/// member who cannot see that will keep joining the longest line.
///
/// `load` is how many items are in their arms. **It has to be here and
/// not only in the climb**, or the estimate and the trip disagree: crew
/// would choose the staircase believing it cheap, then pay the laden
/// rate on it, and go on choosing it forever because nothing they can
/// see ever changes. Both sides read `climb_ticks_per_item`.
#[must_use]
pub fn estimated_trip_ticks(
    shaft: &crate::state::Shaft,
    content: &Content,
    from: FloorIdx,
    to: FloorIdx,
    queued: u32,
    load: u32,
) -> u32 {
    let floors = u32::from(from.abs_diff(to));
    let def = content.shaft(shaft.def);
    let balance = &content.balance.transport;

    match shaft.kind {
        // Nobody travels on a chute, so nothing should ever ask how long
        // it would take. `best_shaft` filters chutes out before this is
        // reached; the arm exists so that a future caller which forgets
        // to gets an answer that reads as "never" rather than a panic or
        // an accidental free ride.
        ShaftKind::Chute => u32::MAX,
        ShaftKind::Stairs => {
            let per_floor = content.balance.crew.climb_ticks_per_floor
                + load * content.balance.crew.climb_ticks_per_item;
            let climb = per_floor * floors;
            // Everyone ahead of you climbs before you do. Occupancy
            // costs one more body's worth on top.
            let ahead = queued + u32::from(!shaft.has_room());
            climb + ahead * climb.max(balance.queue_penalty_ticks)
        }
        ShaftKind::Elevator => {
            // How far the car has to come, plus the standing assumption
            // that you never catch it at the door.
            let approach = shaft
                .cars
                .iter()
                .map(|car| u32::from(car.floor().abs_diff(from)) * def.ticks_per_floor)
                .min()
                .unwrap_or(0);
            let trip = floors * def.ticks_per_floor + balance.dwell_base_ticks * 2;

            // A queue costs two different things and both matter. Every
            // person ahead of you lengthens the dwell whether or not
            // they fit; everyone beyond a carload makes you wait for the
            // car to come back. Without that second term a single
            // elevator would look infinitely scalable, and subtracting
            // capacity first — as this did — made a queue that exactly
            // filled the car cost nothing at all.
            let dwell = queued * balance.dwell_per_unit_ticks;
            let carloads_ahead = queued / u32::from(shaft.capacity.max(1));
            balance.elevator_base_wait_ticks + approach + trip + dwell + carloads_ahead * trip
        }
    }
}
