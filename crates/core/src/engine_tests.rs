use crate::command::*;
use crate::engine::GameEngine;
use crate::state::*;

// ---------------------------------------------------------------------------
// Command roundtrip (serde)
// ---------------------------------------------------------------------------

#[test]
fn command_unit_variant_roundtrip() {
    let cmds = vec![
        GameCommand::Fire,
        GameCommand::SwitchWeapon,
        GameCommand::UseWeaponAbility,
        GameCommand::UseHeroSkill,
        GameCommand::March,
        GameCommand::HireRunner,
        GameCommand::ContinueJourney,
        GameCommand::SaveGame,
    ];
    for cmd in cmds {
        let json = serde_json::to_string(&cmd).unwrap();
        let back: GameCommand = serde_json::from_str(&json).unwrap();
        let re_json = serde_json::to_string(&back).unwrap();
        assert_eq!(json, re_json, "roundtrip failed for: {json}");
    }
}

#[test]
fn command_struct_variant_roundtrip() {
    let cmd = GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let back: GameCommand = serde_json::from_str(&json).unwrap();
    let re_json = serde_json::to_string(&back).unwrap();
    assert_eq!(json, re_json);
}

#[test]
fn command_result_roundtrip() {
    let ok = CommandResult::Ok;
    let err = CommandResult::Error(CommandError::WrongPhase {
        expected: "Travel".into(),
        actual: "Encounter".into(),
    });
    for result in [ok, err] {
        let json = serde_json::to_string(&result).unwrap();
        let _back: CommandResult = serde_json::from_str(&json).unwrap();
    }
}

// ---------------------------------------------------------------------------
// Snapshot shape (catches unintentional bridge changes)
// ---------------------------------------------------------------------------

#[test]
fn hud_snapshot_has_expected_fields() {
    let engine = GameEngine::new(42, HeroClass::Archer);
    let hud = engine.get_hud_state();
    let json = serde_json::to_value(&hud).unwrap();

    let obj = json.as_object().expect("HudSnapshot should be an object");
    let expected_keys = [
        "ammo_primary",
        "ammo_secondary",
        "personal_ammo",
        "weapon_ability_cooldown",
        "hero_skill_cooldown",
        "active_weapon",
        "current_wave",
        "total_waves",
        "wave_state",
        "gold",
        "companion_statuses",
        "hero_hp_fraction",
        "tower_hp_fraction",
        "enemies_remaining",
    ];
    for key in expected_keys {
        assert!(obj.contains_key(key), "HudSnapshot missing field: {key}");
    }
}

#[test]
fn tower_snapshot_has_expected_fields() {
    let engine = GameEngine::new(42, HeroClass::Archer);
    let tower = engine.get_tower_state();
    let json = serde_json::to_value(&tower).unwrap();

    let obj = json.as_object().expect("TowerSnapshot should be an object");
    for key in ["floors", "warehouse", "runners", "width"] {
        assert!(obj.contains_key(key), "TowerSnapshot missing field: {key}");
    }
}

#[test]
fn hero_snapshot_has_expected_fields() {
    let engine = GameEngine::new(42, HeroClass::Archer);
    let hero = engine.get_hero_state();
    let json = serde_json::to_value(&hero).unwrap();

    let obj = json.as_object().expect("HeroSnapshot should be an object");
    for key in [
        "class",
        "level",
        "xp",
        "stats",
        "perks",
        "weapon_primary",
        "weapon_secondary",
        "trinket",
    ] {
        assert!(obj.contains_key(key), "HeroSnapshot missing field: {key}");
    }
}

// ---------------------------------------------------------------------------
// Engine basics
// ---------------------------------------------------------------------------

#[test]
fn new_game_starts_in_travel_phase() {
    let engine = GameEngine::new(42, HeroClass::Archer);
    assert_eq!(engine.state.phase, GamePhase::Travel);
}

#[test]
fn new_game_hero_matches_class() {
    let engine = GameEngine::new(42, HeroClass::Commander);
    assert_eq!(engine.state.tower.hero.class, HeroClass::Commander);
    assert_eq!(engine.state.tower.hero.hero_skill, HeroSkill::Rally);
}

#[test]
fn tick_outside_encounter_is_noop() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    assert!(engine.state.encounter.is_none());
    let tick_before = engine.state.tick;
    let sounds = engine.tick(0.1);
    // Tick counter still advances, but no sounds produced (no encounter)
    assert!(sounds.is_empty());
    assert!(engine.state.tick > tick_before);
}

#[test]
fn save_load_roundtrip() {
    let engine = GameEngine::new(12345, HeroClass::Engineer);
    let saved = engine.save();
    let mut engine2 = GameEngine::new(0, HeroClass::Archer);
    engine2.load(&saved).unwrap();
    assert_eq!(engine2.state.meta.seed, 12345);
    assert_eq!(engine2.state.tower.hero.class, HeroClass::Engineer);
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

#[test]
fn deterministic_new_game() {
    let a = GameEngine::new(42, HeroClass::Archer);
    let b = GameEngine::new(42, HeroClass::Archer);
    assert_eq!(
        a.save(),
        b.save(),
        "Same seed + class must produce identical GameState"
    );
}

#[test]
fn deterministic_tick_sequence() {
    let mut a = GameEngine::new(42, HeroClass::Archer);
    let mut b = GameEngine::new(42, HeroClass::Archer);
    // Run several ticks
    for _ in 0..100 {
        a.tick(1.0 / 30.0);
        b.tick(1.0 / 30.0);
    }
    assert_eq!(
        a.save(),
        b.save(),
        "Same seed + same ticks must produce identical state"
    );
}
