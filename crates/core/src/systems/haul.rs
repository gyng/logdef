//! Hauling — the crew move things, and the shafts decide how fast.
//!
//! This is the system the whole design rests on. In Factorio a belt
//! serves one lane forever; here every chain you add loads the same
//! stairs everybody else is using. A crew member's trip is an L: walk
//! to the shaft column, climb, walk to the destination. The shaft has a
//! capacity, so the second crew member waits — visibly, with a counter
//! the cross-section renders as stress.
//!
//! Two invariants worth stating because everything else assumes them:
//!
//! * **Nothing picked up is ever destroyed.** If a destination fills
//!   while a crew member is en route, they get re-tasked to somewhere
//!   else; if there is nowhere, they stand and hold it.
//! * **Two crew never chase the same crate.** Assignment subtracts what
//!   other crew have already committed to, at both ends of the trip.

use crate::content::{Content, ShaftKind};
use crate::fx::Fx;
use crate::ids::{DaypartIdx, FloorIdx, ItemIdx, RoomId, ShaftId, SlotIdx};
use crate::state::crew::HaulPickup;
use crate::state::{Crew, CrewState, GameState, HaulDestination, HaulTask, Tower};

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    // Move the crew out so each member can be advanced while the tower
    // is also borrowed mutably. `take` on a Vec is a pointer move.
    let mut crew = std::mem::take(&mut state.crew);
    let mut hauls = 0u64;
    let daypart = state.clock.daypart(content);
    let queues = shaft_queues(&crew, &state.tower);
    // Repair spends poles off the shelves, and the assignment pass runs
    // with the crew moved out of state, so the figure comes with it.
    let poles = super::repair::repair_item(content).map_or(0, |item| state.stock_of(item));

    for member in &mut crew {
        advance(
            member,
            &mut state.tower,
            content,
            &queues,
            daypart,
            sounds,
            &mut hauls,
        );
    }

    assign_idle(&mut crew, &state.tower, content, &queues, daypart, poles);

    state.crew = crew;
    state.stats.hauls_completed += hauls;
}

// ---------------------------------------------------------------------------
// Per-crew state machine
// ---------------------------------------------------------------------------

