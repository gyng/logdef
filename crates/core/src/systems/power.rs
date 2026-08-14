//! Charge income and the two draws that don't belong to another system.
//!
//! Split into three entry points because charge priority *is* the tick
//! order (see `state/power.rs`): income has to land before anything
//! spends, lighting has to come after transport and production so it
//! yields to them, and striding pays last of all.

use crate::content::{Content, RoomCategory};
use crate::fx::Fx;
use crate::state::GameState;
use crate::state::power::{FULL, PER_TICK, PowerUse};

use super::SoundEvent;

/// Recompute capacity, then collect from the sails and the burner.
/// Runs first, before any consumer.
pub fn income(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    state.power.begin_tick();
    for room in state
        .tower
        .floors
        .iter_mut()
        .flat_map(|floor| floor.rooms.iter_mut())
    {
        room.power_refused = false;
        room.burning = false;
        room.exhaust_refused = false;
    }
    sync_floor_storage(state, content);

    heartseed_trickle(state, content);
    // `heartseed_trickle` credits the compatibility total. Attach that
    // new charge to the Heartseed's physical floor before island
    // allocation; otherwise the per-floor sum overwrites and silently
    // discards every trickle tick.
    sync_floor_storage(state, content);

    // **Generation first, then demand, then the split** (`SYSTEMS.md`
    // §6.39). Nothing draws during the tick any more: every consumer is
    // told what fraction of its appetite it may run at, and the whole
    // division happens here, once, before anything moves.
    // Everything from here runs in charge per 100 ticks (`PER_TICK`),
    // because striding is 0.2 a tick and a per-tick table would round it
    // to nothing.
    allocate_floor_network(state, content, sounds);
}

fn floor_capacities(state: &GameState, content: &Content) -> Vec<i64> {
    state
        .tower
        .floors
        .iter()
        .map(|floor| {
            let banks = floor
                .rooms
                .iter()
                .filter(|room| !room.is_wrecked(content))
                .filter_map(|room| content.room(room.def).bank.as_ref())
                .map(|bank| bank.capacity)
                .sum::<i64>();
            let heart = floor.rooms.iter().any(|room| {
                content.room(room.def).category == RoomCategory::Heart && !room.is_wrecked(content)
            });
            banks
                + if heart {
                    content.balance.power.starting_charge
                } else {
                    0
                }
        })
        .collect()
}

fn sync_floor_storage(state: &mut GameState, content: &Content) {
    let caps = floor_capacities(state, content);
    let count = caps.len();
    let uninitialised = state.power.floor_charge.len() != count;
    state.power.floor_charge.resize(count, 0);
    state.power.floor_spend_acc.resize(count, 0);
    state.power.floor_fill_acc.resize(count, 0);
    for (held, cap) in state.power.floor_charge.iter_mut().zip(&caps) {
        *held = (*held).clamp(0, *cap);
    }
    let target = state.power.charge.clamp(0, caps.iter().sum());
    let held: i64 = state.power.floor_charge.iter().sum();
    if uninitialised || held != target {
        if held < target {
            let mut left = target - held;
            for (floor, cap) in state.power.floor_charge.iter_mut().zip(&caps) {
                let add = left.min((*cap - *floor).max(0));
                *floor += add;
                left -= add;
            }
        } else {
            let mut left = held - target;
            for floor in &mut state.power.floor_charge {
                let take = left.min(*floor);
                *floor -= take;
                left -= take;
            }
        }
    }
    state.power.capacity = caps.iter().sum();
    state.power.charge = state.power.floor_charge.iter().sum();
}

fn boundary_capacities(state: &GameState, content: &Content) -> Vec<i64> {
    let mut result = vec![0; state.tower.floors.len().saturating_sub(1)];
    for shaft in &state.tower.shafts {
        let def = content.shaft(shaft.def);
        if def.power_capacity <= 0 || shaft.health.max <= 0 || shaft.is_severed() {
            continue;
        }
        let effective = def.power_capacity * shaft.health.hp / shaft.health.max;
        for boundary in usize::from(shaft.low)..usize::from(shaft.high) {
            if let Some(cap) = result.get_mut(boundary) {
                *cap += effective * PER_TICK;
            }
        }
    }
    result
}

