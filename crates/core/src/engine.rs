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
}

impl GameEngine {
    pub fn new(seed: u64, class: HeroClass) -> Self {
        let state = GameState::new(seed, class);
        Self {
            state,
            accumulator: 0.0,
        }
    }

    /// Process a game command. Validates before applying.
    pub fn send_command(&mut self, cmd: GameCommand) -> CommandResult {
        // TODO: validate command against current phase and resources
        match self.apply_command(cmd) {
            Ok(()) => CommandResult::Ok,
            Err(e) => CommandResult::Error(e),
        }
    }

    fn apply_command(&mut self, _cmd: GameCommand) -> Result<(), CommandError> {
        // TODO: match on command variants, mutate state
        Ok(())
    }

    /// Advance the simulation by real_dt seconds.
    /// Uses a fixed timestep accumulator — runs 0 or 1 sim ticks per call.
    /// Returns sound events from any tick that ran.
    pub fn tick(&mut self, real_dt: Scalar) -> Vec<SoundEvent> {
        self.accumulator += real_dt;
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

    // -- Typed accessors for React bridge --

    pub fn get_hud_state(&self) -> HudSnapshot {
        let state = &self.state;
        let encounter = state.encounter.as_ref();

        HudSnapshot {
            ammo_primary: 0, // TODO: read from hero's rack
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
                    ammo: 0, // TODO: read from companion's rack
                    injured: c.injured,
                    displaced: c.position.is_none(),
                })
                .collect(),
            hero_hp_fraction: 1.0,  // TODO
            tower_hp_fraction: 1.0, // TODO
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

    pub fn get_validation_warnings(&self) -> Vec<ValidationWarning> {
        let mut warnings = Vec::new();
        // TODO: check for weapon/ammo mismatches, unassigned companions,
        // transport gaps, etc.
        let _ = &mut warnings;
        warnings
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

        Self {
            phase: GamePhase::Travel,
            tower: Tower {
                floors: Vec::new(),
                foundation: Foundation {
                    leg_type: LegType::Chicken,
                    current_hp: 100.0,
                    max_hp: 100.0,
                    max_floors: 5,
                    maintenance_cost: 0,
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
                        damage: 10.0,
                        fire_rate: 1.0,
                        modifiers: Vec::new(),
                    },
                    weapon_secondary: Weapon {
                        base_type: WeaponBaseType::Melee,
                        sub_type: "dagger".into(),
                        damage: 5.0,
                        fire_rate: 2.0,
                        modifiers: Vec::new(),
                    },
                    active_weapon: WeaponSlot::Primary,
                    trinket: None,
                    personal_ammo: 15,
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
                chapters: Vec::new(),
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
                gold: 100,
                materials: Vec::new(),
                ticks_remaining: 30,
                ticks_per_prep: 30,
            },
            rng: DeterministicRng::new(seed),
            tick: 0,
            elapsed: 0.0,
        }
    }
}