fn advance(
    crew: &mut Crew,
    tower: &mut Tower,
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
    sounds: &mut Vec<SoundEvent>,
    hauls: &mut u64,
) {
    let balance = &content.balance.crew;

    match crew.state {
        CrewState::Idle => {
            // Assignment happens in a second pass so every crew member
            // sees the same picture of demand.
        }

        CrewState::Riding { .. } => {
            // Cargo. The transport system moves them and opens the
            // doors; there is nothing for the crew member to do.
            crew.wait_ticks = 0;
        }

        CrewState::Repairing { .. } => {
            // The repair system owns them until the job is done.
            crew.wait_ticks = 0;
        }

        CrewState::Walking { to_slot } => {
            crew.wait_ticks = 0;
            let target = Fx::from_int(i32::from(to_slot));
            let step = Fx::ratio(1, balance.walk_ticks_per_slot.max(1) as i32);
            let arrived = if crew.slot_fx < target {
                crew.slot_fx += step;
                crew.slot_fx >= target
            } else {
                crew.slot_fx -= step;
                crew.slot_fx <= target
            };
            if arrived {
                crew.slot_fx = target;
                crew.state = resume(crew, tower, content, queues, daypart);
            }
        }

        CrewState::Boarding { shaft, to_floor } => {
            let kind = tower.shaft(shaft).map(|s| s.kind);
            // A repair job is a reason to be in the queue too. Checking
            // only for a haul task left a crew member sent to mend
            // something bouncing between Boarding and Idle forever,
            // never actually climbing.
            if crew.task.is_none() && crew.repair.is_none() {
                // Whatever they were headed for was demolished while
                // they queued. Step out of the line.
                crew.state = CrewState::Idle;
            } else if kind == Some(ShaftKind::Stairs) {
                if claim_shaft(tower, shaft) {
                    crew.wait_ticks = 0;
                    crew.state = CrewState::Climbing { shaft, to_floor };
                } else {
                    // The bottleneck, made visible. No dashboard needed.
                    crew.wait_ticks = crew.wait_ticks.saturating_add(1);
                }
            } else if kind.is_none() {
                // The shaft was demolished out from under them.
                crew.state = CrewState::Idle;
            } else {
                // Waiting for a car. Boarding is the transport system's
                // job; all that happens here is the wait accumulating,
                // which is what tints them red.
                crew.wait_ticks = crew.wait_ticks.saturating_add(1);
            }
        }

        CrewState::Climbing { shaft, to_floor } => {
            crew.wait_ticks = 0;
            let target = Fx::from_int(i32::from(to_floor));
            let step = Fx::ratio(1, balance.climb_ticks_per_floor.max(1) as i32);
            let arrived = if crew.floor_fx < target {
                crew.floor_fx += step;
                crew.floor_fx >= target
            } else {
                crew.floor_fx -= step;
                crew.floor_fx <= target
            };
            if arrived {
                crew.floor_fx = target;
                release_shaft(tower, shaft);
                crew.state = resume(crew, tower, content, queues, daypart);
            }
        }

        CrewState::Loading { ticks_left } => {
            crew.wait_ticks = 0;
            if ticks_left > 0 {
                crew.state = CrewState::Loading {
                    ticks_left: ticks_left - 1,
                };
                return;
            }
            if collect(crew, tower) {
                sounds.push(SoundEvent::Pickup);
                crew.state = next_leg(crew, tower, content, queues, daypart);
            } else {
                // Somebody else got there first, or the room emptied.
                crew.task = None;
                crew.state = CrewState::Idle;
            }
        }

        CrewState::Unloading { ticks_left } => {
            crew.wait_ticks = 0;
            if ticks_left > 0 {
                crew.state = CrewState::Unloading {
                    ticks_left: ticks_left - 1,
                };
                return;
            }
            let delivered = deposit(crew, tower);
            if delivered > 0 {
                *hauls += 1;
                sounds.push(SoundEvent::Deliver);
            }
            crew.task = None;
            crew.state = CrewState::Idle;
            // Anything that didn't fit stays in hand; the assignment
            // pass will find it a new home next tick.
        }
    }
}

/// What a crew member does after finishing a leg — which depends on
/// whether they are hauling or mending.
fn resume(
    crew: &Crew,
    tower: &Tower,
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
) -> CrewState {
    match crew.repair {
        Some(job) => repair_leg(crew, tower, content, queues, daypart, job),
        None => next_leg(crew, tower, content, queues, daypart),
    }
}

/// Decide the next leg for a crew member who just finished one.
fn next_leg(
    crew: &Crew,
    tower: &Tower,
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
) -> CrewState {
    let balance = &content.balance.crew;
    let Some(task) = &crew.task else {
        return CrewState::Idle;
    };

    let (target_floor, target_slot) = if crew.is_carrying() {
        (task.to_floor, task.to_slot)
    } else {
        match task.pickup {
            Some(pickup) => (pickup.floor, pickup.slot),
            None => (task.to_floor, task.to_slot),
        }
    };

    let floor = crew.floor();
    let slot = crew.slot();

    if floor == target_floor {
        if slot == target_slot {
            return if crew.is_carrying() {
                CrewState::Unloading {
                    ticks_left: balance.unload_ticks,
                }
            } else {
                CrewState::Loading {
                    ticks_left: balance.load_ticks,
                }
            };
        }
        return CrewState::Walking {
            to_slot: target_slot,
        };
    }

    let Some(shaft) = best_shaft(tower, content, queues, daypart, floor, target_floor) else {
        // Nothing spans this trip. The assignment pass filters for
        // reachability, so this is belt and braces.
        return CrewState::Idle;
    };
    let column = tower.shaft(shaft).map_or(0, |s| s.slot);
    if slot != column {
        return CrewState::Walking { to_slot: column };
    }
    CrewState::Boarding {
        shaft,
        to_floor: target_floor,
    }
}