fn path_capacity(boundaries: &[i64], from: usize, to: usize) -> i64 {
    if from == to {
        return i64::MAX;
    }
    let (low, high) = if from < to { (from, to) } else { (to, from) };
    boundaries[low..high].iter().copied().min().unwrap_or(0)
}

fn use_path(boundaries: &mut [i64], from: usize, to: usize, amount: i64) {
    let (low, high) = if from < to { (from, to) } else { (to, from) };
    for cap in &mut boundaries[low..high] {
        *cap -= amount;
    }
}

fn draw_nearest(target: usize, want: &mut i64, sources: &mut [i64], boundaries: &mut [i64]) -> i64 {
    let mut got = 0;
    for distance in 0..sources.len() {
        for (source, available) in sources.iter_mut().enumerate() {
            if source.abs_diff(target) != distance || *available <= 0 || *want <= 0 {
                continue;
            }
            let take = (*want)
                .min(*available)
                .min(path_capacity(boundaries, source, target));
            if take > 0 {
                *available -= take;
                *want -= take;
                got += take;
                use_path(boundaries, source, target, take);
            }
        }
    }
    got
}

fn demand_by_floor(state: &GameState, content: &Content) -> Vec<Vec<i64>> {
    let count = state.tower.floors.len();
    let mut demand = vec![vec![0; PowerUse::ALL.len()]; count];
    let tick = state.tick;
    let manned = super::manned_rooms(state, content);
    for shaft in state
        .tower
        .shafts
        .iter()
        .filter(|shaft| !shaft.is_severed())
    {
        let def = content.shaft(shaft.def);
        let rate = if def.ticks_per_floor > 0 {
            def.charge_per_floor * PER_TICK / i64::from(def.ticks_per_floor)
        } else {
            0
        };
        let moving = shaft
            .cars
            .iter()
            .filter(|car| matches!(car.state, crate::state::CarState::Moving))
            .count() as i64;
        if let Some(row) = demand.get_mut(usize::from(shaft.low)) {
            row[PowerUse::Lifts.index()] += moving * rate;
        }
    }
    for floor in &state.tower.floors {
        let row = &mut demand[usize::from(floor.index)];
        for room in &floor.rooms {
            let rt = content.room_rt(room.def);
            let ready = rt.craft_ticks > 0
                && room.is_working(content, tick)
                && super::staffed(content, room, &manned)
                && rt
                    .recipe_inputs
                    .iter()
                    .enumerate()
                    .all(|(i, (_, amount, _))| {
                        room.inputs.get(i).is_some_and(|s| s.count >= *amount)
                    })
                && rt
                    .recipe_outputs
                    .iter()
                    .enumerate()
                    .all(|(i, (_, amount, _))| {
                        room.outputs.get(i).is_some_and(|s| s.space() >= *amount)
                    });
            if ready {
                row[PowerUse::Works.index()] += content.room(room.def).power_draw * PER_TICK;
            }
            if !state.siege.enemies.is_empty()
                && let Some(defence) = content.room(room.def).defence.as_ref()
                && room.is_working(content, tick)
                && room.inputs.iter().any(|s| s.count >= defence.ammo_per_shot)
            {
                row[PowerUse::Guns.index()] += if defence.reload_ticks > 0 {
                    defence.charge_per_shot * PER_TICK / i64::from(defence.reload_ticks)
                } else {
                    defence.charge_per_shot * PER_TICK
                };
            }
        }
        if exposure_pct(state, content) < content.balance.clock.night_light_threshold {
            row[PowerUse::Lamps.index()] =
                content.balance.power.light_charge_per_100_ticks_per_floor;
        }
    }
    if state.walking && !state.world.is_blocked() && !demand.is_empty() {
        demand[0][PowerUse::Legs.index()] = content.balance.power.stride_charge_per_100_ticks;
    }
    demand
}

