//! Hauling — the crew move things, and the shafts decide how fast.
//!
//! This is the system the whole design rests on. In Factorio a belt
//! serves one lane forever; here every chain you add loads the same
//! stairs everybody else is using. A crew member's trip is an L: walk
//! to the shaft column, climb, walk to the destination. The shaft has a
//! capacity, so the second crew member waits — visibly, with a counter
//! the cross-section renders as stress.
//!
//! Two invariants worth stating because everything else assumes them:
//!
//! * **Nothing picked up is ever destroyed.** If a destination fills
//!   while a crew member is en route, they get re-tasked to somewhere
//!   else; if there is nowhere, they stand and hold it.
//! * **Two crew never chase the same crate.** Assignment subtracts what
//!   other crew have already committed to, at both ends of the trip.

use crate::content::{Content, ShaftKind};
use crate::fx::Fx;
use crate::ids::{DaypartIdx, FloorIdx, ItemIdx, RoomId, ShaftId, SlotIdx};
use crate::state::crew::HaulPickup;
use crate::state::{Crew, CrewState, Errand, GameState, HaulDestination, HaulTask, Job, Tower};

use super::SoundEvent;
use super::needs::{effective_ticks, practice_pct, work_pct};

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    // Move the crew out so each member can be advanced while the tower
    // is also borrowed mutably. `take` on a Vec is a pointer move.
    let mut crew = std::mem::take(&mut state.crew);
    let mut hauls = 0u64;
    let mut meals = 0u64;
    let daypart = state.clock.daypart(content);
    let queues = shaft_queues(&crew, &state.tower);
    // Repair spends poles off the shelves, and the assignment pass runs
    // with the crew moved out of state, so the figure comes with it.
    let poles = super::repair::repair_item(content).map_or(0, |item| state.stock_of(item));
    let awake_shift = super::needs::shift_now(state, content);
    // Working in the dark is the third penalty, and the one that
    // connects the rota to charge. `lit` is true all day, so this only
    // bites in a brown-out — not merely on a dark night.
    let lit = state.power.lit;

    for member in &mut crew {
        advance(
            member,
            &mut state.tower,
            content,
            &queues,
            daypart,
            lit,
            sounds,
            &mut hauls,
            &mut meals,
        );
    }

    assign_idle(
        &mut crew,
        &state.tower,
        &state.siege.enemies,
        content,
        &queues,
        daypart,
        poles,
        awake_shift,
        &state.work,
    );

    state.crew = crew;
    state.stats.hauls_completed += hauls;
    state.stats.meals_eaten += meals;
}

