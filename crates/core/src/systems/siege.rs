//! Creatures, damage, and provocation.
//!
//! There is no encounter and no arena. Waves arrive on the terrain
//! layer the tower is already walking through, and what they attack is
//! the infrastructure the player built — a panel, a room, a shaft
//! column. That last one is the point of the whole milestone: a severed
//! shaft splits the tower's circulation, and the reroute falls out of
//! `best_shaft` refusing to route through a severed column rather than
//! out of any special case.
//!
//! Runs before `defence`, so emplacements shoot at creatures that have
//! already moved this tick rather than at where they used to be.

use crate::content::{Approach, Content, EnemyEncounter};
use crate::fx::{Fx, Paces, paces_from_fx, paces_from_int, paces_to_int};
use crate::ids::{EnemyId, EnemyIdx, FloorIdx, SlotIdx};
use crate::state::siege::{DamageTarget, Enemy, EnemyState};
use crate::state::{GameState, Health};

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    decay_provocation(state, content);
    maybe_spawn_wave(state, content, sounds);
    advance_enemies(state, content, sounds);
    // After the creatures have moved and before they are reaped, so a
    // thief that arrives this tick and finds somebody already standing
    // there leaves on the same tick rather than getting one free bite.
    shoo_thieves(state, content);
    reap(state);
}

// ---------------------------------------------------------------------------
// Provocation
// ---------------------------------------------------------------------------

/// Attention bleeds off on its own. Everything that raises it —
/// stripping the terrain, burner smoke — is called from the system that
/// does the provoking, so the cost lands next to the act.
///
/// **Unconditional, and it was worth finding out why.** This used to say
/// "walking quietly bleeds attention off", which reads as though a
/// stopped tower should keep what it has drawn — and `DECISIONS.md` §11
/// makes striding the free answer to a wave, so standing still not being
/// free is an appealing symmetry.
///
/// Measured, gating the decay on `strode` is pathological. A berthed
/// tower never sheds, so provocation climbs to the ceiling and stays
/// there: `journey.rs`'s berthing comparison went from seconds a seed to
/// minutes, because every berthed policy sat in permanent siege until
/// `PATIENCE` ran out, and on the one seed that finished, **careful
/// berthing fell from +25.5% against never stopping to +0.0%.** That is
/// M5's fourth exit criterion — is stopping at a ruin ever the right
/// call — answered "no" by a change made for tidiness.
///
/// So the unconditional decay is what keeps a stopped tower out of a
/// spiral, and the *comment* was the thing that was wrong. What makes
/// walking the answer to a wave is `cling_ticks` (§11), which only counts
/// down while the legs run — not this.
fn decay_provocation(state: &mut GameState, content: &Content) {
    // Nothing to bleed off when nothing has been provoked. Without this
    // guard the decay kept draining the shared accumulator while the
    // clamp held provocation at zero, so every point a tower earned by
    // harvesting was cancelled by a decay that had had no effect — and
    // the knob could never turn at all.
    if state.siege.provocation <= 0 {
        state.siege.provocation_acc = state.siege.provocation_acc.max(0);
        return;
    }
    let balance = &content.balance.siege;
    state.siege.provocation_acc -= balance.provocation_decay_per_100_ticks;
    while state.siege.provocation_acc <= -100 {
        state.siege.provocation_acc += 100;
        state.siege.provocation = (state.siege.provocation - 1).max(0);
    }
}

/// Raise provocation by a fractional amount, carried in an accumulator
/// so a slow trickle adds up instead of truncating to nothing.
pub fn provoke_hundredths(state: &mut GameState, content: &Content, hundredths: i64) {
    if hundredths <= 0 {
        return;
    }
    let ceiling = content.balance.siege.provocation_max;
    state.siege.provocation_acc += hundredths;
    while state.siege.provocation_acc >= 100 {
        state.siege.provocation_acc -= 100;
        state.siege.provoke(1, ceiling);
    }
}

// ---------------------------------------------------------------------------
// Waves
// ---------------------------------------------------------------------------