fn allocate_floor_network(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let count = state.tower.floors.len();
    let demand = demand_by_floor(state, content);
    let mut generation = vec![0; count];
    let mut release = vec![0; count];
    let tick = state.tick;
    for floor in &state.tower.floors {
        let at = usize::from(floor.index);
        for room in &floor.rooms {
            let def = content.room(room.def);
            if let Some(burner) = def.burner.as_ref() {
                let fuelled = room
                    .inputs
                    .first()
                    .is_some_and(|fuel| fuel.count >= burner.fuel_per_burn);
                if fuelled && room.is_working(content, tick) && burner.burn_ticks > 0 {
                    generation[at] +=
                        burner.charge_per_burn * PER_TICK / i64::from(burner.burn_ticks);
                }
            }
            if !room.is_wrecked(content) {
                if let Some(bank) = def.bank.as_ref() {
                    release[at] += bank.discharge_per_tick * PER_TICK;
                }
                if def.category == RoomCategory::Heart {
                    release[at] += content.balance.power.heartseed_rail_per_tick * PER_TICK;
                }
            }
        }
        release[at] = release[at].min(state.power.floor_charge[at].saturating_mul(PER_TICK));
    }
    state.power.rail = (generation.iter().sum::<i64>() + release.iter().sum::<i64>()) / PER_TICK;

    let initial_generation = generation.clone();
    let initial_release = release.clone();
    let mut boundaries = boundary_capacities(state, content);
    let mut served = vec![vec![FULL; PowerUse::ALL.len()]; count];
    for use_ in &state.power.priority {
        let circuit = use_.index();
        let mut delivered = vec![0; count];
        for floor in 0..count {
            if demand[floor][circuit] > 0 {
                served[floor][circuit] = 0;
            }
        }

        // Progressive proportional fill. Every requesting floor is
        // raised toward the same service fraction before any floor gets
        // the next share. Ten-per-mille steps keep this bounded while
        // limiting rounding/order effects to one percentage point. The
        // rotating start prevents floor zero from being a hidden sixth
        // priority while remaining deterministic and replay-safe.
        const FAIR_STEP: i64 = 10;
        let start = (state.tick as usize + circuit) % count.max(1);
        for target in (FAIR_STEP..=FULL).step_by(FAIR_STEP as usize) {
            for offset in 0..count {
                let floor = (start + offset) % count;
                let asked = demand[floor][circuit];
                if asked <= 0 {
                    continue;
                }
                let target_amount = (asked * target + FULL - 1) / FULL;
                let mut left = (target_amount - delivered[floor]).max(0);
                if left == 0 {
                    continue;
                }
                let got_generation =
                    draw_nearest(floor, &mut left, &mut generation, &mut boundaries);
                let got_bank = draw_nearest(floor, &mut left, &mut release, &mut boundaries);
                let got = got_generation + got_bank;
                delivered[floor] += got;
            }
        }
        for floor in 0..count {
            let asked = demand[floor][circuit];
            if asked > 0 {
                served[floor][circuit] = (delivered[floor] * FULL / asked).min(FULL);
            }
        }
    }
    let mut total_demand = vec![0; PowerUse::ALL.len()];
    let mut total_served = vec![0; PowerUse::ALL.len()];
    for floor in 0..count {
        for use_ in PowerUse::ALL {
            let asked = demand[floor][use_.index()];
            total_demand[use_.index()] += asked;
            total_served[use_.index()] += asked * served[floor][use_.index()] / FULL;
        }
    }
    state.power.demand = total_demand.clone();
    state.power.satisfaction = PowerUse::ALL
        .iter()
        .map(|use_| {
            let asked = total_demand[use_.index()];
            if asked <= 0 {
                FULL
            } else {
                total_served[use_.index()] * FULL / asked
            }
        })
        .collect();
    state.power.floor_satisfaction = served;
    state.power.brownout = PowerUse::ALL
        .iter()
        .any(|use_| state.power.served(*use_) < FULL);

    // Spare generation charges the nearest reachable physical bank.
    // Transmission capacity is shared with live load, so a thin stairs
    // trunk may run a deck or recharge it, but cannot do both for free.
    let caps = floor_capacities(state, content);
    let mut banked = vec![0; count];
    for floor in 0..count {
        let room_rate = (caps[floor] - state.power.floor_charge[floor]).max(0) * PER_TICK;
        let mut want = room_rate;
        banked[floor] = draw_nearest(floor, &mut want, &mut generation, &mut boundaries);
    }

    let used_generation: Vec<i64> = initial_generation
        .iter()
        .zip(&generation)
        .map(|(a, b)| a - b)
        .collect();
    let used_release: Vec<i64> = initial_release
        .iter()
        .zip(&release)
        .map(|(a, b)| a - b)
        .collect();
    for floor in 0..count {
        state.power.floor_spend_acc[floor] += used_release[floor];
        let whole = state.power.floor_spend_acc[floor] / PER_TICK;
        state.power.floor_spend_acc[floor] -= whole * PER_TICK;
        state.power.floor_charge[floor] = (state.power.floor_charge[floor] - whole).max(0);
        state.power.floor_fill_acc[floor] += banked[floor];
        let filled = state.power.floor_fill_acc[floor] / PER_TICK;
        state.power.floor_fill_acc[floor] -= filled * PER_TICK;
        state.power.floor_charge[floor] =
            (state.power.floor_charge[floor] + filled).min(caps[floor]);
    }
    state.power.charge = state.power.floor_charge.iter().sum();
    state.power.spent_last =
        (used_generation.iter().sum::<i64>() + used_release.iter().sum::<i64>()) / PER_TICK;
    let billed_generation: Vec<i64> = used_generation
        .iter()
        .zip(&banked)
        .map(|(used, stored)| used + stored)
        .collect();
    allocate_exhaust_and_burn(state, content, &billed_generation, sounds);
}

