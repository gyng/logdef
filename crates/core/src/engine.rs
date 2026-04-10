use std::sync::Arc;
use std::time::Duration;

// Perf instrumentation uses std::time::Instant, which panics on
// wasm32-unknown-unknown. Gate on non-wasm + debug_assertions.
#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
use std::time::Instant;

use crate::command::{CommandError, CommandResult, GameCommand, StatType};
use crate::registry::Registry;
use crate::snapshot::*;
use crate::state::*;
use crate::systems;
use crate::types::*;

/// The game engine: owns GameState, processes commands, runs simulation ticks.
#[derive(Debug, Clone)]
pub struct GameEngine {
    pub registry: Arc<Registry>,
    pub state: GameState,
    accumulator: Scalar,
    /// Counter for generating unique entity IDs.
    next_id: u32,
    perf: EnginePerfState,
}

#[derive(Debug, Clone, Default)]
struct PerfMetricState {
    calls: u64,
    last_ms: f64,
    avg_ms: f64,
    max_ms: f64,
}

#[cfg_attr(any(not(debug_assertions), target_arch = "wasm32"), allow(dead_code))]
impl PerfMetricState {
    fn record(&mut self, duration: Duration) {
        let millis = duration.as_secs_f64() * 1000.0;
        self.calls += 1;
        self.last_ms = millis;
        if self.calls == 1 {
            self.avg_ms = millis;
            self.max_ms = millis;
            return;
        }
        let prior_calls = (self.calls - 1) as f64;
        self.avg_ms = ((self.avg_ms * prior_calls) + millis) / self.calls as f64;
        self.max_ms = self.max_ms.max(millis);
    }

