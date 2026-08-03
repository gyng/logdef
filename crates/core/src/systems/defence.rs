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

pub fn run(state: &mut GameState, content: &Content, sounds: &mut Vec<SoundEvent>) {
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

            // Nearest live creature inside range. "Nearest" rather than
            // "weakest" or "strongest" because a battery has no
            // judgement — the player's judgement went into where they
            // put it.
            let range = crate::fx::paces_from_int(defence.range_paces);
            let target = state
                .siege
                .enemies
                .iter()
                .filter(|enemy| !enemy.state.is_going())
                .filter(|enemy| (enemy.at - tower_at).abs() <= range)
                .min_by_key(|enemy| ((enemy.at - tower_at).abs(), enemy.id.0));
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
            sounds.push(SoundEvent::EnemyDown);
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