/// Allocate intact stacks that reach the roof. Burners claim first so
/// missing exhaust can never rebuild the dry-tower deadlock; they still
/// run without a stack, but their smoke retains its full provocation.
fn allocate_exhaust_and_burn(
    state: &mut GameState,
    content: &Content,
    used_by_floor: &[i64],
    sounds: &mut Vec<SoundEvent>,
) {
    let top = state.tower.top_floor();
    let mut stacks: Vec<(u8, u8, i64)> = state
        .tower
        .shafts
        .iter()
        .filter(|shaft| {
            shaft.kind == crate::content::ShaftKind::VentStack
                && !shaft.is_severed()
                && shaft.high == top
        })
        .map(|shaft| {
            (
                shaft.low,
                shaft.slot,
                content.shaft(shaft.def).exhaust_capacity,
            )
        })
        .collect();

    burn_fuel_for(state, content, used_by_floor, sounds, &mut stacks);

    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            let def = content.room(room.def);
            if def.exhaust_draw <= 0
                || def.burner.is_some()
                || !room.is_working(content, state.tick)
            {
                continue;
            }
            let rt = content.room_rt(room.def);
            let ready = rt
                .recipe_inputs
                .iter()
                .enumerate()
                .all(|(i, (_, amount, _))| room.inputs.get(i).is_some_and(|s| s.count >= *amount))
                && rt
                    .recipe_outputs
                    .iter()
                    .enumerate()
                    .all(|(i, (_, amount, _))| {
                        room.outputs.get(i).is_some_and(|s| s.space() >= *amount)
                    });
            if ready
                && !take_exhaust(
                    &mut stacks,
                    floor.index,
                    room.slot,
                    room.width,
                    def.exhaust_draw,
                )
            {
                room.exhaust_refused = true;
            }
        }
    }
}

fn take_exhaust(
    stacks: &mut [(u8, u8, i64)],
    floor: u8,
    room_slot: u8,
    room_width: u8,
    draw: i64,
) -> bool {
    let room_end = u16::from(room_slot) + u16::from(room_width);
    let Some(index) = stacks
        .iter()
        .enumerate()
        .filter(|(_, (low, stack_slot, remaining))| {
            let touches = room_end == u16::from(*stack_slot)
                || u16::from(*stack_slot) + 1 == u16::from(room_slot);
            *low <= floor && touches && *remaining >= draw
        })
        // Prefer the nearest inlet below the machine. Capacity is the
        // second key so two equally local stacks share truthfully until
        // one is fuller; slot is the stable authored tie-break.
        .min_by_key(|(_, (low, stack_slot, remaining))| {
            (floor - *low, std::cmp::Reverse(*remaining), *stack_slot)
        })
        .map(|(index, _)| index)
    else {
        return false;
    };
    let remaining = &mut stacks[index].2;
    *remaining -= draw;
    true
}

