//! Intake — the tower strips what it walks past, what it stops at, and
//! what it grows.
//!
//! Three sources. Two of them are exact opposites (`SYSTEMS.md` §3.4): a
//! `Terrain` source accrues against the ground actually covered, so a
//! stopped tower harvests nothing at all, while a `Ruin` source accrues
//! per tick and only while the tower is stopped with a ruin inside the
//! rig's reach. **Walking harvests bamboo; stopping harvests scrap** —
//! so the stop/go decision is also an intake-mix decision, which is the
//! cleanest statement of what M3 is for.
//!
//! `Sun` is the third, and it is the one that does not take a side
//! (§5.2). It accrues per tick scaled by the sun actually reaching the
//! tower, moving or not, which is what stops every stop being a pure
//! loss — and it is the sun axis finally buying something that is not
//! charge.
//!
//! Either way the room accumulates *effort* — Q8.8 paces of ground for
//! a terrain source, Q8.8 ticks of work for a ruin — and every time
//! that crosses the effort one item costs, it pushes an item into the
//! outbox.
//!
//! Accumulating effort against a threshold, rather than accumulating a
//! per-tick fraction of an item, is what makes the yield multiplier
//! mean anything. See `paces_per_item` for what the old shape cost.
//!
//! A full outbox stalls the accumulator rather than discarding the
//! overflow: the arm visibly stops, and nothing vanishes silently. That
//! stall is the feedback — the room goes quiet because nobody is
//! collecting from it. A stalled rig leaves the ruin whatever it has
//! not given up.

