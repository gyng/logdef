use crate::balance::*;
use crate::command::*;
use crate::engine::GameEngine;
use crate::state::*;
use crate::types::*;

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
fn new_game_starts_in_map_view() {
    let engine = GameEngine::new(42, HeroClass::Archer);
    assert_eq!(engine.state.phase, GamePhase::MapView);
}

#[test]
fn new_game_hero_matches_class() {
    let engine = GameEngine::new(42, HeroClass::Commander);
    assert_eq!(engine.state.tower.hero.class, HeroClass::Commander);
    assert_eq!(engine.state.tower.hero.hero_skill, HeroSkill::Rally);
}

#[test]
fn new_game_has_correct_starting_resources() {
    let engine = GameEngine::new(42, HeroClass::Archer);
    assert_eq!(engine.state.economy.gold, STARTING_GOLD);
    let wood = engine
        .state
        .economy
        .materials
        .iter()
        .find(|m| m.resource == ResourceType::Wood)
        .unwrap();
    assert_eq!(wood.current, STARTING_WOOD);
    let stone = engine
        .state
        .economy
        .materials
        .iter()
        .find(|m| m.resource == ResourceType::Stone)
        .unwrap();
    assert_eq!(stone.current, STARTING_STONE);
}

#[test]
fn new_game_has_hardcoded_map() {
    let engine = GameEngine::new(42, HeroClass::Archer);
    assert_eq!(engine.state.journey.chapters.len(), 1);
    let chapter = &engine.state.journey.chapters[0];
    assert_eq!(chapter.nodes.len(), 4);
    assert_eq!(chapter.edges.len(), 3);
}

#[test]
fn tick_outside_encounter_is_noop() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    assert!(engine.state.encounter.is_none());
    let tick_before = engine.state.tick;
    let sounds = engine.tick(0.1);
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
// Command processing — Prep
// ---------------------------------------------------------------------------

#[test]
fn select_node_transitions_to_travel() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    assert_eq!(engine.state.phase, GamePhase::MapView);

    let result = engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });
    assert!(matches!(result, CommandResult::Ok));
    assert_eq!(engine.state.phase, GamePhase::Travel);
    assert_eq!(engine.state.economy.ticks_remaining, CH1_TICKS);
}

#[test]
fn select_unreachable_node_fails() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    // Node 3 is not reachable from node 0 (only node 1 is)
    let result = engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(3),
    });
    assert!(matches!(result, CommandResult::Error(_)));
}

#[test]
fn build_floor_deducts_ticks_and_materials() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    // First move to Travel phase
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });

    let ticks_before = engine.state.economy.ticks_remaining;
    let wood_before = engine
        .state
        .economy
        .materials
        .iter()
        .find(|m| m.resource == ResourceType::Wood)
        .unwrap()
        .current;

    let result = engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    assert!(matches!(result, CommandResult::Ok));

    assert_eq!(
        engine.state.economy.ticks_remaining,
        ticks_before - FLOOR_TICK_COST
    );
    let wood_after = engine
        .state
        .economy
        .materials
        .iter()
        .find(|m| m.resource == ResourceType::Wood)
        .unwrap()
        .current;
    assert_eq!(wood_after, wood_before - WOOD_FLOOR_MATERIAL_COST);
    assert_eq!(engine.state.tower.floors.len(), 1);
    assert_eq!(engine.state.tower.balconies.len(), 1);
}

#[test]
fn build_floor_fails_without_ticks() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });
    // Drain ticks
    engine.state.economy.ticks_remaining = 1;

    let result = engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    assert!(matches!(
        result,
        CommandResult::Error(CommandError::InsufficientTicks { .. })
    ));
}

#[test]
fn place_building_on_floor() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });

    let result = engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        building_type: BuildingType::Fletcher,
    });
    assert!(matches!(result, CommandResult::Ok));
    assert!(engine.state.tower.floors[0].building.is_some());
    let building = engine.state.tower.floors[0].building.as_ref().unwrap();
    assert_eq!(building.building_type, BuildingType::Fletcher);
}