/// Is the tower's living core still there and unwrecked?
///
/// Shared by the trickle and the rail floor, which are the same
/// argument applied to the reservoir and to the pipe: both are what
/// keeps a stalled tower recoverable, and both should stop mattering at
/// the same moment, because a tower whose Heartseed is gone has lost
/// anyway.
fn heartseed_alive(state: &GameState, content: &Content) -> bool {
    state
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .any(|room| {
            content.room(room.def).category == RoomCategory::Heart && !room.is_wrecked(content)
        })
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
    if !heartseed_alive(state, content) {
        return;
    }
    state.power.trickle_acc += Fx::ratio(i32::try_from(per_100).unwrap_or(i32::MAX), 100);
    let whole = state.power.trickle_acc.floor_int();
    if whole > 0 {
        state.power.trickle_acc -= Fx::from_int(whole);
        let amount = i64::from(whole);
        if let Some(at) = state.tower.floors.iter().position(|floor| {
            floor.rooms.iter().any(|room| {
                content.room(room.def).category == RoomCategory::Heart && !room.is_wrecked(content)
            })
        }) {
            let caps = floor_capacities(state, content);
            let room = (caps[at] - state.power.floor_charge[at]).max(0);
            let taken = amount.min(room);
            state.power.floor_charge[at] += taken;
            state.power.charge += taken;
            state.power.income_last += taken;
        }
    }
}

/// Charge the burners for the generation the tower actually used.
///
/// **Fuel is now proportional to draw, which is the whole Factorio
/// half of `SYSTEMS.md` §6.39.** A burner is a ceiling, not a pump: it
/// offers `charge_per_burn / burn_ticks` a tick and is billed for what
/// is taken. A stalk is still worth exactly `charge_per_burn`, so the
/// measured economy — three stalks a day for the developed fixture —
/// carries across unchanged.
///
/// **This supersedes the headroom logic rather than discarding it**, and
/// the lessons it was built from are worth keeping written down, because
/// each was a measured failure:
///
/// - An always-on burner outran the whole harvest: it wanted 2 bamboo
///   per 100 ticks against a cutter arm cutting 0.0075 a tick, so it
///   asked for 2.7x everything the tower could cut, and the mill
///   completed zero crafts in 1,800 ticks.
/// - Holding only at `charge == capacity` spent a whole stalk to add 50
///   into a bank with 50 points of room, because `add` clips the rest.
/// - A shared headroom figure let three burners all see the same room
///   and light together, so a fourteen-floor tower with three earned
///   *less* than the same tower with one — 543 against 1,839 over a day.
///
/// All three were symptoms of a burner that produced on its own
/// schedule. Billing for consumption cannot reproduce any of them: there
/// is no burn to waste, no headroom to misjudge, and three burners
/// billed for one tower's draw cost exactly what one does.
///
/// Fractional stalks accumulate in `Fx` so a tower drawing a trickle
/// still eventually pays for a whole one instead of burning free.
fn burn_fuel_for(
    state: &mut GameState,
    content: &Content,
    used_by_floor: &[i64],
    sounds: &mut Vec<SoundEvent>,
    exhaust: &mut [(u8, u8, i64)],
) {
    if used_by_floor.iter().all(|used| *used <= 0) {
        return;
    }
    let tick = state.tick;
    // Spread the bill across the burners that could have supplied it, in
    // tower order, so the arithmetic is deterministic and a burner that
    // is out of fuel is simply not billed.
    let mut burned = 0i64;
    let mut smoke_hundredths = 0i64;
    let mut lit = false;
    for floor in &mut state.tower.floors {
        let mut owed = used_by_floor
            .get(usize::from(floor.index))
            .copied()
            .unwrap_or(0);
        for room in &mut floor.rooms {
            if owed <= 0 {
                break;
            }
            let Some(burner) = content.room(room.def).burner.as_ref() else {
                continue;
            };
            if burner.burn_ticks == 0 || burner.charge_per_burn <= 0 {
                continue;
            }
            if !room.is_working(content, tick) {
                continue;
            }
            let rate = burner.charge_per_burn * PER_TICK / i64::from(burner.burn_ticks);
            let Some(fuel) = room.inputs.first_mut() else {
                continue;
            };
            if fuel.count < burner.fuel_per_burn {
                continue;
            }
            let mine = owed.min(rate);
            if mine <= 0 {
                continue;
            }
            owed -= mine;
            room.burning = true;
            let exhaust_draw = content.room(room.def).exhaust_draw;
            let vented = exhaust_draw <= 0
                || take_exhaust(exhaust, floor.index, room.slot, room.width, exhaust_draw);
            room.exhaust_refused = !vented;
            lit = true;
            // Stalks owed for this much charge, carried in fixed point
            // so a trickle is not free.
            // `mine` is charge per 100 ticks; a stalk is worth
            // `charge_per_burn` of it over a hundred ticks. Counted as
            // whole charge owed and divided once, so nothing rounds.
            room.burn_acc += mine;
            let per_stalk = burner.charge_per_burn * PER_TICK;
            let whole = room.burn_acc / per_stalk * burner.fuel_per_burn;
            if whole > 0 {
                let take = whole.min(fuel.count);
                fuel.withdraw(take);
                room.burn_acc -= take / burner.fuel_per_burn.max(1) * per_stalk;
                burned += take;
                let smoke_pct = if vented {
                    burner.vented_provocation_pct
                } else {
                    100
                };
                smoke_hundredths += take * content.balance.siege.provocation_per_burn * smoke_pct;
            }
        }
    }

    if burned > 0 {
        state.stats.fuel_burned += burned as u64;
        // Smoke. The dirty fallback is not free — this is what stops the
        // burner being the answer to every dark night, and it now scales
        // with fuel actually consumed rather than with burns completed.
        super::siege::provoke_hundredths(state, content, smoke_hundredths);
    }
    if lit {
        sounds.push(SoundEvent::Burn);
    }
}

