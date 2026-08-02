//! Runner-driven supply chain.
//!
//! Each tick we advance every runner through its state machine
//! and greedily assign demands to idle runners.
//!
//! Demand priority (highest first):
//!   1. A crafter's starving input buffer (e.g. Fletcher needs wood).
//!      Feeding active production beats every other destination — a
//!      stalled chain is a player-visible failure.
//!   2. Empty depot cache slot matching the resource. Caches are how
//!      ammo reaches racks; cache placement is the player's main
//!      logistics decision.
//!   3. Warehouse storage as a last resort (overflow).
//!
//! Once a cache is filled, the rack on the same floor pulls from it
//! instantly (the rack is physically attached to the balcony — no runner
//! needed). Without a cache, racks stay empty.
//!
//! Movement is **L-shaped**: every facility lives at a (floor, slot)
//! position, and a runner getting from A to B walks horizontally on
//! its current floor to a transport column, climbs the column to the
//! destination floor, then walks horizontally to the destination slot.
//! Each leg is a separate Moving state. Picking the right transport
//! column close to your busy facilities is the SimTower-style
//! positioning game the player plays.

use crate::engine::weapon_resource;
use crate::registry::Registry;
use crate::snapshot::SoundEvent;
use crate::state::*;
use crate::types::{Scalar, TransportId};

use super::FLOOR_HEIGHT_PX;

/// Visible width of one slot in pixels (mirrored on the frontend).
/// Combined with `runner.speed` (px/sec) this gives a per-slot walk
/// time. SLOT_PX = 40 + speed 50 = 0.8s per slot, so walking the
/// full 8-wide floor costs 6.4s — meaningful but snappy.
pub const SLOT_PX: Scalar = 40.0;

/// Seconds spent loading a crate at the source.
const LOAD_TIME: Scalar = 0.5;

/// Seconds spent unloading a crate at the destination.
const UNLOAD_TIME: Scalar = 0.5;

/// Sentinel for "no transport claimed" — used by horizontal walks
/// (no transport involved) and by queued runners waiting for a
/// segment to free up.
const NO_TRANSPORT: TransportId = TransportId(0);

pub fn run(state: &mut GameState, registry: &Registry, dt: Scalar, sounds: &mut Vec<SoundEvent>) {
    if state.encounter.is_none() && state.drill.is_none() {
        return;
    }

    advance_runners(state, dt, sounds);
    assign_idle_runners(state);
    drain_caches_to_racks(state, sounds);
    auto_pull_hero_ammo(state, registry);
}

// ---------------------------------------------------------------------------
// Runner state machine
// ---------------------------------------------------------------------------

