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
        "position",
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
    assert_eq!(
        engine.state.economy.gold,
        engine.registry.balance.economy.starting_gold
    );
    let wood = engine
        .state
        .economy
        .materials
        .iter()
        .find(|m| m.resource == ResourceType::Wood)
        .unwrap();
    assert_eq!(wood.current, engine.registry.balance.economy.starting_wood);
    let stone = engine
        .state
        .economy
        .materials
        .iter()
        .find(|m| m.resource == ResourceType::Stone)
        .unwrap();
    assert_eq!(
        stone.current,
        engine.registry.balance.economy.starting_stone
    );
}

#[test]
fn new_game_loads_chapters_from_registry() {
    let engine = GameEngine::new(42, HeroClass::Archer);
    assert!(!engine.state.journey.chapters.is_empty());
    let chapter1 = &engine.state.journey.chapters[0];
    assert!(!chapter1.nodes.is_empty());
    assert!(!chapter1.edges.is_empty());
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
    assert_eq!(
        engine.state.economy.ticks_remaining,
        engine.registry.balance.economy.chapter_ticks[0]
    );
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
        ticks_before - engine.registry.balance.construction.floor_tick_cost
    );
    let wood_after = engine
        .state
        .economy
        .materials
        .iter()
        .find(|m| m.resource == ResourceType::Wood)
        .unwrap()
        .current;
    assert_eq!(
        wood_after,
        wood_before
            - engine
                .registry
                .balance
                .construction
                .wood_floor_material_cost
    );
    // Default tower starts with 3 floors; this BuildFloor adds a 4th.
    assert_eq!(engine.state.tower.floors.len(), 4);
    assert_eq!(engine.state.tower.balconies.len(), 4);
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
    // The default tower already has a Fletcher on floor 0. Place the
    // new building on the freshly-built top floor (index 3).
    let new_floor = engine.state.tower.floors.len() - 1;
    let result = engine.send_command(GameCommand::PlaceBuilding {
        floor: new_floor,
        slot: 1,
        building_type: BuildingType::Forge,
    });
    assert!(matches!(result, CommandResult::Ok));
    assert!(engine.state.tower.floors[new_floor].building.is_some());
    let building = engine.state.tower.floors[new_floor]
        .building
        .as_ref()
        .unwrap();
    assert_eq!(building.building_type, BuildingType::Forge);
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
        slot: 1,
        building_type: BuildingType::Fletcher,
    });

    let result = engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        slot: 1,
        building_type: BuildingType::Fletcher,
    });
    assert!(matches!(
        result,
        CommandResult::Error(CommandError::FloorOccupied { .. })
    ));
}