/// Pick the shaft that gets this crew member up fastest.
///
/// The estimate does not have to be right — it has to be deterministic
/// and roughly sensible, so that a player who builds an elevator sees
/// the crew start using it, and a player whose staircase is jammed sees
/// them route around it.
fn best_shaft(
    tower: &Tower,
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
    from: FloorIdx,
    to: FloorIdx,
) -> Option<ShaftId> {
    tower
        .shafts
        .iter()
        .enumerate()
        // Crew cannot ride a dumbwaiter, however convenient it looks.
        .filter(|(_, shaft)| shaft.kind != ShaftKind::Dumbwaiter)
        .filter(|(_, shaft)| shaft.serves_trip(from, to, daypart))
        .min_by_key(|(index, shaft)| {
            let queued = queues.get(*index).copied().unwrap_or(0);
            (
                super::transport::estimated_trip_ticks(shaft, content, from, to, queued),
                // Stable tie-break, so two equal shafts don't flap.
                shaft.id.0,
            )
        })
        .map(|(_, shaft)| shaft.id)
}

/// How many crew are queued at each shaft, parallel to `tower.shafts`.
///
/// Computed once at the top of the tick and handed down, because a crew
/// member choosing a route needs to see the whole queue picture while
/// the borrow checker only lets them see themselves. Last tick's
/// picture is fine — and deterministic, which matters more.
fn shaft_queues(crew: &[Crew], tower: &Tower) -> Vec<u32> {
    tower
        .shafts
        .iter()
        .map(|shaft| {
            crew.iter()
                .filter(|member| {
                    matches!(member.state, CrewState::Boarding { shaft: at, .. } if at == shaft.id)
                })
                .count() as u32
        })
        .collect()
}

fn claim_shaft(tower: &mut Tower, id: ShaftId) -> bool {
    let Some(shaft) = tower.shaft_mut(id) else {
        return false;
    };
    if !shaft.has_room() {
        return false;
    }
    shaft.riders += 1;
    true
}

fn release_shaft(tower: &mut Tower, id: ShaftId) {
    if let Some(shaft) = tower.shaft_mut(id) {
        shaft.riders = shaft.riders.saturating_sub(1);
    }
}

/// Take the load out of the source room. False if it's no longer there.
fn collect(crew: &mut Crew, tower: &mut Tower) -> bool {
    let Some(task) = &crew.task else {
        return false;
    };
    let Some(pickup) = task.pickup else {
        return false;
    };
    let (item, wanted) = (task.item, task.amount);

    let Some(floor) = tower.floor_mut(pickup.floor) else {
        return false;
    };
    let Some(room) = floor.rooms.iter_mut().find(|r| r.id == pickup.room) else {
        return false;
    };
    let Some(stack) = room.outputs.iter_mut().find(|s| s.item == item) else {
        return false;
    };
    let taken = stack.withdraw(wanted);
    if taken == 0 {
        return false;
    }
    crew.carrying = Some((item, taken));
    true
}

/// Put the load down. Returns how many items landed.
fn deposit(crew: &mut Crew, tower: &mut Tower) -> i64 {
    let Some((item, held)) = crew.carrying else {
        return 0;
    };
    let Some(task) = &crew.task else {
        return 0;
    };

    let placed = match task.destination {
        HaulDestination::Inbox { room, input } => {
            deposit_inbox(tower, task.to_floor, room, input as usize, item, held)
        }
        HaulDestination::Shelf { room } => deposit_shelf(tower, task.to_floor, room, item, held),
    };

    let left = held - placed;
    crew.carrying = if left > 0 { Some((item, left)) } else { None };
    placed
}