fn advance_runners(state: &mut GameState, dt: Scalar, sounds: &mut Vec<SoundEvent>) {
    let runner_count = state.tower.runners.len();
    for i in 0..runner_count {
        // Snapshot the fields we need to mutate; we'll write them back
        // at the end. This sidesteps the dual mutable borrow between
        // `runners[i]` and the rest of `state.tower`.
        let mut current_state = state.tower.runners[i].state.clone();
        let mut carried = state.tower.runners[i].carried.clone();
        let mut task = state.tower.runners[i].task.clone();
        let mut current_floor = state.tower.runners[i].current_floor;
        let mut current_slot = state.tower.runners[i].current_slot;
        let runner_base_speed = state.tower.runners[i].speed.max(1.0);

        match current_state.clone() {
            RunnerState::Idle { .. } => {
                // Task assignment happens in a separate pass so all
                // runners see a consistent demand picture.
            }

            RunnerState::Moving {
                from,
                to,
                progress,
                via,
                from_slot,
                to_slot,
            } => {
                let new_progress = progress
                    + dt / leg_duration(
                        &state.tower,
                        from,
                        to,
                        from_slot,
                        to_slot,
                        via,
                        runner_base_speed,
                    );
                if new_progress >= 1.0 {
                    // Leg complete. Update runner position, release
                    // any transport reservation, and decide what's
                    // next: another leg, or a load/unload at target.
                    current_floor = to;
                    current_slot = to_slot;
                    if via != NO_TRANSPORT {
                        release_transport(&mut state.tower, via);
                    }
                    current_state = decide_next(
                        &mut state.tower,
                        &task,
                        carried.is_some(),
                        current_floor,
                        current_slot,
                    );
                } else {
                    current_state = RunnerState::Moving {
                        from,
                        to,
                        progress: new_progress,
                        via,
                        from_slot,
                        to_slot,
                    };
                }
            }

            RunnerState::Loading { at_floor, timer } => {
                let new_timer = timer - dt;
                if new_timer > 0.0 {
                    current_state = RunnerState::Loading {
                        at_floor,
                        timer: new_timer,
                    };
                } else if let Some(t) = task.as_ref() {
                    if take_crate_from_building(&mut state.tower, at_floor, t.resource) {
                        carried = Some(Crate {
                            resource: t.resource,
                        });
                        sounds.push(SoundEvent::RunnerPickup);
                        current_state =
                            decide_next(&mut state.tower, &task, true, current_floor, current_slot);
                    } else {
                        // Source dried up before we could pick up.
                        task = None;
                        current_state = RunnerState::Idle { at_floor };
                    }
                } else {
                    current_state = RunnerState::Idle { at_floor };
                }
            }

            RunnerState::Unloading { at_floor, timer } => {
                let new_timer = timer - dt;
                if new_timer > 0.0 {
                    current_state = RunnerState::Unloading {
                        at_floor,
                        timer: new_timer,
                    };
                } else {
                    if let (Some(t), Some(crate_)) = (task.as_ref(), carried.as_ref()) {
                        deliver_crate(&mut state.tower, &t.destination, crate_);
                        state.deliveries_completed = state.deliveries_completed.saturating_add(1);
                        sounds.push(SoundEvent::RunnerDeliver);
                    }
                    carried = None;
                    task = None;
                    current_state = RunnerState::Idle { at_floor };
                }
            }

            RunnerState::Queued { .. } => {
                // Re-attempt the next leg toward whatever target the
                // task currently demands. Stays Queued if still
                // blocked.
                current_state = decide_next(
                    &mut state.tower,
                    &task,
                    carried.is_some(),
                    current_floor,
                    current_slot,
                );
            }
        }

        // Write back the mutated runner snapshot.
        state.tower.runners[i].state = current_state;
        state.tower.runners[i].carried = carried;
        state.tower.runners[i].task = task;
        state.tower.runners[i].current_floor = current_floor;
        state.tower.runners[i].current_slot = current_slot;
    }
}

/// Time in seconds for one leg given runner speed.
fn leg_duration(
    tower: &Tower,
    from_floor: usize,
    to_floor: usize,
    from_slot: u8,
    to_slot: u8,
    via: TransportId,
    runner_speed: Scalar,
) -> Scalar {
    if from_floor == to_floor {
        // Horizontal walk on a single floor.
        let dx = (to_slot as i32 - from_slot as i32).unsigned_abs() as Scalar;
        ((dx * SLOT_PX) / runner_speed).max(0.01)
    } else {
        // Vertical climb via the claimed transport.
        let speed_mul = tower
            .transports
            .iter()
            .find(|t| t.id == via)
            .map(|t| t.speed_mul)
            .unwrap_or(1.0);
        let dy = (to_floor as i32 - from_floor as i32).unsigned_abs() as Scalar;
        ((dy * FLOOR_HEIGHT_PX) / (runner_speed * speed_mul)).max(0.01)
    }
}

