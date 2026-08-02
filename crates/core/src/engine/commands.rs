//! Command validation and application.
//!
//! Every handler validates fully **before** it mutates anything, so a
//! rejected command leaves state byte-identical. That property is what
//! lets the recorder log only accepted commands and still reproduce the
//! run exactly.

use crate::command::{CommandError, GameCommand};
use crate::content::{Content, RoomCategory};
use crate::ids::{FloorIdx, ItemIdx, SlotIdx};
use crate::state::tower::{Floor, Room};
use crate::state::{GameState, Tower};

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
    }
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