fn maybe_spawn_wave(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let balance = &content.balance.siege;
    if state.tick < state.siege.next_wave_tick {
        return;
    }
    state.siege.next_wave_tick = state.tick + u64::from(balance.wave_interval_ticks.max(1));

    // A tower that has not drawn attention is left alone. The opening
    // of a run is quiet not because of a difficulty setting but because
    // nothing has noticed it yet — and that means the first wave is
    // always something the player did.
    if state.siege.provocation <= 0 {
        return;
    }

    let night = state.clock.sun_pct(content) < content.balance.clock.night_light_threshold;
    let eligible: Vec<EnemyIdx> = content
        .enemies
        .iter()
        .enumerate()
        // A feral warden is not summoned by attention — it is summoned
        // by berthing at the ruin it guards (`SYSTEMS.md` §3.4) — so it
        // is excluded from the ordinary pool explicitly rather than
        // fenced off with an out-of-range `min_provocation`.
        .filter(|(_, def)| def.encounter == EnemyEncounter::Ordinary)
        .filter(|(_, def)| def.min_provocation <= state.siege.provocation)
        .filter(|(_, def)| !def.night_only || night)
        // Where you are, as well as how loud you have been. The
        // mire-hulk is the coast's own problem and meeting one in the
        // jungle because a run was noisy would make region 3 less
        // itself rather than more.
        .filter(|(_, def)| def.min_region <= state.world.region.get() as u16)
        .filter(|(_, def)| def.threat > 0)
        .map(|(i, _)| EnemyIdx(i as u16))
        .collect();
    if eligible.is_empty() {
        return;
    }

    // Threat budget scales with how much attention the tower has drawn.
    // This is the only difficulty dial, and the player turns it by
    // playing rather than from a menu.
    //
    // Nothing comes until that budget can afford something on its own.
    // Applying `base_threat` as an unconditional floor meant the first
    // point of provocation bought a full opening wave, which turned a
    // tower that had barely started harvesting into one under siege.
    let scaled = scaled_wave_threat(state, content);
    let cheapest = eligible
        .iter()
        .map(|idx| content.enemy(*idx).threat)
        .min()
        .unwrap_or(i64::MAX);
    if scaled < cheapest {
        return;
    }
    // Past that point a wave is never a token single creature.
    let budget = scaled.max(balance.base_threat);

    let approaches = state.world.current_approaches(content);
    if fill_budget(state, content, &eligible, budget, None, Some(approaches)) > 0 {
        sounds.push(SoundEvent::WaveArrives);
    }
}

/// Provocation translated through the route's current pressure.
/// Kept as one named seam so tests can prove the route card and wave
/// allocator use the same multiplication.
#[must_use]
pub(crate) fn scaled_wave_threat(state: &GameState, content: &Content) -> i64 {
    state.siege.provocation * content.balance.siege.threat_per_100_provocation / 100
        * state.world.current_threat_pct(content)
        / 100
}

/// Something went down. Put what it was carrying on the shelves.
///
/// **One place, called from every path that can kill.** There are three
/// — a dart battery's shot, a cutter arm's blade, and whatever comes
/// next — and a drop that only happens on one of them is a drop that
/// depends on *how* you fought, which is not a distinction this game
/// makes anywhere else.
///
/// Anything that does not fit is lost, and that is the honest outcome
/// rather than a special case: a tower with nowhere to put two alloy
/// has told you something about itself. It is also the same rule the
/// waypoints keep.
pub fn felled(state: &mut GameState, content: &Content, def: EnemyIdx) {
    let Some(runtime) = content.enemy_runtime.get(def.get()) else {
        return;
    };
    for (item, amount) in &runtime.drops {
        let mut left = *amount;
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                left -= room.shelve(*item, left);
                if left <= 0 {
                    break;
                }
            }
            if left <= 0 {
                break;
            }
        }
    }
}