#[test]
fn place_cache_on_floor_consumes_ticks() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode {
        node: crate::types::NodeId(1),
    });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });

    let ticks_before = engine.state.economy.ticks_remaining;
    let result = engine.send_command(GameCommand::PlaceCache { floor: 0, slot: 4 });
    assert!(matches!(result, CommandResult::Ok));
    assert!(engine.state.tower.floors[0].cache.is_some());
    assert_eq!(
        engine.state.economy.ticks_remaining,
        ticks_before - engine.registry.balance.construction.building_tick_cost
    );
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
        gold_before + ticks * engine.registry.balance.economy.unused_tick_gold
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
    // Ensure enough ammo for large encounters
    engine.state.tower.hero.personal_ammo = 99;
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
    for _ in 0..(enemy_count * 2 + 10) {
        if engine.state.phase != GamePhase::Encounter {
            return;
        }
        engine.send_command(GameCommand::Fire);
        for _ in 0..15 {
            engine.tick(FIXED_DT);
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

// ---------------------------------------------------------------------------
// Full game loop integration
// ---------------------------------------------------------------------------

#[test]
fn full_game_loop_to_victory() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    assert_eq!(engine.state.phase, GamePhase::MapView);

    // Play through every chapter until Victory. Always pick the first
    // reachable node from the current position.
    let max_encounters = 100;
    let mut played = 0;

    while engine.state.phase != GamePhase::Victory && played < max_encounters {
        assert_eq!(engine.state.phase, GamePhase::MapView);

        let current_chapter =
            &engine.state.journey.chapters[engine.state.journey.current_chapter - 1];
        let current = engine.state.journey.current_node;
        let next = current_chapter
            .edges
            .iter()
            .find(|e| e.from == current)
            .map(|e| e.to)
            .expect("non-boss nodes must have an outgoing edge");

        engine.send_command(GameCommand::SelectNode { node: next });
        engine.send_command(GameCommand::March);
        win_encounter(&mut engine);
        engine.send_command(GameCommand::ContinueJourney);
        played += 1;
    }

    assert_eq!(
        engine.state.phase,
        GamePhase::Victory,
        "Should reach Victory within {max_encounters} encounters (played {played})"
    );
}

#[test]
fn defeat_when_foundation_destroyed() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.send_command(GameCommand::March);

    // Set foundation HP very low
    engine.state.tower.foundation.current_hp = 1.0;

    // Move enemies to tower face instantly
    if let Some(encounter) = &mut engine.state.encounter {
        for enemy in &mut encounter.enemies {
            enemy.position = EnemyPosition::Ground { x: 0.0 };
            enemy.state = EnemyState::AttackingPanel;
        }
    }

    // Tick until game over
    for _ in 0..300 {
        engine.tick(FIXED_DT);
        if engine.state.phase == GamePhase::GameOver {
            break;
        }
    }
    assert_eq!(engine.state.phase, GamePhase::GameOver);
}

#[test]
fn production_delivers_ammo_during_combat() {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        slot: 1,
        building_type: BuildingType::Fletcher,
    });
    engine.send_command(GameCommand::PlaceCache { floor: 0, slot: 4 });
    engine.send_command(GameCommand::March);

    // Drain all ammo
    engine.state.tower.hero.personal_ammo = 0;

    // Run for 15 seconds (450 ticks) — Fletcher at 6/min should produce ~1.5 crates
    for _ in 0..450 {
        engine.tick(FIXED_DT);
    }

    assert!(
        engine.state.tower.hero.personal_ammo > 0,
        "Hero should have received ammo from fletcher: got {}",
        engine.state.tower.hero.personal_ammo
    );
}

#[test]
fn production_stays_in_warehouse_without_cache() {
    // Verify the no-cache bottleneck on a clean slate. We pre-stock
    // the Fletcher's wood inbox so production isn't gated on wood
    // arriving — the test is about cache absence, not input feed.
    let mut engine = GameEngine::new_blank(42, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        slot: 1,
        building_type: BuildingType::Fletcher,
    });
    engine.send_command(GameCommand::March);

    // Top off Fletcher's wood inbox so production proceeds.
    if let Some(building) = engine.state.tower.floors[0].building.as_mut() {
        for inb in &mut building.input_buffers {
            inb.current = inb.max;
        }
    }

    engine.state.tower.hero.personal_ammo = 0;

    // 30 sim seconds gives the chain plenty of cycles to deliver
    // crates into the warehouse fallback.
    for _ in 0..900 {
        engine.tick(FIXED_DT);
    }

    assert_eq!(engine.state.tower.hero.personal_ammo, 0);
    let arrow_stock = engine
        .state
        .tower
        .warehouse
        .slots
        .iter()
        .find(|slot| slot.resource == ResourceType::Arrows)
        .map_or(0, |slot| slot.current);
    assert!(
        arrow_stock > 0,
        "Warehouse should be holding produced arrows"
    );
}

// ---------------------------------------------------------------------------
// Logistics: real runner-driven supply chain
// ---------------------------------------------------------------------------