/// Lamps, after dark.
///
/// **No longer a purchase.** Lighting asked for a hundred ticks at a
/// time because a per-tick rate of 0.02 a floor truncated to nothing;
/// under satisfaction the demand is stated per tick and served as a
/// fraction, so the block purchase and its credit meter are gone
/// (`SYSTEMS.md` §6.39).
///
/// A partly served lamp circuit reads as unlit. That is deliberately
/// coarser than the mill, which genuinely turns slower: a lamp at
/// two-thirds power is a dim room, and the tower has one bit for that
/// today. `PowerView::satisfaction` carries the fraction so the renderer
/// can dim rather than switch, and interpolating `dark_work_pct` across
/// it is left for whoever wants it.
pub fn lighting(state: &mut GameState, content: &Content) {
    if exposure_pct(state, content) >= content.balance.clock.night_light_threshold {
        // Daylight. Nothing to pay for, and nothing to switch on.
        state.power.lit = true;
        return;
    }
    state.power.lit = !state.power.short(PowerUse::Lamps);
}

/// The legs, as a fraction of a full stride.
///
/// **Returns per-mille rather than a yes or no.** A tower one joule
/// short used to stop dead; it now walks slower, which is both the
/// Factorio behaviour and the readable one — `stride.rs` already scales
/// its step by `drag_pct` for a mire-hulk, and this is the same
/// multiplication for the same reason.
#[must_use]
pub fn stride_pct(state: &GameState, content: &Content) -> i64 {
    // A tower with nowhere to walk does not pay to try. Standing at an
    // unanswered fork or at the far edge of the journey is a stop like
    // any other stop, and charging for it would quietly bleed a player
    // who was only thinking.
    if !state.walking || state.world.is_blocked() {
        return 0;
    }
    let _ = content;
    state.power.served(PowerUse::Legs)
}