/// Wake what a ruin has instead of a lock.
///
/// Called by `intake` the first time a rig takes anything out of a given
/// ruin (`SYSTEMS.md` §3.4), in exactly the shape [`provoke_hundredths`]
/// is called in — the price lands next to the act, so it is impossible
/// to add a new way of disturbing a ruin and forget to make it wake.
/// `held` is what the ruin still had at that moment, which is what the
/// wave is sized against; `at` is where the ruin stands.
///
/// **This is the counterweight to the cling rule, and it needs no new
/// mechanic.** `DECISIONS.md` §11 counts a creature's grip down only
/// while the tower is actually striding, so walking is a real, free
/// answer to a wave. A berthed tower has `strode == false` — stopping
/// is what the berth *is* — so nothing clinging to it loses its grip.
/// The one place in the game worth stopping for is the one place the
/// escape hatch is shut, and it shuts itself: no rule forbids walking
/// away, walking away simply ends the salvage.
pub fn rouse_wardens(
    state: &mut GameState,
    content: &Content,
    at: Paces,
    held: i64,
    sounds: &mut Vec<SoundEvent>,
) {
    // Whatever the pack says an ordinary wave may not draw. A creature
    // excluded from the provocation pool is by definition summoned some
    // other way, and berthing is the only other way there is.
    let wardens: Vec<EnemyIdx> = content
        .enemies
        .iter()
        .enumerate()
        .filter(|(_, def)| def.encounter == EnemyEncounter::RuinResident && def.threat > 0)
        .map(|(i, _)| EnemyIdx(i as u16))
        .collect();
    let Some(cheapest) = wardens.iter().map(|idx| content.enemy(*idx).threat).min() else {
        return;
    };

    let balance = &content.balance.journey;
    // Floored at a single warden: a ruin with anything in it is guarded,
    // and one that holds a great deal is guarded in proportion.
    let budget = (held * balance.warden_threat_per_100_salvage / 100).max(cheapest);

    // Out of the ruin rather than off the usual horizon, offset so
    // there are a few seconds between the ground moving and the first
    // bite. Never nearer than that offset even when the ruin is behind
    // the tower, because a creature that woke level with the tower
    // would be biting before the player had seen it.
    let wake = paces_from_int(balance.warden_wake_paces);
    let from = (at + wake).max(state.world.distance + wake);

    if fill_budget(state, content, &wardens, budget, Some(from), None) > 0 {
        sounds.push(SoundEvent::WaveArrives);
    }
}

/// Spend a threat budget on whatever fits, cheapest-first as a fallback
/// so a small budget still produces something rather than nothing.
///
/// `at` is where the creatures appear: `None` scatters them over the
/// last stretch of the ordinary approach, so a wave trickles in rather
/// than arriving as a wall; `Some` puts them all in one place, which so
/// far means all out of the same roused ruin. Returns how many came.
fn fill_budget(
    state: &mut GameState,
    content: &Content,
    pool: &[EnemyIdx],
    budget: i64,
    at: Option<Paces>,
    approaches: Option<crate::content::ApproachWeights>,
) -> u32 {
    let mut remaining = budget;
    let mut spawned = 0;
    while remaining > 0 {
        let affordable: Vec<EnemyIdx> = pool
            .iter()
            .copied()
            .filter(|idx| content.enemy(*idx).threat <= remaining)
            .collect();
        if affordable.is_empty() {
            break;
        }
        let weighted_total = approaches.map_or(0, |weights| {
            affordable
                .iter()
                .map(|idx| weights.for_approach(content.enemy(*idx).approach))
                .sum()
        });
        let def = if weighted_total > 0 {
            let mut draw = state.rng.sim.range(1, weighted_total);
            let mut picked = affordable[0];
            for candidate in &affordable {
                let weight = approaches
                    .expect("positive weighted total has approach weights")
                    .for_approach(content.enemy(*candidate).approach);
                if draw <= weight {
                    picked = *candidate;
                    break;
                }
                draw -= weight;
            }
            picked
        } else {
            // Wardens have their own authored encounter, and a heavily
            // specialised route can temporarily have no matching
            // affordable creature. Both use the old deterministic
            // uniform fallback rather than silently cancelling a wave.
            let Some(pick) = state.rng.sim.index(affordable.len()) else {
                break;
            };
            affordable[pick]
        };
        remaining -= content.enemy(def).threat;
        let from = match at {
            Some(at) => at,
            None => {
                let ahead = content.balance.siege.spawn_paces_ahead;
                let jitter = state.rng.sim.range(0, ahead / 4);
                state.world.distance + paces_from_int(ahead + jitter)
            }
        };
        spawn_at(state, content, def, from);
        spawned += 1;
        // A hard stop, so a pathological budget cannot allocate
        // unboundedly and stall the tick.
        if spawned >= 64 {
            break;
        }
    }
    spawned
}