// ---------------------------------------------------------------------------
// Per-crew state machine
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn advance(
    crew: &mut Crew,
    tower: &mut Tower,
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
    lit: bool,
    sounds: &mut Vec<SoundEvent>,
    hauls: &mut u64,
    meals: &mut u64,
) {
    let balance = &content.balance.crew;
    // Starving, tired, and working unlit compose into one figure that
    // stretches the *duration* of every leg. See `needs::effective_ticks`
    // for why it is the duration and never the `Fx` step.
    //
    // **Practice at hauling multiplies straight into it**, and it is
    // sound to fold in here because every leg this figure reaches — the
    // walk, the climb, the loading and the unloading — is porter work.
    // The two states that are somebody else's job, `Repairing` and
    // `Shooing`, get their durations from `arrive` and never read this.
    let pct = work_pct(crew, content, lit) * practice_pct(crew, Job::Haul, content) / 100;

    match crew.state {
        CrewState::Idle => {
            // Assignment happens in a second pass so every crew member
            // sees the same picture of demand.
        }

        // **Standing at a post, and that is the whole of it.** No legs,
        // no queue, and `wait_ticks` held at zero — somebody working a
        // station is not blocked, they are exactly where they were sent,
        // and a red tint on them would make the only bottleneck
        // instrument in the game lie. Leaving is the needs system's job
        // (hunger and the rota outrank a posting) or the player's.
        CrewState::Manning { .. } => {
            crew.wait_ticks = 0;
        }

        // **Standing between a thief and the outbox.** Counts down and
        // then clears; `siege::run` is what notices somebody is there
        // and sends the creature off, because whether a crow leaves is
        // the siege's business and not the porter's.
        CrewState::Shooing { ticks_left } => {
            crew.wait_ticks = 0;
            if ticks_left > 0 {
                crew.state = CrewState::Shooing {
                    ticks_left: ticks_left - 1,
                };
                return;
            }
            crew.errand = None;
            crew.state = CrewState::Idle;
        }

        CrewState::Sleeping => {
            // Off shift. Rest accrues in `needs`; waking is the
            // assignment pass's job, since a woken crew member is just
            // somebody with nothing to do yet.
            crew.wait_ticks = 0;
        }

        CrewState::Eating { ticks_left } => {
            crew.wait_ticks = 0;
            if ticks_left > 0 {
                crew.state = CrewState::Eating {
                    ticks_left: ticks_left - 1,
                };
                return;
            }
            // The meal is taken on completion, not on arrival — so a
            // meal somebody else ate while this one sat down simply
            // isn't there, and the errand clears the same way a
            // `Loading` that lost its crate does.
            if take_meal(crew, tower, content) {
                crew.hunger = 0;
                *meals += 1;
                sounds.push(SoundEvent::MealServed);
            }
            crew.errand = None;
            crew.state = CrewState::Idle;
        }

        CrewState::Riding { .. } => {
            // Cargo. The transport system moves them and opens the
            // doors; there is nothing for the crew member to do.
            crew.wait_ticks = 0;
        }

        CrewState::Repairing { .. } => {
            // The repair system owns them until the job is done.
            crew.wait_ticks = 0;
        }

        CrewState::Walking { to_slot } => {
            crew.wait_ticks = 0;
            let target = Fx::from_int(i32::from(to_slot));
            let ticks = effective_ticks(balance.walk_ticks_per_slot, pct);
            let step = Fx::ratio(1, ticks.max(1) as i32);
            let arrived = if crew.slot_fx < target {
                crew.slot_fx += step;
                crew.slot_fx >= target
            } else {
                crew.slot_fx -= step;
                crew.slot_fx <= target
            };
            if arrived {
                crew.slot_fx = target;
                crew.state = resume(crew, tower, content, queues, daypart);
            }
        }

        CrewState::Boarding { shaft, to_floor } => {
            let kind = tower.shaft(shaft).map(|s| s.kind);
            // An errand is a reason to be in the queue too. Checking
            // only for a haul task left a crew member sent to mend
            // something bouncing between Boarding and Idle forever,
            // never actually climbing — and a meal and a bed queue for
            // the same stairs on the same terms.
            if crew.task.is_none() && crew.errand.is_none() {
                // Whatever they were headed for was demolished while
                // they queued. Step out of the line.
                crew.state = CrewState::Idle;
            } else if kind == Some(ShaftKind::Stairs) {
                if claim_shaft(tower, shaft) {
                    crew.wait_ticks = 0;
                    crew.state = CrewState::Climbing { shaft, to_floor };
                } else {
                    // The bottleneck, made visible. No dashboard needed.
                    crew.wait_ticks = crew.wait_ticks.saturating_add(1);
                }
            } else if kind.is_none() {
                // The shaft was demolished out from under them.
                crew.state = CrewState::Idle;
            } else {
                // Waiting for a car. Boarding is the transport system's
                // job; all that happens here is the wait accumulating,
                // which is what tints them red.
                crew.wait_ticks = crew.wait_ticks.saturating_add(1);
            }
        }

        CrewState::Climbing { shaft, to_floor } => {
            crew.wait_ticks = 0;
            let target = Fx::from_int(i32::from(to_floor));
            // A laden climber is slow, and an empty one is not. The
            // same sum is in `estimated_trip_ticks`; if these two ever
            // drift apart, crew choose a shaft on one number and pay
            // another, and the mistake is invisible from outside.
            let load = crew.carrying.map_or(0, |(_, count)| count.max(0) as u32);
            let ticks = effective_ticks(
                balance.climb_ticks_per_floor + load * balance.climb_ticks_per_item,
                pct,
            );
            let step = Fx::ratio(1, ticks.max(1) as i32);
            let arrived = if crew.floor_fx < target {
                crew.floor_fx += step;
                crew.floor_fx >= target
            } else {
                crew.floor_fx -= step;
                crew.floor_fx <= target
            };
            if arrived {
                crew.floor_fx = target;
                release_shaft(tower, shaft);
                crew.state = resume(crew, tower, content, queues, daypart);
            }
        }

        CrewState::Loading { ticks_left } => {
            crew.wait_ticks = 0;
            if ticks_left > 0 {
                crew.state = CrewState::Loading {
                    ticks_left: ticks_left - 1,
                };
                return;
            }
            if collect(crew, tower) {
                sounds.push(SoundEvent::Pickup);
                crew.state = next_leg(crew, tower, content, queues, daypart);
            } else {
                // Somebody else got there first, or the room emptied.
                crew.task = None;
                crew.state = CrewState::Idle;
            }
        }

        CrewState::Unloading { ticks_left } => {
            crew.wait_ticks = 0;
            if ticks_left > 0 {
                crew.state = CrewState::Unloading {
                    ticks_left: ticks_left - 1,
                };
                return;
            }
            let spilling = matches!(
                crew.task.as_ref().map(|task| task.destination),
                Some(HaulDestination::Spill { .. })
            );
            let delivered = deposit(crew, tower);
            if delivered > 0 {
                *hauls += 1;
                // A spill is not a delivery and must not sound like one.
                // Something the tower worked for has just been thrown
                // away, and the player should hear that happen.
                sounds.push(if spilling {
                    SoundEvent::Spill
                } else {
                    SoundEvent::Deliver
                });
            }
            crew.task = None;
            crew.state = CrewState::Idle;
            // Anything that didn't fit stays in hand; the assignment
            // pass will find it a new home next tick.
        }
    }
}

/// What a crew member does after finishing a leg — which depends on
/// whether they are hauling or on an errand.
fn resume(
    crew: &Crew,
    tower: &Tower,
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
) -> CrewState {
    match crew.errand {
        Some(errand) => errand_leg(crew, tower, content, queues, daypart, errand),
        None => next_leg(crew, tower, content, queues, daypart),
    }
}

/// Decide the next leg for a crew member who just finished one.
fn next_leg(
    crew: &Crew,
    tower: &Tower,
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
) -> CrewState {
    let balance = &content.balance.crew;
    let Some(task) = &crew.task else {
        return CrewState::Idle;
    };

    let (target_floor, target_slot) = if crew.is_carrying() {
        (task.to_floor, task.to_slot)
    } else {
        match task.pickup {
            Some(pickup) => (pickup.floor, pickup.slot),
            None => (task.to_floor, task.to_slot),
        }
    };

    let floor = crew.floor();
    let slot = crew.slot();

    if floor == target_floor {
        if slot == target_slot {
            return if crew.is_carrying() {
                CrewState::Unloading {
                    ticks_left: balance.unload_ticks,
                }
            } else {
                CrewState::Loading {
                    ticks_left: balance.load_ticks,
                }
            };
        }
        return CrewState::Walking {
            to_slot: target_slot,
        };
    }

    let load = crew.carrying.map_or(0, |(_, count)| count.max(0) as u32);
    let Some(shaft) = best_shaft(tower, content, queues, daypart, floor, target_floor, load) else {
        // Nothing spans this trip. The assignment pass filters for
        // reachability, so this is belt and braces.
        return CrewState::Idle;
    };
    let column = tower.shaft(shaft).map_or(0, |s| s.slot);
    if slot != column {
        return CrewState::Walking { to_slot: column };
    }
    CrewState::Boarding {
        shaft,
        to_floor: target_floor,
    }
}

