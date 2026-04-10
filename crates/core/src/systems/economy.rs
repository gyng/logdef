use crate::registry::Registry;
use crate::snapshot::SoundEvent;
use crate::state::*;
use crate::types::Scalar;

#[allow(clippy::ptr_arg)] // Will push to sounds when kill events are emitted
pub fn run(
    state: &mut GameState,
    _registry: &Registry,
    _dt: Scalar,
    _sounds: &mut Vec<SoundEvent>,
) {
    let encounter = match &state.encounter {
        Some(e) => e,
        None => return,
    };

    // ── Gold from kills ─────────────────────────────────────
    // Check for newly dead enemies and award bounties
    for enemy in &encounter.enemies {
        if enemy.state != EnemyState::Dead {
            continue;
        }
        // Only award bounty once: when hp just dropped to/below 0
        // We use a simple check: dead enemies with hp <= 0 that we haven't counted
        // For MVP, we award bounty in the death cleanup in combat.rs by checking
        // a flag. Instead, let's track it here by checking hp <= 0 and state == Dead.
        // To avoid double-counting, we award bounty when hp transitions below 0
        // (handled via death_cleanup in combat.rs setting state to Dead).
        // For now, we handle bounty at encounter end.
    }

    // ── Encounter completion bonus ──────────────────────────
    if state.phase == GamePhase::PostCombat {
        // This runs once when transitioning — but since we check each tick,
        // we need a flag. For MVP, handle in the phase transition instead.
    }
}

/// Called once when an encounter ends in victory. Awards kill bounties.
pub fn award_encounter_rewards(state: &mut GameState, registry: &Registry) {
    let encounter = match &state.encounter {
        Some(e) => e,
        None => return,
    };

    let mut gold_earned: u32 = 0;
    for enemy in &encounter.enemies {
        if enemy.state == EnemyState::Dead {
            let Some(enemy_def) = registry.enemy_by_archetype(enemy.archetype) else {
                continue;
            };
            // MVP: all kills count as hero kills (1.5x multiplier)
            gold_earned +=
                (enemy_def.bounty as Scalar * registry.balance.economy.hero_kill_multiplier) as u32;
        }
    }
    gold_earned += registry.balance.economy.encounter_completion_bonus;
    state.economy.gold += gold_earned;

    // Encounter XP and stat point reward.
    state.tower.hero.xp += 10;
    let level_threshold = state.tower.hero.level * 30;
    if state.tower.hero.xp >= level_threshold {
        state.tower.hero.xp = 0;
        state.tower.hero.level += 1;
        state.tower.hero.stats.unspent_points += 1;
    }
}
