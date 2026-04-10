use crate::balance::*;
use crate::command::{CommandError, CommandResult, GameCommand};
use crate::snapshot::*;
use crate::state::*;
use crate::systems;
use crate::types::*;

/// The game engine: owns GameState, processes commands, runs simulation ticks.
#[derive(Debug, Clone)]
pub struct GameEngine {
    pub state: GameState,
    accumulator: Scalar,
    /// Counter for generating unique entity IDs.
    next_id: u32,
}

impl GameEngine {
    pub fn new(seed: u64, class: HeroClass) -> Self {
        let state = GameState::new(seed, class);
        Self {
            state,
            accumulator: 0.0,
            next_id: 100,
        }
    }

    #[allow(dead_code)] // Will be used for companion assignment
    fn next_entity_id(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        id
    }

    fn next_projectile_id(&mut self) -> ProjectileId {
        let id = ProjectileId(self.next_id);
        self.next_id += 1;
        id
    }

    fn next_enemy_id(&mut self) -> EnemyId {
        let id = EnemyId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Process a game command. Validates phase before applying.
    pub fn send_command(&mut self, cmd: GameCommand) -> CommandResult {
        match self.apply_command(cmd) {
            Ok(()) => CommandResult::Ok,
            Err(e) => CommandResult::Error(e),
        }
    }

    fn require_phase(&self, expected: GamePhase) -> Result<(), CommandError> {
        if self.state.phase != expected {
            return Err(CommandError::WrongPhase {
                expected: format!("{expected:?}"),
                actual: format!("{:?}", self.state.phase),
            });
        }
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value)] // cmd is destructured in match arms
    fn apply_command(&mut self, cmd: GameCommand) -> Result<(), CommandError> {
        match cmd {
            // ── Prep / tower building ───────────────────────────
            GameCommand::BuildFloor { material } => {
                self.require_phase(GamePhase::Travel)?;
                let eco = &self.state.economy;
                if eco.ticks_remaining < FLOOR_TICK_COST {
                    return Err(CommandError::InsufficientTicks {
                        needed: FLOOR_TICK_COST,
                        available: eco.ticks_remaining,
                    });
                }
                let (mat_type, mat_cost, panel_hp) = match material {
                    FloorMaterial::Wood => {
                        (ResourceType::Wood, WOOD_FLOOR_MATERIAL_COST, WOOD_PANEL_HP)
                    }
                    FloorMaterial::Stone => (
                        ResourceType::Stone,
                        STONE_FLOOR_MATERIAL_COST,
                        STONE_PANEL_HP,
                    ),
                    FloorMaterial::Iron => {
                        return Err(CommandError::InvalidCommand {
                            reason: "Iron floors not available in v1".into(),
                        });
                    }
                };
                let available = self.get_material(mat_type);
                if available < mat_cost {
                    return Err(CommandError::InsufficientMaterials {
                        resource: mat_type,
                        needed: mat_cost,
                        available,
                    });
                }
                if self.state.tower.floors.len() >= self.state.tower.foundation.max_floors {
                    return Err(CommandError::InvalidCommand {
                        reason: format!(
                            "Max {} floors reached",
                            self.state.tower.foundation.max_floors
                        ),
                    });
                }

                self.state.economy.ticks_remaining -= FLOOR_TICK_COST;
                self.deduct_material(mat_type, mat_cost);

                let floor_idx = self.state.tower.floors.len();
                self.state.tower.floors.push(Floor {
                    index: floor_idx,
                    building: None,
                    cache: None,
                    panel: WallPanel {
                        current_hp: panel_hp,
                        max_hp: panel_hp,
                        is_breached: false,
                    },
                    material,
                    transport_segments: Vec::new(),
                    floor_width_used: 0.0,
                    floor_width_max: 1.0,
                });

                let balcony_id = BalconyId(floor_idx as u32);
                self.state.tower.balconies.push(Balcony {
                    id: balcony_id,
                    floor: floor_idx,
                    rack: AmmoRack {
                        resource: ResourceType::Arrows,
                        current: 0,
                        max: AMMO_RACK_MAX,
                        destroyed: false,
                    },
                    cover_level: CoverLevel::Exposed,
                    occupant: None,
                });

                Ok(())
            }

            GameCommand::PlaceBuilding {
                floor,
                building_type,
            } => {
                self.require_phase(GamePhase::Travel)?;

                if floor >= self.state.tower.floors.len() {
                    return Err(CommandError::InvalidFloor { index: floor });
                }
                if self.state.tower.floors[floor].building.is_some() {
                    return Err(CommandError::FloorOccupied { index: floor });
                }

                let (tick_cost, mat_type, mat_cost, resource_out, rate, op_cost, buf_max) =
                    match building_type {
                        BuildingType::Fletcher => (
                            BUILDING_TICK_COST,
                            ResourceType::Wood,
                            FLETCHER_WOOD_COST,
                            ResourceType::Arrows,
                            FLETCHER_RATE,
                            FLETCHER_OPERATING_COST,
                            FLETCHER_BUFFER_MAX,
                        ),
                        BuildingType::Quarry => (
                            BUILDING_TICK_COST,
                            ResourceType::Stone,
                            FORGE_STONE_COST,
                            ResourceType::Bolts,
                            FORGE_RATE,
                            FORGE_OPERATING_COST,
                            FORGE_BUFFER_MAX,
                        ),
                        _ => {
                            return Err(CommandError::InvalidCommand {
                                reason: format!("{building_type:?} not available in MVP"),
                            });
                        }
                    };

                let eco = &self.state.economy;
                if eco.ticks_remaining < tick_cost {
                    return Err(CommandError::InsufficientTicks {
                        needed: tick_cost,
                        available: eco.ticks_remaining,
                    });
                }
                let available = self.get_material(mat_type);
                if available < mat_cost {
                    return Err(CommandError::InsufficientMaterials {
                        resource: mat_type,
                        needed: mat_cost,
                        available,
                    });
                }

                self.state.economy.ticks_remaining -= tick_cost;
                self.deduct_material(mat_type, mat_cost);

                self.state.tower.floors[floor].building = Some(Building {
                    building_type,
                    tier: ProductionTier::T1,
                    output_buffer: ResourceBuffer {
                        resource: resource_out,
                        current: 0,
                        max: buf_max,
                    },
                    input_buffers: Vec::new(),
                    production_rate: rate,
                    operating_cost: op_cost,
                    is_active: true,
                });

                Ok(())
            }

            // ── Map navigation ──────────────────────────────────
            GameCommand::SelectNode { node } => {
                self.require_phase(GamePhase::MapView)?;
                let chapter = &self.state.journey.chapters[self.state.journey.current_chapter - 1];
                let reachable = chapter
                    .edges
                    .iter()
                    .any(|e| e.from == self.state.journey.current_node && e.to == node);
                if !reachable {
                    return Err(CommandError::InvalidTarget);
                }
                self.state.journey.current_node = node;
                self.state.phase = GamePhase::Travel;
                // Reset tick budget for this stop
                self.state.economy.ticks_remaining = match self.state.journey.current_chapter {
                    1 => CH1_TICKS,
                    2 => CH2_TICKS,
                    _ => CH3_TICKS,
                };
                Ok(())
            }

            GameCommand::March => {
                self.require_phase(GamePhase::Travel)?;
                // Convert unused ticks to gold
                let unused = self.state.economy.ticks_remaining;
                self.state.economy.gold += unused * UNUSED_TICK_GOLD;
                self.state.economy.ticks_remaining = 0;

                // Determine difficulty from current node
                let chapter = &self.state.journey.chapters[self.state.journey.current_chapter - 1];
                let node = chapter
                    .nodes
                    .iter()
                    .find(|n| n.id == self.state.journey.current_node);
                let difficulty = node.and_then(|n| n.difficulty).unwrap_or(1);
                let is_boss = node.is_some_and(|n| n.node_type == NodeType::Boss);

                // Generate encounter
                let encounter = self.generate_encounter(difficulty, is_boss);
                self.state.encounter = Some(encounter);
                self.state.phase = GamePhase::Encounter;
                Ok(())
            }

            // ── Combat input ────────────────────────────────────
            GameCommand::AimAt { direction } => {
                self.require_phase(GamePhase::Encounter)?;
                self.state.tower.hero.aim_direction = direction;
                Ok(())
            }

            GameCommand::Fire => {
                self.require_phase(GamePhase::Encounter)?;

                // Extract weapon properties before any mutable borrows
                let hero = &self.state.tower.hero;
                let weapon = match hero.active_weapon {
                    WeaponSlot::Primary => &hero.weapon_primary,
                    WeaponSlot::Secondary => &hero.weapon_secondary,
                };
                let base_type = weapon.base_type;
                let damage = weapon.damage;
                let needs_ammo = base_type != WeaponBaseType::Melee;
                let hero_id = hero.id;
                let speed = match base_type {
                    WeaponBaseType::Bow => SHORTBOW_PROJ_SPEED,
                    WeaponBaseType::Crossbow => 400.0,
                    WeaponBaseType::Staff => 500.0,
                    WeaponBaseType::Thrown => 250.0,
                    WeaponBaseType::Melee => 0.0,
                };

                if needs_ammo && self.state.tower.hero.personal_ammo == 0 {
                    return Err(CommandError::InsufficientMaterials {
                        resource: ResourceType::Arrows,
                        needed: 1,
                        available: 0,
                    });
                }

                if needs_ammo {
                    self.state.tower.hero.personal_ammo -= 1;
                }

                if base_type == WeaponBaseType::Melee {
                    // Melee: instant damage to nearest enemy in range
                    if let Some(encounter) = &mut self.state.encounter
                        && let Some(enemy) = encounter
                            .enemies
                            .iter_mut()
                            .filter(|e| e.state != EnemyState::Dead)
                            .filter(|e| match &e.position {
                                EnemyPosition::Ground { x } => *x <= DAGGER_RANGE,
                                _ => false,
                            })
                            .min_by(|a, b| {
                                let ax = match &a.position {
                                    EnemyPosition::Ground { x } => *x,
                                    _ => BATTLEFIELD_WIDTH,
                                };
                                let bx = match &b.position {
                                    EnemyPosition::Ground { x } => *x,
                                    _ => BATTLEFIELD_WIDTH,
                                };
                                ax.partial_cmp(&bx).unwrap()
                            })
                        {
                            enemy.hp -= damage;
                        }
                } else {
                    // Ranged: create projectile
                    let proj_id = self.next_projectile_id();
                    if let Some(encounter) = &mut self.state.encounter {
                        encounter.projectiles.push(Projectile {
                            id: proj_id,
                            source: hero_id,
                            weapon_type: base_type,
                            position: Vec2::new(0.0, 0.0),
                            velocity: Vec2::new(speed, 0.0),
                            gravity: 0.0,
                            damage,
                            modifier: None,
                            state: ProjectileState::Flying,
                        });
                    }
                }

                Ok(())
            }

            GameCommand::SwitchWeapon => {
                self.require_phase(GamePhase::Encounter)?;
                self.state.tower.hero.active_weapon = match self.state.tower.hero.active_weapon {
                    WeaponSlot::Primary => WeaponSlot::Secondary,
                    WeaponSlot::Secondary => WeaponSlot::Primary,
                };
                Ok(())
            }

            // ── Post-combat ─────────────────────────────────────
            GameCommand::ContinueJourney => {
                self.require_phase(GamePhase::PostCombat)?;
                let current = self.state.journey.current_node;
                self.state.journey.visited_nodes.push(current);
                self.state.encounter = None;

                // Check if current node is boss (chapter complete → victory)
                let chapter = &self.state.journey.chapters[self.state.journey.current_chapter - 1];
                if chapter.boss_node == current {
                    // Check if there are more chapters
                    if self.state.journey.current_chapter >= self.state.journey.chapters.len() {
                        self.state.phase = GamePhase::Victory;
                    } else {
                        self.state.journey.current_chapter += 1;
                        self.state.journey.current_node = self.state.journey.chapters
                            [self.state.journey.current_chapter - 1]
                            .nodes[0]
                            .id;
                        self.state.phase = GamePhase::MapView;
                    }
                } else {
                    self.state.phase = GamePhase::MapView;
                }
                Ok(())
            }

            // ── Unimplemented (MVP) ─────────────────────────────
            _ => Err(CommandError::InvalidCommand {
                reason: "Command not implemented in MVP".into(),
            }),
        }
    }

