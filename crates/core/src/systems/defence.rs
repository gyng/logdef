//! Emplacements shooting back.
//!
//! A dart battery is an ordinary room with an ordinary input stack. It
//! is fed by the same crew, up the same shafts, in competition with the
//! same mill — and it goes quiet when it runs dry for exactly the same
//! reason a mill does. That equivalence is the design: an emplacement
//! with its own private supply would turn combat into a parallel game
//! instead of a load test on the one already running.
//!
//! There is no aiming. The player's verbs here are where they put the
//! battery, whether the chain can keep it fed, and — from the priority
//! setting — what it shoots first.

use crate::content::{Content, DefenceEffect};
use crate::ids::EnemyId;
use crate::state::GameState;
use crate::state::power::PowerUse;
use crate::state::siege::EnemyState;

use super::{CombatEvent, CombatSource, CombatTarget, SoundEvent};

/// The cutter arm's other half: anything clinging to the tower within
/// the arm's own floor is in the arc of a working blade.
///
/// **No ammo, no reload, no range.** An emplacement is a supply
/// question — `defence.rs`'s whole opening paragraph is about a battery
/// competing with the mill for the same crew and the same shafts. This
/// is not that: it is a machine already doing its job, and a creature
/// that climbs into it. The player who built an arm to harvest has
/// already built the thing that answers a skitter, and finding that out
/// is a better moment than being sold a weapon.
///
/// It only reaches what is *attached*, and only on its own floor, which
/// is what keeps it from being a free emplacement: a wave that goes for
/// the roof of a tall tower is not answered by a boom on the ground.
///
/// The tone gate (`DECISIONS.md` §8) is the reason this is a *harvest*
/// room that happens to cut rather than a weapon that happens to
/// harvest. The tower defends itself with the tools it works with.
fn cut_what_climbs_in(
    state: &mut GameState,
    content: &Content,
    sounds: &mut Vec<SoundEvent>,
    combat: &mut Vec<CombatEvent>,
) {
    let tick = state.tick;
    let mut struck: Vec<(CombatSource, EnemyId, i64)> = Vec::new();

    for floor in &state.tower.floors {
        for room in &floor.rooms {
            let damage = content.room_rt(room.def).melee_damage;
            if damage <= 0 || !room.is_working(content, tick) {
                continue;
            }
            for enemy in &state.siege.enemies {
                let EnemyState::Attacking { target } = enemy.state else {
                    continue;
                };
                let at = match target {
                    crate::state::siege::DamageTarget::Panel { floor } => Some(floor),
                    crate::state::siege::DamageTarget::Room { floor, .. } => Some(floor),
                    // A borer inside a shaft column and anything at the
                    // Heartseed are past the skin, and an arm swinging
                    // outboard cannot reach either.
                    _ => None,
                };
                if at == Some(floor.index) {
                    struck.push((
                        CombatSource {
                            room_id: room.id,
                            room_def: room.def,
                            floor: floor.index,
                            slot: room.slot,
                        },
                        enemy.id,
                        damage,
                    ));
                }
            }
        }
    }

    if struck.is_empty() {
        return;
    }
    for (source, id, damage) in struck {
        let Some(enemy) = state.siege.enemies.iter_mut().find(|enemy| enemy.id == id) else {
            continue;
        };
        if enemy.state.is_going() {
            continue;
        }
        combat.push(CombatEvent::CutterStruck {
            source,
            target: CombatTarget {
                enemy_id: enemy.id,
                enemy_def: enemy.def,
                at: enemy.at,
            },
        });
        enemy.hp -= damage;
        if enemy.hp <= 0 {
            enemy.state = EnemyState::Dying;
            enemy.fade_left = content.balance.siege.enemy_fade_ticks;
            let def = enemy.def;
            sounds.push(SoundEvent::EnemyDown);
            super::siege::felled(state, content, def);
        } else {
            sounds.push(SoundEvent::Shot);
        }
    }
}