fn deposit_inbox(
    tower: &mut Tower,
    floor: FloorIdx,
    room_id: RoomId,
    input: usize,
    item: ItemIdx,
    amount: i64,
) -> i64 {
    let Some(floor) = tower.floor_mut(floor) else {
        return 0;
    };
    let Some(room) = floor.rooms.iter_mut().find(|r| r.id == room_id) else {
        return 0;
    };
    let Some(stack) = room.inputs.get_mut(input) else {
        return 0;
    };
    if stack.item != item {
        return 0;
    }
    stack.deposit(amount)
}

fn deposit_shelf(
    tower: &mut Tower,
    floor: FloorIdx,
    room_id: RoomId,
    item: ItemIdx,
    amount: i64,
) -> i64 {
    let Some(floor) = tower.floor_mut(floor) else {
        return 0;
    };
    let Some(room) = floor.rooms.iter_mut().find(|r| r.id == room_id) else {
        return 0;
    };
    room.shelve(item, amount)
}

// ---------------------------------------------------------------------------
// Task assignment
// ---------------------------------------------------------------------------

/// Priority of feeding a live recipe. Beats stockpiling, always.
const PRIORITY_INBOX: i64 = 3;
/// Priority of putting something on a shelf.
const PRIORITY_SHELF: i64 = 2;

fn assign_idle(
    crew: &mut [Crew],
    tower: &Tower,
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
    poles: i64,
) {
    for i in 0..crew.len() {
        if !matches!(crew[i].state, CrewState::Idle) {
            continue;
        }

        // Idle but still committed: they have just stepped off a car
        // part-way through a journey. Pick the trip back up rather than
        // re-deciding it, or a crew member could ride an elevator and
        // then immediately choose a different errand.
        if crew[i].task.is_some() {
            let next = next_leg(&crew[i], tower, content, queues, daypart);
            crew[i].state = next;
            continue;
        }

        // On the way to damage, or standing on it.
        if let Some(job) = crew[i].repair {
            crew[i].state = repair_leg(&crew[i], tower, content, queues, daypart, job);
            continue;
        }

        // Damage outranks a new errand. A crew member already holding
        // something finishes that first — putting a load down where it
        // does not belong to go and mend a wall would lose the load.
        if !crew[i].is_carrying()
            && let Some((target, floor, slot)) =
                super::repair::pick_repair(tower, content, poles, crew, i, crew[i].floor())
        {
            let job = crate::state::RepairJob {
                target,
                floor,
                slot,
            };
            crew[i].repair = Some(job);
            crew[i].wait_ticks = 0;
            crew[i].state = repair_leg(&crew[i], tower, content, queues, daypart, job);
            continue;
        }

        let task = if let Some((item, held)) = crew[i].carrying {
            // Already holding something: find it a home rather than
            // picking up more. Nothing is ever dropped on the floor.
            find_destination(tower, content, crew, i, item, held).map(
                |(destination, to_floor, to_slot, _)| HaulTask {
                    item,
                    amount: held,
                    pickup: None,
                    to_floor,
                    to_slot,
                    destination,
                },
            )
        } else {
            pick_task(tower, content, crew, i, queues, daypart)
        };

        let Some(task) = task else {
            // Nothing to do is not stress. `wait_ticks` drives the red
            // tint, and a crew member standing around because the
            // tower has no work is telling the player something quite
            // different from one stuck at the foot of a jammed
            // staircase — conflating them makes the only bottleneck
            // instrument in the game lie.
            crew[i].wait_ticks = 0;
            continue;
        };
        crew[i].wait_ticks = 0;
        crew[i].task = Some(task);
        let next = next_leg(&crew[i], tower, content, queues, daypart);
        crew[i].state = next;
    }
}