fn spawn_at(state: &mut GameState, content: &Content, def: EnemyIdx, at: Paces) {
    let id = state.alloc_enemy_id();
    let hp = content.enemy(def).hp;
    state.siege.enemies.push(Enemy {
        id,
        def,
        at,
        hp,
        state: EnemyState::Approaching,
        attack_cooldown: 0,
        cling_left: content.enemy(def).cling_ticks,
        fade_left: 0,
    });
}

/// Rouse the one creature authored into a journey landmark.
///
/// The waypoint is the uniqueness boundary: it is generated once and
/// can be taken once. This second guard prevents an accidental second
/// command path from turning a resident back into a farmable wave.
pub(crate) fn spawn_landmark_resident(
    state: &mut GameState,
    content: &Content,
    def: EnemyIdx,
    at: Paces,
) {
    if content.enemy(def).encounter != EnemyEncounter::LandmarkResident
        || state
            .siege
            .enemies
            .iter()
            .any(|enemy| enemy.def == def && !enemy.state.is_going())
    {
        return;
    }
    spawn_at(state, content, def, at);
}

// ---------------------------------------------------------------------------
// Approach and attack
// ---------------------------------------------------------------------------

/// Within this many whole paces of the tower, a creature is in contact.
const CONTACT_PACES: i64 = 2;