/// Pick the shaft that gets this crew member up fastest.
///
/// The estimate does not have to be right — it has to be deterministic
/// and roughly sensible, so that a player who builds an elevator sees
/// the crew start using it, and a player whose staircase is jammed sees
/// them route around it.
fn best_shaft(
    tower: &Tower,
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
    from: FloorIdx,
    to: FloorIdx,
    // Items in the traveller's arms. Zero when the caller is only
    // asking whether *any* shaft spans the trip, since reachability is
    // not a function of what someone is holding.
    load: u32,
) -> Option<ShaftId> {
    tower
        .shafts
        .iter()
        .enumerate()
        // Nobody throws themselves down a chute, however fast it would
        // be.
        .filter(|(_, shaft)| shaft.kind != ShaftKind::Chute)
        .filter(|(_, shaft)| shaft.serves_trip(from, to, daypart))
        .min_by_key(|(index, shaft)| {
            let queued = queues.get(*index).copied().unwrap_or(0);
            (
                super::transport::estimated_trip_ticks(shaft, content, from, to, queued, load),
                // Stable tie-break, so two equal shafts don't flap.
                shaft.id.0,
            )
        })
        .map(|(_, shaft)| shaft.id)
}

/// How many crew are queued at each shaft, parallel to `tower.shafts`.
///
/// Computed once at the top of the tick and handed down, because a crew
/// member choosing a route needs to see the whole queue picture while
/// the borrow checker only lets them see themselves. Last tick's
/// picture is fine — and deterministic, which matters more.
fn shaft_queues(crew: &[Crew], tower: &Tower) -> Vec<u32> {
    tower
        .shafts
        .iter()
        .map(|shaft| {
            crew.iter()
                .filter(|member| {
                    matches!(member.state, CrewState::Boarding { shaft: at, .. } if at == shaft.id)
                })
                .count() as u32
        })
        .collect()
}

fn claim_shaft(tower: &mut Tower, id: ShaftId) -> bool {
    let Some(shaft) = tower.shaft_mut(id) else {
        return false;
    };
    if !shaft.has_room() {
        return false;
    }
    shaft.riders += 1;
    true
}

fn release_shaft(tower: &mut Tower, id: ShaftId) {
    if let Some(shaft) = tower.shaft_mut(id) {
        shaft.riders = shaft.riders.saturating_sub(1);
    }
}

/// Take the load out of the source room. False if it's no longer there.
fn collect(crew: &mut Crew, tower: &mut Tower) -> bool {
    let Some(task) = &crew.task else {
        return false;
    };
    let Some(pickup) = task.pickup else {
        return false;
    };
    let (item, wanted) = (task.item, task.amount);

    let Some(floor) = tower.floor_mut(pickup.floor) else {
        return false;
    };
    let Some(room) = floor.rooms.iter_mut().find(|r| r.id == pickup.room) else {
        return false;
    };
    // Outbox first, then the shelves. A storeroom is a buffer, not a
    // bin: whatever goes onto a shelf has to be able to come off it
    // again, or an item with no `take_stock` consumer — bamboo, scrap —
    // is stranded there for the rest of the run.
    let taken = room
        .outputs
        .iter_mut()
        .find(|s| s.item == item)
        .map_or(0, |stack| stack.withdraw(wanted));
    let taken = if taken > 0 {
        taken
    } else {
        room.unshelve(item, wanted)
    };
    if taken == 0 {
        return false;
    }
    crew.carrying = Some((item, taken));
    true
}

/// Put the load down. Returns how many items landed.
fn deposit(crew: &mut Crew, tower: &mut Tower) -> i64 {
    let Some((item, held)) = crew.carrying else {
        return 0;
    };
    let Some(task) = &crew.task else {
        return 0;
    };

    let placed = match task.destination {
        HaulDestination::Inbox { room, input } => {
            deposit_inbox(tower, task.to_floor, room, input as usize, item, held)
        }
        HaulDestination::Shelf { room } => deposit_shelf(tower, task.to_floor, room, item, held),
        // Down the chute and gone. The only place in the game where a
        // carried item is destroyed, and it is destroyed *deliberately*,
        // by a piece of infrastructure the player built and a crew
        // member who walked to it — which is what makes it a decision
        // rather than the silent drop `haul.rs`'s invariant forbids.
        HaulDestination::Spill { shaft } => {
            if tower
                .shaft(shaft)
                .is_some_and(|s| s.kind == ShaftKind::Chute)
            {
                held
            } else {
                // The chute was torn out while they walked to it. Keep
                // hold of the load; the assignment pass finds it
                // somewhere else, or they carry it.
                0
            }
        }
    };

    let left = held - placed;
    crew.carrying = if left > 0 { Some((item, left)) } else { None };
    placed
}

fn deposit_inbox(
    tower: &mut Tower,
    floor: FloorIdx,
    room_id: RoomId,
    input: usize,
    item: ItemIdx,
    amount: i64,
) -> i64 {
    let Some(floor) = tower.floor_mut(floor) else {
        return 0;
    };
    let Some(room) = floor.rooms.iter_mut().find(|r| r.id == room_id) else {
        return 0;
    };
    let Some(stack) = room.inputs.get_mut(input) else {
        return 0;
    };
    if stack.item != item {
        return 0;
    }
    stack.deposit(amount)
}

fn deposit_shelf(
    tower: &mut Tower,
    floor: FloorIdx,
    room_id: RoomId,
    item: ItemIdx,
    amount: i64,
) -> i64 {
    let Some(floor) = tower.floor_mut(floor) else {
        return 0;
    };
    let Some(room) = floor.rooms.iter_mut().find(|r| r.id == room_id) else {
        return 0;
    };
    room.shelve(item, amount)
}

// ---------------------------------------------------------------------------
// Task assignment
// ---------------------------------------------------------------------------

/// Priority of feeding a live recipe. Beats stockpiling, always.
const PRIORITY_INBOX: i64 = 3;
/// Priority of putting something on a shelf.
const PRIORITY_SHELF: i64 = 2;
/// Priority of throwing something away.
///
/// Below a shelf, so **a tower with anywhere useful to put a load never
/// spills one.** A chute is where something goes when there is nowhere
/// for it to go, and nothing else — which is what keeps it an escape
/// hatch rather than a policy.
const PRIORITY_SPILL: i64 = 1;