pub fn run(
    state: &mut GameState,
    content: &Content,
    sounds: &mut Vec<SoundEvent>,
    combat: &mut Vec<CombatEvent>,
) {
    cut_what_climbs_in(state, content, sounds, combat);
    if state.siege.enemies.is_empty() {
        // Still tick reload timers down, so a battery that has been
        // waiting is ready the moment something arrives.
        cool_down(state, content);
        return;
    }

    let mut shots: Vec<Shot> = Vec::new();
    let tower_at = state.world.distance;
    let tick = state.tick;
    let state_power = &state.power.clone();
    let top_floor = state.tower.top_floor();

    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            let Some(defence) = content.room(room.def).defence.as_ref() else {
                continue;
            };
            if content.room(room.def).top_floor_only && floor.index != top_floor {
                continue;
            }
            // A damaged battery fires more slowly, on exactly the
            // same rule as a damaged mill mills more slowly.
            if !room.is_working(content, tick) {
                continue;
            }

            // The reload is where a starved switchboard lands: a gun
            // on a half-served circuit takes twice as long to come round
            // again (`SYSTEMS.md` §6.39).
            if room.progress > 0 {
                room.progress -= reload_step(state_power, content, floor.index, room.def);
                continue;
            }

            // Ammo is an input stack like any other.
            let Some(ammo) = room
                .inputs
                .iter_mut()
                .find(|stack| stack.item == defence_ammo(content, room.def))
            else {
                continue;
            };
            if ammo.count < defence.ammo_per_shot {
                continue;
            }

            // Nearest live creature inside range. "Nearest" rather than
            // "weakest" or "strongest" because a battery has no judgement
            // of its own; the player's judgement went into where they put it.
            let range = crate::fx::paces_from_int(defence.range_paces);
            // **And whether this emplacement answers that approach at
            // all** (`SYSTEMS.md` §6.22). An empty `targets` means all
            // three, which is what the thorn gun and the dart battery
            // are; a lantern mast looks up and nothing else.
            //
            let answers = |enemy: &crate::state::siege::Enemy| {
                defence.targets.is_empty()
                    || defence.targets.contains(&content.enemy(enemy.def).approach)
            };
            let in_reach = |enemy: &&crate::state::siege::Enemy| {
                !enemy.state.is_going() && (enemy.at - tower_at).abs() <= range && answers(enemy)
            };
            let target = state
                .siege
                .enemies
                .iter()
                .filter(in_reach)
                .min_by_key(|enemy| {
                    let control = state
                        .siege
                        .controls
                        .iter()
                        .find(|control| control.enemy == enemy.id);
                    // A net is crowd control, not a rope shredder. Hold a
                    // fresh creature before refreshing one already tied up;
                    // once the whole reachable group is restrained, refresh
                    // the restraint closest to expiring.
                    let restraint = if defence.effect == DefenceEffect::Tangle {
                        control.map_or((0, 0), |control| (1, control.ticks_left))
                    } else {
                        (0, 0)
                    };
                    (restraint, (enemy.at - tower_at).abs(), enemy.id.0)
                });
            let Some(target) = target else {
                continue;
            };

            // **The switchboard, last of all** (`SYSTEMS.md` §6.36).
            // Drawn here rather than earlier so a gun with nothing to
            // shoot at costs the tower nothing — an emplacement idling
            // through a quiet afternoon should not be holding the pipe
            // shut against the mill.
            //
            // A refusal **holds fire**: the reload is not spent, the
            // dart is not spent, and the shot happens on whichever tick
            // the charge is there. Nothing this game hands the player
            // ever vanishes, and ammunition burned into a brown-out
            // would be exactly that.
            // **A half-powered gun reloads at half speed rather than
            // holding fire** (`SYSTEMS.md` §6.39). The shot itself is
            // never partial — half a dart is not a thing — so the
            // fraction lands on the reload below, and what a starved
            // switchboard costs an emplacement is rate rather than
            // ammunition. Nothing this game hands the player vanishes,
            // and that still holds: a dart is spent only when it flies.
            if defence.charge_per_shot > 0
                && state.power.served_at(floor.index, PowerUse::Guns) <= 0
            {
                continue;
            }

            ammo.withdraw(defence.ammo_per_shot);
            room.progress = defence.reload_ticks;
            shots.push(Shot {
                source: CombatSource {
                    room_id: room.id,
                    room_def: room.def,
                    floor: floor.index,
                    slot: room.slot,
                },
                target: target.id,
                damage: defence.damage,
                effect: defence.effect,
                control_ticks: defence.control_ticks,
                control_speed_pct: defence.control_speed_pct,
                pulse_radius: crate::fx::paces_from_int(defence.pulse_radius_paces),
            });
        }
    }

    if shots.is_empty() {
        return;
    }

    for shot in shots {
        let Some(primary) = state
            .siege
            .enemies
            .iter()
            .find(|enemy| enemy.id == shot.target)
        else {
            continue;
        };
        if primary.state.is_going() {
            continue;
        }
        let target = CombatTarget {
            enemy_id: primary.id,
            enemy_def: primary.def,
            at: primary.at,
        };
        combat.push(CombatEvent::EmplacementFired {
            source: shot.source,
            target,
        });
        match shot.effect {
            DefenceEffect::Direct => strike(state, content, shot.target, shot.damage, sounds),
            DefenceEffect::Tangle => {
                strike(state, content, shot.target, shot.damage, sounds);
                restrain(
                    state,
                    shot.target,
                    shot.control_ticks,
                    shot.control_speed_pct,
                );
            }
            DefenceEffect::Resonance => {
                let centre = target.at;
                let affected: Vec<EnemyId> = state
                    .siege
                    .enemies
                    .iter()
                    .filter(|enemy| {
                        !enemy.state.is_going() && (enemy.at - centre).abs() <= shot.pulse_radius
                    })
                    .map(|enemy| enemy.id)
                    .collect();
                for id in affected {
                    strike(state, content, id, shot.damage, sounds);
                    restrain(state, id, shot.control_ticks, shot.control_speed_pct);
                }
            }
            DefenceEffect::Repel => {
                if let Some(enemy) = state.siege.enemies.iter_mut().find(|e| e.id == shot.target) {
                    enemy.state = EnemyState::Leaving;
                    enemy.fade_left = content.balance.siege.enemy_fade_ticks;
                    sounds.push(SoundEvent::EnemyLeaves);
                }
            }
            DefenceEffect::RootWard => {
                strike(state, content, shot.target, shot.damage, sounds);
                let alive = state
                    .siege
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.id == shot.target && !enemy.state.is_going());
                if let Some(enemy) = alive {
                    enemy.state = EnemyState::Approaching;
                    enemy.at = tower_at;
                    enemy.attack_cooldown = 0;
                    restrain(
                        state,
                        shot.target,
                        shot.control_ticks,
                        shot.control_speed_pct,
                    );
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Shot {
    source: CombatSource,
    target: EnemyId,
    damage: i64,
    effect: DefenceEffect,
    control_ticks: u32,
    control_speed_pct: i64,
    pulse_radius: crate::fx::Paces,
}

fn strike(
    state: &mut GameState,
    content: &Content,
    id: EnemyId,
    damage: i64,
    sounds: &mut Vec<SoundEvent>,
) {
    let Some(enemy) = state.siege.enemies.iter_mut().find(|enemy| enemy.id == id) else {
        return;
    };
    if enemy.state.is_going() {
        return;
    }
    enemy.hp -= damage;
    if enemy.hp <= 0 {
        enemy.state = EnemyState::Dying;
        enemy.fade_left = content.balance.siege.enemy_fade_ticks;
        let def = enemy.def;
        sounds.push(SoundEvent::EnemyDown);
        super::siege::felled(state, content, def);
    } else {
        sounds.push(SoundEvent::Shot);
    }
}

fn restrain(state: &mut GameState, id: EnemyId, ticks: u32, speed_pct: i64) {
    if ticks == 0
        || state
            .siege
            .enemies
            .iter()
            .any(|enemy| enemy.id == id && enemy.state.is_going())
    {
        return;
    }
    if let Some(control) = state
        .siege
        .controls
        .iter_mut()
        .find(|control| control.enemy == id)
    {
        control.ticks_left = control.ticks_left.max(ticks);
        control.speed_pct = control.speed_pct.min(speed_pct);
    } else {
        state
            .siege
            .controls
            .push(crate::state::siege::EnemyControl {
                enemy: id,
                ticks_left: ticks,
                speed_pct: speed_pct.clamp(0, 100),
            });
        state.siege.controls.sort_by_key(|control| control.enemy.0);
    }
    if let Some(enemy) = state.siege.enemies.iter_mut().find(|enemy| enemy.id == id)
        && matches!(enemy.state, EnemyState::Attacking { .. })
    {
        enemy.attack_cooldown = enemy.attack_cooldown.max(ticks);
    }
}

/// Reload timers keep running when nothing is in range.
fn cool_down(state: &mut GameState, content: &Content) {
    let power = state.power.clone();
    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            if content.room(room.def).defence.is_some() && room.progress > 0 {
                room.progress -= reload_step(&power, content, floor.index, room.def);
            }
        }
    }
}

/// Ticks of reload a gun gets this tick: one, or none if its circuit is
/// dark. An unpowered emplacement — a thorn gun, a tanglenet — is never
/// slowed, which is what keeps a browned-out tower able to answer the
/// ground (`SYSTEMS.md` §6.36).
fn reload_step(
    power: &crate::state::power::Power,
    content: &Content,
    floor: crate::ids::FloorIdx,
    def: crate::ids::RoomIdx,
) -> u32 {
    let draws = content
        .room(def)
        .defence
        .as_ref()
        .is_some_and(|defence| defence.charge_per_shot > 0);
    if !draws {
        return 1;
    }
    // Per-mille of a tick, accumulated by the room's own counter would
    // be finer — but a reload is tens of ticks, so serving it whole
    // ticks at the circuit's rate is within a tick of exact and needs no
    // new state.
    u32::from(power.served_at(floor, PowerUse::Guns) >= crate::state::power::FULL / 2)
}

/// The item a battery eats, resolved once at load like everything else.
fn defence_ammo(content: &Content, room: crate::ids::RoomIdx) -> crate::ids::ItemIdx {
    content
        .room_rt(room)
        .defence_ammo
        .unwrap_or(crate::ids::ItemIdx(0))
}
