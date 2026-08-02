use std::hint::black_box;
use std::time::Instant;

use supply_line_core::engine::GameEngine;
use supply_line_core::registry::Registry;
use supply_line_core::state::*;
use supply_line_core::types::*;

fn make_floor(registry: &Registry, index: usize) -> Floor {
    let building_def = registry
        .building_by_type(BuildingType::Fletcher)
        .expect("registry must define fletcher");
    Floor {
        index,
        building: Some(Building {
            building_type: BuildingType::Fletcher,
            tier: building_def.tier,
            output_buffer: ResourceBuffer {
                resource: building_def.output_resource,
                current: 8,
                max: building_def.output_buffer_max,
            },
            input_buffers: building_def
                .inputs
                .iter()
                .map(|inp| ResourceBuffer {
                    resource: inp.resource,
                    current: inp.buffer_max,
                    max: inp.buffer_max,
                })
                .collect(),
            production_rate: building_def.production_rate,
            operating_cost: building_def.operating_cost,
            is_active: true,
            production_progress: 0.0,
            slot: 1,
            width_slots: building_def.width_slots,
        }),
        cache: None,
        panel: WallPanel {
            current_hp: registry.balance.construction.wood_panel_hp,
            max_hp: registry.balance.construction.wood_panel_hp,
            is_breached: false,
        },
        material: FloorMaterial::Wood,
        transport_segments: Vec::new(),
        floor_width_used: 0.5,
        floor_width_max: 1.0,
        slots: 8,
    }
}

fn make_busy_engine() -> GameEngine {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    let registry = engine.registry.clone();
    let grunt = registry
        .enemy_by_archetype(EnemyArchetype::Grunt)
        .expect("registry must define grunt");
    let runner = registry
        .enemy_by_archetype(EnemyArchetype::Runner)
        .expect("registry must define runner");
    let shortbow = registry
        .weapon_by_sub_type("Shortbow")
        .expect("registry must define shortbow");

    engine.state.phase = GamePhase::Encounter;
    engine.state.tower.floors = vec![
        make_floor(&registry, 0),
        make_floor(&registry, 1),
        make_floor(&registry, 2),
    ];
    engine.state.tower.hero.personal_ammo = registry.balance.hero.personal_ammo;

    let enemies = (0..40)
        .map(|idx| Enemy {
            id: EnemyId(1000 + idx),
            archetype: if idx % 7 == 0 {
                EnemyArchetype::Armored
            } else if idx % 3 == 0 {
                EnemyArchetype::Runner
            } else {
                EnemyArchetype::Grunt
            },
            position: EnemyPosition::Ground {
                x: 100.0 + idx as Scalar * 12.0,
            },
            hp: 500.0,
            max_hp: 500.0,
            speed: if idx % 3 == 0 {
                runner.speed
            } else {
                grunt.speed
            },
            state: EnemyState::Approaching,
            stuck_arrows: Vec::new(),
        })
        .collect();

    let projectiles = (0..24)
        .map(|idx| Projectile {
            id: ProjectileId(2000 + idx),
            source: engine.state.tower.hero.id,
            weapon_type: WeaponBaseType::Bow,
            position: Vec2::new(10.0 + idx as Scalar * 8.0, 0.0),
            velocity: Vec2::new(shortbow.projectile_speed.unwrap_or(0.0), 0.0),
            gravity: 0.0,
            damage: shortbow.damage,
            modifier: None,
            state: ProjectileState::Flying,
        })
        .collect();

    engine.state.encounter = Some(EncounterState {
        enemies,
        projectiles,
        loot_on_ground: Vec::new(),
        waves: vec![Wave {
            enemies: Vec::new(),
            spawn_delay: 0.0,
        }],
        current_wave: 0,
        wave_state: WaveState::Active,
        terrain_modifier: None,
        interior_raiders: Vec::new(),
    });

    engine
}

fn run_scenario(iterations: u32) {
    let mut engine = make_busy_engine();
    for _ in 0..iterations {
        black_box(engine.tick(FIXED_DT));
    }
    black_box(engine.get_perf_state());
}

fn main() {
    const RUNS: u32 = 200;
    const TICKS_PER_RUN: u32 = 180;

    println!("representative encounter scenario benchmark");
    println!("runs: {RUNS}, ticks per run: {TICKS_PER_RUN}");

    for _ in 0..10 {
        run_scenario(TICKS_PER_RUN);
    }

    let start = Instant::now();
    for _ in 0..RUNS {
        run_scenario(TICKS_PER_RUN);
    }
    let elapsed = start.elapsed();
    let total_ticks = u128::from(RUNS) * u128::from(TICKS_PER_RUN);
    let ns_per_tick = elapsed.as_nanos() / total_ticks;

    println!(
        "scenario total={:.3} ms  ns/tick={ns_per_tick}",
        elapsed.as_secs_f64() * 1000.0
    );

    let sample = make_busy_engine();
    let perf = sample.get_perf_state();
    println!(
        "initial sim snapshot frame_ticks={} tick_calls={}",
        perf.ticks_last_frame, perf.tick_total.calls
    );
}
