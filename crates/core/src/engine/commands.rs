//! Command validation and application.
//!
//! Every handler validates fully **before** it mutates anything, so a
//! rejected command leaves state byte-identical. That property is what
//! lets the recorder log only accepted commands and still reproduce the
//! run exactly.

use crate::command::{CommandError, GameCommand};
use crate::content::{Content, RoomCategory, ShaftKind};
use crate::ids::{FloorIdx, ItemIdx, ShaftId, SlotIdx};
use crate::state::tower::{Floor, Room};
use crate::state::{Car, GameState, Shaft, ShaftProgram, Tower};

pub fn apply(
    state: &mut GameState,
    content: &Content,
    cmd: &GameCommand,
) -> Result<(), CommandError> {
    match cmd {
        GameCommand::SetSpeed { speed } => {
            state.speed = *speed;
            Ok(())
        }
        GameCommand::BuildFloor => build_floor(state, content),
        GameCommand::PlaceRoom { room, floor, slot } => {
            place_room(state, content, room, *floor, *slot)
        }
        GameCommand::RemoveRoom { floor, slot } => remove_room(state, content, *floor, *slot),
        GameCommand::SetRoomActive {
            floor,
            slot,
            active,
        } => set_room_active(state, *floor, *slot, *active),
        GameCommand::BuildShaft {
            shaft,
            low,
            high,
            slot,
        } => build_shaft(state, content, shaft, *low, *high, *slot),
        GameCommand::RemoveShaft { id } => remove_shaft(state, *id),
        GameCommand::SetShaftProgram {
            id,
            daypart,
            served,
            priority,
        } => set_shaft_program(state, content, *id, *daypart, served, *priority),
        GameCommand::SetStriding { walking } => {
            state.walking = *walking;
            Ok(())
        }
    }
}

fn set_room_active(
    state: &mut GameState,
    floor: FloorIdx,
    slot: SlotIdx,
    active: bool,
) -> Result<(), CommandError> {
    let Some(target) = state.tower.floor_mut(floor) else {
        return Err(CommandError::NoSuchFloor { floor });
    };
    let Some(room) = target.rooms.iter_mut().find(|room| room.covers(slot)) else {
        return Err(CommandError::NoRoomThere { floor, slot });
    };
    room.active = active;
    Ok(())
}

fn build_shaft(
    state: &mut GameState,
    content: &Content,
    shaft_id: &str,
    low: FloorIdx,
    high: FloorIdx,
    slot: SlotIdx,
) -> Result<(), CommandError> {
    let Some(def_idx) = content.shaft_idx(shaft_id) else {
        return Err(CommandError::UnknownShaft {
            shaft: shaft_id.to_string(),
        });
    };
    let def = content.shaft(def_idx);

    if high <= low {
        return Err(CommandError::BadSpan {
            low,
            high,
            min_span: def.min_span,
            max_span: def.max_span,
        });
    }
    let top = state.tower.top_floor();
    if high > top {
        return Err(CommandError::NoSuchFloor { floor: high });
    }

    // Span counts both ends, so a two-floor shaft spans floors n and
    // n+1 — which is what a player means by "two floors".
    let span = high - low + 1;
    if span < def.min_span || (def.max_span != 0 && span > def.max_span) {
        return Err(CommandError::BadSpan {
            low,
            high,
            min_span: def.min_span,
            max_span: def.max_span,
        });
    }

    // The column has to be clear on every floor it passes through.
    for floor in low..=high {
        if state.tower.slot_range_blocked(floor, slot, 1) {
            return Err(CommandError::SlotOccupied { floor, slot });
        }
    }

    let cost = content.shaft_rt(def_idx).build_cost.clone();
    check_stock(state, content, &cost)?;
    spend(state, &cost);

    let id = state.alloc_shaft_id();
    let dayparts = content.dayparts.len().max(1);
    let floors = content.balance.tower.max_floors as usize;
    let cars = (0..def.cars)
        .map(|_| {
            let mut car = Car::new();
            car.pos = crate::fx::Fx::from_int(i32::from(low));
            car
        })
        .collect();

    state.tower.shafts.push(Shaft {
        id,
        def: def_idx,
        kind: def.kind,
        low,
        high,
        slot,
        capacity: def.capacity,
        riders: 0,
        cars,
        programs: vec![ShaftProgram::all_floors(floors); dayparts],
    });
    Ok(())
}