/// Given a runner's current position and its task, decide what to
/// do next: walk a leg, climb a leg, load, unload, or wait queued.
fn decide_next(
    tower: &mut Tower,
    task: &Option<RunnerTask>,
    carrying: bool,
    cur_floor: usize,
    cur_slot: u8,
) -> RunnerState {
    let Some(t) = task else {
        return RunnerState::Idle {
            at_floor: cur_floor,
        };
    };
    let (target_floor, target_slot) = if carrying {
        (t.dropoff_floor, t.dropoff_slot)
    } else {
        (t.pickup_floor, t.pickup_slot)
    };

    // Already at target — load or unload.
    if cur_floor == target_floor && cur_slot == target_slot {
        return if carrying {
            RunnerState::Unloading {
                at_floor: cur_floor,
                timer: UNLOAD_TIME,
            }
        } else {
            RunnerState::Loading {
                at_floor: cur_floor,
                timer: LOAD_TIME,
            }
        };
    }

    // Same floor — single horizontal walk.
    if cur_floor == target_floor {
        return RunnerState::Moving {
            from: cur_floor,
            to: cur_floor,
            progress: 0.0,
            via: NO_TRANSPORT,
            from_slot: cur_slot,
            to_slot: target_slot,
        };
    }

    // Different floors — pick a transport whose column we can reach.
    // We prefer the closest column to where we're standing today, so
    // a long horizontal walk to a far transport doesn't dwarf the
    // vertical climb.
    let Some(via) = best_transport_for(tower, cur_floor, target_floor) else {
        // No transport spans this trip — stuck queued waiting for
        // the player to build infrastructure.
        return RunnerState::Queued {
            at_transport: NO_TRANSPORT,
            position_in_queue: 0,
        };
    };
    let column_slot = tower
        .transports
        .iter()
        .find(|t| t.id == via)
        .map(|t| t.slot)
        .unwrap_or(0);

    // First, walk horizontally to the column on our current floor.
    if cur_slot != column_slot {
        return RunnerState::Moving {
            from: cur_floor,
            to: cur_floor,
            progress: 0.0,
            via: NO_TRANSPORT,
            from_slot: cur_slot,
            to_slot: column_slot,
        };
    }

    // We're at the column. Try to claim a slot on the segment.
    if !claim_transport_id(tower, via) {
        return RunnerState::Queued {
            at_transport: via,
            position_in_queue: 0,
        };
    }

    RunnerState::Moving {
        from: cur_floor,
        to: target_floor,
        progress: 0.0,
        via,
        from_slot: column_slot,
        to_slot: column_slot,
    }
}

/// Pick the best transport whose floor range covers `[from, to]`.
/// Today: highest speed_mul wins. Tie-break: earliest in vec.
fn best_transport_for(tower: &Tower, from: usize, to: usize) -> Option<TransportId> {
    let going_down = to < from;
    let low = from.min(to);
    let high = from.max(to);
    tower
        .transports
        .iter()
        .filter(|t| t.low_floor <= low && t.high_floor >= high)
        .filter(|t| match t.direction {
            TransportDirection::Both => true,
            TransportDirection::DownOnly => going_down,
            TransportDirection::UpOnly => !going_down,
        })
        .max_by(|a, b| a.speed_mul.partial_cmp(&b.speed_mul).unwrap())
        .map(|t| t.id)
}

/// Reserve a slot on a specific transport segment by id. Returns
/// false if the segment is already at capacity.
fn claim_transport_id(tower: &mut Tower, id: TransportId) -> bool {
    let Some(t) = tower.transports.iter_mut().find(|t| t.id == id) else {
        return false;
    };
    if t.occupancy >= t.capacity {
        return false;
    }
    t.occupancy = t.occupancy.saturating_add(1);
    true
}

/// Release the runner's hold on a transport segment.
fn release_transport(tower: &mut Tower, id: TransportId) {
    if let Some(t) = tower.transports.iter_mut().find(|t| t.id == id) {
        t.occupancy = t.occupancy.saturating_sub(1);
    }
}

// ---------------------------------------------------------------------------
// Demand scanning + task assignment
// ---------------------------------------------------------------------------

fn assign_idle_runners(state: &mut GameState) {
    let runner_count = state.tower.runners.len();
    for i in 0..runner_count {
        let RunnerState::Idle { at_floor } = state.tower.runners[i].state else {
            continue;
        };
        let cur_slot = state.tower.runners[i].current_slot;
        let Some(task) = pick_best_task(&state.tower, at_floor) else {
            continue;
        };
        state.tower.runners[i].task = Some(task.clone());
        let next_state = decide_next(&mut state.tower, &Some(task), false, at_floor, cur_slot);
        state.tower.runners[i].state = next_state;
    }
}