fn advance_enemies(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let tower_at = state.world.distance;
    // Siege runs before stride, so `paces_last` is the exact distance the
    // tower covered on the preceding tick. This is deliberately stronger
    // than `strode`: it also supplies the real short step into a fork and
    // zero at a block, keeping restraints and contact in world coordinates.
    let tower_step = state.paces_last;
    let strode = tower_step > 0;
    let balance = &content.balance.siege;
    // Read this tick's restraints before aging them. Defence runs after
    // siege, so a net thrown on the previous tick gets exactly its
    // authored number of approach ticks here.
    let controls: Vec<(EnemyId, i64)> = state
        .siege
        .controls
        .iter()
        .map(|control| (control.enemy, control.speed_pct))
        .collect();
    for control in &mut state.siege.controls {
        control.ticks_left = control.ticks_left.saturating_sub(1);
    }
    state.siege.controls.retain(|control| {
        control.ticks_left > 0
            && state
                .siege
                .enemies
                .iter()
                .any(|enemy| enemy.id == control.enemy && !enemy.state.is_going())
    });
    let mut enemies = std::mem::take(&mut state.siege.enemies);

    for enemy in &mut enemies {
        if enemy.state.is_going() {
            continue;
        }
        let def = content.enemy(enemy.def);

        // A walking tower carries itself out from under the things
        // holding on to it. This is the answer to a wave that costs no
        // poles and no emplacement — and the reason stopping to work is
        // a decision rather than a free action. A tower that has
        // stopped, or that cannot afford the charge to walk, shakes
        // nothing off at all.
        //
        // The clock runs on anything that has reached the tower, not
        // only on what is mid-bite. A creature that has run out of
        // things to chew drops back to circling, and if that did not
        // count against its grip a tower stripped to its Heartseed
        // would keep a wave orbiting it forever.
        //
        // It is set once, at spawn, and never renewed. Resetting it on
        // contact looked tidier but meant a creature that lost its
        // target and later found another — which is exactly what
        // happens when the crew mend a panel it had already chewed
        // through — got a fresh full grip for it. Repairing during a
        // wave would have made the wave last longer.
        let at_tower = paces_to_int((enemy.at - tower_at).abs()) <= CONTACT_PACES;
        if strode && at_tower {
            enemy.cling_left = enemy.cling_left.saturating_sub(1);
            if enemy.cling_left == 0 {
                enemy.state = EnemyState::Leaving;
                enemy.fade_left = balance.enemy_fade_ticks;
                continue;
            }
        }

        match enemy.state {
            EnemyState::Approaching => {
                let speed_pct = controls
                    .iter()
                    .find(|(id, _)| *id == enemy.id)
                    .map_or(100, |(_, pct)| *pct);
                let step = paces_from_fx(
                    Fx::ratio(def.speed_paces_per_100_ticks as i32, 100)
                        * Fx::ratio(speed_pct as i32, 100),
                );
                // Control holds a creature off the hull as well as
                // slowing its own approach. Compensating the same share
                // of this tick's tower stride makes 0% an actual anchor
                // at arm's length rather than something the walking
                // tower immediately runs into.
                enemy.at += tower_step * (100 - speed_pct) / 100;
                // Closing on the tower from ahead. The tower is also
                // moving, which is why this is a gap rather than a
                // fixed distance.
                enemy.at -= step;
                if paces_to_int((enemy.at - tower_at).abs()) <= CONTACT_PACES {
                    enemy.at = tower_at;
                    if let Some(target) = pick_prize(state, content, def) {
                        enemy.state = EnemyState::Attacking { target };
                        enemy.attack_cooldown = def.attack_ticks;
                        sounds.push(SoundEvent::EnemyContact);
                    }
                }
            }
            EnemyState::Attacking { target } => {
                // Stay with the tower rather than being left behind by
                // its stride.
                enemy.at = tower_at;

                if enemy.attack_cooldown > 0 {
                    enemy.attack_cooldown -= 1;
                    continue;
                }
                enemy.attack_cooldown = def.attack_ticks;
                // A thief takes instead of breaking. Nothing about the
                // tower's hit points changes and a morning's harvest is
                // gone — a loss the repair system cannot answer, which
                // is the whole reason this creature shape exists.
                let landed = if def.steals {
                    steal_from(state, target, sounds)
                } else {
                    apply_damage(state, content, target, def.damage, sounds)
                };
                if !landed {
                    // Whatever it was chewing is gone. Find something
                    // else, or go back to circling.
                    enemy.state = match pick_prize(state, content, def) {
                        Some(next) => EnemyState::Attacking { target: next },
                        None => EnemyState::Approaching,
                    };
                }
            }
            EnemyState::Dying | EnemyState::Leaving => {}
        }
    }

    state.siege.enemies = enemies;
}

/// What a creature settles on: a full outbox for a thief, the hull for
/// everything else.
///
/// Split from `pick_target` rather than folded into it because the two
/// are asking different questions. A biter wants whatever is nearest and
/// intact; a thief wants whatever has something in it, and a tower whose
/// outboxes are all empty is a tower a crow has no reason to land on —
/// which is haul throughput quietly buying you safety, and the nicest
/// thing about this creature.
fn pick_prize(
    state: &GameState,
    content: &Content,
    def: &crate::content::EnemyDef,
) -> Option<DamageTarget> {
    if !def.steals {
        return pick_target(state, content, def.approach);
    }
    // Highest floor first, because a crow comes down from the canopy and
    // the roof is what it sees. Ties to the lowest slot, so the choice is
    // a pure function of state.
    state
        .tower
        .floors
        .iter()
        .rev()
        .flat_map(|floor| floor.rooms.iter().map(move |room| (floor.index, room)))
        .find(|(_, room)| room.outputs.iter().any(|stack| stack.count > 0))
        .map(|(floor, room)| DamageTarget::Room {
            floor,
            slot: room.slot,
        })
}

