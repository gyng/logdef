//! Charge income and the two draws that don't belong to another system.
//!
//! Split into three entry points because charge priority *is* the tick
//! order (see `state/power.rs`): income has to land before anything
//! spends, lighting has to come after transport and production so it
//! yields to them, and striding pays last of all.

use crate::content::Content;
use crate::fx::Fx;
use crate::state::GameState;
use crate::state::power::Credit;

use super::SoundEvent;

/// Recompute capacity, then collect from the sails and the burner.
/// Runs first, before any consumer.
pub fn income(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    state.power.begin_tick();
    state.power.capacity = bank_capacity(state, content);
    state.power.charge = state.power.charge.min(state.power.capacity);

    let exposure = exposure_pct(state, content);
    collect_solar(state, content, exposure);
    run_burners(state, content, sounds);
}

/// Sunlight reaching the sails: the day's curve, scaled by how much of
/// it this stretch of jungle lets through.
#[must_use]
pub fn exposure_pct(state: &GameState, content: &Content) -> i64 {
    let sun = state.clock.sun_pct(content);
    let terrain = state
        .world
        .band_at(state.world.distance)
        .map_or(100, |band| content.terrain_runtime[band.kind.get()].sun_pct);
    sun * terrain / 100
}

fn bank_capacity(state: &GameState, content: &Content) -> i64 {
    let banks: i64 = state
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .filter_map(|room| content.room(room.def).bank.as_ref())
        .map(|bank| bank.capacity)
        .sum();
    // The Heartseed holds a little on its own, so a tower with no banks
    // is merely poor rather than instantly dead.
    banks + content.balance.power.starting_charge
}

fn collect_solar(state: &mut GameState, content: &Content, exposure: i64) {
    let top = state.tower.top_floor();
    let rate: i64 = state
        .tower
        .floors
        .iter()
        .filter(|floor| floor.index == top)
        .flat_map(|floor| floor.rooms.iter())
        .filter_map(|room| content.room(room.def).solar.as_ref())
        .map(|solar| solar.charge_per_100_ticks)
        .sum();

    if rate == 0 || exposure <= 0 {
        return;
    }

    // Accumulate in fixed point so a trickle of sun still adds up
    // instead of truncating to nothing every tick.
    let per_tick = Fx::ratio(rate as i32, 100) * Fx::ratio(exposure as i32, 100);
    state.power.solar_acc += per_tick;
    let whole = state.power.solar_acc.floor_int();
    if whole > 0 {
        state.power.solar_acc -= Fx::from_int(whole);
        state.power.add(i64::from(whole));
    }
}

/// Burners turn the contested material into power. They run on the same
/// progress machinery as a recipe, and they eat from an inbox the crew
/// have to keep full — so lighting the burner competes with the mill
/// for exactly the same bamboo.
fn run_burners(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let mut produced = 0i64;

    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            let Some(burner) = content.room(room.def).burner.as_ref() else {
                continue;
            };
            if !room.active {
                room.progress = 0;
                continue;
            }
            let Some(fuel) = room.inputs.first_mut() else {
                continue;
            };
            if fuel.count < burner.fuel_per_burn {
                continue;
            }

            room.progress += 1;
            if room.progress < burner.burn_ticks {
                continue;
            }
            fuel.withdraw(burner.fuel_per_burn);
            room.progress = 0;
            produced += burner.charge_per_burn;
        }
    }

    if produced > 0 {
        state.power.add(produced);
        sounds.push(SoundEvent::Burn);
    }
}

/// Lamps, after dark. Third in priority: the tower goes dark before the
/// chain stalls, but it keeps its lights before it keeps walking.
pub fn lighting(state: &mut GameState, content: &Content) {
    if exposure_pct(state, content) >= content.balance.clock.night_light_threshold {
        // Daylight. Nothing to pay for, and nothing to switch on.
        state.power.lit = true;
        return;
    }
    let floors = state.tower.floors.len() as i64;
    let per_100 = content.balance.power.light_charge_per_100_ticks_per_floor * floors;
    state.power.lit = state.power.buy_block(per_100, Credit::Light);
}

/// The legs. Last in priority, so a tower short of charge stops walking
/// before anything inside it stops working. Returns whether the tower
/// could afford to move this tick.
pub fn pay_for_stride(state: &mut GameState, content: &Content) -> bool {
    if !state.walking {
        return false;
    }
    let per_100 = content.balance.power.stride_charge_per_100_ticks;
    state.power.buy_block(per_100, Credit::Stride)
}
