use crate::engine::weapon_resource;
use crate::registry::Registry;
use crate::snapshot::SoundEvent;
use crate::state::*;
use crate::types::Scalar;

#[allow(clippy::ptr_arg)] // Will push to sounds when runner events are added
pub fn run(state: &mut GameState, registry: &Registry, _dt: Scalar, sounds: &mut Vec<SoundEvent>) {
    if state.encounter.is_none() {
        return;
    }

    for floor in &mut state.tower.floors {
        if let Some(building) = &mut floor.building
            && building.output_buffer.current > 0
        {
            let moved = building.output_buffer.current;
            let resource = building.output_buffer.resource;
            let stored = fill_slots(
                &mut state.tower.warehouse.slots,
                state.tower.warehouse.capacity_per_slot,
                resource,
                moved,
            );
            building.output_buffer.current -= stored;
            if stored > 0 {
                sounds.push(SoundEvent::RunnerDeliver);
            }
        }
    }

    for floor in &mut state.tower.floors {
        let Some(cache) = &mut floor.cache else {
            continue;
        };
        for slot in &mut cache.slots {
            let needed = slot.max.saturating_sub(slot.current);
            if needed == 0 {
                continue;
            }
            let moved = withdraw_slots(&mut state.tower.warehouse.slots, slot.resource, needed);
            slot.current += moved;
            if moved > 0 {
                sounds.push(SoundEvent::RunnerPickup);
            }
        }
    }

    for balcony in &mut state.tower.balconies {
        if balcony.rack.destroyed {
            continue;
        }
        let Some(cache) = state
            .tower
            .floors
            .get_mut(balcony.floor)
            .and_then(|floor| floor.cache.as_mut())
        else {
            continue;
        };
        let Some(slot) = cache
            .slots
            .iter_mut()
            .find(|slot| slot.resource == balcony.rack.resource)
        else {
            continue;
        };
        let needed = balcony.rack.max.saturating_sub(balcony.rack.current);
        if needed == 0 {
            continue;
        }
        let moved = needed.min(slot.current);
        slot.current -= moved;
        balcony.rack.current += moved;
        if moved > 0 {
            sounds.push(SoundEvent::RunnerDeliver);
        }
    }

    let Some(hero_resource) = weapon_resource(state.tower.hero.weapon_primary.base_type) else {
        return;
    };
    let desired = registry.balance.hero.personal_ammo;
    let needed = desired.saturating_sub(state.tower.hero.personal_ammo);
    if needed == 0 {
        return;
    }

    let hero_position = state.tower.hero.position;
    let Some(hero_balcony) = state
        .tower
        .balconies
        .iter_mut()
        .find(|balcony| balcony.id == hero_position)
    else {
        return;
    };
    if hero_balcony.rack.resource != hero_resource {
        hero_balcony.rack.resource = hero_resource;
        hero_balcony.rack.current = 0;
    }

    let moved = needed.min(hero_balcony.rack.current);
    hero_balcony.rack.current -= moved;
    state.tower.hero.personal_ammo += moved;
}

fn fill_slots(
    slots: &mut Vec<ResourceBuffer>,
    default_capacity: u32,
    resource: ResourceType,
    amount: u32,
) -> u32 {
    if amount == 0 {
        return 0;
    }

    let mut remaining = amount;
    for slot in slots.iter_mut().filter(|slot| slot.resource == resource) {
        let space = slot.max.saturating_sub(slot.current);
        let moved = remaining.min(space);
        slot.current += moved;
        remaining -= moved;
        if remaining == 0 {
            return amount;
        }
    }

    while remaining > 0 {
        let moved = remaining.min(default_capacity);
        slots.push(ResourceBuffer {
            resource,
            current: moved,
            max: default_capacity,
        });
        remaining -= moved;
    }

    amount
}

fn withdraw_slots(slots: &mut [ResourceBuffer], resource: ResourceType, amount: u32) -> u32 {
    if amount == 0 {
        return 0;
    }

    let mut remaining = amount;
    for slot in slots.iter_mut().filter(|slot| slot.resource == resource) {
        let moved = remaining.min(slot.current);
        slot.current -= moved;
        remaining -= moved;
        if remaining == 0 {
            break;
        }
    }

    amount - remaining
}