#[test]
fn place_building_on_occupied_floor_fails() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        building_type: BuildingType::Fletcher,
    });

    let result = engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        building_type: BuildingType::Fletcher,
    });
    assert!(matches!(
        result,
        CommandResult::Error(CommandError::FloorOccupied { .. })
    ));
}

// ---------------------------------------------------------------------------
// Command processing — Combat
// ---------------------------------------------------------------------------

#[test]
fn march_transitions_to_encounter() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });

    let result = engine.send_command(GameCommand::March);
    assert!(matches!(result, CommandResult::Ok));
    assert_eq!(engine.state.phase, GamePhase::Encounter);
    assert!(engine.state.encounter.is_some());

    let encounter = engine.state.encounter.as_ref().unwrap();
    assert!(!encounter.enemies.is_empty());
}

#[test]
fn march_converts_unused_ticks_to_gold() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });

    let gold_before = engine.state.economy.gold;
    let ticks = engine.state.economy.ticks_remaining;
    engine.send_command(GameCommand::March);

    assert_eq!(
        engine.state.economy.gold,
        gold_before + ticks * UNUSED_TICK_GOLD
    );
}

#[test]
fn fire_creates_projectile_and_costs_ammo() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });
    engine.send_command(GameCommand::March);

    let ammo_before = engine.state.tower.hero.personal_ammo;
    let result = engine.send_command(GameCommand::Fire);
    assert!(matches!(result, CommandResult::Ok));

    assert_eq!(engine.state.tower.hero.personal_ammo, ammo_before - 1);
    let projectiles = &engine.state.encounter.as_ref().unwrap().projectiles;
    assert_eq!(projectiles.len(), 1);
}

#[test]
fn fire_melee_costs_no_ammo() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });
    engine.send_command(GameCommand::March);
    engine.send_command(GameCommand::SwitchWeapon); // Switch to dagger

    let ammo_before = engine.state.tower.hero.personal_ammo;
    let result = engine.send_command(GameCommand::Fire);
    assert!(matches!(result, CommandResult::Ok));
    assert_eq!(engine.state.tower.hero.personal_ammo, ammo_before);
}

#[test]
fn fire_with_no_ammo_fails() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });
    engine.send_command(GameCommand::March);
    engine.state.tower.hero.personal_ammo = 0;

    let result = engine.send_command(GameCommand::Fire);
    assert!(matches!(
        result,
        CommandResult::Error(CommandError::InsufficientMaterials { .. })
    ));
}

#[test]
fn switch_weapon_toggles() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });
    engine.send_command(GameCommand::March);

    assert_eq!(engine.state.tower.hero.active_weapon, WeaponSlot::Primary);
    engine.send_command(GameCommand::SwitchWeapon);
    assert_eq!(engine.state.tower.hero.active_weapon, WeaponSlot::Secondary);
    engine.send_command(GameCommand::SwitchWeapon);
    assert_eq!(engine.state.tower.hero.active_weapon, WeaponSlot::Primary);
}

#[test]
fn fire_in_wrong_phase_fails() {
    let engine = &mut GameEngine::new(42, HeroClass::Archer);
    // Phase is MapView, not Encounter
    let result = engine.send_command(GameCommand::Fire);
    assert!(matches!(
        result,
        CommandResult::Error(CommandError::WrongPhase { .. })
    ));
}

// ---------------------------------------------------------------------------
// Encounter snapshot
// ---------------------------------------------------------------------------

#[test]
fn encounter_snapshot_available_during_combat() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    assert!(engine.get_encounter_state().is_none());

    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });
    engine.send_command(GameCommand::March);

    let snap = engine.get_encounter_state();
    assert!(snap.is_some());
    let snap = snap.unwrap();
    assert!(!snap.enemies.is_empty());
    assert!(snap.enemies_remaining > 0);
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

#[test]
fn deterministic_command_sequence() {
    let mut a = GameEngine::new(42, HeroClass::Archer);
    let mut b = GameEngine::new(42, HeroClass::Archer);

    for engine in [&mut a, &mut b] {
        engine.send_command(GameCommand::SelectNode {
            node: crate::types::NodeId(1),
        });
        engine.send_command(GameCommand::BuildFloor {
            material: FloorMaterial::Wood,
        });
        engine.send_command(GameCommand::March);
        engine.send_command(GameCommand::Fire);
        engine.send_command(GameCommand::Fire);
    }

    assert_eq!(
        a.save(),
        b.save(),
        "Same seed + same commands must produce identical state"
    );
}