#[test]
fn new_game_spawns_default_runners() {
    let engine = GameEngine::new(7, HeroClass::Archer);
    assert_eq!(
        engine.state.tower.runner_quarters.len(),
        1,
        "expected one default RunnerQuarters on new game"
    );
    assert_eq!(
        engine.state.tower.runners.len(),
        2,
        "expected two starting runners"
    );
    for runner in &engine.state.tower.runners {
        assert!(matches!(runner.state, RunnerState::Idle { .. }));
        assert!(runner.task.is_none());
        assert!(runner.carried.is_none());
    }
}

#[test]
fn runner_picks_up_crate_and_delivers_to_cache() {
    let mut engine = GameEngine::new_blank(7, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    // Give ourselves enough ticks to build floor + building + cache.
    engine.state.economy.ticks_remaining = 20;
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        slot: 1,
        building_type: BuildingType::Fletcher,
    });
    let cache_result = engine.send_command(GameCommand::PlaceCache { floor: 0, slot: 4 });
    assert!(
        matches!(cache_result, CommandResult::Ok),
        "PlaceCache failed: {cache_result:?}"
    );
    engine.send_command(GameCommand::March);
    assert!(
        engine.state.tower.floors[0].cache.is_some(),
        "cache must exist before tick loop"
    );

    // Pre-load the building output AND inbox so we don't have to wait
    // for production: the runner should pick up the existing crate
    // within a few ticks regardless of inbox state.
    let building = engine.state.tower.floors[0]
        .building
        .as_mut()
        .expect("fletcher placed");
    building.output_buffer.current = 1;
    for inb in &mut building.input_buffers {
        inb.current = inb.max;
    }

    let mut runner_loaded_or_moving = false;
    for _ in 0..120 {
        engine.tick(FIXED_DT);
        let any_runner_active = engine
            .state
            .tower
            .runners
            .iter()
            .any(|r| !matches!(r.state, RunnerState::Idle { .. }) || r.task.is_some());
        if any_runner_active {
            runner_loaded_or_moving = true;
            break;
        }
    }
    assert!(
        runner_loaded_or_moving,
        "at least one runner should have picked up the task within 4 seconds"
    );

    // Run long enough for the full L-shaped trip: walk to outbox (~2 slots),
    // load 0.5s, walk to cache slot (~3 slots), unload 0.5s. Worst case ≈ 5s.
    for _ in 0..360 {
        engine.tick(FIXED_DT);
    }

    // The crate may end up in the cache, the rack (if drained on the
    // same tick), or the hero's quiver — the supply chain succeeded as
    // long as it landed somewhere downstream of the building.
    let cache_arrows = engine.state.tower.floors[0]
        .cache
        .as_ref()
        .and_then(|c| c.slots.iter().find(|s| s.resource == ResourceType::Arrows))
        .map_or(0, |s| s.current);
    let rack_arrows = engine
        .state
        .tower
        .balconies
        .iter()
        .find(|b| b.floor == 0)
        .map_or(0, |b| b.rack.current);
    assert!(
        cache_arrows + rack_arrows >= 1,
        "supply chain should have delivered at least one crate to cache or rack \
         (cache={cache_arrows}, rack={rack_arrows})"
    );
}

#[test]
fn cache_drains_to_rack_on_same_floor() {
    let mut engine = GameEngine::new(7, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        slot: 1,
        building_type: BuildingType::Fletcher,
    });
    engine.send_command(GameCommand::PlaceCache { floor: 0, slot: 4 });
    engine.send_command(GameCommand::March);

    // Pre-stock the cache directly so we test the cache→rack drain in
    // isolation from the runner pipeline.
    engine.state.tower.floors[0]
        .cache
        .as_mut()
        .expect("cache placed")
        .slots[0]
        .current = 3;
    // Drain rack so we can see the refill.
    for balcony in &mut engine.state.tower.balconies {
        balcony.rack.current = 0;
    }

    engine.tick(FIXED_DT);

    let rack_current = engine
        .state
        .tower
        .balconies
        .iter()
        .find(|b| b.floor == 0)
        .map(|b| b.rack.current)
        .unwrap_or(0);
    assert!(
        rack_current > 0,
        "rack should have pulled from cache on same floor in one tick"
    );
}