fn remove_shaft(state: &mut GameState, id: ShaftId) -> Result<(), CommandError> {
    let Some(position) = state.tower.shafts.iter().position(|shaft| shaft.id == id) else {
        return Err(CommandError::NoSuchShaft { id });
    };
    // The built-in stairs are the baseline every route falls back to.
    // Without them a crew member could be stranded on a floor with no
    // way down, which is not a decision, just a soft lock.
    if state.tower.shafts[position].kind == ShaftKind::Stairs {
        return Err(CommandError::Undemolishable {
            room: "the stairs".into(),
        });
    }

    state.tower.shafts.remove(position);

    // Anyone waiting for, or riding, that shaft is put back on their
    // own two feet where they stand. Nothing they carry is lost.
    for member in &mut state.crew {
        let affected = match member.state {
            crate::state::CrewState::Boarding { shaft, .. }
            | crate::state::CrewState::Climbing { shaft, .. }
            | crate::state::CrewState::Riding { shaft, .. } => shaft == id,
            _ => false,
        };
        if affected {
            let (floor, slot) = (member.floor(), member.slot());
            member.snap_to(floor, slot);
            member.state = crate::state::CrewState::Idle;
        }
    }
    Ok(())
}

fn set_shaft_program(
    state: &mut GameState,
    content: &Content,
    id: ShaftId,
    daypart: u16,
    served: &[bool],
    priority: crate::state::ShaftPriority,
) -> Result<(), CommandError> {
    if daypart as usize >= content.dayparts.len() {
        return Err(CommandError::NoSuchDaypart { daypart });
    }
    let Some(shaft) = state.tower.shaft_mut(id) else {
        return Err(CommandError::NoSuchShaft { id });
    };
    let Some(program) = shaft.programs.get_mut(daypart as usize) else {
        return Err(CommandError::NoSuchDaypart { daypart });
    };
    // Keep the stored length: a program is indexed by floor, and the
    // tower can grow after the player last edited it.
    for (index, slot) in program.served.iter_mut().enumerate() {
        *slot = served.get(index).copied().unwrap_or(true);
    }
    program.priority = priority;
    Ok(())
}

fn build_floor(state: &mut GameState, content: &Content) -> Result<(), CommandError> {
    let balance = &content.balance.tower;

    if state.tower.floors.len() >= balance.max_floors as usize {
        return Err(CommandError::FloorLimit {
            max_floors: balance.max_floors,
        });
    }

    let cost = resolve_cost(content, &balance.floor_cost)?;
    check_stock(state, content, &cost)?;
    spend(state, &cost);

    let index = state.tower.floors.len() as FloorIdx;
    state
        .tower
        .floors
        .push(Floor::new(index, balance.floor_slots));

    // The stairs grow with the tower. Growing taller is never free —
    // from M1 the new top floor also displaces the sail deck.
    extend_stairs(&mut state.tower, index);
    Ok(())
}

/// Stretch every full-height shaft to cover the new top floor.
fn extend_stairs(tower: &mut Tower, new_top: FloorIdx) {
    let previous_top = new_top.saturating_sub(1);
    for shaft in &mut tower.shafts {
        if shaft.high == previous_top {
            shaft.high = new_top;
        }
    }
}

