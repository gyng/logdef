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

use crate::content::Content;
use crate::ids::EnemyId;
use crate::state::GameState;
use crate::state::siege::EnemyState;

use super::SoundEvent;

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
fn cut_what_climbs_in(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    let tick = state.tick;
    let mut struck: Vec<(EnemyId, i64)> = Vec::new();

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
                    struck.push((enemy.id, damage));
                }
            }
        }
    }

    if struck.is_empty() {
        return;
    }
    for (id, damage) in struck {
        let Some(enemy) = state.siege.enemies.iter_mut().find(|enemy| enemy.id == id) else {
            continue;
        };
        if enemy.state.is_going() {
            continue;
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
}

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
    cut_what_climbs_in(state, content, sounds);
    if state.siege.enemies.is_empty() {
        // Still tick reload timers down, so a battery that has been
        // waiting is ready the moment something arrives.
        cool_down(state, content);
        return;
    }

    let mut shots: Vec<(EnemyId, i64)> = Vec::new();
    let tower_at = state.world.distance;
    let tick = state.tick;

    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            let Some(defence) = content.room(room.def).defence.as_ref() else {
                continue;
            };
            // A damaged battery fires more slowly, on exactly the
            // same rule as a damaged mill mills more slowly.
            if !room.is_working(content, tick) {
                continue;
            }

            if room.progress > 0 {
                room.progress -= 1;
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

            // Nearest live creature inside range — unless the player
            // has named one. "Nearest" rather than "weakest" or
            // "strongest" because a battery has no judgement of its own;
            // the player's judgement went into where they put it. A
            // focus does not give the battery judgement either, it gives
            // the player a second moment to supply theirs, and it costs
            // attention during a wave to use.
            //
            // A focus out of range is not a refusal to fire: the
            // emplacement falls back to nearest, because a battery that
            // sat idle while something chewed on the tower would be a
            // trap rather than a decision.
            let range = crate::fx::paces_from_int(defence.range_paces);
            let in_reach = |enemy: &&crate::state::siege::Enemy| {
                !enemy.state.is_going() && (enemy.at - tower_at).abs() <= range
            };
            let focused = state
                .siege
                .focus
                .and_then(|id| state.siege.enemies.iter().find(|enemy| enemy.id == id))
                .filter(|enemy| in_reach(enemy));
            let target = focused.or_else(|| {
                state
                    .siege
                    .enemies
                    .iter()
                    .filter(in_reach)
                    .min_by_key(|enemy| ((enemy.at - tower_at).abs(), enemy.id.0))
            });
            let Some(target) = target else {
                continue;
            };

            ammo.withdraw(defence.ammo_per_shot);
            room.progress = defence.reload_ticks;
            shots.push((target.id, defence.damage));
        }
    }

    if shots.is_empty() {
        return;
    }

    for (id, damage) in shots {
        let Some(enemy) = state.siege.enemies.iter_mut().find(|enemy| enemy.id == id) else {
            continue;
        };
        if enemy.state.is_going() {
            continue;
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
}

/// Reload timers keep running when nothing is in range.
fn cool_down(state: &mut GameState, content: &Content) {
    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            if content.room(room.def).defence.is_some() && room.progress > 0 {
                room.progress -= 1;
            }
        }
    }
}

/// The item a battery eats, resolved once at load like everything else.
fn defence_ammo(content: &Content, room: crate::ids::RoomIdx) -> crate::ids::ItemIdx {
    content
        .room_rt(room)
        .defence_ammo
        .unwrap_or(crate::ids::ItemIdx(0))
}