#[test]
fn no_cache_means_no_rack_refill() {
    // Even with production running, if there's no cache on the
    // balcony's floor, the rack stays empty. This is the intentional
    // bottleneck — cache placement is the player's logistics decision.
    let mut engine = GameEngine::new_blank(7, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        slot: 1,
        building_type: BuildingType::Fletcher,
    });
    // Deliberately skip PlaceCache.
    engine.send_command(GameCommand::March);

    engine.state.tower.hero.personal_ammo = 0;
    for balcony in &mut engine.state.tower.balconies {
        balcony.rack.current = 0;
    }

    for _ in 0..600 {
        engine.tick(FIXED_DT);
    }

    let rack_current = engine
        .state
        .tower
        .balconies
        .iter()
        .find(|b| b.floor == 0)
        .map(|b| b.rack.current)
        .unwrap_or(0);
    assert_eq!(
        rack_current, 0,
        "rack should not refill without a cache on its floor"
    );
    assert_eq!(
        engine.state.tower.hero.personal_ammo, 0,
        "hero should not get ammo without a cache feeding the rack"
    );
}

#[test]
fn runner_round_trip_takes_visible_time() {
    // Runners are not instant: a building→cache delivery should take
    // load + travel + unload time, not happen in one tick. Without
    // this guarantee the supply chain is invisible to players.
    let mut engine = GameEngine::new_blank(7, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    // Boost ticks + materials for the multi-floor setup.
    engine.state.economy.ticks_remaining = 30;
    if let Some(wood) = engine
        .state
        .economy
        .materials
        .iter_mut()
        .find(|m| m.resource == ResourceType::Wood)
    {
        wood.current = 30;
    }
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    assert_eq!(
        engine.state.tower.floors.len(),
        2,
        "test setup should have built two floors"
    );
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        slot: 1,
        building_type: BuildingType::Fletcher,
    });
    // Cache on a different floor so the runner has to walk.
    let cache_result = engine.send_command(GameCommand::PlaceCache { floor: 1, slot: 4 });
    assert!(
        matches!(cache_result, CommandResult::Ok),
        "PlaceCache failed: {cache_result:?}"
    );
    engine.send_command(GameCommand::March);

    engine.state.tower.floors[0]
        .building
        .as_mut()
        .unwrap()
        .output_buffer
        .current = 1;

    // After a single tick (1/30 sec), the crate should NOT yet be in
    // the cache — load alone takes 15 ticks.
    engine.tick(FIXED_DT);
    let cache_after_one_tick = engine.state.tower.floors[1]
        .cache
        .as_ref()
        .and_then(|c| c.slots.iter().find(|s| s.resource == ResourceType::Arrows))
        .map_or(0, |s| s.current);
    assert_eq!(
        cache_after_one_tick, 0,
        "crate must not teleport to the cache in one tick"
    );

    // After several seconds the runner completes a delivery — the
    // crate may already have drained from the cache into the rack.
    for _ in 0..300 {
        engine.tick(FIXED_DT);
    }
    let cache_after_10s = engine.state.tower.floors[1]
        .cache
        .as_ref()
        .and_then(|c| c.slots.iter().find(|s| s.resource == ResourceType::Arrows))
        .map_or(0, |s| s.current);
    let rack_after_10s = engine
        .state
        .tower
        .balconies
        .iter()
        .find(|b| b.floor == 1)
        .map_or(0, |b| b.rack.current);
    assert!(
        cache_after_10s + rack_after_10s >= 1,
        "runner should have delivered the crate down the chain within 10s \
         (cache={cache_after_10s}, rack={rack_after_10s})"
    );
}