/// The priority ladder for an idle crew member:
///
/// 1. a task already under way — pick the trip back up rather than
///    re-deciding it
/// 2. a load in hand with somewhere to put it — finish the delivery;
///    nothing carried is ever dropped
/// 3. off shift — go to a bunk, or lie down where they are
/// 4. past `hungry_ticks` — go and eat
/// 5. damage worth a shift — mend it
/// 6. a haul
///
/// Eating above mending is deliberate: a crew member past
/// `starving_ticks` mends slowly too, and a meal is 300 ticks against a
/// repair shift's 80 plus the walk. Feeding them first is the cheaper
/// order.
///
/// Off shift sits *below* a task already under way, and that is what
/// holds the invariant that a crew member never falls asleep holding
/// something: going off shift stops them taking new work, and they head
/// for a bunk once their hands are empty.
#[allow(clippy::too_many_arguments)]
fn assign_idle(
    crew: &mut [Crew],
    tower: &Tower,
    enemies: &[crate::state::siege::Enemy],
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
    poles: i64,
    awake_shift: crate::content::Shift,
    work: &[Job],
) {
    for i in 0..crew.len() {
        let off_shift = crew[i].shift != awake_shift;

        // A sleeper whose shift has come round wakes up — and only
        // then. **Crew are never woken automatically.** An attack does
        // not rouse anybody: if the simulation woke people when things
        // got bad, the rota would be decorative and the interesting
        // decision — do I burn tomorrow morning to answer tonight —
        // would be made by the game instead of the player.
        if crew[i].is_asleep() {
            if off_shift {
                continue;
            }
            crew[i].errand = None;
            crew[i].state = CrewState::Idle;
        }

        if !matches!(crew[i].state, CrewState::Idle) {
            continue;
        }

        // Idle but still committed: they have just stepped off a car
        // part-way through a journey. Pick the trip back up rather than
        // re-deciding it, or a crew member could ride an elevator and
        // then immediately choose a different errand.
        if crew[i].task.is_some() {
            let next = next_leg(&crew[i], tower, content, queues, daypart);
            crew[i].state = next;
            continue;
        }

        // On the way somewhere, or standing on the spot.
        if let Some(errand) = crew[i].errand {
            // A meal that went while they walked. Clear and look again
            // rather than standing over an empty outbox.
            if matches!(errand, Errand::Meal { .. }) && find_meal(tower, content, 0).is_none() {
                crew[i].errand = None;
            } else {
                crew[i].state = errand_leg(&crew[i], tower, content, queues, daypart, errand);
                continue;
            }
        }

        // **Stranded carriers may do anything except put the load
        // down**, and that rule governs the whole of the rest of this
        // ladder.
        //
        // "Hands empty" is the ordinary condition for going to bed,
        // going to eat, or picking up a repair, because a load in hand
        // is a trip half-finished and nothing carried is ever dropped.
        // But a tower with no free shelf and no hungry room — the
        // shelf-typing deadlock `BALANCE.md`'s `storeroom` row
        // describes — strands whoever is holding something, and a
        // stranded carrier has no trip to finish. Gating the rest of
        // the ladder on empty hands then means somebody who never
        // sleeps and never eats again: permanently tired, permanently
        // starving, working at 36% for the rest of the run, with the
        // deadlock as the invisible cause. That reads as a bug rather
        // than as a consequence of the deadlock it actually is.
        //
        // The precedent is already here: `pick_repair` below has let a
        // stranded carrier mend while holding a crate since M3,
        // measured, because the alternative took repair down to a
        // quarter of what the same tower managed with room to spare.
        // Sleep and meals join it on the same terms. The load stays in
        // their hands throughout and is delivered when somewhere opens
        // up, which is the invariant that actually matters.
        let stranded = crew[i].is_carrying()
            && crew[i].carrying.is_some_and(|(item, held)| {
                // A chute counts here: somebody holding something they
                // can throw away is not stranded.
                find_destination(tower, content, crew, i, item, held, false, true).is_none()
            });
        let free_to_choose = !crew[i].is_carrying() || stranded;

        // Off shift. Bed if there is one; the deck if not.
        if off_shift && free_to_choose {
            crew[i].wait_ticks = 0;
            crew[i].errand = find_bunk(tower, content, crew, i, crew[i].floor());
            crew[i].state = match crew[i].errand {
                Some(errand) => errand_leg(&crew[i], tower, content, queues, daypart, errand),
                // Nowhere to lie down, so they lie down here. Visible,
                // drawn as such, and refilling rest at half the rate a
                // bed would — survivable and visibly degrading.
                None => CrewState::Sleeping,
            };
            continue;
        }

        // Hungry. An errand outranking a new haul, but never a delivery
        // that could still be finished.
        if crew[i].hunger >= crate::systems::needs::hungry_ticks(&crew[i], content)
            && free_to_choose
            && let Some(errand) = find_meal(tower, content, crew[i].floor())
        {
            crew[i].wait_ticks = 0;
            crew[i].errand = Some(errand);
            crew[i].state = errand_leg(&crew[i], tower, content, queues, daypart, errand);
            continue;
        }

        // **The four jobs, in whatever order the player put them.**
        // Everything above this line is fixed and always outranks them:
        // a trip already under way, the rota, and dinner. Those are not
        // jobs and are not offered as settings — see `Job`.
        //
        // Note where `free_to_choose` bites. Somebody carrying something
        // that has a home to go to fails every arm but the haul,
        // whatever the order says, which is what keeps the order a
        // preference about *what to start* rather than a licence to put
        // a crate down in a corridor.
        let mut chose = false;
        for job in work {
            match job {
                // A crow in the outbox is taking something right now,
                // which is the argument for it going first by default: a
                // wrecked panel has already happened and will still be
                // there in a minute.
                Job::Answer => {
                    if free_to_choose
                        && let Some((floor, slot)) =
                            super::siege::thief_at_work(enemies, content, crew)
                    {
                        let errand = Errand::Shoo { floor, slot };
                        crew[i].errand = Some(errand);
                        crew[i].wait_ticks = 0;
                        crew[i].state =
                            errand_leg(&crew[i], tower, content, queues, daypart, errand);
                        chose = true;
                    }
                }

                // Never above a delivery already under way: putting a
                // load down where it does not belong in order to go and
                // mend a wall would lose the load. A stranded carrier
                // mends while holding it, per `free_to_choose` above —
                // mending costs the load nothing, since `repair::run`
                // never touches `carrying`.
                Job::Mend => {
                    if free_to_choose
                        && let Some((target, floor, slot)) = super::repair::pick_repair(
                            tower,
                            content,
                            poles,
                            crew,
                            i,
                            crew[i].floor(),
                        )
                    {
                        let errand = Errand::Repair {
                            target,
                            floor,
                            slot,
                        };
                        crew[i].errand = Some(errand);
                        crew[i].wait_ticks = 0;
                        crew[i].state =
                            errand_leg(&crew[i], tower, content, queues, daypart, errand);
                        chose = true;
                    }
                }

                // A standing order about what somebody does with their
                // working day. Above hauling by default because that is
                // the entire trade the player is making — somebody at a
                // post is somebody not on the stairs — and a player who
                // ranks hauling higher is saying *the post can wait
                // until the shelves are clear*, which is a real thing to
                // want and is visible the moment they say it.
                //
                // The room is looked up fresh rather than cached with
                // the order: a room can be demolished under somebody's
                // feet, and an id that no longer resolves simply ends
                // the posting.
                Job::Man => {
                    if free_to_choose && let Some(room) = crew[i].stationed {
                        match tower.locate(room) {
                            Some((floor, slot)) => {
                                let errand = Errand::Station { room, floor, slot };
                                crew[i].errand = Some(errand);
                                crew[i].wait_ticks = 0;
                                crew[i].state =
                                    errand_leg(&crew[i], tower, content, queues, daypart, errand);
                                chose = true;
                            }
                            None => crew[i].stationed = None,
                        }
                    }
                }

                Job::Haul => {
                    let task = if let Some((item, held)) = crew[i].carrying {
                        // Already holding something: find it a home
                        // rather than picking up more. Nothing is ever
                        // dropped on the floor. This is a re-home rather
                        // than a pickup, so a shelf is a legitimate
                        // destination — and so is a chute, which is the
                        // escape hatch for a carrier the tower has no
                        // room for.
                        find_destination(tower, content, crew, i, item, held, false, true).map(
                            |(destination, to_floor, to_slot, _)| HaulTask {
                                item,
                                amount: held,
                                pickup: None,
                                to_floor,
                                to_slot,
                                destination,
                            },
                        )
                    } else {
                        pick_task(tower, content, crew, i, queues, daypart)
                    };
                    if let Some(task) = task {
                        crew[i].wait_ticks = 0;
                        crew[i].task = Some(task);
                        let next = next_leg(&crew[i], tower, content, queues, daypart);
                        crew[i].state = next;
                        chose = true;
                    }
                }
            }
            if chose {
                break;
            }
        }

        if !chose {
            // Nothing to do is not stress. `wait_ticks` drives the red
            // tint, and a crew member standing around because the
            // tower has no work is telling the player something quite
            // different from one stuck at the foot of a jammed
            // staircase — conflating them makes the only bottleneck
            // instrument in the game lie.
            crew[i].wait_ticks = 0;
        }
    }
}