/// Take one load out of whatever the target room has finished.
///
/// Returns false when there is nothing to take, which sends the thief
/// looking elsewhere exactly as an empty target sends a biter looking —
/// the two share the "whatever it was working on is gone" path rather
/// than each having their own.
///
/// **Outboxes only, never shelves.** A crow takes what is lying out,
/// which is the finished work sitting where a crew member has not got
/// to yet; getting it onto a shelf is what putting it away *means*. That
/// makes the loss legible ("I was slow clearing that outbox") and gives
/// haul throughput a defensive value it has never had.
fn steal_from(state: &mut GameState, target: DamageTarget, sounds: &mut Vec<SoundEvent>) -> bool {
    let DamageTarget::Room { floor, slot } = target else {
        // Panels and shafts hold nothing. A thief that picked one is a
        // thief with nothing to do here.
        return false;
    };
    let Some(floor) = state.tower.floor_mut(floor) else {
        return false;
    };
    let Some(room) = floor.rooms.iter_mut().find(|room| room.covers(slot)) else {
        return false;
    };
    for stack in &mut room.outputs {
        if stack.withdraw(1) > 0 {
            state.stats.items_stolen += 1;
            sounds.push(SoundEvent::Steal);
            return true;
        }
    }
    false
}

/// How much the things currently attached slow the legs, in percent.
///
/// **The mire-hulk's whole mechanic, and it inverts the answer to every
/// other wave.** Since M2 the reply to something clinging to the tower
/// has been to keep walking until it loses its grip (`cling_ticks`). A
/// hulk takes hold of a leg, so the tower it is riding walks more slowly
/// and therefore sheds it *later* — the usual answer making itself
/// worse, which is the only reason a fifth biter would have been worth
/// authoring.
///
/// Multiplicative across attached creatures and floored, so two hulks
/// are worse than one without ever bringing the tower to a stop. A
/// creature that halted the legs outright would end the run rather than
/// pressure it (`SYSTEMS.md` §2.2).
#[must_use]
pub fn drag_pct(state: &GameState, content: &Content) -> i64 {
    let mut pct = 100i64;
    for enemy in &state.siege.enemies {
        if !matches!(enemy.state, EnemyState::Attacking { .. }) {
            continue;
        }
        let drag = content.enemy(enemy.def).drag_pct;
        if drag > 0 {
            pct = pct * drag / 100;
        }
    }
    pct.clamp(10, 100)
}

/// What this kind of creature goes for, given what the tower currently
/// has. Returning `None` means there is nothing here it cares about.
fn pick_target(state: &GameState, content: &Content, approach: Approach) -> Option<DamageTarget> {
    match approach {
        // Ground creatures work on the lowest intact panel, then on
        // whatever is behind it.
        Approach::Ground => state
            .tower
            .floors
            .iter()
            .find(|floor| !floor.panel.is_broken())
            .map(|floor| DamageTarget::Panel { floor: floor.index })
            .or_else(|| lowest_room(state, content)),

        // Leapers drop onto the top deck. Height is exposure, and this
        // is the mechanism that makes that true.
        Approach::Canopy => {
            let top = state.tower.top_floor();
            let floor = state.tower.floor(top)?;
            if !floor.panel.is_broken() {
                return Some(DamageTarget::Panel { floor: top });
            }
            floor
                .rooms
                .iter()
                .find(|room| !room.health.is_broken())
                .map(|room| DamageTarget::Room {
                    floor: top,
                    slot: room.slot,
                })
        }

        // Borers go for circulation. Severing a shaft is the emergency
        // this whole system exists to produce.
        Approach::Burrow => state
            .tower
            .shafts
            .iter()
            .find(|shaft| !shaft.health.is_broken())
            .map(|shaft| DamageTarget::Shaft { id: shaft.id })
            .or_else(|| lowest_room(state, content)),
    }
}