// ---------------------------------------------------------------------------
// Combat simulation
// ---------------------------------------------------------------------------

fn setup_combat_engine() -> GameEngine {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });
    engine.send_command(GameCommand::March);
    engine
}

#[test]
fn enemies_move_toward_tower() {
    let mut engine = setup_combat_engine();
    let enemy_x_before = match &engine.state.encounter.as_ref().unwrap().enemies[0].position {
        EnemyPosition::Ground { x } => *x,
        _ => panic!("Expected ground enemy"),
    };

    // Run 30 ticks (1 second)
    for _ in 0..30 {
        engine.tick(crate::types::FIXED_DT);
    }

    let enemy_x_after = match &engine.state.encounter.as_ref().unwrap().enemies[0].position {
        EnemyPosition::Ground { x } => *x,
        _ => panic!("Expected ground enemy"),
    };
    assert!(
        enemy_x_after < enemy_x_before,
        "Enemy should move left: was {enemy_x_before}, now {enemy_x_after}"
    );
}

#[test]
fn projectile_kills_enemy() {
    let mut engine = setup_combat_engine();

    // Place an enemy close so projectile reaches it quickly
    if let Some(encounter) = &mut engine.state.encounter {
        encounter.enemies.clear();
        let id = EnemyId(999);
        encounter.enemies.push(Enemy {
            id,
            archetype: EnemyArchetype::Grunt,
            position: EnemyPosition::Ground { x: 100.0 },
            hp: 15.0, // One shortbow shot kills it
            max_hp: 15.0,
            speed: 0.0, // Stationary for test
            state: EnemyState::Approaching,
            stuck_arrows: Vec::new(),
        });
    }

    // Fire
    engine.send_command(GameCommand::Fire);

    // Tick until projectile reaches enemy (~100px / 300px/s = ~0.33s = ~10 ticks)
    for _ in 0..30 {
        engine.tick(crate::types::FIXED_DT);
    }

    let enemy = &engine.state.encounter.as_ref().unwrap().enemies[0];
    assert_eq!(enemy.state, EnemyState::Dead, "Enemy should be dead");
}

#[test]
fn encounter_ends_when_all_enemies_dead() {
    let mut engine = setup_combat_engine();
    win_encounter(&mut engine);
    assert_eq!(
        engine.state.phase,
        GamePhase::PostCombat,
        "Phase should be PostCombat after all enemies dead"
    );
}

/// Helper: kill all enemies and tick until PostCombat
fn win_encounter(engine: &mut GameEngine) {
    if let Some(encounter) = &mut engine.state.encounter {
        for (i, enemy) in encounter.enemies.iter_mut().enumerate() {
            enemy.hp = 1.0;
            enemy.position = EnemyPosition::Ground {
                x: 50.0 + i as Scalar * 30.0,
            };
            enemy.speed = 0.0;
        }
    }
    let enemy_count = engine.state.encounter.as_ref().unwrap().enemies.len();
    for _ in 0..(enemy_count + 5) {
        engine.send_command(GameCommand::Fire);
        for _ in 0..10 {
            engine.tick(FIXED_DT);
        }
        if engine.state.phase == GamePhase::PostCombat {
            return;
        }
    }
}

#[test]
fn encounter_awards_gold() {
    let mut engine = setup_combat_engine();
    let gold_before = engine.state.economy.gold;
    win_encounter(&mut engine);

    assert!(
        engine.state.economy.gold > gold_before,
        "Gold should increase after encounter: was {gold_before}, now {}",
        engine.state.economy.gold
    );
}

#[test]
fn continue_journey_returns_to_map() {
    let mut engine = setup_combat_engine();
    win_encounter(&mut engine);

    assert_eq!(engine.state.phase, GamePhase::PostCombat);
    engine.send_command(GameCommand::ContinueJourney);
    assert_eq!(engine.state.phase, GamePhase::MapView);
}