    fn snapshot(&self) -> PerfMetricSnapshot {
        PerfMetricSnapshot {
            calls: self.calls,
            last_ms: self.last_ms,
            avg_ms: self.avg_ms,
            max_ms: self.max_ms,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct EnginePerfState {
    ticks_last_frame: u32,
    frame_sim: PerfMetricState,
    tick_total: PerfMetricState,
    production: PerfMetricState,
    transport: PerfMetricState,
    companion_ai: PerfMetricState,
    projectiles: PerfMetricState,
    combat: PerfMetricState,
    economy: PerfMetricState,
}

#[cfg_attr(any(not(debug_assertions), target_arch = "wasm32"), allow(dead_code))]
impl EnginePerfState {
    fn record_tick(&mut self, profile: &systems::TickProfile) {
        self.tick_total.record(profile.total);
        self.production.record(profile.production);
        self.transport.record(profile.transport);
        self.companion_ai.record(profile.companion_ai);
        self.projectiles.record(profile.projectiles);
        self.combat.record(profile.combat);
        self.economy.record(profile.economy);
    }

    fn record_frame(&mut self, ticks: u32, duration: Duration) {
        self.ticks_last_frame = ticks;
        self.frame_sim.record(duration);
    }

    fn snapshot(&self) -> SimPerfSnapshot {
        SimPerfSnapshot {
            ticks_last_frame: self.ticks_last_frame,
            frame_sim: self.frame_sim.snapshot(),
            tick_total: self.tick_total.snapshot(),
            systems: SystemPerfSnapshot {
                production: self.production.snapshot(),
                transport: self.transport.snapshot(),
                companion_ai: self.companion_ai.snapshot(),
                projectiles: self.projectiles.snapshot(),
                combat: self.combat.snapshot(),
                economy: self.economy.snapshot(),
            },
        }
    }
}

impl GameEngine {
    pub fn new(seed: u64, class: HeroClass) -> Self {
        let registry = Arc::new(Registry::load_embedded().unwrap_or_else(|errors| {
            let rendered = errors
                .iter()
                .map(|err| format!("{}: {}", err.path, err.message))
                .collect::<Vec<_>>()
                .join("\n");
            panic!("Failed to load embedded registry:\n{rendered}");
        }));
        let state = GameState::new(seed, class, &registry);
        Self {
            registry,
            state,
            accumulator: 0.0,
            next_id: 100,
            perf: EnginePerfState::default(),
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
                let floor_tick_cost = self.registry.balance.construction.floor_tick_cost;
                if eco.ticks_remaining < floor_tick_cost {
                    return Err(CommandError::InsufficientTicks {
                        needed: floor_tick_cost,
                        available: eco.ticks_remaining,
                    });
                }
                let (mat_type, mat_cost, panel_hp) = match material {
                    FloorMaterial::Wood => (
                        ResourceType::Wood,
                        self.registry.balance.construction.wood_floor_material_cost,
                        self.registry.balance.construction.wood_panel_hp,
                    ),
                    FloorMaterial::Stone => (
                        ResourceType::Stone,
                        self.registry.balance.construction.stone_floor_material_cost,
                        self.registry.balance.construction.stone_panel_hp,
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

                self.state.economy.ticks_remaining -= floor_tick_cost;
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
                let rack_resource = weapon_resource(self.state.tower.hero.weapon_primary.base_type)
                    .unwrap_or(ResourceType::Arrows);
                self.state.tower.balconies.push(Balcony {
                    id: balcony_id,
                    floor: floor_idx,
                    rack: AmmoRack {
                        resource: rack_resource,
                        current: 0,
                        max: self.registry.balance.hero.ammo_rack_max,
                        destroyed: false,
                    },
                    cover_level: CoverLevel::Exposed,
                    occupant: (self.state.tower.hero.position == balcony_id)
                        .then_some(self.state.tower.hero.id),
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

                let Some(building_def) = self.registry.building_by_type(building_type) else {
                    return Err(CommandError::InvalidCommand {
                        reason: format!("{building_type:?} not available in MVP"),
                    });
                };
                let build_tick_cost = building_def.build_tick_cost;
                let build_resource = building_def.build_resource;
                let build_resource_cost = building_def.build_resource_cost;
                let tier = building_def.tier;
                let output_resource = building_def.output_resource;
                let output_buffer_max = building_def.output_buffer_max;
                let production_rate = building_def.production_rate;
                let operating_cost = building_def.operating_cost;

                let eco = &self.state.economy;
                if eco.ticks_remaining < build_tick_cost {
                    return Err(CommandError::InsufficientTicks {
                        needed: build_tick_cost,
                        available: eco.ticks_remaining,
                    });
                }
                let available = self.get_material(build_resource);
                if available < build_resource_cost {
                    return Err(CommandError::InsufficientMaterials {
                        resource: build_resource,
                        needed: build_resource_cost,
                        available,
                    });
                }

                self.state.economy.ticks_remaining -= build_tick_cost;
                self.deduct_material(build_resource, build_resource_cost);

                self.state.tower.floors[floor].building = Some(Building {
                    building_type,
                    tier,
                    output_buffer: ResourceBuffer {
                        resource: output_resource,
                        current: 0,
                        max: output_buffer_max,
                    },
                    input_buffers: Vec::new(),
                    production_rate,
                    operating_cost,
                    is_active: true,
                });

                Ok(())
            }

            GameCommand::PlaceCache { floor } => {
                self.require_phase(GamePhase::Travel)?;

                if floor >= self.state.tower.floors.len() {
                    return Err(CommandError::InvalidFloor { index: floor });
                }
                if self.state.tower.floors[floor].cache.is_some() {
                    return Err(CommandError::FloorOccupied { index: floor });
                }

                let build_tick_cost = self.registry.balance.construction.building_tick_cost;
                if self.state.economy.ticks_remaining < build_tick_cost {
                    return Err(CommandError::InsufficientTicks {
                        needed: build_tick_cost,
                        available: self.state.economy.ticks_remaining,
                    });
                }

                self.state.economy.ticks_remaining -= build_tick_cost;
                let cache_resource = self
                    .state
                    .tower
                    .balconies
                    .iter()
                    .find(|balcony| balcony.floor == floor)
                    .map_or(ResourceType::Arrows, |balcony| balcony.rack.resource);
                self.state.tower.floors[floor].cache = Some(DepotCache {
                    slots: vec![ResourceBuffer {
                        resource: cache_resource,
                        current: 0,
                        max: self.registry.balance.hero.ammo_rack_max * 2,
                    }],
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
                // Look up the selected node's type before we move.
                let node_type = chapter
                    .nodes
                    .iter()
                    .find(|n| n.id == node)
                    .map(|n| n.node_type)
                    .unwrap_or(NodeType::Combat);

                self.state.journey.current_node = node;
                // Reset tick budget for this stop
                let chapter_index = self.state.journey.current_chapter.saturating_sub(1);
                let base_ticks = self
                    .registry
                    .balance
                    .economy
                    .chapter_ticks
                    .get(chapter_index)
                    .copied()
                    .unwrap_or_else(|| {
                        *self
                            .registry
                            .balance
                            .economy
                            .chapter_ticks
                            .last()
                            .unwrap_or(&0)
                    });
                self.state.economy.ticks_remaining = base_ticks;

                // Dispatch on node type
                match node_type {
                    NodeType::Combat | NodeType::EliteCombat | NodeType::Boss => {
                        self.state.phase = GamePhase::Travel;
                    }
                    NodeType::Merchant => {
                        self.enter_merchant();
                    }
                    NodeType::Rest => {
                        // Grant bonus ticks (already applied above +2)
                        self.state.economy.ticks_remaining = base_ticks + 2;
                        // Full-repair all panels
                        for floor in &mut self.state.tower.floors {
                            floor.panel.current_hp = floor.panel.max_hp;
                            floor.panel.is_breached = false;
                        }
                        self.state.phase = GamePhase::Travel;
                    }
                    NodeType::Mystery => {
                        self.apply_mystery_event();
                        self.state.phase = GamePhase::Travel;
                    }
                }
                Ok(())
            }

            GameCommand::March => {
                self.require_phase(GamePhase::Travel)?;
                // Convert unused ticks to gold
                let unused = self.state.economy.ticks_remaining;
                self.state.economy.gold += unused * self.registry.balance.economy.unused_tick_gold;
                self.state.economy.ticks_remaining = 0;

                // Determine difficulty from current node
                let chapter = &self.state.journey.chapters[self.state.journey.current_chapter - 1];
                let node = chapter
                    .nodes
                    .iter()
                    .find(|n| n.id == self.state.journey.current_node);
                let difficulty = node.and_then(|n| n.difficulty).unwrap_or(1);
                let is_boss = node.is_some_and(|n| n.node_type == NodeType::Boss);

                // Refill ammo from warehouse before combat
                self.state.tower.hero.personal_ammo = self.registry.balance.hero.personal_ammo;

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
                let weapon_def = self
                    .registry
                    .weapon_by_sub_type(&weapon.sub_type)
                    .ok_or_else(|| CommandError::InvalidCommand {
                        reason: format!("Unknown weapon definition for {}", weapon.sub_type),
                    })?;
                let base_type = weapon.base_type;
                let damage = weapon.damage;
                let needs_ammo = base_type != WeaponBaseType::Melee;
                let hero_id = hero.id;
                let speed = weapon_def.projectile_speed.unwrap_or(0.0);
                let melee_range = weapon_def.range;
                let battlefield_width = self.registry.balance.combat.battlefield_width;

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
                                EnemyPosition::Ground { x } => *x <= melee_range,
                                _ => false,
                            })
                            .min_by(|a, b| {
                                let ax = match &a.position {
                                    EnemyPosition::Ground { x } => *x,
                                    _ => battlefield_width,
                                };
                                let bx = match &b.position {
                                    EnemyPosition::Ground { x } => *x,
                                    _ => battlefield_width,
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
                let phase = self.state.phase;
                if phase != GamePhase::PostCombat && phase != GamePhase::Merchant {
                    return Err(CommandError::WrongPhase {
                        expected: "PostCombat or Merchant".into(),
                        actual: format!("{phase:?}"),
                    });
                }
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

            GameCommand::EquipWeapon { slot, weapon } => {
                // Allowed in any non-combat phase
                if self.state.phase == GamePhase::Encounter {
                    return Err(CommandError::WrongPhase {
                        expected: "non-combat".into(),
                        actual: format!("{:?}", self.state.phase),
                    });
                }
                match slot {
                    WeaponSlot::Primary => self.state.tower.hero.weapon_primary = weapon,
                    WeaponSlot::Secondary => self.state.tower.hero.weapon_secondary = weapon,
                }
                Ok(())
            }

            GameCommand::AllocateStat { stat } => {
                // Allowed in any non-combat phase
                if self.state.tower.hero.stats.unspent_points == 0 {
                    return Err(CommandError::InvalidCommand {
                        reason: "No unspent stat points".into(),
                    });
                }
                let stats = &mut self.state.tower.hero.stats;
                stats.unspent_points -= 1;
                match stat {
                    StatType::Precision => stats.precision += 1,
                    StatType::DrawPower => stats.draw_power += 1,
                    StatType::Tempo => stats.tempo += 1,
                    StatType::Grit => stats.grit += 1,
                    StatType::Salvage => stats.salvage += 1,
                }
                Ok(())
            }

            GameCommand::BuyItem { item_index } => {
                self.require_phase(GamePhase::Merchant)?;
                // MVP merchant sells the first 3 trinkets from the registry
                // at a fixed 20 gold each.
                let stock: Vec<_> = self.registry.trinkets.values().take(3).cloned().collect();
                let Some(trinket_def) = stock.get(item_index) else {
                    return Err(CommandError::InvalidTarget);
                };
                const PRICE: u32 = 20;
                if self.state.economy.gold < PRICE {
                    return Err(CommandError::InsufficientGold {
                        needed: PRICE,
                        available: self.state.economy.gold,
                    });
                }
                self.state.economy.gold -= PRICE;
                self.state.tower.hero.trinket = Some(Trinket {
                    id: trinket_def.id.0.clone(),
                    name: trinket_def.name.clone(),
                });
                Ok(())
            }

            GameCommand::UseWeaponAbility => {
                self.require_phase(GamePhase::Encounter)?;
                if self.state.tower.hero.weapon_ability_cooldown > 0.0 {
                    return Err(CommandError::InvalidCommand {
                        reason: "Weapon ability on cooldown".into(),
                    });
                }
                // Ability effect: fire a burst of 3 projectiles in a
                // spread that all hit the frontmost enemy. This is the
                // MVP "burst moment" for all weapons.
                let hero = &self.state.tower.hero;
                let weapon = match hero.active_weapon {
                    WeaponSlot::Primary => &hero.weapon_primary,
                    WeaponSlot::Secondary => &hero.weapon_secondary,
                };
                let base_type = weapon.base_type;
                let damage = weapon.damage * 1.8;
                let hero_id = hero.id;
                let speed = match base_type {
                    WeaponBaseType::Bow => 300.0,
                    WeaponBaseType::Crossbow => 400.0,
                    WeaponBaseType::Staff => 500.0,
                    WeaponBaseType::Thrown => 250.0,
                    WeaponBaseType::Melee => 0.0,
                };

                self.state.tower.hero.weapon_ability_cooldown = 12.0;
                let ids = [
                    self.next_projectile_id(),
                    self.next_projectile_id(),
                    self.next_projectile_id(),
                ];
                if let Some(encounter) = self.state.encounter.as_mut() {
                    for (i, id) in ids.into_iter().enumerate() {
                        encounter.projectiles.push(Projectile {
                            id,
                            source: hero_id,
                            weapon_type: base_type,
                            position: Vec2::new(0.0, (i as Scalar - 1.0) * 4.0),
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

            GameCommand::UseHeroSkill => {
                self.require_phase(GamePhase::Encounter)?;
                if self.state.tower.hero.hero_skill_cooldown > 0.0 {
                    return Err(CommandError::InvalidCommand {
                        reason: "Hero skill on cooldown".into(),
                    });
                }
                // Skills have class-specific effects. MVP implements
                // one tangible benefit per class.
                let skill = self.state.tower.hero.hero_skill;
                let encounter = self.state.encounter.as_mut();
                match (skill, encounter) {
                    (HeroSkill::Focus, Some(enc)) => {
                        // Focus: deal heavy damage to the nearest enemy.
                        if let Some(target) = enc
                            .enemies
                            .iter_mut()
                            .filter(|e| e.state != EnemyState::Dead)
                            .min_by(|a, b| {
                                let ax = match a.position {
                                    EnemyPosition::Ground { x } => x,
                                    _ => Scalar::INFINITY,
                                };
                                let bx = match b.position {
                                    EnemyPosition::Ground { x } => x,
                                    _ => Scalar::INFINITY,
                                };
                                ax.partial_cmp(&bx).unwrap()
                            })
                        {
                            target.hp -= 60.0;
                        }
                    }
                    (HeroSkill::Overclock, Some(enc)) => {
                        // Overclock: damage all enemies in a spread
                        // (lightning-like AoE).
                        for enemy in enc
                            .enemies
                            .iter_mut()
                            .filter(|e| e.state != EnemyState::Dead)
                            .take(3)
                        {
                            enemy.hp -= 25.0;
                        }
                    }
                    (HeroSkill::Rally, _) => {
                        // Rally: boost companion accuracy temporarily.
                        // MVP: permanent small boost (+0.05) up to 0.95.
                        for companion in self.state.tower.companions.iter_mut() {
                            companion.accuracy = (companion.accuracy + 0.05).min(0.95);
                        }
                    }
                    _ => {}
                }
                self.state.tower.hero.hero_skill_cooldown = 20.0;
                Ok(())
            }

            // ── Unimplemented (MVP) ─────────────────────────────
            _ => Err(CommandError::InvalidCommand {
                reason: "Command not implemented in MVP".into(),
            }),
        }
    }

    fn generate_encounter(&mut self, difficulty: u8, is_boss: bool) -> EncounterState {
        let budget = self
            .registry
            .balance
            .combat
            .difficulty_budgets
            .get(difficulty.saturating_sub(1) as usize)
            .copied()
            .unwrap_or_else(|| {
                *self
                    .registry
                    .balance
                    .combat
                    .difficulty_budgets
                    .last()
                    .unwrap_or(&0)
            });

        let mut enemies = Vec::new();
        let mut remaining_budget = budget;
        let battlefield_width = self.registry.balance.combat.battlefield_width;
        let chapter_index = self.state.journey.current_chapter.saturating_sub(1);
        let runner_id = self
            .registry
            .enemy_by_archetype(EnemyArchetype::Runner)
            .expect("registry must define runner")
            .id
            .clone();
        let chapter_has_runner = self
            .registry
            .chapters
            .get(chapter_index)
            .or_else(|| self.registry.chapters.first())
            .expect("registry must contain at least one chapter")
            .enemy_pool
            .iter()
            .any(|id| id == &runner_id);
        let grunt = self
            .registry
            .enemy_by_archetype(EnemyArchetype::Grunt)
            .expect("registry must define grunt");
        let grunt_archetype = grunt.archetype;
        let grunt_hp = grunt.hp;
        let grunt_speed = grunt.speed;
        let grunt_threat = grunt.threat;
        let runner = self
            .registry
            .enemy_by_archetype(EnemyArchetype::Runner)
            .expect("registry must define runner");
        let runner_archetype = runner.archetype;
        let runner_hp = runner.hp;
        let runner_speed = runner.speed;
        let runner_threat = runner.threat;
        let armored = self
            .registry
            .enemy_by_archetype(EnemyArchetype::Armored)
            .expect("registry must define armored");
        let armored_archetype = armored.archetype;
        let armored_hp = armored.hp;
        let armored_speed = armored.speed;

        // Simple enemy generation: fill budget with grunts and runners
        let mut spawn_x = battlefield_width;
        while remaining_budget >= grunt_threat {
            let (archetype, hp, speed, threat) = if chapter_has_runner
                && remaining_budget >= runner_threat
                && self.state.rng.next_f32() > 0.6
            {
                (runner_archetype, runner_hp, runner_speed, runner_threat)
            } else {
                (grunt_archetype, grunt_hp, grunt_speed, grunt_threat)
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

        // For boss encounters, spawn a real boss instead of a tough grunt.
        // Falls back to Armored if no boss def is loaded.
        let _ = (armored_archetype, armored_hp, armored_speed);
        if is_boss {
            let boss_data = self
                .registry
                .enemy_by_archetype(EnemyArchetype::BossGround)
                .or_else(|| self.registry.enemy_by_archetype(EnemyArchetype::Armored))
                .map(|def| (def.archetype, def.hp, def.speed));
            if let Some((archetype, hp, speed)) = boss_data {
                let id = self.next_enemy_id();
                enemies.push(Enemy {
                    id,
                    archetype,
                    position: EnemyPosition::Ground {
                        x: battlefield_width + 200.0,
                    },
                    hp,
                    max_hp: hp,
                    speed,
                    state: EnemyState::Approaching,
                    stuck_arrows: Vec::new(),
                });
            }
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

    // ── Merchant / Mystery ──────────────────────────────────

    fn enter_merchant(&mut self) {
        // Simple stock: first 3 weapons and first 2 trinkets from registry.
        // Deterministic via BTreeMap iteration order.
        self.state.phase = GamePhase::Merchant;
        // (merchant state is displayed by reading registry directly in the
        //  frontend; we don't persist a snapshot here for MVP)
    }

    fn apply_mystery_event(&mut self) {
        // Small event pool — roll a d6 on the deterministic RNG.
        let roll = self.state.rng.range(1, 6);
        match roll {
            1 => {
                // Stumble upon gold
                self.state.economy.gold += 15;
            }
            2 => {
                // Supplies
                for mat in self.state.economy.materials.iter_mut() {
                    if mat.resource == ResourceType::Wood || mat.resource == ResourceType::Stone {
                        mat.current += 3;
                    }
                }
            }
            3 => {
                // Refresh cooldowns
                self.state.tower.hero.weapon_ability_cooldown = 0.0;
                self.state.tower.hero.hero_skill_cooldown = 0.0;
            }
            4 => {
                // Heal all panels
                for floor in &mut self.state.tower.floors {
                    floor.panel.current_hp = floor.panel.max_hp;
                    floor.panel.is_breached = false;
                }
            }
            5 => {
                // Boost companion accuracy
                for companion in self.state.tower.companions.iter_mut() {
                    companion.accuracy = (companion.accuracy + 0.1).min(0.95);
                }
            }
            _ => {
                // A small tax (tradeoff variety)
                self.state.economy.gold = self.state.economy.gold.saturating_sub(5);
            }
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
        let mut ticks_this_frame = 0;

        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        let frame_start = Instant::now();

        while self.accumulator >= FIXED_DT {
            let result = systems::tick(&mut self.state, &self.registry, FIXED_DT);

            #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
            self.perf.record_tick(&result.profile);

            all_sounds.extend(result.sounds);
            self.accumulator -= FIXED_DT;
            ticks_this_frame += 1;
        }

        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        self.perf
            .record_frame(ticks_this_frame, frame_start.elapsed());

        #[cfg(any(not(debug_assertions), target_arch = "wasm32"))]
        {
            let _ = ticks_this_frame;
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

    pub fn get_economy_state(&self) -> EconomySnapshot {
        let economy = &self.state.economy;
        EconomySnapshot {
            gold: economy.gold,
            materials: economy.materials.clone(),
            ticks_remaining: economy.ticks_remaining,
        }
    }

    pub fn get_gold(&self) -> u32 {
        self.state.economy.gold
    }

    pub fn get_merchant_state(&self) -> MerchantSnapshot {
        let items = self
            .registry
            .trinkets
            .values()
            .take(3)
            .enumerate()
            .map(|(index, def)| MerchantItem {
                index,
                id: def.id.0.clone(),
                name: def.name.clone(),
                description: def.description.clone(),
                price: 20,
            })
            .collect();
        MerchantSnapshot {
            items,
            gold: self.state.economy.gold,
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
        let mut warnings = Vec::new();

        // No floors → tower has no defenses
        if self.state.tower.floors.is_empty() {
            warnings.push(ValidationWarning {
                severity: WarningSeverity::Warning,
                message: "Tower has no floors. Build at least one before marching.".into(),
            });
        }

        // No production buildings → hero will run out of ammo if encounter is long
        let has_production = self.state.tower.floors.iter().any(|f| f.building.is_some());
        if !has_production
            && self.state.tower.hero.weapon_primary.base_type != WeaponBaseType::Melee
        {
            warnings.push(ValidationWarning {
                severity: WarningSeverity::Info,
                message: "No production buildings. Ammo will not regenerate during combat.".into(),
            });
        }

        // Damaged panels
        let damaged = self
            .state
            .tower
            .floors
            .iter()
            .filter(|f| f.panel.current_hp < f.panel.max_hp * 0.5)
            .count();
        if damaged > 0 {
            warnings.push(ValidationWarning {
                severity: WarningSeverity::Warning,
                message: format!("{damaged} panel(s) below 50% HP. Consider a Rest node."),
            });
        }

        // Foundation low
        if self.state.tower.foundation.current_hp < self.state.tower.foundation.max_hp * 0.3 {
            warnings.push(ValidationWarning {
                severity: WarningSeverity::Critical,
                message: "Foundation HP is critical. Tower may fall this encounter.".into(),
            });
        }

        warnings
    }

    pub fn get_perf_state(&self) -> SimPerfSnapshot {
        self.perf.snapshot()
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
    pub fn new(seed: u64, class: HeroClass, registry: &Registry) -> Self {
        use crate::rng::DeterministicRng;

        let hero_def = registry
            .hero_for_class(class)
            .expect("registry must define selected hero class");
        let primary_weapon_def = registry
            .weapon(&hero_def.starting_weapon_primary)
            .expect("registry must define primary starting weapon");
        let secondary_weapon_def = registry
            .weapon(&hero_def.starting_weapon_secondary)
            .expect("registry must define secondary starting weapon");
        let chapters = registry
            .chapters
            .iter()
            .map(|chapter| ChapterMap {
                nodes: chapter.nodes.clone(),
                edges: chapter.edges.clone(),
                boss_node: chapter.boss_node,
            })
            .collect::<Vec<_>>();
        let current_node = chapters
            .first()
            .and_then(|chapter| chapter.nodes.first())
            .map_or(NodeId(0), |node| node.id);

        Self {
            phase: GamePhase::MapView,
            tower: Tower {
                floors: Vec::new(),
                foundation: Foundation {
                    leg_type: LegType::Chicken,
                    current_hp: registry.balance.construction.foundation_hp,
                    max_hp: registry.balance.construction.foundation_hp,
                    max_floors: registry.balance.construction.max_floors,
                    maintenance_cost: registry.balance.economy.leg_maintenance,
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
                    stats: hero_def.starting_stats.clone(),
                    level: 1,
                    xp: 0,
                    perks: Vec::new(),
                    weapon_primary: weapon_from_def(primary_weapon_def),
                    weapon_secondary: weapon_from_def(secondary_weapon_def),
                    active_weapon: WeaponSlot::Primary,
                    trinket: None,
                    personal_ammo: registry.balance.hero.personal_ammo,
                    aim_direction: Vec2::new(1.0, 0.0),
                    weapon_ability_cooldown: 0.0,
                    hero_skill_cooldown: 0.0,
                    hero_skill: hero_def.skill,
                },
                companions: starting_companions(registry),
            },
            encounter: None,
            journey: JourneyState {
                destination: DestinationId(0),
                current_chapter: 1,
                chapters,
                current_node,
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
                gold: registry.balance.economy.starting_gold,
                materials: vec![
                    ResourceBuffer {
                        resource: ResourceType::Wood,
                        current: registry.balance.economy.starting_wood,
                        max: 99,
                    },
                    ResourceBuffer {
                        resource: ResourceType::Stone,
                        current: registry.balance.economy.starting_stone,
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
                ticks_per_prep: registry.balance.economy.chapter_ticks[0],
            },
            rng: DeterministicRng::new(seed),
            tick: 0,
            elapsed: 0.0,
        }
    }
}

fn weapon_from_def(def: &crate::registry::WeaponDef) -> Weapon {
    Weapon {
        base_type: def.base_type,
        sub_type: def.sub_type.clone(),
        damage: def.damage,
        fire_rate: def.fire_rate,
        modifiers: Vec::new(),
    }
}

pub(crate) fn weapon_resource(base_type: WeaponBaseType) -> Option<ResourceType> {
    match base_type {
        WeaponBaseType::Bow => Some(ResourceType::Arrows),
        WeaponBaseType::Crossbow => Some(ResourceType::Bolts),
        WeaponBaseType::Staff => Some(ResourceType::Mana),
        WeaponBaseType::Thrown => Some(ResourceType::Thrown),
        WeaponBaseType::Melee => None,
    }
}

/// Build the starting companion roster for a new run.
/// MVP: hero starts with Ren (Mark) and Drift (Pinning) auto-assigned
/// to the first balcony. Companion roster management moves to the prep
/// UI in a later milestone.
fn starting_companions(registry: &Registry) -> Vec<Companion> {
    let default_bow = registry
        .weapons
        .values()
        .find(|w| matches!(w.base_type, WeaponBaseType::Bow | WeaponBaseType::Crossbow));
    let weapon = default_bow.map(weapon_from_def).unwrap_or_else(|| Weapon {
        base_type: WeaponBaseType::Bow,
        sub_type: "shortbow".into(),
        damage: 10.0,
        fire_rate: 1.2,
        modifiers: Vec::new(),
    });

    vec![
        Companion {
            id: EntityId(1000),
            name: "Ren".into(),
            position: Some(BalconyId(0)),
            passive: CompanionPassive::Mark,
            accuracy: 0.4,
            combat_xp: 0,
            weapon: weapon.clone(),
            trinket: None,
            target_order: TargetOrder::Closest,
            fire_discipline: FireDiscipline::AtWill,
            wage: 2,
            injured: false,
            injury_remaining: 0,
        },
        Companion {
            id: EntityId(1001),
            name: "Drift".into(),
            position: Some(BalconyId(0)),
            passive: CompanionPassive::PinningShots,
            accuracy: 0.4,
            combat_xp: 0,
            weapon,
            trinket: None,
            target_order: TargetOrder::Closest,
            fire_discipline: FireDiscipline::AtWill,
            wage: 2,
            injured: false,
            injury_remaining: 0,
        },
    ]
}
