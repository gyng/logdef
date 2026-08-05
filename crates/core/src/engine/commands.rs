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
        GameCommand::SetPowerPriority { order } => set_power_priority(state, order),
        GameCommand::FocusEnemy { enemy } => focus_enemy(state, *enemy),
        GameCommand::StationCrew { crew, room } => station_crew(state, *crew, *room),
        GameCommand::EquipCrew { crew, kit } => equip_crew(state, content, *crew, kit.as_deref()),
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
        GameCommand::TakeFork { branch } => take_fork(state, content, *branch),
        GameCommand::Trade { offer } => trade(state, content, *offer),
        GameCommand::Recruit => recruit(state, content),
        GameCommand::Reinforce => reinforce(state, content),
        GameCommand::SetShift { crew, shift } => set_shift(state, *crew, *shift),
    }
}

/// Put one crew member on a shift.
///
/// The whole of the mechanism: nothing is woken, nothing is cancelled,
/// no errand is disturbed. "Awake" is derived from this and the daypart
/// every tick, so a sleeping day worker set to `Night` at midnight is
/// awake on the very next tick, and a day worker set to `Night` at noon
/// walks to a bed as soon as their hands are empty.
fn set_shift(
    state: &mut GameState,
    crew: crate::ids::CrewId,
    shift: crate::content::Shift,
) -> Result<(), CommandError> {
    let Some(member) = state.crew.iter_mut().find(|member| member.id == crew) else {
        return Err(CommandError::NoSuchCrew { crew });
    };
    member.shift = shift;
    Ok(())
}

/// Answer the pending fork.
///
/// Validated fully before anything moves, per `DECISIONS.md` §4:
/// `World::answer_fork` throws away terrain generated past the split,
/// so a rejected answer must never reach it.
fn take_fork(state: &mut GameState, content: &Content, branch: u8) -> Result<(), CommandError> {
    let Some(fork) = state.world.fork else {
        return Err(CommandError::NoForkPending);
    };
    if usize::from(branch) >= fork.branches.len() {
        return Err(CommandError::NoSuchBranch { branch });
    }
    state.world.answer_fork(content, branch);
    Ok(())
}

/// The enclave the tower is berthed at, and which region it belongs to.
///
/// **Returns the region index too, from M5.** With one settlement in the
/// game "the enclave" was unambiguous and this returned the first one it
/// found; with three, every board, every recruit count and every stock
/// list is per region, and a command that did not know *which* enclave
/// it was talking to would spend the coast's recruits out of the
/// jungle's stock.
fn berthed_enclave<'a>(
    state: &GameState,
    content: &'a Content,
) -> Result<(usize, &'a crate::content::EnclaveRuntime), CommandError> {
    let region = state
        .world
        .berthed_enclave(content, state.strode)
        .ok_or(CommandError::NotBerthedAtAnEnclave)?;
    let enclave = content
        .region_rt(region)
        .enclave
        .as_ref()
        .ok_or(CommandError::NotBerthedAtAnEnclave)?;
    Ok((region.0 as usize, enclave))
}

/// Take one of the enclave's posted offers.
///
/// Validated to the last check before anything moves, per
/// `DECISIONS.md` §4: the shelves have to have the goods *and* somewhere
/// to put what comes back, or a trade could take payment and drop the
/// return on the floor.
fn trade(state: &mut GameState, content: &Content, offer: u8) -> Result<(), CommandError> {
    let (region, enclave) = berthed_enclave(state, content)?;
    let index = usize::from(offer);
    let deal = *enclave
        .offers
        .get(index)
        .ok_or(CommandError::NoSuchOffer { offer })?;

    if state
        .enclave_stock
        .get(region)
        .and_then(|board| board.get(index))
        .copied()
        .unwrap_or(0)
        <= 0
    {
        return Err(CommandError::OfferExhausted { offer });
    }
    let (give_item, give_amount) = deal.give;
    let held = state.stock_of(give_item);
    if held < give_amount {
        return Err(CommandError::InsufficientStock {
            item: content.item(give_item).id.clone(),
            needed: give_amount,
            available: held,
        });
    }

    state.take_stock(give_item, give_amount);
    let (take_item, take_amount) = deal.take;
    // Whatever will not fit stays with the traders rather than
    // vanishing. Nothing this game hands the player is ever silently
    // discarded — see `haul.rs` on carried loads.
    let landed = state.shelve(take_item, take_amount);
    state.enclave_stock[region][index] -= 1;
    state.stats.traded += landed as u64;
    Ok(())
}