    fn generate_encounter(&mut self, difficulty: u8, is_boss: bool) -> EncounterState {
        let budget = match difficulty {
            1 => DIFFICULTY_1_BUDGET,
            2 => DIFFICULTY_2_BUDGET,
            _ => DIFFICULTY_3_BUDGET,
        };

        let mut enemies = Vec::new();
        let mut remaining_budget = budget;

        // Simple enemy generation: fill budget with grunts and runners
        let mut spawn_x = BATTLEFIELD_WIDTH;
        while remaining_budget >= GRUNT_THREAT {
            let (archetype, hp, speed, threat) =
                if remaining_budget >= RUNNER_ENEMY_THREAT && self.state.rng.next_f32() > 0.6 {
                    (
                        EnemyArchetype::Runner,
                        RUNNER_ENEMY_HP,
                        RUNNER_ENEMY_SPEED,
                        RUNNER_ENEMY_THREAT,
                    )
                } else {
                    (EnemyArchetype::Grunt, GRUNT_HP, GRUNT_SPEED, GRUNT_THREAT)
                };

            let id = self.next_enemy_id();
            enemies.push(Enemy {
                id,
                archetype,
                position: EnemyPosition::Ground {
                    x: spawn_x + self.state.rng.next_f32() * 100.0,
                },
                hp,
                max_hp: hp,
                speed,
                state: EnemyState::Approaching,
                stuck_arrows: Vec::new(),
            });
            remaining_budget = remaining_budget.saturating_sub(threat);
            spawn_x += 40.0; // Stagger spawn positions
        }

        // For boss encounters, add a tough enemy
        if is_boss {
            let id = self.next_enemy_id();
            enemies.push(Enemy {
                id,
                archetype: EnemyArchetype::Armored,
                position: EnemyPosition::Ground {
                    x: BATTLEFIELD_WIDTH + 200.0,
                },
                hp: ARMORED_HP,
                max_hp: ARMORED_HP,
                speed: ARMORED_SPEED,
                state: EnemyState::Approaching,
                stuck_arrows: Vec::new(),
            });
        }

        EncounterState {
            enemies,
            projectiles: Vec::new(),
            loot_on_ground: Vec::new(),
            waves: vec![Wave {
                enemies: Vec::new(),
                spawn_delay: 0.0,
            }],
            current_wave: 0,
            wave_state: WaveState::Active,
            terrain_modifier: None,
            interior_raiders: Vec::new(),
        }
    }

