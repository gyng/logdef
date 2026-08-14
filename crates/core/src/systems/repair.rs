//! Putting the tower back together.
//!
//! Repair is a chain sink like any other: it costs poles off the
//! shelves and crew time off the same three people who are trying to
//! keep the mill fed. That competition is the interesting part. Triage
//! — deciding to leave a floor breached because the chain matters more
//! right now — is a decision the player only gets to make if repair is
//! neither automatic nor free, so it is neither.
//!
//! Repair reuses the crew's existing legs. A crew member walks to the
//! damage the same way they walk to a crate, using the same L-shaped
//! path and the same shafts — which means a severed shaft can leave
//! damage unreachable, and that is correct rather than a bug.

use crate::content::Content;
use crate::ids::{FloorIdx, ItemIdx, SlotIdx};
use crate::state::siege::DamageTarget;
use crate::state::{Crew, CrewState, GameState, Tower};

use super::SoundEvent;

/// A crew member is already at the damage and working on it.
pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let balance = &content.balance.siege;
    let Some(poles) = content.item_idx("item.poles") else {
        return;
    };

    let mut mended = 0i64;
    let mut spent = 0i64;
    let mut crew = std::mem::take(&mut state.crew);

    for member in &mut crew {
        let CrewState::Repairing { target, ticks_left } = member.state else {
            continue;
        };
        if ticks_left > 0 {
            member.state = CrewState::Repairing {
                target,
                ticks_left: ticks_left - 1,
            };
            continue;
        }

        // A shift of work is done. Pay for it, then apply it.
        //
        // **A mender's kit buys hit points, not time.** The shift takes
        // exactly as long either way and costs poles in proportion to
        // what it mends, so the kit makes somebody's hour worth more
        // rather than making repair cheap — the poles still come off the
        // shelves at the same rate per point.
        // A kit is lent; a knack came aboard with the person. They
        // compose, so a mender holding a mender's kit is the best the
        // tower can do about a wall (`SYSTEMS.md` §6.25).
        let kit_pct = member
            .kit
            .and_then(|item| content.item(item).kit.as_ref())
            .map_or(100, |kit| kit.mend_pct);
        let mend_pct = (kit_pct * member.trait_pct(content, |t| t.mend_pct) / 100).max(1);
        let per_shift = (balance.repair_hp_per_shift * mend_pct / 100).max(1);
        let cost = balance.repair_poles_per_10_hp * per_shift / 10;
        if state.stock_of(poles) < cost {
            // No materials. Stand down rather than mending for free —
            // running out of poles mid-repair is a real outcome.
            member.state = CrewState::Idle;
            member.errand = None;
            continue;
        }

        let healed = mend(state, target, per_shift);
        if healed == 0 {
            // Already fixed, or gone. Nothing to pay for.
            member.state = CrewState::Idle;
            member.errand = None;
            continue;
        }
        state.take_stock(poles, cost);
        spent += cost;
        mended += healed;
        sounds.push(SoundEvent::Repair);

        // One shift brings a destroyed system back online, then releases
        // the crew and poles to the live chain. The former Restore
        // posture spent 48 extra poles, produced fewer crafts and
        // prevented no additional wrecks in the measured fixture.
        member.state = CrewState::Idle;
        member.errand = None;
    }

    state.crew = crew;
    state.stats.hp_repaired += mended as u64;
    state.stats.repair_poles_spent += spent as u64;
}

/// Ticks of crew time one shift of repair takes.
#[must_use]
pub fn shift_ticks(content: &Content, hp: i64) -> u32 {
    content.balance.siege.repair_ticks_per_hp * (hp.max(1) as u32)
}

fn mend(state: &mut GameState, target: DamageTarget, amount: i64) -> i64 {
    match target {
        DamageTarget::Panel { floor } => state
            .tower
            .floor_mut(floor)
            .map_or(0, |floor| floor.panel.heal(amount)),
        DamageTarget::Room { floor, slot } => state.tower.floor_mut(floor).map_or(0, |floor| {
            floor
                .rooms
                .iter_mut()
                .find(|room| room.covers(slot))
                .map_or(0, |room| room.health.heal(amount))
        }),
        DamageTarget::Shaft { id } => state
            .tower
            .shaft_mut(id)
            .map_or(0, |shaft| shaft.health.heal(amount)),
        // The Heartseed is repaired as the room it is.
        DamageTarget::Heart => 0,
    }
}