/// Have the settlement plate the tower's shell.
///
/// The one permanent upgrade in the game, and the reason a run that
/// salvages has something to do with the metal: scrap's only other
/// consumer is the trade board, so a tower that berths late banks
/// metal it cannot spend. Plating turns it into hull.
fn reinforce(state: &mut GameState, content: &Content) -> Result<(), CommandError> {
    let (region, enclave) = berthed_enclave(state, content)?;
    let Some((cost, panel_hp)) = enclave.reinforce.clone() else {
        return Err(CommandError::NoShellWorkHere);
    };
    if state.shell_work_left.get(region).copied().unwrap_or(0) == 0 {
        return Err(CommandError::NoShellWorkLeft);
    }
    check_stock(state, content, &cost)?;

    spend(state, &cost);
    state.shell_work_left[region] -= 1;
    state.tower.reinforce(panel_hp);
    Ok(())
}

/// Take somebody aboard.
fn recruit(state: &mut GameState, content: &Content) -> Result<(), CommandError> {
    let (region, enclave) = berthed_enclave(state, content)?;
    if state.enclave_recruits.get(region).copied().unwrap_or(0) == 0 {
        return Err(CommandError::NobodyToRecruit);
    }
    let cap = content.balance.crew.crew_cap;
    if state.crew.len() >= usize::from(cap) {
        return Err(CommandError::CrewFull { cap });
    }
    let cost = enclave.recruit_cost.clone();
    check_stock(state, content, &cost)?;

    spend(state, &cost);
    state.enclave_recruits[region] -= 1;
    state.add_crew(content);
    Ok(())
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
        health: crate::state::Health::full(content.balance.siege.shaft_hp),
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
    // Plated to whatever the shell has been plated to, so growing
    // taller never grows a soft spot.
    state.tower.floors.push(Floor::new(
        index,
        balance.floor_slots,
        content.balance.siege.panel_hp + state.tower.shell_bonus,
    ));

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
    if let Some(min_floor) = def.min_floor
        && floor < min_floor
    {
        return Err(CommandError::FloorTooLow { floor, min_floor });
    }
    if def.unique && state.tower.count_of(def_idx) > 0 {
        return Err(CommandError::AlreadyPlaced {
            room: room_id.to_string(),
        });
    }
    // **The opening ladder** (`SYSTEMS.md` §6.11). Validated here rather
    // than filtered in the UI, because a menu that merely hides a card
    // is a rule the player can only discover by not seeing something,
    // and because this one is legitimately a fact about `GameState` —
    // it reads the tower's own rooms, so it is deterministic and a
    // replay carries it. The journal's unlocks are the other kind and
    // deliberately never come near this function.
    if let Some(needs) = content.room_rt(def_idx).unlocked_by
        && state.tower.count_of(needs) == 0
    {
        return Err(CommandError::Locked {
            room: room_id.to_string(),
            needs: content.room(needs).id.clone(),
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
                    // A spill names a shaft, not a room, so tearing out
                    // a room can never invalidate one. Tearing out the
                    // *chute* is handled where shafts are removed.
                    crate::state::HaulDestination::Spill { .. } => false,
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

/// Rank the four charge uses. Validates fully before mutating
/// (`DECISIONS.md` §4): an order that is not a permutation of the four
/// is rejected whole rather than partly applied.
fn set_power_priority(
    state: &mut GameState,
    order: &[crate::state::power::PowerUse],
) -> Result<(), CommandError> {
    use crate::state::power::PowerUse;
    let complete = PowerUse::ALL
        .iter()
        .all(|use_| order.iter().filter(|entry| *entry == use_).count() == 1);
    if order.len() != PowerUse::ALL.len() || !complete {
        return Err(CommandError::BadPowerPriority { given: order.len() });
    }
    state.power.priority = order.to_vec();
    Ok(())
}

/// Point every emplacement at one creature, or stop pointing.
///
/// Validated before mutating (`DECISIONS.md` §4): an id nobody is
/// carrying is rejected rather than stored, so the highlight in the
/// cross-section can never be aimed at nothing.
fn focus_enemy(
    state: &mut GameState,
    enemy: Option<crate::ids::EnemyId>,
) -> Result<(), CommandError> {
    if let Some(id) = enemy
        && !state.siege.enemies.iter().any(|out| out.id == id)
    {
        return Err(CommandError::NoSuchEnemy { id });
    }
    state.siege.focus = enemy;
    Ok(())
}

/// Post somebody to a room, or call them back.
///
/// Validated before mutating (`DECISIONS.md` §4): an unknown person or a
/// room that is not standing is refused whole, so a posting can never
/// name something that is not there.
///
/// It does **not** move anybody or cancel what they are doing. The
/// assignment pass picks the order up next tick, which means a crew
/// member mid-delivery finishes it first — the same courtesy every other
/// errand gets, and the reason nothing a crew member is carrying is ever
/// dropped.
fn station_crew(
    state: &mut GameState,
    crew: crate::ids::CrewId,
    room: Option<crate::ids::RoomId>,
) -> Result<(), CommandError> {
    if !state.crew.iter().any(|member| member.id == crew) {
        return Err(CommandError::NoSuchCrew { crew });
    }
    if let Some(id) = room
        && state.tower.locate(id).is_none()
    {
        return Err(CommandError::NoRoomThere { floor: 0, slot: 0 });
    }
    let Some(member) = state.crew.iter_mut().find(|member| member.id == crew) else {
        return Err(CommandError::NoSuchCrew { crew });
    };
    member.stationed = room;
    Ok(())
}

/// Lend somebody a kit, or take back the one they have.
///
/// Validated to the last check before anything moves (`DECISIONS.md`
/// §4): the person has to exist, the item has to exist, it has to *be* a
/// kit, and it has to be on the shelves. A half-applied equip would
/// either duplicate a kit or lose one.
fn equip_crew(
    state: &mut GameState,
    content: &Content,
    crew: crate::ids::CrewId,
    kit: Option<&str>,
) -> Result<(), CommandError> {
    if !state.crew.iter().any(|member| member.id == crew) {
        return Err(CommandError::NoSuchCrew { crew });
    }
    let wanted = match kit {
        None => None,
        Some(id) => {
            let idx = content
                .item_idx(id)
                .ok_or_else(|| CommandError::NotAKit { item: id.into() })?;
            if content.item(idx).kit.is_none() {
                return Err(CommandError::NotAKit { item: id.into() });
            }
            if state.stock_of(idx) < 1 {
                return Err(CommandError::InsufficientStock {
                    item: id.into(),
                    needed: 1,
                    available: state.stock_of(idx),
                });
            }
            Some(idx)
        }
    };

    // Whatever they were carrying goes back on the shelves first, so
    // swapping one kit for another cannot lose the old one. If there is
    // nowhere to put it the swap is refused rather than quietly
    // destroying it — nothing this game hands the player ever vanishes.
    let held = state
        .crew
        .iter()
        .find(|member| member.id == crew)
        .and_then(|member| member.kit);
    if let Some(old) = held
        && state.shelve(old, 1) < 1
    {
        return Err(CommandError::InsufficientStock {
            item: content.item(old).id.clone(),
            needed: 1,
            available: 0,
        });
    }
    if let Some(idx) = wanted {
        state.take_stock(idx, 1);
    }
    let Some(member) = state.crew.iter_mut().find(|member| member.id == crew) else {
        return Err(CommandError::NoSuchCrew { crew });
    };
    member.kit = wanted;
    Ok(())
}