    // ── Material helpers ────────────────────────────────────

    fn get_material(&self, resource: ResourceType) -> u32 {
        self.state
            .economy
            .materials
            .iter()
            .find(|m| m.resource == resource)
            .map_or(0, |m| m.current)
    }

    fn deduct_material(&mut self, resource: ResourceType, amount: u32) {
        if let Some(mat) = self
            .state
            .economy
            .materials
            .iter_mut()
            .find(|m| m.resource == resource)
        {
            mat.current = mat.current.saturating_sub(amount);
        }
    }

    // ── Simulation tick ─────────────────────────────────────

    /// Advance the simulation by real_dt seconds.
    /// Uses a fixed timestep accumulator. Capped to MAX_TICKS_PER_FRAME
    /// to prevent spiral-of-death after long pauses (e.g., tab switch).
    /// Returns sound events from any ticks that ran.
    pub fn tick(&mut self, real_dt: Scalar) -> Vec<SoundEvent> {
        let capped_dt = real_dt.min(FIXED_DT * MAX_TICKS_PER_FRAME as Scalar);
        self.accumulator += capped_dt;
        let mut all_sounds = Vec::new();

        while self.accumulator >= FIXED_DT {
            let sounds = systems::tick(&mut self.state, FIXED_DT);
            all_sounds.extend(sounds);
            self.accumulator -= FIXED_DT;
        }

        all_sounds
    }