/// Route a crew member to wherever their errand is, then set them to
/// work on it. Reuses the haul legs exactly — which is why a severed
/// shaft can put damage, a meal or a bed out of reach, and why that is
/// correct rather than a bug.
///
/// One router for all three arms rather than three parallel `Option`s
/// alongside `task`, which would duplicate this routing three times.
fn errand_leg(
    crew: &Crew,
    tower: &Tower,
    content: &Content,
    queues: &[u32],
    daypart: DaypartIdx,
    errand: Errand,
) -> CrewState {
    let floor = crew.floor();
    let slot = crew.slot();

    // An errand whose room went away — a bunk demolished under a
    // sleeper, a canteen removed mid-meal — clears rather than spinning,
    // exactly as `Boarding` already handles a demolished shaft.
    if let Some(room) = errand.room()
        && !tower
            .floor(errand.floor())
            .is_some_and(|f| f.rooms.iter().any(|r| r.id == room))
    {
        return CrewState::Idle;
    }

    if floor == errand.floor() {
        if slot == errand.slot() {
            return arrive(crew, content, errand);
        }
        return CrewState::Walking {
            to_slot: errand.slot(),
        };
    }

    let load = crew.carrying.map_or(0, |(_, count)| count.max(0) as u32);
    let Some(shaft) = best_shaft(tower, content, queues, daypart, floor, errand.floor(), load)
    else {
        // Cut off from it. Stand down rather than spin; the assignment
        // pass will try again once a route exists.
        return CrewState::Idle;
    };
    let column = tower.shaft(shaft).map_or(0, |s| s.slot);
    if slot != column {
        return CrewState::Walking { to_slot: column };
    }
    CrewState::Boarding {
        shaft,
        to_floor: errand.floor(),
    }
}

/// What standing on the spot means, per errand.
///
/// **Two of the five get shorter with practice, and three do not.** A
/// repair shift and a shooing are work, and somebody who has done a lot
/// of either is quicker at it. A meal, a bed and a post are not: eating
/// faster is not a skill anybody wants modelled, sleep is the one thing
/// in the game that is deliberately not optimisable, and a post has no
/// duration at all — what practice buys a stationed worker is in
/// `production`, not here.
fn arrive(crew: &Crew, content: &Content, errand: Errand) -> CrewState {
    match errand {
        Errand::Repair { target, .. } => CrewState::Repairing {
            target,
            ticks_left: effective_ticks(
                super::repair::shift_ticks(content, content.balance.siege.repair_hp_per_shift),
                practice_pct(crew, Job::Mend, content),
            ),
        },
        // A meal takes as long as the canteen takes to cook one. Not
        // scaled by `work_pct`: eating is the cure for being slow, not
        // an instance of it, and a starving crew member who ate more
        // slowly would be punished twice for the same failure.
        Errand::Meal { .. } => CrewState::Eating {
            ticks_left: meal_ticks(content),
        },
        Errand::Bunk { .. } => CrewState::Sleeping,
        // No `ticks_left`, unlike every other arm: a posting is
        // open-ended and ends when the player ends it, the room goes, or
        // a need pulls them away.
        Errand::Station { room, .. } => CrewState::Manning { room },
        // Long enough to be a real commitment of somebody's time and
        // short enough that a thief is not answered by a porter lost for
        // the rest of the day.
        Errand::Shoo { .. } => CrewState::Shooing {
            ticks_left: effective_ticks(
                content.balance.siege.shoo_ticks,
                practice_pct(crew, Job::Answer, content),
            ),
        },
    }
}