/// The worst damage this crew member could reach and afford to work on,
/// skipping anything a colleague has already claimed.
///
/// Severed shafts come first: while one is down, part of the tower may
/// be unreachable, so fixing it unblocks everything else. After that,
/// whatever is most broken.
///
/// Takes a stock figure rather than the whole `GameState` because the
/// assignment pass runs with the crew moved out of state — see
/// `haul::run`.
#[must_use]
pub fn pick_repair(
    tower: &Tower,
    content: &Content,
    poles_in_stock: i64,
    crew: &[Crew],
    me: usize,
    from_floor: FloorIdx,
) -> Option<(DamageTarget, FloorIdx, SlotIdx)> {
    // A sleeper handed a repair is a crew member who works in their
    // sleep. `assign_idle` already never gets this far for one, and this
    // is here so a future second caller cannot reintroduce it.
    if crew.get(me).is_some_and(Crew::is_asleep) {
        return None;
    }
    let per_shift = content.balance.siege.repair_hp_per_shift.max(1);
    let cost = content.balance.siege.repair_poles_per_10_hp * per_shift / 10;
    if poles_in_stock < cost {
        return None;
    }

    // Crew do not chase scratches. A shift costs its poles whether it
    // mends twenty hit points or one, so starting one on a panel that
    // has lost four is throwing four poles away — and a tower under
    // regular harassment was doing exactly that, often enough that it
    // could never bank the poles for an elevator. Below a shift's worth
    // of damage, the mark stays on the tower. That is the cross-section
    // doing its job as the health readout rather than a leak.
    let worth_mending = |health: &crate::state::Health| {
        let damaged_enough = health.max - health.hp >= per_shift;
        damaged_enough && health.hp == 0
    };

    let taken = |target: DamageTarget| {
        crew.iter()
            .enumerate()
            .any(|(i, other)| i != me && other.repair_target() == Some(target))
    };

    // A severed shaft is both the most urgent damage and the damage
    // that may be blocking the route to everything else.
    let severed = tower
        .shafts
        .iter()
        .filter(|shaft| worth_mending(&shaft.health))
        .filter(|shaft| !taken(DamageTarget::Shaft { id: shaft.id }))
        .min_by_key(|shaft| (shaft.health.permille(), shaft.id.0));
    if let Some(shaft) = severed {
        // Work is done from the bottom of the column, which is always
        // reachable on foot from the ground.
        return Some((DamageTarget::Shaft { id: shaft.id }, shaft.low, shaft.slot));
    }

    let mut best: Option<(i64, i64, DamageTarget, FloorIdx, SlotIdx)> = None;
    for floor in &tower.floors {
        if worth_mending(&floor.panel) && !taken(DamageTarget::Panel { floor: floor.index }) {
            // A panel is worked on from the outboard edge of its floor.
            let slot = floor.slots.saturating_sub(1);
            consider(
                &mut best,
                floor.panel.permille(),
                i64::from(floor.index.abs_diff(from_floor)),
                DamageTarget::Panel { floor: floor.index },
                floor.index,
                slot,
            );
        }
        for room in &floor.rooms {
            if !worth_mending(&room.health)
                || taken(DamageTarget::Room {
                    floor: floor.index,
                    slot: room.slot,
                })
            {
                continue;
            }
            consider(
                &mut best,
                room.health.permille(),
                i64::from(floor.index.abs_diff(from_floor)),
                DamageTarget::Room {
                    floor: floor.index,
                    slot: room.slot,
                },
                floor.index,
                room.slot,
            );
        }
    }

    best.map(|(_, _, target, floor, slot)| (target, floor, slot))
}

/// Worst first; ties to whatever is nearest, then to the lower floor so
/// the choice is a pure function of state.
fn consider(
    best: &mut Option<(i64, i64, DamageTarget, FloorIdx, SlotIdx)>,
    permille: i64,
    distance: i64,
    target: DamageTarget,
    floor: FloorIdx,
    slot: SlotIdx,
) {
    let better = match best {
        None => true,
        Some((worst, nearest, _, best_floor, _)) => {
            permille < *worst
                || (permille == *worst && distance < *nearest)
                || (permille == *worst && distance == *nearest && floor < *best_floor)
        }
    };
    if better {
        *best = Some((permille, distance, target, floor, slot));
    }
}

/// Poles committed by the automatic emergency-recovery queue.
/// Presentation only: it must quote what crew will actually spend, not
/// the much larger price of cosmetically restoring every scratch.
#[must_use]
pub fn outstanding_repair_cost(state: &GameState, content: &Content) -> i64 {
    // **Only the damage crew will actually mend.**
    //
    // `pick_repair` starts exactly one shift on zero-health machinery.
    // Once that shift restarts the component, it returns to the economy;
    // the former Restore doctrine was cut after spending twice the poles
    // without preventing another wreck. Summing all missing hit points
    // here would quietly preserve that removed policy in the readout.
    //
    // The result was a bill nobody could pay. A tower would sit at 98%
    // whole reporting "15 poles of mending outstanding" for the rest of
    // the run, because most of that was scratches the crew were right to
    // leave — and the number is presentation only, read by the roster
    // readout and by the agent tools, both of which phrase it as
    // something the player owes. Found by pricing the backlog per day
    // (`examples/orders.rs`) and noticing days three and four spent
    // nothing while the figure stayed put.
    //
    // What is left on the tower below the threshold is still visible:
    // the cross-section carries the mark, which `pick_repair`'s own note
    // calls the health readout doing its job.
    let per_shift = content.balance.siege.repair_hp_per_shift;
    let worth = |health: &crate::state::Health| if health.hp == 0 { per_shift } else { 0 };
    let mut queued_hp = 0i64;
    for floor in &state.tower.floors {
        queued_hp += worth(&floor.panel);
        for room in &floor.rooms {
            queued_hp += worth(&room.health);
        }
    }
    for shaft in &state.tower.shafts {
        queued_hp += worth(&shaft.health);
    }
    queued_hp * content.balance.siege.repair_poles_per_10_hp / 10
}

/// The item repairs are paid in. Exposed so the UI can name it without
/// hardcoding the string in two places.
#[must_use]
pub fn repair_item(content: &Content) -> Option<ItemIdx> {
    content.item_idx("item.poles")
}