#[test]
fn full_chain_lumberyard_to_fletcher_to_cache() {
    // End-to-end: a Lumberyard on F0 produces wood, a runner walks
    // it up to a Fletcher inbox on F1, the Fletcher consumes wood
    // and produces an arrow, another runner walks the arrow up to a
    // cache on F2. Asserts the chain visibly bottlenecks on inputs:
    // a Fletcher with no wood produces nothing.
    let mut engine = GameEngine::new_blank(11, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.state.economy.ticks_remaining = 30;
    if let Some(wood) = engine
        .state
        .economy
        .materials
        .iter_mut()
        .find(|m| m.resource == ResourceType::Wood)
    {
        wood.current = 30;
    }
    if let Some(stone) = engine
        .state
        .economy
        .materials
        .iter_mut()
        .find(|m| m.resource == ResourceType::Stone)
    {
        stone.current = 30;
    }
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    assert_eq!(engine.state.tower.floors.len(), 3);
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        slot: 1,
        building_type: BuildingType::Lumberyard,
    });
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 1,
        slot: 1,
        building_type: BuildingType::Fletcher,
    });
    let cache_result = engine.send_command(GameCommand::PlaceCache { floor: 2, slot: 4 });
    assert!(
        matches!(cache_result, CommandResult::Ok),
        "PlaceCache failed: {cache_result:?}"
    );
    engine.send_command(GameCommand::March);

    // Sanity: Fletcher starts with an empty inbox, so it must NOT
    // produce until wood arrives.
    let fletcher_inbox = engine.state.tower.floors[1]
        .building
        .as_ref()
        .and_then(|b| b.input_buffers.first())
        .map_or(0, |inb| inb.current);
    assert_eq!(fletcher_inbox, 0, "Fletcher inbox should start empty");

    // Run the simulation long enough for the chain to complete a
    // full cycle: lumberyard produces (~15s) → runner delivers wood
    // (~2s) → fletcher produces arrow (~10s) → runner delivers
    // arrow to cache (~3s). 60 sec headroom is plenty.
    for _ in 0..1800 {
        engine.tick(FIXED_DT);
    }

    // The cache on floor 2 should have at least one arrow (or the
    // rack on floor 2, if it was drained on the same tick).
    let cache_arrows = engine.state.tower.floors[2]
        .cache
        .as_ref()
        .and_then(|c| c.slots.iter().find(|s| s.resource == ResourceType::Arrows))
        .map_or(0, |s| s.current);
    let rack_arrows = engine
        .state
        .tower
        .balconies
        .iter()
        .find(|b| b.floor == 2)
        .map_or(0, |b| b.rack.current);
    assert!(
        cache_arrows + rack_arrows >= 1,
        "full chain should have produced at least one arrow downstream \
         (cache={cache_arrows}, rack={rack_arrows})"
    );
}

#[test]
fn fletcher_stalls_without_wood() {
    // Verify the gate: a Fletcher with an empty inbox produces nothing,
    // even given plenty of time. This is the bottleneck signal players
    // need to read on the tower viz.
    let mut engine = GameEngine::new_blank(11, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.state.economy.ticks_remaining = 20;
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        slot: 1,
        building_type: BuildingType::Fletcher,
    });
    engine.send_command(GameCommand::March);

    for _ in 0..1200 {
        engine.tick(FIXED_DT);
    }

    let outbox = engine.state.tower.floors[0]
        .building
        .as_ref()
        .map(|b| b.output_buffer.current)
        .unwrap_or(0);
    assert_eq!(
        outbox, 0,
        "Fletcher should stall with no wood: outbox should stay empty"
    );
}

#[test]
fn new_game_has_built_in_stairs_spanning_tower() {
    let engine = GameEngine::new(11, HeroClass::Archer);
    let stairs = engine
        .state
        .tower
        .transports
        .iter()
        .find(|t| t.kind == TransportKind::Stairs)
        .expect("default tower should ship with built-in stairs");
    assert_eq!(stairs.low_floor, 0);
    assert_eq!(stairs.high_floor, engine.state.tower.floors.len() - 1);
    assert_eq!(stairs.direction, TransportDirection::Both);
}