    /// Fraction of a tick elapsed — used by the renderer for interpolation.
    pub fn interpolation_alpha(&self) -> Scalar {
        self.accumulator / FIXED_DT
    }

    // ── Typed accessors for React bridge ────────────────────

    pub fn get_phase(&self) -> GamePhase {
        self.state.phase
    }

    pub fn get_hud_state(&self) -> HudSnapshot {
        let state = &self.state;
        let encounter = state.encounter.as_ref();

        HudSnapshot {
            ammo_primary: state.tower.hero.personal_ammo,
            ammo_secondary: 0,
            personal_ammo: state.tower.hero.personal_ammo,
            weapon_ability_cooldown: state.tower.hero.weapon_ability_cooldown,
            hero_skill_cooldown: state.tower.hero.hero_skill_cooldown,
            active_weapon: state.tower.hero.active_weapon,
            current_wave: encounter.map_or(0, |e| e.current_wave),
            total_waves: encounter.map_or(0, |e| e.waves.len()),
            wave_state: encounter.map_or(WaveState::Lull { timer: 0.0 }, |e| e.wave_state.clone()),
            gold: state.economy.gold,
            companion_statuses: state
                .tower
                .companions
                .iter()
                .map(|c| CompanionHudStatus {
                    name: c.name.clone(),
                    ammo: 0,
                    injured: c.injured,
                    displaced: c.position.is_none(),
                })
                .collect(),
            hero_hp_fraction: state.tower.foundation.current_hp / state.tower.foundation.max_hp,
            tower_hp_fraction: if state.tower.floors.is_empty() {
                1.0
            } else {
                let total: Scalar = state.tower.floors.iter().map(|f| f.panel.current_hp).sum();
                let max: Scalar = state.tower.floors.iter().map(|f| f.panel.max_hp).sum();
                if max > 0.0 { total / max } else { 1.0 }
            },
            enemies_remaining: encounter.map_or(0, |e| {
                e.enemies
                    .iter()
                    .filter(|e| e.state != EnemyState::Dead)
                    .count()
            }),
        }
    }