/// Route a crew member to the damage they have been assigned, then set
/// them working. Reuses the haul legs exactly — which is why a severed
/// shaft can put damage out of reach, and why that is correct rather
/// than a bug.
fn repair_leg(
    crew: &Crew,
    tower: &Tower,
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
    job: crate::state::RepairJob,
) -> CrewState {
    let floor = crew.floor();
    let slot = crew.slot();

    if floor == job.floor {
        if slot == job.slot {
            return CrewState::Repairing {
                target: job.target,
                ticks_left: super::repair::shift_ticks(
                    content,
                    content.balance.siege.repair_hp_per_shift,
                ),
            };
        }
        return CrewState::Walking { to_slot: job.slot };
    }

    let Some(shaft) = best_shaft(tower, content, queues, daypart, floor, job.floor) else {
        // Cut off from the damage. Stand down rather than spin; the
        // assignment pass will try again once a route exists.
        return CrewState::Idle;
    };
    let column = tower.shaft(shaft).map_or(0, |s| s.slot);
    if slot != column {
        return CrewState::Walking { to_slot: column };
    }
    CrewState::Boarding {
        shaft,
        to_floor: job.floor,
    }
}

/// Score every collectable pile against every valid destination and
/// take the best. Ties break on the lowest (pickup, dropoff) position
/// so the choice is a pure function of state.
fn pick_task(
    tower: &Tower,
    content: &Content,
    crew: &[Crew],
    me: usize,
    queues: &[u32],
    daypart: DaypartIdx,
) -> Option<HaulTask> {
    let capacity = content.balance.crew.carry_capacity.max(1);
    let from_floor = crew[me].floor();
    let from_slot = crew[me].slot();

    let mut best: Option<(i64, HaulTask)> = None;

    for floor in &tower.floors {
        for room in &floor.rooms {
            for stack in &room.outputs {
                let committed = committed_pickup(crew, me, room.id, stack.item);
                let available = stack.count - committed;
                if available <= 0 {
                    continue;
                }
                if from_floor != floor.index
                    && best_shaft(tower, content, queues, daypart, from_floor, floor.index)
                        .is_none()
                {
                    continue;
                }

                let Some((destination, to_floor, to_slot, priority)) = find_destination(
                    tower,
                    content,
                    crew,
                    me,
                    stack.item,
                    available.min(capacity),
                ) else {
                    continue;
                };
                if floor.index != to_floor
                    && best_shaft(tower, content, queues, daypart, floor.index, to_floor).is_none()
                {
                    continue;
                }

                let amount = available.min(capacity);
                let travel = travel_cost(
                    from_floor,
                    from_slot,
                    floor.index,
                    room.outbox_slot(),
                    to_floor,
                    to_slot,
                );
                let score = priority * 1000 - travel;

                let task = HaulTask {
                    item: stack.item,
                    amount,
                    pickup: Some(HaulPickup {
                        room: room.id,
                        floor: floor.index,
                        slot: room.outbox_slot(),
                    }),
                    to_floor,
                    to_slot,
                    destination,
                };

                let better = match &best {
                    None => true,
                    Some((best_score, best_task)) => {
                        score > *best_score || (score == *best_score && tie_break(&task, best_task))
                    }
                };
                if better {
                    best = Some((score, task));
                }
            }
        }
    }

    best.map(|(_, task)| task)
}

/// Deterministic tie-break: lowest pickup floor, then slot, then
/// dropoff floor, then slot.
fn tie_break(candidate: &HaulTask, incumbent: &HaulTask) -> bool {
    let key = |t: &HaulTask| {
        let p = t.pickup.map_or((u8::MAX, u8::MAX), |p| (p.floor, p.slot));
        (p.0, p.1, t.to_floor, t.to_slot, t.item.0)
    };
    key(candidate) < key(incumbent)
}

/// Manhattan-ish cost: floors are twice as expensive as slots, because
/// climbing is slower and contends for a shared shaft.
fn travel_cost(
    from_floor: FloorIdx,
    from_slot: SlotIdx,
    pick_floor: FloorIdx,
    pick_slot: SlotIdx,
    to_floor: FloorIdx,
    to_slot: SlotIdx,
) -> i64 {
    let df = |a: FloorIdx, b: FloorIdx| i64::from(a.abs_diff(b));
    let ds = |a: SlotIdx, b: SlotIdx| i64::from(a.abs_diff(b));
    2 * (df(from_floor, pick_floor) + df(pick_floor, to_floor))
        + ds(from_slot, pick_slot)
        + ds(pick_slot, to_slot)
}

