use crate::balance::*;
use crate::snapshot::SoundEvent;
use crate::state::*;
use crate::types::Scalar;

pub fn run(state: &mut GameState, dt: Scalar, sounds: &mut Vec<SoundEvent>) {
    let encounter = match &mut state.encounter {
        Some(e) => e,
        None => return,
    };

    // ── Move projectiles ────────────────────────────────────
    for proj in &mut encounter.projectiles {
        if !matches!(proj.state, ProjectileState::Flying) {
            continue;
        }
        proj.position.x += proj.velocity.x * dt;
        proj.position.y += proj.velocity.y * dt;

        // Out of bounds → remove
        if proj.position.x > BATTLEFIELD_WIDTH || proj.position.x < -50.0 {
            proj.state = ProjectileState::OnGround;
            sounds.push(SoundEvent::ProjectileMiss {
                position: proj.position,
            });
        }
    }

    // ── Sweep collision ─────────────────────────────────────
    // Simple 1D: check if any flying projectile's x overlaps an alive enemy's x
    // We need to collect hits first, then apply damage (avoid double-borrow)
    let mut hits: Vec<(usize, usize, Scalar)> = Vec::new(); // (proj_idx, enemy_idx, damage)

    for (pi, proj) in encounter.projectiles.iter().enumerate() {
        if !matches!(proj.state, ProjectileState::Flying) {
            continue;
        }
        for (ei, enemy) in encounter.enemies.iter().enumerate() {
            if enemy.state == EnemyState::Dead {
                continue;
            }
            let enemy_x = match &enemy.position {
                EnemyPosition::Ground { x } => *x,
                _ => continue,
            };
            // Hit if projectile x is within 15px of enemy x (hit tolerance)
            if (proj.position.x - enemy_x).abs() < 15.0 {
                hits.push((pi, ei, proj.damage));
                break; // One projectile hits one enemy
            }
        }
    }

    // Apply hits
    for (pi, ei, damage) in hits {
        encounter.projectiles[pi].state = ProjectileState::Stuck {
            in_entity: encounter.enemies[ei].id,
        };
        encounter.enemies[ei].hp -= damage;

        let pos = match &encounter.enemies[ei].position {
            EnemyPosition::Ground { x } => crate::types::Vec2::new(*x, 0.0),
            _ => crate::types::Vec2::zero(),
        };
        sounds.push(SoundEvent::ProjectileHit {
            weapon_type: encounter.projectiles[pi].weapon_type,
            position: pos,
        });
    }

    // ── Cleanup non-flying projectiles to prevent unbounded growth ──
    encounter
        .projectiles
        .retain(|p| matches!(p.state, ProjectileState::Flying));
}