    pub fn get_tower_state(&self) -> TowerSnapshot {
        let tower = &self.state.tower;
        TowerSnapshot {
            floors: tower
                .floors
                .iter()
                .map(|f| FloorSnapshot {
                    index: f.index,
                    building: f.building.clone(),
                    cache: f.cache.clone(),
                    panel_hp_fraction: if f.panel.max_hp > 0.0 {
                        f.panel.current_hp / f.panel.max_hp
                    } else {
                        1.0
                    },
                    material: f.material,
                })
                .collect(),
            warehouse: tower.warehouse.clone(),
            runners: tower.runners.clone(),
            width: tower.width,
        }
    }

    pub fn get_journey_state(&self) -> JourneySnapshot {
        let j = &self.state.journey;
        JourneySnapshot {
            current_chapter: j.current_chapter,
            chapters: j.chapters.clone(),
            current_node: j.current_node,
            visited_nodes: j.visited_nodes.clone(),
        }
    }

    pub fn get_hero_state(&self) -> HeroSnapshot {
        let h = &self.state.tower.hero;
        HeroSnapshot {
            class: h.class,
            level: h.level,
            xp: h.xp,
            stats: h.stats.clone(),
            perks: h.perks.clone(),
            weapon_primary: h.weapon_primary.clone(),
            weapon_secondary: h.weapon_secondary.clone(),
            trinket: h.trinket.clone(),
        }
    }

    pub fn get_encounter_state(&self) -> Option<EncounterSnapshot> {
        self.state.encounter.as_ref().map(|e| EncounterSnapshot {
            enemies: e
                .enemies
                .iter()
                .filter(|en| en.state != EnemyState::Dead)
                .map(|en| EnemySnapshot {
                    id: en.id,
                    archetype: en.archetype,
                    x: match &en.position {
                        EnemyPosition::Ground { x } => *x,
                        _ => 0.0,
                    },
                    hp_fraction: if en.max_hp > 0.0 {
                        en.hp / en.max_hp
                    } else {
                        0.0
                    },
                })
                .collect(),
            projectiles: e
                .projectiles
                .iter()
                .filter(|p| matches!(p.state, ProjectileState::Flying))
                .map(|p| ProjectileSnapshot {
                    id: p.id,
                    x: p.position.x,
                    y: p.position.y,
                })
                .collect(),
            current_wave: e.current_wave,
            total_waves: e.waves.len(),
            enemies_remaining: e
                .enemies
                .iter()
                .filter(|en| en.state != EnemyState::Dead)
                .count(),
        })
    }

    pub fn get_validation_warnings(&self) -> Vec<ValidationWarning> {
        Vec::new()
    }

    pub fn save(&self) -> String {
        serde_json::to_string(&self.state).expect("GameState must be serializable")
    }

    pub fn load(&mut self, data: &str) -> Result<(), String> {
        let state: GameState =
            serde_json::from_str(data).map_err(|e| format!("Failed to load save: {e}"))?;
        self.state = state;
        self.accumulator = 0.0;
        Ok(())
    }
}