fn lowest_room(state: &GameState, content: &Content) -> Option<DamageTarget> {
    // The Heartseed is the last thing anything reaches, so it is only
    // targeted once there is nothing else standing.
    let heart = content
        .rooms
        .iter()
        .position(|room| room.category == crate::content::RoomCategory::Heart)
        .map(|i| crate::ids::RoomIdx(i as u16));

    let ordinary = state.tower.floors.iter().find_map(|floor| {
        floor
            .rooms
            .iter()
            .find(|room| !room.health.is_broken() && Some(room.def) != heart)
            .map(|room| DamageTarget::Room {
                floor: floor.index,
                slot: room.slot,
            })
    });
    ordinary.or(Some(DamageTarget::Heart))
}

/// Apply damage to whatever the coordinate currently resolves to.
/// Returns false when the target no longer exists — the player may have
/// demolished it mid-bite.
fn apply_damage(
    state: &mut GameState,
    content: &Content,
    target: DamageTarget,
    amount: i64,
    sounds: &mut Vec<SoundEvent>,
) -> bool {
    match target {
        DamageTarget::Panel { floor } => {
            let Some(floor) = state.tower.floor_mut(floor) else {
                return false;
            };
            if floor.panel.is_broken() {
                return false;
            }
            if floor.panel.hurt(amount) {
                sounds.push(SoundEvent::Breach);
            } else {
                sounds.push(SoundEvent::Impact);
            }
            true
        }
        DamageTarget::Room { floor, slot } => {
            let Some(floor) = state.tower.floor_mut(floor) else {
                return false;
            };
            let Some(room) = floor.rooms.iter_mut().find(|room| room.covers(slot)) else {
                return false;
            };
            if room.health.is_broken() {
                return false;
            }
            let destroyed = room.health.hurt(amount);
            sounds.push(if destroyed {
                SoundEvent::Wrecked
            } else {
                SoundEvent::Impact
            });
            true
        }
        DamageTarget::Shaft { id } => {
            let Some(shaft) = state.tower.shaft_mut(id) else {
                return false;
            };
            if shaft.health.is_broken() {
                return false;
            }
            if shaft.health.hurt(amount) {
                sounds.push(SoundEvent::Severed);
                // Anyone on it comes off where they stand. Nothing they
                // carry is lost — see `DECISIONS.md` §8 on breakage
                // being legible rather than punitive.
                evict_riders(state, id);
            } else {
                sounds.push(SoundEvent::Impact);
            }
            true
        }
        DamageTarget::Heart => {
            let heart = content
                .rooms
                .iter()
                .position(|room| room.category == crate::content::RoomCategory::Heart)
                .map(|i| crate::ids::RoomIdx(i as u16));
            let Some(heart) = heart else {
                return false;
            };
            for floor in &mut state.tower.floors {
                for room in &mut floor.rooms {
                    if room.def != heart || room.health.is_broken() {
                        continue;
                    }
                    if room.health.hurt(amount) {
                        state.siege.lost = true;
                        sounds.push(SoundEvent::HeartseedLost);
                    } else {
                        sounds.push(SoundEvent::Impact);
                    }
                    return true;
                }
            }
            false
        }
    }
}

/// Put everyone riding or queueing for a shaft back on their own feet.
/// Shared with the demolition path in `engine/commands.rs`, because a
/// severed shaft and a torn-out one leave the crew in the same spot.
pub fn evict_riders(state: &mut GameState, shaft: crate::ids::ShaftId) {
    for member in &mut state.crew {
        let affected = match member.state {
            crate::state::CrewState::Boarding { shaft: at, .. }
            | crate::state::CrewState::Climbing { shaft: at, .. }
            | crate::state::CrewState::Riding { shaft: at, .. } => at == shaft,
            _ => false,
        };
        if affected {
            let (floor, slot) = (member.floor(), member.slot());
            member.snap_to(floor, slot);
            member.state = crate::state::CrewState::Idle;
        }
    }
    if let Some(shaft) = state.tower.shaft_mut(shaft) {
        shaft.riders = 0;
        for car in &mut shaft.cars {
            car.riders.clear();
        }
    }
}