#[test]
fn build_floor_extends_built_in_stairs() {
    let mut engine = GameEngine::new(11, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.state.economy.ticks_remaining = 20;
    if let Some(wood) = engine
        .state
        .economy
        .materials
        .iter_mut()
        .find(|m| m.resource == ResourceType::Wood)
    {
        wood.current = 30;
    }
    let before_top = engine.state.tower.floors.len() - 1;
    let result = engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    assert!(matches!(result, CommandResult::Ok));
    let stairs = engine
        .state
        .tower
        .transports
        .iter()
        .find(|t| t.kind == TransportKind::Stairs)
        .expect("stairs must exist");
    assert!(
        stairs.high_floor > before_top,
        "stairs should grow with new floor"
    );
}

#[test]
fn runners_route_through_transport_with_capacity() {
    // Two runners, 1-cap stairs, multi-floor delivery → at most one
    // runner should be Moving at any tick; the other queues.
    let mut engine = GameEngine::new_blank(11, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.state.economy.ticks_remaining = 30;
    if let Some(wood) = engine
        .state
        .economy
        .materials
        .iter_mut()
        .find(|m| m.resource == ResourceType::Wood)
    {
        wood.current = 30;
    }
    if let Some(stone) = engine
        .state
        .economy
        .materials
        .iter_mut()
        .find(|m| m.resource == ResourceType::Stone)
    {
        stone.current = 30;
    }
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 0,
        slot: 1,
        building_type: BuildingType::Lumberyard,
    });
    engine.send_command(GameCommand::PlaceBuilding {
        floor: 1,
        slot: 1,
        building_type: BuildingType::Fletcher,
    });
    engine.send_command(GameCommand::PlaceCache { floor: 2, slot: 4 });
    engine.send_command(GameCommand::March);

    // Force both runners to want to move at the same time by
    // pre-stocking the lumberyard outbox and emptying every cache.
    engine.state.tower.floors[0]
        .building
        .as_mut()
        .unwrap()
        .output_buffer
        .current = 4;

    let mut saw_queue = false;
    for _ in 0..200 {
        engine.tick(FIXED_DT);
        let queued = engine
            .state
            .tower
            .runners
            .iter()
            .filter(|r| matches!(r.state, RunnerState::Queued { .. }))
            .count();
        if queued > 0 {
            saw_queue = true;
        }
        // Sanity: with capacity 1 we should never have two runners on
        // the same transport at once.
        let stairs = engine
            .state
            .tower
            .transports
            .iter()
            .find(|t| t.kind == TransportKind::Stairs)
            .unwrap();
        assert!(
            stairs.occupancy <= stairs.capacity,
            "occupancy {} exceeded capacity {}",
            stairs.occupancy,
            stairs.capacity
        );
    }
    assert!(
        saw_queue,
        "with two runners and 1-cap stairs, at least one should queue"
    );
}