/// Best home for `amount` of `item`: a hungry recipe first, a shelf
/// second. Returns the destination plus its priority.
fn find_destination(
    tower: &Tower,
    content: &Content,
    crew: &[Crew],
    me: usize,
    item: ItemIdx,
    amount: i64,
) -> Option<(HaulDestination, FloorIdx, SlotIdx, i64)> {
    let mut best: Option<(i64, i64, HaulDestination, FloorIdx, SlotIdx)> = None;

    for floor in &tower.floors {
        for room in &floor.rooms {
            // 1. A recipe that eats this item and has buffer space.
            for (index, stack) in room.inputs.iter().enumerate() {
                if stack.item != item {
                    continue;
                }
                let destination = HaulDestination::Inbox {
                    room: room.id,
                    input: index as u8,
                };
                let inbound = committed_delivery(crew, me, destination, item);
                let space = stack.space() - inbound;
                if space < amount.min(1) || space <= 0 {
                    continue;
                }
                consider(
                    &mut best,
                    PRIORITY_INBOX,
                    space,
                    destination,
                    floor.index,
                    room.slot,
                );
            }

            // 2. Shelf space.
            if !room.shelves.is_empty() {
                let destination = HaulDestination::Shelf { room: room.id };
                let inbound = committed_delivery(crew, me, destination, item);
                let space = room.shelf_space_for(item) - inbound;
                if space > 0 {
                    consider(
                        &mut best,
                        PRIORITY_SHELF,
                        space,
                        destination,
                        floor.index,
                        room.slot,
                    );
                }
            }
        }
    }

    let _ = content;
    best.map(|(priority, _, destination, floor, slot)| (destination, floor, slot, priority))
}

/// Keep the highest-priority destination; within a priority, the
/// emptiest; ties by lowest position.
fn consider(
    best: &mut Option<(i64, i64, HaulDestination, FloorIdx, SlotIdx)>,
    priority: i64,
    space: i64,
    destination: HaulDestination,
    floor: FloorIdx,
    slot: SlotIdx,
) {
    let candidate = (priority, space, destination, floor, slot);
    let better = match best {
        None => true,
        Some((bp, bs, _, bf, bsl)) => {
            (priority, space) > (*bp, *bs)
                || ((priority, space) == (*bp, *bs) && (floor, slot) < (*bf, *bsl))
        }
    };
    if better {
        *best = Some(candidate);
    }
}

/// How much of `item` other crew have already promised to take out of
/// `room`. Prevents two crew chasing the same crate.
///
/// Crew who have already collected don't count: their task still names
/// the source room, but the items are in their hands and out of the
/// stack. Counting them would keep the pile looking spoken-for long
/// after it refilled.
fn committed_pickup(crew: &[Crew], me: usize, room: RoomId, item: ItemIdx) -> i64 {
    crew.iter()
        .enumerate()
        .filter(|(i, other)| *i != me && !other.is_carrying())
        .filter_map(|(_, other)| other.task.as_ref())
        .filter(|task| task.item == item)
        .filter_map(|task| task.pickup.map(|p| (p.room, task.amount)))
        .filter(|(pickup_room, _)| *pickup_room == room)
        .map(|(_, amount)| amount)
        .sum()
}

/// How much of `item` is already inbound to `destination`. Prevents
/// overfilling a buffer that two crew both targeted.
fn committed_delivery(
    crew: &[Crew],
    me: usize,
    destination: HaulDestination,
    item: ItemIdx,
) -> i64 {
    crew.iter()
        .enumerate()
        .filter(|(i, _)| *i != me)
        .filter_map(|(_, other)| other.task.as_ref())
        .filter(|task| task.item == item && task.destination == destination)
        .map(|task| task.amount)
        .sum()
}