fn place_room(
    state: &mut GameState,
    content: &Content,
    room_id: &str,
    floor: FloorIdx,
    slot: SlotIdx,
) -> Result<(), CommandError> {
    let Some(def_idx) = content.room_idx(room_id) else {
        return Err(CommandError::UnknownRoom {
            room: room_id.to_string(),
        });
    };
    let def = content.room(def_idx);

    let Some(target) = state.tower.floor(floor) else {
        return Err(CommandError::NoSuchFloor { floor });
    };

    if slot as u16 + def.width as u16 > target.slots as u16 {
        return Err(CommandError::SlotOutOfRange {
            slot,
            width: def.width,
            floor_slots: target.slots,
        });
    }
    if let Some(max_floor) = def.max_floor
        && floor > max_floor
    {
        return Err(CommandError::FloorTooHigh { floor, max_floor });
    }
    if def.unique && state.tower.count_of(def_idx) > 0 {
        return Err(CommandError::AlreadyPlaced {
            room: room_id.to_string(),
        });
    }
    if state.tower.slot_range_blocked(floor, slot, def.width) {
        return Err(CommandError::SlotOccupied { floor, slot });
    }

    let cost = content.room_rt(def_idx).build_cost.clone();
    check_stock(state, content, &cost)?;
    spend(state, &cost);

    let id = state.alloc_room_id();
    let room = Room::new(id, def_idx, slot, content);
    if let Some(target) = state.tower.floor_mut(floor) {
        target.rooms.push(room);
        target.rooms.sort_by_key(|r| r.slot);
    }
    Ok(())
}

fn remove_room(
    state: &mut GameState,
    content: &Content,
    floor: FloorIdx,
    slot: SlotIdx,
) -> Result<(), CommandError> {
    let Some(target) = state.tower.floor(floor) else {
        return Err(CommandError::NoSuchFloor { floor });
    };
    let Some(position) = target.rooms.iter().position(|room| room.covers(slot)) else {
        return Err(CommandError::NoRoomThere { floor, slot });
    };

    let def_idx = target.rooms[position].def;
    let def = content.room(def_idx);
    if def.category == RoomCategory::Heart {
        return Err(CommandError::Undemolishable {
            room: def.id.clone(),
        });
    }

    let removed_id = target.rooms[position].id;
    if let Some(target) = state.tower.floor_mut(floor) {
        target.rooms.remove(position);
    }

    // Any crew heading to or from that room now have a stale task.
    // Clearing the task alone is enough: the state machine finishes the
    // current leg — releasing any shaft it holds — and then idles.
    // Anything already in hand stays in hand and gets re-homed.
    for member in &mut state.crew {
        let stale = member.task.as_ref().is_some_and(|task| {
            task.pickup.is_some_and(|pickup| pickup.room == removed_id)
                || match task.destination {
                    crate::state::HaulDestination::Inbox { room, .. }
                    | crate::state::HaulDestination::Shelf { room } => room == removed_id,
                }
        });
        if stale {
            member.task = None;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Stock: the chain pays for the tower
// ---------------------------------------------------------------------------

fn resolve_cost(
    content: &Content,
    entries: &[crate::content::CostEntryDef],
) -> Result<Vec<(ItemIdx, i64)>, CommandError> {
    entries
        .iter()
        .map(|entry| {
            content
                .item_idx(&entry.item)
                .map(|idx| (idx, entry.amount))
                .ok_or_else(|| CommandError::UnknownRoom {
                    room: entry.item.clone(),
                })
        })
        .collect()
}

/// Check every line of a cost before spending any of it, so a failed
/// build never leaves the shelves half-emptied.
fn check_stock(
    state: &GameState,
    content: &Content,
    cost: &[(ItemIdx, i64)],
) -> Result<(), CommandError> {
    for (item, amount) in cost {
        let available = state.stock_of(*item);
        if available < *amount {
            return Err(CommandError::InsufficientStock {
                item: content.item(*item).id.clone(),
                needed: *amount,
                available,
            });
        }
    }
    Ok(())
}

fn spend(state: &mut GameState, cost: &[(ItemIdx, i64)]) {
    for (item, amount) in cost {
        state.take_stock(*item, *amount);
    }
}