/// Run down the fade timers and clear out whatever has finished.
///
/// Only kills count toward `repelled`. A creature the tower simply
/// walked away from is gone, but nobody saw it off.
fn reap(state: &mut GameState) {
    for enemy in &mut state.siege.enemies {
        if enemy.state.is_going() {
            enemy.fade_left = enemy.fade_left.saturating_sub(1);
        }
    }
    state.siege.repelled += state
        .siege
        .enemies
        .iter()
        .filter(|enemy| enemy.state == EnemyState::Dying && enemy.fade_left == 0)
        .count() as u64;
    state
        .siege
        .enemies
        .retain(|enemy| !(enemy.state.is_going() && enemy.fade_left == 0));
}

/// Health of the tower as a whole, for the readout. Panels, rooms, and
/// shafts averaged by hit points, so one wrecked room in a big tower
/// reads as a scratch and a severed spine reads as serious.
#[must_use]
pub fn tower_integrity_permille(state: &GameState) -> i64 {
    let mut total = Health { hp: 0, max: 0 };
    for floor in &state.tower.floors {
        total.hp += floor.panel.hp;
        total.max += floor.panel.max;
        for room in &floor.rooms {
            total.hp += room.health.hp;
            total.max += room.health.max;
        }
    }
    for shaft in &state.tower.shafts {
        total.hp += shaft.health.hp;
        total.max += shaft.health.max;
    }
    total.permille()
}

/// A room a thief is working that nobody has gone to yet.
///
/// **Thieves only.** A crow can be shooed; a mire hulk cannot, and
/// pretending otherwise would turn standing in a doorway into combat —
/// exactly what `DECISIONS.md` §8 rules out. The `steals` flag is the
/// line, and it is the same flag that decides what a creature does when
/// it arrives.
///
/// Returns the room's coordinates rather than the creature's id, for the
/// reason `DamageTarget` gives: a crew member walks to a *place*, and a
/// creature that has moved on by the time they get there should end the
/// errand rather than send them chasing it round the tower.
#[must_use]
pub fn thief_at_work(
    enemies: &[Enemy],
    content: &Content,
    crew: &[crate::state::Crew],
) -> Option<(FloorIdx, SlotIdx)> {
    let taken = |floor: FloorIdx, slot: SlotIdx| {
        crew.iter().any(|member| {
            matches!(
                member.errand,
                Some(crate::state::Errand::Shoo { floor: f, slot: s }) if f == floor && s == slot
            )
        })
    };
    enemies
        .iter()
        .filter(|enemy| !enemy.state.is_going())
        .filter(|enemy| content.enemy(enemy.def).steals)
        .filter_map(|enemy| match enemy.state {
            EnemyState::Attacking {
                target: DamageTarget::Room { floor, slot },
            } => Some((floor, slot)),
            _ => None,
        })
        .find(|(floor, slot)| !taken(*floor, *slot))
}

/// Send off any thief that has found somebody standing in the room.
///
/// **Nobody fights, and nothing is counted.** The creature goes to
/// `Leaving`, which shares the fade with a cling timer running out and
/// deliberately does *not* add to `repelled` — that number means the
/// darts worked, and conflating it with standing in a doorway would stop
/// it measuring the thing it exists to measure. Being asked to leave is
/// not being seen off, and the tower keeps no score of it either way.
fn shoo_thieves(state: &mut GameState, content: &Content) {
    let manned: Vec<(FloorIdx, SlotIdx)> = state
        .crew
        .iter()
        .filter(|member| matches!(member.state, crate::state::CrewState::Shooing { .. }))
        .filter_map(|member| match member.errand {
            Some(crate::state::Errand::Shoo { floor, slot }) => Some((floor, slot)),
            _ => None,
        })
        .collect();
    if manned.is_empty() {
        return;
    }
    let fade = content.balance.siege.enemy_fade_ticks;
    for enemy in &mut state.siege.enemies {
        if enemy.state.is_going() || !content.enemy(enemy.def).steals {
            continue;
        }
        let EnemyState::Attacking {
            target: DamageTarget::Room { floor, slot },
        } = enemy.state
        else {
            continue;
        };
        if manned.contains(&(floor, slot)) {
            enemy.state = EnemyState::Leaving;
            enemy.fade_left = fade;
        }
    }
}
