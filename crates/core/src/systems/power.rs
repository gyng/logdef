//! Charge income and the two draws that don't belong to another system.
//!
//! Split into three entry points because charge priority *is* the tick
//! order (see `state/power.rs`): income has to land before anything
//! spends, lighting has to come after transport and production so it
//! yields to them, and striding pays last of all.

use crate::content::{Content, RoomCategory};
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

    heartseed_trickle(state, content);
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

/// What the Heartseed makes on its own — the floor that keeps a
/// stalled tower recoverable.
///
/// Accrued in fixed point so a trickle this small still adds up
/// instead of truncating to nothing every tick. See
/// `PowerBalance::heartseed_charge_per_100_ticks` for why it exists;
/// the short version is that after M6 cut the sails, every other
/// source of charge required already having some.
///
/// Gated on the Heartseed being present and **intact**, so it is the
/// tower's living core doing it rather than a rule about the number
/// zero. A tower whose Heartseed is gone has lost anyway.
///
/// **Intact, not `is_working` — and the difference is a bricked run.**
/// `is_working` also reads `active`, and `SetRoomActive` accepts the
/// Heartseed like any other room. So a player who switched everything
/// off turned off the floor under the economy and walked straight back
/// into the deadlock this exists to prevent: caught by
/// `e2e/smoke.spec.ts`, which does exactly that and then reported one
/// pole after 120,000 ticks. The Heartseed is grown rather than built
/// and cannot be removed; it is not a machine with a switch, and the
/// trickle should not pretend otherwise.
fn heartseed_trickle(state: &mut GameState, content: &Content) {
    let per_100 = content.balance.power.heartseed_charge_per_100_ticks;
    if per_100 <= 0 {
        return;
    }
    let alive = state
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .any(|room| {
            content.room(room.def).category == RoomCategory::Heart && !room.is_wrecked(content)
        });
    if !alive {
        return;
    }
    state.power.trickle_acc += Fx::ratio(i32::try_from(per_100).unwrap_or(i32::MAX), 100);
    let whole = state.power.trickle_acc.floor_int();
    if whole > 0 {
        state.power.trickle_acc -= Fx::from_int(whole);
        state.power.add(i64::from(whole));
    }
}

/// Burners turn the contested material into power. They run on the same
/// progress machinery as a recipe, and they eat from an inbox the crew
/// have to keep full — so lighting the burner competes with the mill
/// for exactly the same bamboo.
///
/// **A burner with nowhere to put the charge waits.** This was
/// harmless while the sails carried the tower and a burner was the
/// switch you threw on a dark night; when M6 cut the sails and made it
/// the only income, an always-on burner became a permanent drain that
/// outran the whole harvest. Measured on the opening tower: it wanted
/// 2 bamboo per 100 ticks against a cutter arm cutting 0.0075 a tick,
/// so it asked for **2.7x everything the tower could cut** — and
/// because `find_destination` feeds the emptiest inbox first, an
/// inbox that could never fill took every stalk and the mill completed
/// zero crafts in 1,800 ticks.
///
/// Idling on a full bank fixes it at the root rather than by tuning
/// the appetite down: consumption becomes what the tower actually
/// spends, so the fuel bill scales with striding and lamps and
/// thornwrights instead of with wall-clock time. The smoke scales with
/// it too, which is the better story — a tower that spends hard smokes
/// hard.
///
/// **Headroom, not fullness, and the difference cost a re-measure.**
/// The first version held only at `charge == capacity`, so a 400-point
/// burn into a bank with 50 points of room spent a whole stalk to add
/// 50 — `power.add` clips the rest. Measured, that made the efficiency
/// change do nothing at all: 22 stalks burned by tick 7,200 at 200 a
/// burn, and 22 at 400. A burner will not light unless the bank can
/// take the whole burn.
///
/// This requires a bank bigger than one burn, which
/// `power.starting_charge` alone satisfies several times over. If a
/// pack is ever authored where it does not, the burner silently never
/// runs.
///
/// **The headroom is spent as it goes, not read once.** A shared
/// figure lets every burner in the tower see the same room and all
/// light at once, and `power.add` then clips the surplus — three
/// burners paying three stalks to bank one burn's worth. Measured on
/// `examples/charge.rs`, that made a fourteen-floor tower with three
/// burners earn **less** than the same tower with one (543 against
/// 1,839 over a day), which is the tell: more of a thing cannot
/// produce less of what it makes.
///
/// Progress is *held*, not reset: the fuel has not been withdrawn yet,
/// and stalling in place is the same rule intake and production keep.
fn run_burners(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let mut produced = 0i64;
    let mut burns = 0i64;
    let mut burned = 0i64;
    let tick = state.tick;
    let mut headroom = state.power.capacity - state.power.charge;

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
            if burner.charge_per_burn > headroom {
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
            headroom -= burner.charge_per_burn;
            produced += burner.charge_per_burn;
            burned += burner.fuel_per_burn;
            burns += 1;
        }
    }

    if produced > 0 {
        state.stats.fuel_burned += burned as u64;
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