use crate::content::{Content, IntakeSource};
use crate::fx::Fx;
use crate::state::GameState;

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let yield_pct = state.world.current_yield_pct(content);
    // Sun after terrain. Since M6 cut the sails this is the only thing
    // in the game the sky still pays for, and it pays in food.
    let exposure = super::power::exposure_pct(state, content);
    let tick = state.tick;
    // Which floor is the roof, read before the floors come out of the
    // tower below. `top_floor_only` used to be enforced in exactly one
    // place — `collect_solar`'s roof filter — so cutting the sails cut
    // the rule with them, and the garden was left claiming a
    // restriction nothing applied. A `shaded` room that keeps growing
    // at full rate is a diegetic lie (`DECISIONS.md` §8): the flag the
    // cross-section dims is the flag that stops the crop.
    let top = state.tower.top_floor();
    let manned = super::manned_rooms(state, content);

    // Both of these are one tick old, deliberately. Intake runs fourth
    // and stride runs eleventh, because tick order *is* charge priority
    // and walking is the first thing a tower short of power gives up
    // (`SYSTEMS.md` §3.8), so intake cannot know how far the tower has
    // moved on the tick it is running in. `paces_last` is the
    // quantitative sibling of `strode`, which siege's cling logic has
    // been reading one tick late since M2. What must not happen is
    // someone moving stride earlier to close the lag: that reorders
    // charge priority and invalidates every golden replay, to fix
    // something nobody can perceive at 30 Hz.
    let paces = state.paces_last;
    // Berthing is implicit — there is no `Berth` command. A tower that
    // is not walking is berthed at whatever happens to be in reach, and
    // one with nothing in reach is simply stopped. The same flag is what
    // siege reads to decide whether anything clinging to the tower loses
    // its grip, which is not a coincidence: see `siege::rouse_wardens`.
    let berthed = !state.strode;

    let mut harvested = 0i64;
    let mut salvaged = 0i64;

    // The floors come out of the tower for the loop: extracting from a
    // ruin writes to the world and wakes what lives in it, and neither
    // is reachable through a borrow of the floors.
    let mut floors = std::mem::take(&mut state.tower.floors);
    for floor in &mut floors {
        for room in &mut floor.rooms {
            let rt = content.room_rt(room.def);
            let (Some(item), Some(source)) = (rt.intake_item, rt.intake_source) else {
                continue;
            };
            let Some(slot) = room.outputs.iter().position(|s| s.item == item) else {
                continue;
            };
            // A damaged arm strips less; a wrecked one strips nothing.
            if !room.is_working(content, tick) {
                continue;
            }
            // And an unstaffed farm strips nothing at all: the garden
            // is the one room in the pack with `crew_required`, and it
            // is where the opening teaches that rooms are run by people
            // (`SYSTEMS.md` §6.11).
            if !super::staffed(content, room, &manned) {
                continue;
            }

            if room.outputs[slot].is_full() {
                // Stalled: hold the accumulator so partial work
                // survives until a crew member frees space. The arm
                // visibly stops — that silence is the feedback, so
                // there is no event and no warning banner.
                continue;
            }

            match source {
                IntakeSource::Terrain { paces_per_item } => {
                    // Credit the ground actually covered, and spend it
                    // against what an item costs in this band. A stopped
                    // tower adds nothing because `paces_last` is zero —
                    // no special case says so, which is the whole point
                    // of measuring the ground rather than the clock.
                    let needed = terrain_effort(paces_per_item, yield_pct);
                    room.intake_acc += Fx(i32::try_from(paces).unwrap_or(i32::MAX));
                    while room.intake_acc >= needed && room.outputs[slot].deposit(1) == 1 {
                        room.intake_acc -= needed;
                        harvested += 1;
                        credit(&mut state.stats, item);
                        sounds.push(SoundEvent::Harvest);
                    }
                }

                IntakeSource::Sun { ticks_per_item } => {
                    // One tick of work, thresholded by how much sun is
                    // reaching the tower. Exposure is sun *after*
                    // terrain, so a garden under dense canopy grows
                    // almost nothing.
                    //
                    // Built over, it grows nothing at all — that is the
                    // cost of height stated where the player can see
                    // it, and it is the same `top_floor_only` the
                    // snapshot dims the room for.
                    //
                    // No `berthed` check and no `paces` term: this is
                    // the source that does not care.
                    let reaching = if content.room(room.def).top_floor_only && floor.index != top {
                        0
                    } else {
                        exposure
                    };
                    let needed = sun_effort(ticks_per_item, reaching);
                    if needed >= Fx(i32::MAX) {
                        // Full shade. Nothing grows, and holding the
                        // accumulator means what was grown in the light
                        // survives the walk through the dark.
                        continue;
                    }
                    room.intake_acc += Fx::ONE;
                    while room.intake_acc >= needed && room.outputs[slot].deposit(1) == 1 {
                        room.intake_acc -= needed;
                        harvested += 1;
                        credit(&mut state.stats, item);
                        sounds.push(SoundEvent::Harvest);
                    }
                }

                IntakeSource::Ruin {
                    ticks_per_item,
                    range_paces,
                } => {
                    if !berthed {
                        continue;
                    }
                    let Some(ruin) = state.world.ruin_in_reach(range_paces) else {
                        continue;
                    };
                    // One tick of work, and never scaled by the band's
                    // yield: a rig is not drawing from the terrain, and
                    // what is inside a ruin is not growing.
                    let needed = ruin_effort(ticks_per_item);
                    room.intake_acc += Fx::ONE;
                    while room.intake_acc >= needed {
                        let held = state.world.features[ruin].salvage;
                        if held <= 0 || room.outputs[slot].deposit(1) != 1 {
                            break;
                        }
                        room.intake_acc -= needed;
                        state.world.features[ruin].salvage = held - 1;
                        salvaged += 1;
                        credit(&mut state.stats, item);
                        sounds.push(SoundEvent::Harvest);

                        if !state.world.features[ruin].roused {
                            state.world.features[ruin].roused = true;
                            let at = state.world.features[ruin].at;
                            super::siege::rouse_wardens(state, content, at, held, sounds);
                        }
                    }
                }
            }
        }
    }
    state.tower.floors = floors;

    state.stats.items_harvested += (harvested + salvaged) as u64;

    // Stripping the terrain is noticed, and so is taking a ruin apart.
    // Both costs land next to the act rather than in a separate
    // bookkeeping pass, so it is impossible to add a new way of pulling
    // things out of the world and forget to make it provoking.
    let balance = &content.balance.siege;
    let noise = harvested * balance.provocation_per_100_harvested
        + salvaged * balance.provocation_per_100_salvaged;
    super::siege::provoke_hundredths(state, content, noise);
}

