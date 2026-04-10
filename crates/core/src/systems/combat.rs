use crate::balance::*;
use crate::snapshot::SoundEvent;
use crate::state::*;
use crate::types::Scalar;

pub fn run(state: &mut GameState, dt: Scalar, sounds: &mut Vec<SoundEvent>) {
    let encounter = match &mut state.encounter {
        Some(e) => e,
        None => return,
    };

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
                    super::economy::award_encounter_rewards(state);
                    state.phase = GamePhase::PostCombat;
                    return;
                } else {
                    encounter.wave_state = WaveState::Lull { timer: WAVE_DELAY };
                }
            }
        }
    }

    // ── Enemy movement ──────────────────────────────────────
    for enemy in &mut encounter.enemies {
        if enemy.state == EnemyState::Dead {
            continue;
        }
        if let EnemyPosition::Ground { x } = &mut enemy.position
            && *x > 0.0
            && enemy.state == EnemyState::Approaching
        {
            *x -= enemy.speed * dt;
            if *x <= 0.0 {
                *x = 0.0;
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
        let damage_per_tick = match enemy.archetype {
            EnemyArchetype::Grunt => GRUNT_DAMAGE * GRUNT_ATTACK_RATE * dt,
            EnemyArchetype::Runner => RUNNER_ENEMY_DAMAGE * RUNNER_ENEMY_ATTACK_RATE * dt,
            EnemyArchetype::Armored => ARMORED_DAMAGE * ARMORED_ATTACK_RATE * dt,
            _ => GRUNT_DAMAGE * dt,
        };
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