/// How long sitting down to a meal takes. Read off the canteen's own
/// craft time so a designer who makes cooking slower makes eating look
/// slower too, rather than the two drifting apart.
fn meal_ticks(content: &Content) -> u32 {
    meal_item(content)
        .and_then(|item| {
            content
                .room_runtime
                .iter()
                .find(|room| room.recipe_outputs.iter().any(|(out, _, _)| *out == item))
                .map(|room| room.craft_ticks)
        })
        .unwrap_or(300)
}

/// The item a meal is. Named once so no system compares the string.
fn meal_item(content: &Content) -> Option<ItemIdx> {
    content.item_idx("item.meals")
}

/// Take one meal out of the room the errand names. False if it is gone
/// — somebody else got there first, or the room emptied.
fn take_meal(crew: &Crew, tower: &mut Tower, content: &Content) -> bool {
    let Some(Errand::Meal { room, floor, .. }) = crew.errand else {
        return false;
    };
    let Some(item) = meal_item(content) else {
        return false;
    };
    let Some(floor) = tower.floor_mut(floor) else {
        return false;
    };
    let Some(room) = floor.rooms.iter_mut().find(|r| r.id == room) else {
        return false;
    };
    // Outbox first, then the shelves — the same order a haul collects
    // in, and the reason the kitchen chain survives a fully-claimed
    // storeroom: eating at the source means distribution to shelves is
    // an optimisation, not a requirement.
    let taken = room
        .outputs
        .iter_mut()
        .find(|s| s.item == item)
        .map_or(0, |stack| stack.withdraw(1));
    taken > 0 || room.unshelve(item, 1) > 0
}

/// Where the nearest meal is, and where to stand to eat it.
///
/// Not a reservation system on purpose: two hungry crew may both walk to
/// the same last meal, and the one who arrives second finds it gone and
/// looks again. That is the failure path `Loading` already has, and
/// adding bookkeeping to prevent it would buy a rare case at the cost of
/// a permanent one.
fn find_meal(tower: &Tower, content: &Content, from: FloorIdx) -> Option<Errand> {
    let item = meal_item(content)?;
    let mut best: Option<(u16, FloorIdx, SlotIdx, RoomId)> = None;
    for floor in &tower.floors {
        for room in &floor.rooms {
            let has = room.outputs.iter().any(|s| s.item == item && s.count > 0)
                || room
                    .shelves
                    .iter()
                    .any(|shelf| shelf.item == Some(item) && shelf.count > 0);
            if !has {
                continue;
            }
            let distance = floor.index.abs_diff(from);
            let candidate = (
                u16::from(distance),
                floor.index,
                room.outbox_slot(),
                room.id,
            );
            if best.is_none_or(|current| candidate < current) {
                best = Some(candidate);
            }
        }
    }
    best.map(|(_, floor, slot, room)| Errand::Meal { room, floor, slot })
}

/// A bed with space in it, or `None` for nowhere to lie down.
///
/// **Occupancy is derived, never stored.** How many sleepers a bunk
/// holds is counted by scanning the crew whose errand names it — the
/// same trick `shaft_queues` uses to give one crew member a picture of
/// the whole queue while the borrow checker only lets them see
/// themselves. `Room` gains no field, and there is no counter to get out
/// of step with reality. Beds are claimed in crew order, which is
/// creation order, which is `CrewId` order, so who gets the last bed is
/// a pure function of state.
fn find_bunk(
    tower: &Tower,
    content: &Content,
    crew: &[Crew],
    me: usize,
    from: FloorIdx,
) -> Option<Errand> {
    let mut best: Option<(u16, FloorIdx, SlotIdx, RoomId)> = None;
    for floor in &tower.floors {
        for room in &floor.rooms {
            let beds = content
                .room_runtime
                .get(room.def.0 as usize)
                .map_or(0, |def| def.sleepers);
            if beds == 0 {
                continue;
            }
            let claimed = crew
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != me)
                .filter(|(_, other)| {
                    other
                        .errand
                        .is_some_and(|e| e.is_bunk() && e.room() == Some(room.id))
                })
                .count();
            if claimed >= usize::from(beds) {
                continue;
            }
            let distance = floor.index.abs_diff(from);
            let candidate = (u16::from(distance), floor.index, room.slot, room.id);
            if best.is_none_or(|current| candidate < current) {
                best = Some(candidate);
            }
        }
    }
    best.map(|(_, floor, slot, room)| Errand::Bunk { room, floor, slot })
}

