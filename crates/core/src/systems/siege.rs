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

use crate::content::{Approach, Content};
use crate::fx::{Fx, paces_from_fx, paces_from_int, paces_to_int};
use crate::ids::EnemyIdx;
use crate::state::siege::{DamageTarget, Enemy, EnemyState};
use crate::state::{GameState, Health};

use super::SoundEvent;

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    decay_provocation(state, content);
    maybe_spawn_wave(state, content, sounds);
    advance_enemies(state, content, sounds);
    reap(state);
}

// ---------------------------------------------------------------------------
// Provocation
// ---------------------------------------------------------------------------

/// Walking quietly bleeds attention off. Everything that raises it —
/// stripping the terrain, burner smoke — is called from the system that
/// does the provoking, so the cost lands next to the act.
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
        .filter(|(_, def)| def.min_provocation <= state.siege.provocation)
        .filter(|(_, def)| !def.night_only || night)
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
    let scaled = state.siege.provocation * balance.threat_per_100_provocation / 100;
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

    // Fill the budget with whatever fits, cheapest-first as a fallback
    // so a small budget still produces something rather than nothing.
    let mut remaining = budget;
    let mut spawned = 0;
    while remaining > 0 {
        let affordable: Vec<EnemyIdx> = eligible
            .iter()
            .copied()
            .filter(|idx| content.enemy(*idx).threat <= remaining)
            .collect();
        if affordable.is_empty() {
            break;
        }
        let Some(pick) = state.rng.sim.index(affordable.len()) else {
            break;
        };
        let def = affordable[pick];
        remaining -= content.enemy(def).threat;
        spawn(state, content, def);
        spawned += 1;
        // A hard stop, so a pathological budget cannot allocate
        // unboundedly and stall the tick.
        if spawned >= 64 {
            break;
        }
    }

    if spawned > 0 {
        sounds.push(SoundEvent::WaveArrives);
    }
}

fn spawn(state: &mut GameState, content: &Content, def: EnemyIdx) {
    let balance = &content.balance.siege;
    // Scatter arrivals over the last stretch of the approach so a wave
    // trickles in rather than appearing as a wall.
    let jitter = state.rng.sim.range(0, balance.spawn_paces_ahead / 4);
    let at = state.world.distance + paces_from_int(balance.spawn_paces_ahead + jitter);
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

// ---------------------------------------------------------------------------
// Approach and attack
// ---------------------------------------------------------------------------

/// Within this many whole paces of the tower, a creature is in contact.
const CONTACT_PACES: i64 = 2;

fn advance_enemies(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let tower_at = state.world.distance;
    let strode = state.strode;
    let balance = &content.balance.siege;
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
                let step = paces_from_fx(Fx::ratio(def.speed_paces_per_100_ticks as i32, 100));
                // Closing on the tower from ahead. The tower is also
                // moving, which is why this is a gap rather than a
                // fixed distance.
                enemy.at -= step;
                if paces_to_int((enemy.at - tower_at).abs()) <= CONTACT_PACES {
                    enemy.at = tower_at;
                    if let Some(target) = pick_target(state, content, def.approach) {
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
                let landed = apply_damage(state, content, target, def.damage, sounds);
                if !landed {
                    // Whatever it was chewing is gone. Find something
                    // else, or go back to circling.
                    enemy.state = match pick_target(state, content, def.approach) {
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
