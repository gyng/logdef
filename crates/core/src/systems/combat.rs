use crate::registry::Registry;
use crate::snapshot::SoundEvent;
use crate::state::*;
use crate::types::Scalar;

pub fn run(state: &mut GameState, registry: &Registry, dt: Scalar, sounds: &mut Vec<SoundEvent>) {
    // ── Decrement hero cooldowns (before taking encounter borrow) ──
    if state.encounter.is_none() {
        return;
    }
    if state.tower.hero.weapon_ability_cooldown > 0.0 {
        state.tower.hero.weapon_ability_cooldown =
            (state.tower.hero.weapon_ability_cooldown - dt).max(0.0);
    }
    if state.tower.hero.hero_skill_cooldown > 0.0 {
        state.tower.hero.hero_skill_cooldown = (state.tower.hero.hero_skill_cooldown - dt).max(0.0);
    }

    let encounter = state
        .encounter
        .as_mut()
        .expect("encounter presence checked above");

    // ── Death cleanup (runs first so wave checks see accurate state) ──
    for enemy in &mut encounter.enemies {
        if enemy.state != EnemyState::Dead && enemy.hp <= 0.0 {
            enemy.state = EnemyState::Dead;
            let pos = match &enemy.position {
                EnemyPosition::Ground { x } => crate::types::Vec2::new(*x, 0.0),
                _ => crate::types::Vec2::zero(),
            };
            sounds.push(SoundEvent::EnemyDeath {
                archetype: enemy.archetype,
                position: pos,
            });
        }
    }

    // ── Wave management ─────────────────────────────────────
    match &mut encounter.wave_state {
        WaveState::Lull { timer } => {
            *timer -= dt;
            if *timer <= 0.0 {
                encounter.wave_state = WaveState::Active;
                if encounter.current_wave < encounter.waves.len().saturating_sub(1) {
                    encounter.current_wave += 1;
                    sounds.push(SoundEvent::WaveStart {
                        wave_number: encounter.current_wave,
                    });
                }
            }
        }
        WaveState::Active => {
            let all_dead = encounter
                .enemies
                .iter()
                .all(|e| e.state == EnemyState::Dead);
            if all_dead {
                if encounter.current_wave >= encounter.waves.len().saturating_sub(1) {
                    sounds.push(SoundEvent::EncounterVictory);
                    super::economy::award_encounter_rewards(state, registry);
                    state.phase = GamePhase::PostCombat;
                    return;
                } else {
                    encounter.wave_state = WaveState::Lull {
                        timer: registry.balance.combat.wave_delay,
                    };
                }
            }
        }
    }

    // ── Enemy movement ──────────────────────────────────────
    let num_floors_for_climb = state.tower.floors.len().max(1) as Scalar;
    for enemy in &mut encounter.enemies {
        if enemy.state == EnemyState::Dead {
            continue;
        }

        // Ground enemies walk left until they reach the tower face.
        if let EnemyPosition::Ground { x } = &mut enemy.position
            && enemy.state == EnemyState::Approaching
            && *x > 0.0
        {
            *x -= enemy.speed * dt;
            if *x <= 0.0 {
                *x = 0.0;
                // Climbers transition to climbing the tower face.
                // Other ground enemies begin attacking the base panel.
                match enemy.archetype {
                    EnemyArchetype::Climber => {
                        enemy.position = EnemyPosition::Climbing { floor: 0.0 };
                        enemy.state = EnemyState::Climbing;
                    }
                    _ => {
                        enemy.state = EnemyState::AttackingPanel;
                    }
                }
            }
        }

        // Climbers progress up the tower face until they reach the top.
        if let EnemyPosition::Climbing { floor } = &mut enemy.position
            && enemy.state == EnemyState::Climbing
        {
            // Climb at (speed / 60) floors per second.
            *floor += (enemy.speed / 60.0) * dt;
            if *floor >= num_floors_for_climb {
                *floor = num_floors_for_climb;
                // Top of the tower — attack panels
                enemy.state = EnemyState::AttackingPanel;
            }
        }

        // Flyers hover above ground and approach. For MVP they get
        // translated to Ground { x } equivalents — vertical movement
        // is visual only.
        if let EnemyPosition::Flying { x, y: _ } = &mut enemy.position
            && enemy.state == EnemyState::Approaching
        {
            *x -= enemy.speed * dt;
            if *x <= 0.0 {
                enemy.state = EnemyState::AttackingPanel;
            }
        }
    }

    // ── Enemy attacking panels ──────────────────────────────
    let num_floors = state.tower.floors.len();
    for enemy in &mut encounter.enemies {
        if enemy.state != EnemyState::AttackingPanel {
            continue;
        }
        let Some(enemy_def) = registry.enemy_by_archetype(enemy.archetype) else {
            continue;
        };
        let damage_per_tick = enemy_def.damage * enemy_def.attack_rate * dt;
        if num_floors > 0 {
            let floor = &mut state.tower.floors[0];
            if !floor.panel.is_breached {
                floor.panel.current_hp -= damage_per_tick;
                if floor.panel.current_hp <= 0.0 {
                    floor.panel.current_hp = 0.0;
                    floor.panel.is_breached = true;
                    sounds.push(SoundEvent::PanelBreach { floor: 0 });
                }
            }
        } else {
            state.tower.foundation.current_hp -= damage_per_tick;
        }
    }

    // ── Foundation damage from breached panels ──────────────
    let breached_count = state
        .tower
        .floors
        .iter()
        .filter(|f| f.panel.is_breached)
        .count();
    if breached_count > 0 {
        state.tower.foundation.current_hp -= breached_count as Scalar * 2.0 * dt;
    }

    // ── Defeat check ────────────────────────────────────────
    if state.tower.foundation.current_hp <= 0.0 {
        state.tower.foundation.current_hp = 0.0;
        sounds.push(SoundEvent::EncounterDefeat);
        state.phase = GamePhase::GameOver;
    }
}