/// Score every collectable pile against every valid destination and
/// take the best. Ties break on the lowest (pickup, dropoff) position
/// so the choice is a pure function of state.
fn pick_task(
    tower: &Tower,
    content: &Content,
    crew: &[Crew],
    me: usize,
    queues: &[u32],
    daypart: DaypartIdx,
) -> Option<HaulTask> {
    // A harness is one more thing in the arms, for the one person
    // wearing it — `DESIGN.md` insight 1 answered by equipment rather
    // than by architecture, and much smaller than a shaft.
    // A kit is lent and can be taken back; a trait came aboard with the
    // person (`SYSTEMS.md` §6.25). They stack, and a trait's bonus may
    // be *negative* — somebody who carries less is somebody you post to
    // a room rather than leave on the stairs.
    let carry_bonus = crew[me]
        .kit
        .and_then(|item| content.item(item).kit.as_ref())
        .map_or(0, |kit| kit.carry_bonus)
        + crew[me].trait_carry_bonus(content);
    let capacity = (content.balance.crew.carry_capacity + carry_bonus).max(1);
    let from_floor = crew[me].floor();
    let from_slot = crew[me].slot();

    let mut best: Option<(i64, HaulTask)> = None;

    for floor in &tower.floors {
        for room in &floor.rooms {
            // Outboxes, then shelves. A shelf pickup may only feed a
            // room that eats the item — see `find_destination`'s
            // `inbox_only`. Without that a crew member would happily
            // carry bamboo from one shelf to another for ever.
            let piles = room
                .outputs
                .iter()
                .map(|stack| (stack.item, stack.count, false))
                .chain(
                    room.shelves
                        .iter()
                        .filter_map(|shelf| Some((shelf.item?, shelf.count, true))),
                );
            for (pile_item, pile_count, from_shelf) in piles {
                let committed = committed_pickup(crew, me, room.id, pile_item);
                let available = pile_count - committed;
                if available <= 0 {
                    continue;
                }
                if from_floor != floor.index
                    && best_shaft(tower, content, queues, daypart, from_floor, floor.index, 0)
                        .is_none()
                {
                    continue;
                }

                let Some((destination, to_floor, to_slot, priority)) = find_destination(
                    tower,
                    content,
                    crew,
                    me,
                    pile_item,
                    available.min(capacity),
                    from_shelf,
                    // Only what is already on a shelf may be thrown
                    // away. A fresh pickup from an outbox never can, or
                    // the room it came from never stalls.
                    from_shelf,
                ) else {
                    continue;
                };
                if floor.index != to_floor
                    && best_shaft(tower, content, queues, daypart, floor.index, to_floor, 0)
                        .is_none()
                {
                    continue;
                }

                let amount = available.min(capacity);
                let travel = travel_cost(
                    from_floor,
                    from_slot,
                    floor.index,
                    room.outbox_slot(),
                    to_floor,
                    to_slot,
                );
                let score = priority * 1000 - travel;

                let task = HaulTask {
                    item: pile_item,
                    amount,
                    pickup: Some(HaulPickup {
                        room: room.id,
                        floor: floor.index,
                        slot: room.outbox_slot(),
                    }),
                    to_floor,
                    to_slot,
                    destination,
                };

                let better = match &best {
                    None => true,
                    Some((best_score, best_task)) => {
                        score > *best_score || (score == *best_score && tie_break(&task, best_task))
                    }
                };
                if better {
                    best = Some((score, task));
                }
            }
        }
    }

    best.map(|(_, task)| task)
}

/// Deterministic tie-break: lowest pickup floor, then slot, then
/// dropoff floor, then slot.
fn tie_break(candidate: &HaulTask, incumbent: &HaulTask) -> bool {
    let key = |t: &HaulTask| {
        let p = t.pickup.map_or((u8::MAX, u8::MAX), |p| (p.floor, p.slot));
        (p.0, p.1, t.to_floor, t.to_slot, t.item.0)
    };
    key(candidate) < key(incumbent)
}

/// Manhattan-ish cost: floors are twice as expensive as slots, because
/// climbing is slower and contends for a shared shaft.
fn travel_cost(
    from_floor: FloorIdx,
    from_slot: SlotIdx,
    pick_floor: FloorIdx,
    pick_slot: SlotIdx,
    to_floor: FloorIdx,
    to_slot: SlotIdx,
) -> i64 {
    let df = |a: FloorIdx, b: FloorIdx| i64::from(a.abs_diff(b));
    let ds = |a: SlotIdx, b: SlotIdx| i64::from(a.abs_diff(b));
    2 * (df(from_floor, pick_floor) + df(pick_floor, to_floor))
        + ds(from_slot, pick_slot)
        + ds(pick_slot, to_slot)
}