fn pick_best_task(tower: &Tower, runner_floor: usize) -> Option<RunnerTask> {
    let mut best: Option<(i64, RunnerTask)> = None;

    for floor in &tower.floors {
        let Some(building) = &floor.building else {
            continue;
        };
        if building.output_buffer.current == 0 {
            continue;
        }
        let resource = building.output_buffer.resource;

        let Some((destination, dropoff_floor, dropoff_slot, priority)) =
            find_best_destination(tower, resource)
        else {
            continue;
        };

        // Score = priority (large) - distance (small). Higher = better.
        let distance = ((floor.index as i32 - runner_floor as i32).unsigned_abs() as i64)
            + ((dropoff_floor as i32 - floor.index as i32).unsigned_abs() as i64);
        let score = (priority as i64) * 100 - distance;
        let pickup_slot = building_outbox_slot(building);
        let task = RunnerTask {
            pickup_floor: floor.index,
            dropoff_floor,
            resource,
            destination,
            pickup_slot,
            dropoff_slot,
        };
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, task));
        }
    }

    best.map(|(_, task)| task)
}

/// Slot the runner stops at when picking up from a building. Use
/// the right edge of the building (outbox side) so left-anchored
/// transport columns get a horizontal walk across the building's
/// width.
fn building_outbox_slot(b: &Building) -> u8 {
    b.slot.saturating_add(b.width_slots).saturating_sub(1)
}

/// Returns (destination, dropoff_floor, dropoff_slot, priority).
/// Higher priority is preferred. Crafter inboxes beat caches beat
/// warehouse fallback.
fn find_best_destination(
    tower: &Tower,
    resource: ResourceType,
) -> Option<(DeliveryDestination, usize, u8, u32)> {
    // 1. Most starved crafter inbox that consumes this resource.
    let mut best_inbox: Option<(usize, u8, u32)> = None;
    for floor in &tower.floors {
        let Some(building) = &floor.building else {
            continue;
        };
        if !building.is_active {
            continue;
        }
        let Some(slot) = building
            .input_buffers
            .iter()
            .find(|inb| inb.resource == resource && inb.current < inb.max)
        else {
            continue;
        };
        let space = slot.max.saturating_sub(slot.current);
        if best_inbox.as_ref().is_none_or(|(_, _, s)| space > *s) {
            best_inbox = Some((floor.index, building.slot, space));
        }
    }
    if let Some((floor, slot, _)) = best_inbox {
        return Some((DeliveryDestination::Inbox { floor }, floor, slot, 3));
    }

    // 2. Most starved cache slot for this resource.
    let mut best_cache: Option<(usize, u8, u32)> = None;
    for floor in &tower.floors {
        let Some(cache) = &floor.cache else {
            continue;
        };
        let Some(slot) = cache
            .slots
            .iter()
            .find(|s| s.resource == resource && s.current < s.max)
        else {
            continue;
        };
        let cache_space = slot.max.saturating_sub(slot.current);
        let rack_hunger = tower
            .balconies
            .iter()
            .filter(|b| b.floor == floor.index && !b.rack.destroyed && b.rack.resource == resource)
            .map(|b| b.rack.max.saturating_sub(b.rack.current))
            .sum::<u32>();
        let score = cache_space + rack_hunger * 2;
        if best_cache.as_ref().is_none_or(|(_, _, s)| score > *s) {
            best_cache = Some((floor.index, cache.slot, score));
        }
    }
    if let Some((floor, slot, _)) = best_cache {
        return Some((DeliveryDestination::Cache { floor }, floor, slot, 2));
    }

    // 3. Warehouse fallback — always accepts. Drop at the leftmost
    //    slot on F0 by convention so it's near the depot strip.
    Some((DeliveryDestination::Warehouse, 0, 0, 1))
}

