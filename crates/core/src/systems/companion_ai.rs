use crate::registry::Registry;
use crate::snapshot::SoundEvent;
use crate::state::*;
use crate::types::*;

/// Companion AI: each assigned companion with a ranged weapon picks a
/// target based on its target order and fires a projectile on cooldown.
///
/// MVP simplification: companions fire from `x = 0` (tower face, same
/// lane as the hero). Vertical balcony positions don't yet affect
/// trajectories — that's post-MVP.
pub fn run(state: &mut GameState, _registry: &Registry, _dt: Scalar, sounds: &mut Vec<SoundEvent>) {
    let Some(encounter) = state.encounter.as_mut() else {
        return;
    };

    // No alive enemies → nothing to do
    if !encounter
        .enemies
        .iter()
        .any(|e| e.state != EnemyState::Dead)
    {
        return;
    }

    // Walk each assigned companion. Kael (Wall) and Patch (heal)-ish
    // passives intentionally do not fire in this MVP pass.
    let mut spawn_projectiles: Vec<Projectile> = Vec::new();

    for companion in state.tower.companions.iter_mut() {
        if companion.position.is_none() || companion.injured {
            continue;
        }

        // Wall companions physically block but don't shoot.
        if matches!(
            companion.passive,
            CompanionPassive::Wall | CompanionPassive::FieldMedic
        ) {
            continue;
        }

        // Cooldown uses the `weapon_ability_cooldown` field on companion
        // repurposed; simpler: store a per-companion fire timer in
        // `injury_remaining` for MVP? No — that's gross. For MVP do
        // rate-limited fire by comparing tick % interval.

        // Fire at a rate_per_sec = weapon.fire_rate * 0.7 (companion
        // multiplier). At 30hz, interval_ticks = 30 / (fire_rate * 0.7).
        let fire_rate = companion.weapon.fire_rate.max(0.1) * 0.7;
        let interval_ticks = (30.0 / fire_rate).max(1.0) as u64;

        // Skip unless this tick is a fire tick for this companion. Use
        // companion id as a small offset so they don't all fire on the
        // exact same tick.
        let offset = (companion.id.0 as u64) % interval_ticks.max(1);
        if (state.tick + offset) % interval_ticks != 0 {
            continue;
        }

        // Pick a target by target_order
        let Some(target_idx) = pick_target(&encounter.enemies, companion.target_order) else {
            continue;
        };
        let target = &encounter.enemies[target_idx];
        let target_x = match target.position {
            EnemyPosition::Ground { x } => x,
            _ => continue,
        };

        // Apply companion accuracy: a random miss.
        let roll = state.rng.next_f32();
        if roll > companion.accuracy.clamp(0.0, 1.0) {
            continue;
        }

        // Damage = weapon damage * (1 + combat_xp growth scaling)
        let xp_bonus = (companion.combat_xp as Scalar / 50.0).min(0.5);
        let damage = companion.weapon.damage * (1.0 + xp_bonus);
        let speed = match companion.weapon.base_type {
            WeaponBaseType::Bow => 300.0,
            WeaponBaseType::Crossbow => 400.0,
            WeaponBaseType::Staff => 500.0,
            WeaponBaseType::Thrown => 250.0,
            WeaponBaseType::Melee => continue, // no projectile
        };

        spawn_projectiles.push(Projectile {
            id: ProjectileId(0), // overwritten below
            source: companion.id,
            weapon_type: companion.weapon.base_type,
            position: Vec2::new(0.0, 0.0),
            velocity: Vec2::new(speed, 0.0),
            gravity: 0.0,
            damage,
            modifier: None,
            state: ProjectileState::Flying,
        });

        let pos = Vec2::new(target_x, 0.0);
        sounds.push(SoundEvent::WeaponFire {
            weapon_type: companion.weapon.base_type,
            position: pos,
        });
    }

    // Assign IDs and push to encounter
    for mut proj in spawn_projectiles {
        // Use a deterministic per-tick suffix: source.0 * 1000 + tick_nano
        let id_base = proj.source.0;
        proj.id = ProjectileId(id_base.wrapping_mul(997).wrapping_add(state.tick as u32));
        encounter.projectiles.push(proj);
    }
}

/// Pick an enemy index according to the companion's target preference.
fn pick_target(enemies: &[Enemy], order: TargetOrder) -> Option<usize> {
    let alive: Vec<(usize, &Enemy)> = enemies
        .iter()
        .enumerate()
        .filter(|(_, e)| e.state != EnemyState::Dead)
        .filter(|(_, e)| matches!(e.position, EnemyPosition::Ground { .. }))
        .collect();
    if alive.is_empty() {
        return None;
    }

    let pick = match order {
        TargetOrder::Closest => alive
            .iter()
            .min_by(|a, b| position_x(a.1).partial_cmp(&position_x(b.1)).unwrap()),
        TargetOrder::Strongest => alive
            .iter()
            .max_by(|a, b| a.1.max_hp.partial_cmp(&b.1.max_hp).unwrap()),
        TargetOrder::Weakest => alive
            .iter()
            .min_by(|a, b| a.1.hp.partial_cmp(&b.1.hp).unwrap()),
        TargetOrder::Climbers => alive
            .iter()
            .find(|(_, e)| e.archetype == EnemyArchetype::Climber)
            .or_else(|| {
                alive
                    .iter()
                    .min_by(|a, b| position_x(a.1).partial_cmp(&position_x(b.1)).unwrap())
            }),
    };

    pick.map(|(idx, _)| *idx)
}

fn position_x(enemy: &Enemy) -> Scalar {
    match enemy.position {
        EnemyPosition::Ground { x } => x,
        _ => Scalar::INFINITY,
    }
}