/// Best home for `amount` of `item`: a hungry recipe first, a shelf
/// second. Returns the destination plus its priority.
#[allow(clippy::too_many_arguments)]
fn find_destination(
    tower: &Tower,
    content: &Content,
    crew: &[Crew],
    me: usize,
    item: ItemIdx,
    amount: i64,
    inbox_only: bool,
    may_spill: bool,
) -> Option<(HaulDestination, FloorIdx, SlotIdx, i64)> {
    let mut best: Option<(i64, i64, HaulDestination, FloorIdx, SlotIdx)> = None;

    for floor in &tower.floors {
        for room in &floor.rooms {
            // 1. A recipe that eats this item and has buffer space.
            for (index, stack) in room.inputs.iter().enumerate() {
                if stack.item != item {
                    continue;
                }
                let destination = HaulDestination::Inbox {
                    room: room.id,
                    input: index as u8,
                };
                let inbound = committed_delivery(crew, me, destination, item);
                let space = stack.space() - inbound;
                if space < amount.min(1) || space <= 0 {
                    continue;
                }
                consider(
                    &mut best,
                    PRIORITY_INBOX,
                    space,
                    destination,
                    floor.index,
                    room.slot,
                );
            }

            // 2. Shelf space — but never for something already on a
            // shelf. A shelf-to-shelf haul moves nothing anywhere and
            // would loop for ever; a shelf pickup exists only to feed
            // a room that eats the item.
            if !room.shelves.is_empty() && !inbox_only {
                let destination = HaulDestination::Shelf { room: room.id };
                let inbound = committed_delivery(crew, me, destination, item);
                let space = room.shelf_space_for(item) - inbound;
                if space > 0 {
                    consider(
                        &mut best,
                        PRIORITY_SHELF,
                        space,
                        destination,
                        floor.index,
                        room.slot,
                    );
                }
            }
        }
    }

    // 3. A chute — but only for something the tower has no use for.
    //
    // **The escape hatch for a jammed tower.** `BALANCE.md`'s
    // `storeroom` row has described the deadlock since M2: a shelf takes
    // whichever item lands on it first, so a material arriving faster
    // than it is consumed claims shelf after shelf until nothing else
    // can be put down and the chain stops — permanently, because
    // affording the way out needs the poles that are stuck in the mill.
    //
    // **`wanted` is what makes this safe, and the first version without
    // it was a disaster.** Offered to anything that merely had nowhere
    // to go *right now*, crew threw the economy away: measured, a tower
    // whose shelves had been squatted by fiber and rope spilled every
    // stalk of bamboo the arm cut and every pole the mill made, because
    // those were the loads in hand and the shelves were full of the
    // things that had caused the jam. The chute destroyed the useful
    // materials and left the useless ones sitting there.
    //
    // So: **you may only throw away what nothing in the tower wants.**
    // An item is spillable when no room has a live inbox for it — no
    // mill for bamboo, no thornwright for poles, no ropery for fiber.
    // That is legible as a rule ("the chute takes what nobody needs"),
    // it makes a chute safe to leave standing, and it means switching a
    // room off is also how you tell the tower to stop hoarding its
    // input. Rope, whose only consumer is a one-off build cost, is
    // spillable always — which is exactly right.
    //
    // **`may_spill` is what keeps a chute a relief valve instead of a
    // drain, and getting it wrong cost the game its best mechanic.**
    //
    // A spill is offered for a load taken off a *shelf* — clearing a
    // stockpile nothing wants, which is the jam this exists to fix — and
    // for a load already in hand with nowhere to go. It is **never**
    // offered for a load a crew member has just collected from a room's
    // outbox, and that exclusion is the whole of the rule.
    //
    // Without it, a chute quietly disables backpressure. A ropery makes
    // rope, nothing consumes rope, crew carry it straight from the
    // ropery's outbox to the chute, the outbox therefore never fills,
    // the ropery therefore never stalls, and it goes on eating fiber
    // for ever — so a comb goes on harvesting fiber for ever, and crew
    // spend the run ferrying a dead chain into the jungle while the mill
    // starves.
    //
    // A full outbox stalling its room is the oldest rule in the chain
    // (`intake.rs`, `production.rs`) and it is how a tower tells you a
    // branch is pointless. A chute must not be able to answer that
    // question on the player's behalf.
    //
    // What the shelf side is worth, from `examples/journey.rs` totalled
    // over twelve seeds: a tower with a comb and a ropery and no chute
    // harvests **2,924 bamboo against a bare tower's 6,502** — the dead
    // branch costs it 55% of its harvest — and a chute recovers that to
    // **3,970, a third of the way back**. Adding *more crew* made it
    // worse rather than better, which is the tell that this was never
    // crew scarcity.
    //
    // Still safe on the shelf side because of `wanted`: crew empty a
    // shelf of something nothing eats, and never touch a shelf of
    // something a live room is waiting for.
    // Wanted by a live inbox, **or by anything the player could still
    // build**. The second half is not optional: rope's only consumer is
    // a build cost, so without it a chute cheerfully threw away every
    // coil the ropery made and an elevator became unbuildable in a tower
    // that had a chute — which is a worse failure than the one the chute
    // was added to fix. The same is true of poles the moment the last
    // thornwright is switched off.
    //
    // Read off the content pack rather than off any pending intent,
    // because there is no such thing as a pending intent here: a player
    // decides to build by sending a command, and until then the only
    // honest statement is "this is a material the tower builds with".
    let wanted = tower
        .floors
        .iter()
        .flat_map(|f| f.rooms.iter())
        .any(|room| room.active && room.inputs.iter().any(|stack| stack.item == item))
        || content.builds_with(item)
        // **Worth keeping only up to what the whole run's settlements
        // will ever take.** Below that it is salvage; above it, it is
        // more than anybody will buy, and the tower is better off
        // without it. Counted across shelves only — what is in a room's
        // inbox is already spoken for, and what is in a hand is the load
        // being decided about.
        || shelved(tower, item) <= content.settlements_take(item);
    if best.is_none() && !wanted && may_spill {
        for shaft in &tower.shafts {
            if shaft.kind != ShaftKind::Chute {
                continue;
            }
            let inbound =
                committed_delivery(crew, me, HaulDestination::Spill { shaft: shaft.id }, item);
            // A chute never fills, but two crew both routing to one is
            // still two loads thrown away where one would have done.
            if inbound > 0 {
                continue;
            }
            consider(
                &mut best,
                PRIORITY_SPILL,
                amount,
                HaulDestination::Spill { shaft: shaft.id },
                shaft.low,
                shaft.slot,
            );
        }
    }

    let _ = content;
    best.map(|(priority, _, destination, floor, slot)| (destination, floor, slot, priority))
}

/// Keep the highest-priority destination; within a priority, the
/// emptiest; ties by lowest position.
fn consider(
    best: &mut Option<(i64, i64, HaulDestination, FloorIdx, SlotIdx)>,
    priority: i64,
    space: i64,
    destination: HaulDestination,
    floor: FloorIdx,
    slot: SlotIdx,
) {
    let candidate = (priority, space, destination, floor, slot);
    let better = match best {
        None => true,
        Some((bp, bs, _, bf, bsl)) => {
            (priority, space) > (*bp, *bs)
                || ((priority, space) == (*bp, *bs) && (floor, slot) < (*bf, *bsl))
        }
    };
    if better {
        *best = Some(candidate);
    }
}

/// How much of `item` other crew have already promised to take out of
/// `room`. Prevents two crew chasing the same crate.
///
/// Crew who have already collected don't count: their task still names
/// the source room, but the items are in their hands and out of the
/// stack. Counting them would keep the pile looking spoken-for long
/// after it refilled.
fn committed_pickup(crew: &[Crew], me: usize, room: RoomId, item: ItemIdx) -> i64 {
    crew.iter()
        .enumerate()
        .filter(|(i, other)| *i != me && !other.is_carrying())
        .filter_map(|(_, other)| other.task.as_ref())
        .filter(|task| task.item == item)
        .filter_map(|task| task.pickup.map(|p| (p.room, task.amount)))
        .filter(|(pickup_room, _)| *pickup_room == room)
        .map(|(_, amount)| amount)
        .sum()
}

/// How much of `item` is already inbound to `destination`. Prevents
/// overfilling a buffer that two crew both targeted.
/// How much of `item` is sitting on the tower's shelves.
///
/// Shelves only. A room's inbox is stock already promised to that room's
/// recipe, and counting it would let a full mill make the tower think it
/// was richer than it is.
fn shelved(tower: &Tower, item: ItemIdx) -> i64 {
    tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .flat_map(|room| room.shelves.iter())
        .filter(|shelf| shelf.item == Some(item))
        .map(|shelf| shelf.count)
        .sum()
}

fn committed_delivery(
    crew: &[Crew],
    me: usize,
    destination: HaulDestination,
    item: ItemIdx,
) -> i64 {
    crew.iter()
        .enumerate()
        .filter(|(i, _)| *i != me)
        .filter_map(|(_, other)| other.task.as_ref())
        .filter(|task| task.item == item && task.destination == destination)
        .map(|task| task.amount)
        .sum()
}