/// Pull crates from each cache into the rack on the same floor.
fn drain_caches_to_racks(state: &mut GameState, sounds: &mut Vec<SoundEvent>) {
    for balcony in &mut state.tower.balconies {
        if balcony.rack.destroyed {
            continue;
        }
        let Some(floor) = state.tower.floors.get_mut(balcony.floor) else {
            continue;
        };
        let Some(cache) = floor.cache.as_mut() else {
            continue;
        };
        let Some(slot) = cache
            .slots
            .iter_mut()
            .find(|s| s.resource == balcony.rack.resource)
        else {
            continue;
        };
        let needed = balcony.rack.max.saturating_sub(balcony.rack.current);
        let moved = needed.min(slot.current);
        if moved > 0 {
            slot.current -= moved;
            balcony.rack.current += moved;
            sounds.push(SoundEvent::RunnerDeliver);
        }
    }
}

// ---------------------------------------------------------------------------
// Crate transfer helpers
// ---------------------------------------------------------------------------

fn take_crate_from_building(tower: &mut Tower, floor_idx: usize, resource: ResourceType) -> bool {
    let Some(floor) = tower.floors.get_mut(floor_idx) else {
        return false;
    };
    let Some(building) = &mut floor.building else {
        return false;
    };
    if building.output_buffer.resource != resource || building.output_buffer.current == 0 {
        return false;
    }
    building.output_buffer.current -= 1;
    true
}

fn deliver_crate(tower: &mut Tower, destination: &DeliveryDestination, crate_: &Crate) {
    match destination {
        DeliveryDestination::Inbox { floor } => {
            if let Some(floor) = tower.floors.get_mut(*floor)
                && let Some(building) = floor.building.as_mut()
                && let Some(inb) = building
                    .input_buffers
                    .iter_mut()
                    .find(|inb| inb.resource == crate_.resource && inb.current < inb.max)
            {
                inb.current += 1;
                return;
            }
            store_in_warehouse(tower, crate_);
        }
        DeliveryDestination::Rack {
            balcony: balcony_id,
        } => {
            if let Some(balcony) = tower.balconies.iter_mut().find(|b| b.id == *balcony_id)
                && balcony.rack.resource == crate_.resource
                && !balcony.rack.destroyed
                && balcony.rack.current < balcony.rack.max
            {
                balcony.rack.current += 1;
                return;
            }
            store_in_warehouse(tower, crate_);
        }
        DeliveryDestination::Cache { floor } => {
            if let Some(floor) = tower.floors.get_mut(*floor)
                && let Some(cache) = floor.cache.as_mut()
                && let Some(slot) = cache
                    .slots
                    .iter_mut()
                    .find(|s| s.resource == crate_.resource && s.current < s.max)
            {
                slot.current += 1;
                return;
            }
            store_in_warehouse(tower, crate_);
        }
        DeliveryDestination::Warehouse => store_in_warehouse(tower, crate_),
    }
}

fn store_in_warehouse(tower: &mut Tower, crate_: &Crate) {
    let capacity = tower.warehouse.capacity_per_slot;
    if let Some(slot) = tower
        .warehouse
        .slots
        .iter_mut()
        .find(|s| s.resource == crate_.resource && s.current < s.max)
    {
        slot.current += 1;
        return;
    }
    tower.warehouse.slots.push(ResourceBuffer {
        resource: crate_.resource,
        current: 1,
        max: capacity,
    });
}

// ---------------------------------------------------------------------------
// Hero quiver auto-pull
// ---------------------------------------------------------------------------

fn auto_pull_hero_ammo(state: &mut GameState, registry: &Registry) {
    let Some(hero_resource) = weapon_resource(state.tower.hero.weapon_primary.base_type) else {
        return;
    };
    let desired = registry.balance.hero.personal_ammo;
    let needed = desired.saturating_sub(state.tower.hero.personal_ammo);
    if needed == 0 {
        return;
    }

    let hero_position = state.tower.hero.position;
    let Some(balcony) = state
        .tower
        .balconies
        .iter_mut()
        .find(|b| b.id == hero_position)
    else {
        return;
    };
    if balcony.rack.resource != hero_resource {
        balcony.rack.resource = hero_resource;
        balcony.rack.current = 0;
    }
    let moved = needed.min(balcony.rack.current);
    balcony.rack.current -= moved;
    state.tower.hero.personal_ammo += moved;
}
