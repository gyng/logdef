use crate::registry::Registry;
use crate::snapshot::SoundEvent;
use crate::state::*;
use crate::types::Scalar;

pub fn run(state: &mut GameState, registry: &Registry, dt: Scalar, sounds: &mut Vec<SoundEvent>) {
    let encounter = match &mut state.encounter {
        Some(e) => e,
        None => return,
    };

    let battlefield_width = registry.balance.combat.battlefield_width;
    // Projectiles travel beyond the visible battlefield to reach enemies
    // that have not yet entered view (they spawn beyond battlefield_width
    // and walk left).
    let projectile_max_range = battlefield_width * 2.0;

    // ── Move projectiles ────────────────────────────────────
    // Sim convention: y positive is up (height above ground). Hero
    // shoots from y > 0 on the top balcony; gravity (negative) pulls
    // projectiles back toward y = 0 in a parabolic arc. Collision is
    // still 1D on x — projectiles hit any enemy whose x they cross
    // before they hit the ground or fly off the battlefield.
    for proj in &mut encounter.projectiles {
        if !matches!(proj.state, ProjectileState::Flying) {
            continue;
        }
        proj.velocity.y += proj.gravity * dt;
        proj.position.x += proj.velocity.x * dt;
        proj.position.y += proj.velocity.y * dt;

        // Out of bounds: walked off either side, or fell below the
        // ground. Strictly `< 0` so projectiles that spawn at y=0
        // (companions firing from balcony level with gravity=0) stay
        // in flight until they cross the boundary.
        let out_of_x = proj.position.x > projectile_max_range || proj.position.x < -50.0;
        let hit_ground = proj.position.y < 0.0;
        if out_of_x || hit_ground {
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
            // Projectile position is 1D on the horizontal lane; resolve
            // every enemy archetype to a comparable x coordinate.
            let enemy_x = match &enemy.position {
                EnemyPosition::Ground { x } => *x,
                EnemyPosition::Climbing { .. } => 0.0, // at the tower face
                EnemyPosition::Flying { x, .. } => *x,
                EnemyPosition::AtBase => 0.0,
                EnemyPosition::AtPanel { .. } => 0.0,
            };
            if (proj.position.x - enemy_x).abs() < 15.0 {
                hits.push((pi, ei, proj.damage));
                break;
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