/// Ground one item costs in a band of this yield, in Q8.8 paces.
///
/// Richer ground costs fewer paces an item, which is the direction that
/// makes `yield_pct` read the way a designer expects.
///
/// **This is the arithmetic that used to be wrong, and it mattered more
/// than it looks.** Intake accrued a per-tick *fraction of an item*
/// instead: `Fx::ratio(1, 90)`, which in Q8.8 is `Fx(2)` — one item per
/// 128 ticks rather than the authored 90, a 42% shortfall that
/// `BALANCE.md` had put down to shelf space and crew legs. Worse,
/// scaling that truncated fraction was very nearly a no-op:
/// `Fx(2) * 1.40` is `Fx(2)`, so dense canopy and open clearing
/// harvested at *identical* rates, and ruin field and drowned street
/// collapsed together too. Four authored terrain kinds behaved as two,
/// and the flagship contrast of the route being the power mix
/// (`DESIGN.md` pillar 1) did not exist in the simulation at all.
///
/// Accumulating effort against a threshold keeps the precision where it
/// is needed: the threshold is a large number, so its single division
/// rounds by about a thousandth of a pace, and the per-tick credit is
/// exact because `paces_last` is already Q8.8.
#[must_use]
pub fn terrain_effort(paces_per_item: i64, yield_pct: i64) -> Fx {
    let paces = crate::fx::paces_from_int(paces_per_item.max(1));
    let scaled = paces * 100 / yield_pct.max(1);
    // One pace an item is the floor: a room that cost less than that to
    // run would strip a band bare in a handful of ticks.
    let effort = Fx(i32::try_from(scaled).unwrap_or(i32::MAX));
    if effort < Fx::ONE { Fx::ONE } else { effort }
}

/// Credit one harvest to the material it actually was.
///
/// `items_harvested` is a sum and a sum cannot see a change in the
/// *mix*, which is the whole of what M5's second route axis adds — see
/// `RunStats::harvested_by_item`.
fn credit(stats: &mut crate::state::RunStats, item: crate::ids::ItemIdx) {
    if let Some(slot) = stats.harvested_by_item.get_mut(item.0 as usize) {
        *slot += 1;
    }
}

/// Work one item of salvage costs, in Q8.8 ticks.
#[must_use]
pub fn ruin_effort(ticks_per_item: u32) -> Fx {
    Fx::from_int(i32::try_from(ticks_per_item).unwrap_or(i32::MAX).max(1))
}

/// Work one crop costs, in Q8.8 ticks, at this much sun.
///
/// **Scales the threshold, never the per-tick step**, which is the same
/// rule `terrain_effort` exists to enforce and for the same reason. The
/// shape that broke intake before M3 was accruing a *fraction of an
/// item* per tick and scaling that: `Fx::ratio(1, 90)` is `Fx(2)` in
/// Q8.8, and `Fx(2) * 1.40` is also `Fx(2)`, so four authored terrain
/// yields behaved as two and the flagship contrast of `DESIGN.md`
/// pillar 1 was simply absent from the simulation. Here the accrual is
/// a flat `Fx::ONE` a tick and *this* is what moves, so a garden in
/// half sun genuinely takes twice as long and there is no rounding to
/// fall off.
///
/// Returns `Fx(i32::MAX)` in full shade rather than dividing by zero.
/// The caller reads that as "nothing grows" and holds the accumulator,
/// so a crop part-grown in the light survives the walk through the dark.
#[must_use]
pub fn sun_effort(ticks_per_item: u32, exposure_pct: i64) -> Fx {
    if exposure_pct <= 0 {
        return Fx(i32::MAX);
    }
    let ticks = i64::from(ticks_per_item.max(1));
    let scaled = crate::fx::paces_from_int(ticks) * 100 / exposure_pct;
    let effort = Fx(i32::try_from(scaled).unwrap_or(i32::MAX));
    // One tick a crop is the floor, for the same reason one pace an item
    // is: a room cheaper than that fills its buffer in a handful of
    // ticks and stops meaning anything.
    if effort < Fx::ONE { Fx::ONE } else { effort }
}
