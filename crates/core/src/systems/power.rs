//! Charge income and the two draws that don't belong to another system.
//!
//! Split into three entry points because charge priority *is* the tick
//! order (see `state/power.rs`): income has to land before anything
//! spends, lighting has to come after transport and production so it
//! yields to them, and striding pays last of all.

use crate::content::Content;
use crate::fx::Fx;
use crate::state::GameState;
use crate::state::power::{Credit, PowerUse};

use super::SoundEvent;

/// Recompute capacity, then collect from the sails and the burner.
/// Runs first, before any consumer.
pub fn income(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    state.power.begin_tick();
    state.power.capacity = bank_capacity(state, content);
    state.power.charge = state.power.charge.min(state.power.capacity);

    // The *roof's* exposure, which is not the ground's: a taller tower
    // reaches over more of the canopy. Harvest keeps reading the ground
    // figure, because a cutter arm sweeping the forest floor does not
    // care how many storeys are stacked above it.
    let exposure = roof_exposure_pct(state, content);
    collect_solar(state, content, exposure);
    run_burners(state, content, sounds);
    estimate_demand(state, content);
}

/// What each use is likely to want this tick, so the player's ranking
/// can be honoured by draws that happen at different points in it.
///
/// **Estimates, deliberately.** A use that draws early in the tick has
/// to leave room for a higher-ranked one that draws late, and it cannot
/// know exactly what that one will ask for without running it first.
/// Slightly high makes the tower cautious for a tick; slightly low costs
/// the higher-ranked use nothing, because it still draws against
/// whatever is actually left. Neither can create charge or lose it —
/// this only decides who gets refused first.
fn estimate_demand(state: &mut GameState, content: &Content) {
    let mut demand = vec![0i64; 4];

    // Lifts: every car that could move, one tick's worth of a floor.
    demand[PowerUse::Lifts.index()] = state
        .tower
        .shafts
        .iter()
        .map(|shaft| {
            let def = content.shaft(shaft.def);
            let ticks = i64::from(def.ticks_per_floor.max(1));
            shaft.cars.len() as i64 * (def.charge_per_floor / ticks)
        })
        .sum();

    // Works: every powered room switched on. A stalled one may not
    // spend, but it is about to try.
    demand[PowerUse::Works.index()] = state
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .filter(|room| room.active)
        .map(|room| content.room(room.def).power_draw)
        .sum();

    // Lamps and legs buy in hundred-tick blocks, so what they want on
    // any given tick is either a whole block or nothing at all.
    let power = &content.balance.power;
    demand[PowerUse::Lamps.index()] = if state.power.light_credit == 0 {
        power.light_charge_per_100_ticks_per_floor * state.tower.floors.len() as i64
    } else {
        0
    };
    demand[PowerUse::Legs.index()] = if state.power.stride_credit == 0 && state.walking {
        power.stride_charge_per_100_ticks
    } else {
        0
    };

    state.power.demand = demand;
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

/// Sunlight reaching the *roof*, which on a tall tower is more than
/// reaches the ground.
///
/// Each floor above the starting height recovers
/// `canopy_climb_pct_per_floor` points of the terrain's shade, capped at
/// open sky — so this does nothing in a clearing, where there is no
/// shade to recover, and a great deal under canopy.
///
/// **It is the counterweight to `top_floor_only`.** A new top floor
/// shades the sail deck under it, which made growing taller pure loss
/// for the tower's income and left the growth gradient pointing away
/// from the one shape a shaft is worth building for.
#[must_use]
pub fn roof_exposure_pct(state: &GameState, content: &Content) -> i64 {
    let sun = state.clock.sun_pct(content);
    let terrain = state
        .world
        .band_at(state.world.distance)
        .map_or(100, |band| content.terrain_runtime[band.kind.get()].sun_pct);
    let grown = i64::from(
        (state.tower.floors.len() as u8).saturating_sub(content.balance.tower.starting_floors),
    );
    // Recovery, not a bonus: a roof cannot see more than open sky, so
    // this can never lift a clearing above what a clearing gives.
    let lifted = (terrain + grown * content.balance.power.canopy_climb_pct_per_floor).min(100);
    sun * terrain.max(lifted) / 100
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
        .filter(|room| room.is_working(content, state.tick))
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
    let mut burns = 0i64;
    let tick = state.tick;

    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            let Some(burner) = content.room(room.def).burner.as_ref() else {
                continue;
            };
            if !room.is_working(content, tick) {
                if !room.active {
                    room.progress = 0;
                }
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
            burns += 1;
        }
    }

    if produced > 0 {
        state.power.add(produced);
        sounds.push(SoundEvent::Burn);
        // Smoke. The dirty fallback is not free — this is what stops
        // the burner being the answer to every dark night.
        let per_burn = content.balance.siege.provocation_per_burn;
        super::siege::provoke_hundredths(state, content, burns * per_burn * 100);
    }
}

/// Lamps, after dark. Third in priority: the tower goes dark before the
/// chain stalls, but it keeps its lights before it keeps walking.
/// **Lighting is deliberately not gated on anybody being awake, and
/// that is the obvious next idea somebody will have.** M4 gave the tower
/// a shift rota, which raises the question of why a tower whose crew are
/// all in bed pays for lamps. Gating it reads well and it is three
/// lines. It was considered and rejected: the default rota is all-Day,
/// so gating would make every night's lamps free for most towers and
/// quietly relax the brown-out pressure `BALANCE.md`'s power rows were
/// measured against. The night shift is instead made to depend on
/// charge from the other side — a crew member working while `lit` is
/// false works at `dark_work_pct` (`needs::work_pct`), so a brown-out
/// does not just dim the tower, it wastes the shift you staffed. That
/// costs no constants and revalues nothing.
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
    // A tower with nowhere to walk does not pay to try. Standing at an
    // unanswered fork or at the far edge of the journey is a stop like
    // any other stop, and charging for it would quietly bleed a player
    // who was only thinking.
    if !state.walking || state.world.is_blocked() {
        return false;
    }
    let per_100 = content.balance.power.stride_charge_per_100_ticks;
    state.power.buy_block(per_100, Credit::Stride)
}