impl GameState {
    pub fn new(seed: u64, class: HeroClass) -> Self {
        use crate::rng::DeterministicRng;

        let hero_skill = match class {
            HeroClass::Archer => HeroSkill::Focus,
            HeroClass::Engineer => HeroSkill::Overclock,
            HeroClass::Commander => HeroSkill::Rally,
        };

        // Hardcoded 3-node chapter map: start → combat1 → combat2 (boss)
        let chapter = ChapterMap {
            nodes: vec![
                MapNode {
                    id: NodeId(0),
                    node_type: NodeType::Combat,
                    column: 0,
                    difficulty: None,
                    visited: true,
                },
                MapNode {
                    id: NodeId(1),
                    node_type: NodeType::Combat,
                    column: 1,
                    difficulty: Some(1),
                    visited: false,
                },
                MapNode {
                    id: NodeId(2),
                    node_type: NodeType::Combat,
                    column: 2,
                    difficulty: Some(2),
                    visited: false,
                },
                MapNode {
                    id: NodeId(3),
                    node_type: NodeType::Boss,
                    column: 3,
                    difficulty: Some(3),
                    visited: false,
                },
            ],
            edges: vec![
                MapEdge {
                    from: NodeId(0),
                    to: NodeId(1),
                },
                MapEdge {
                    from: NodeId(1),
                    to: NodeId(2),
                },
                MapEdge {
                    from: NodeId(2),
                    to: NodeId(3),
                },
            ],
            boss_node: NodeId(3),
        };

        Self {
            phase: GamePhase::MapView,
            tower: Tower {
                floors: Vec::new(),
                foundation: Foundation {
                    leg_type: LegType::Chicken,
                    current_hp: FOUNDATION_HP,
                    max_hp: FOUNDATION_HP,
                    max_floors: MAX_FLOORS,
                    maintenance_cost: LEG_MAINTENANCE,
                },
                warehouse: Warehouse {
                    slots: Vec::new(),
                    capacity_per_slot: 20,
                },
                runner_quarters: Vec::new(),
                runners: Vec::new(),
                width: TowerWidth::Standard,
                balconies: Vec::new(),
                hero: Hero {
                    id: EntityId(0),
                    position: BalconyId(0),
                    class,
                    stats: HeroStats {
                        precision: 5,
                        draw_power: 5,
                        tempo: 5,
                        grit: 5,
                        salvage: 5,
                        unspent_points: 0,
                    },
                    level: 1,
                    xp: 0,
                    perks: Vec::new(),
                    weapon_primary: Weapon {
                        base_type: WeaponBaseType::Bow,
                        sub_type: "shortbow".into(),
                        damage: SHORTBOW_DAMAGE,
                        fire_rate: SHORTBOW_FIRE_RATE,
                        modifiers: Vec::new(),
                    },
                    weapon_secondary: Weapon {
                        base_type: WeaponBaseType::Melee,
                        sub_type: "dagger".into(),
                        damage: DAGGER_DAMAGE,
                        fire_rate: DAGGER_FIRE_RATE,
                        modifiers: Vec::new(),
                    },
                    active_weapon: WeaponSlot::Primary,
                    trinket: None,
                    personal_ammo: HERO_PERSONAL_AMMO,
                    aim_direction: Vec2::new(1.0, 0.0),
                    weapon_ability_cooldown: 0.0,
                    hero_skill_cooldown: 0.0,
                    hero_skill,
                },
                companions: Vec::new(),
            },
            encounter: None,
            journey: JourneyState {
                destination: DestinationId(0),
                current_chapter: 1,
                chapters: vec![chapter],
                current_node: NodeId(0),
                visited_nodes: Vec::new(),
                available_companions: Vec::new(),
            },
            meta: MetaState {
                seed,
                run_number: 1,
                unlocked_classes: vec![HeroClass::Archer],
                unlocked_weapons: vec![
                    WeaponBaseType::Bow,
                    WeaponBaseType::Crossbow,
                    WeaponBaseType::Melee,
                ],
            },
            economy: EconomyState {
                gold: STARTING_GOLD,
                materials: vec![
                    ResourceBuffer {
                        resource: ResourceType::Wood,
                        current: STARTING_WOOD,
                        max: 99,
                    },
                    ResourceBuffer {
                        resource: ResourceType::Stone,
                        current: STARTING_STONE,
                        max: 99,
                    },
                    ResourceBuffer {
                        resource: ResourceType::Arrows,
                        current: 0,
                        max: 99,
                    },
                    ResourceBuffer {
                        resource: ResourceType::Bolts,
                        current: 0,
                        max: 99,
                    },
                ],
                ticks_remaining: 0,
                ticks_per_prep: CH1_TICKS,
            },
            rng: DeterministicRng::new(seed),
            tick: 0,
            elapsed: 0.0,
        }
    }
}