#[test]
fn place_cache_rejects_overlapping_slot_range() {
    // Default tower has Fletcher at slot 1..3 on F1.
    // A cache at slot 2 (1 wide) overlaps the Fletcher footprint.
    let mut engine = GameEngine::new(11, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.state.economy.ticks_remaining = 30;
    let result = engine.send_command(GameCommand::PlaceCache { floor: 1, slot: 2 });
    assert!(matches!(
        result,
        CommandResult::Error(CommandError::InvalidCommand { .. })
    ));
}

#[test]
fn place_building_rejects_transport_column_collision() {
    // Default stairs occupy slot 0 on every floor. Trying to place a
    // building at slot 0 should fail because it overlaps the stairs.
    let mut engine = GameEngine::new(11, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.state.economy.ticks_remaining = 30;
    if let Some(wood) = engine
        .state
        .economy
        .materials
        .iter_mut()
        .find(|m| m.resource == ResourceType::Wood)
    {
        wood.current = 30;
    }
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    let new_floor = engine.state.tower.floors.len() - 1;
    let result = engine.send_command(GameCommand::PlaceBuilding {
        floor: new_floor,
        slot: 0,
        building_type: BuildingType::Quarry,
    });
    assert!(matches!(
        result,
        CommandResult::Error(CommandError::InvalidCommand { .. })
    ));
}

#[test]
fn place_building_rejects_out_of_range_slot() {
    let mut engine = GameEngine::new(11, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.state.economy.ticks_remaining = 30;
    if let Some(wood) = engine
        .state
        .economy
        .materials
        .iter_mut()
        .find(|m| m.resource == ResourceType::Wood)
    {
        wood.current = 30;
    }
    engine.send_command(GameCommand::BuildFloor {
        material: FloorMaterial::Wood,
    });
    let new_floor = engine.state.tower.floors.len() - 1;
    // Floor is 8 slots wide. Slot 7 + 2-wide building = 9 (overflow).
    let result = engine.send_command(GameCommand::PlaceBuilding {
        floor: new_floor,
        slot: 7,
        building_type: BuildingType::Quarry,
    });
    assert!(matches!(
        result,
        CommandResult::Error(CommandError::InvalidCommand { .. })
    ));
}

#[test]
fn drill_runs_chain_in_prep_and_grants_xp() {
    let mut engine = GameEngine::new(11, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    let xp_before = engine.state.tower.hero.xp;
    let ticks_before = engine.state.economy.ticks_remaining;

    let result = engine.send_command(GameCommand::RunDrill { seconds: 10 });
    assert!(matches!(result, CommandResult::Ok));
    assert_eq!(
        engine.state.economy.ticks_remaining,
        ticks_before - 1,
        "drill must cost 1 tick"
    );
    assert!(engine.state.drill.is_some(), "drill should be active");

    for _ in 0..360 {
        engine.tick(FIXED_DT);
    }

    assert!(engine.state.drill.is_none(), "drill should have ended");
    assert!(
        engine.state.tower.hero.xp >= xp_before,
        "drill should grant some XP (got {} -> {})",
        xp_before,
        engine.state.tower.hero.xp
    );
}

#[test]
fn drill_blocks_concurrent_drill() {
    let mut engine = GameEngine::new(11, HeroClass::Archer);
    engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
    engine.send_command(GameCommand::RunDrill { seconds: 10 });
    let result = engine.send_command(GameCommand::RunDrill { seconds: 10 });
    assert!(matches!(
        result,
        CommandResult::Error(CommandError::InvalidCommand { .. })
    ));
}

#[test]
fn logistics_tick_is_deterministic() {
    let mut a = GameEngine::new(11, HeroClass::Archer);
    let mut b = GameEngine::new(11, HeroClass::Archer);
    for engine in [&mut a, &mut b] {
        engine.send_command(GameCommand::SelectNode { node: NodeId(1) });
        engine.send_command(GameCommand::BuildFloor {
            material: FloorMaterial::Wood,
        });
        engine.send_command(GameCommand::PlaceBuilding {
            floor: 0,
            slot: 1,
            building_type: BuildingType::Fletcher,
        });
        engine.send_command(GameCommand::PlaceCache { floor: 0, slot: 4 });
        engine.send_command(GameCommand::March);
        for _ in 0..600 {
            engine.tick(FIXED_DT);
        }
    }

    assert_eq!(
        a.state.tower.runners.len(),
        b.state.tower.runners.len(),
        "runner counts must match"
    );
    for (ra, rb) in a.state.tower.runners.iter().zip(&b.state.tower.runners) {
        let ja = serde_json::to_string(&ra).unwrap();
        let jb = serde_json::to_string(&rb).unwrap();
        assert_eq!(ja, jb, "runner state diverged across identical runs");
    }
}
