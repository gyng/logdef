use std::hint::black_box;
use std::time::Instant;

use supply_line_core::balance::*;
use supply_line_core::engine::GameEngine;
use supply_line_core::state::*;
use supply_line_core::types::*;

fn make_floor(index: usize) -> Floor {
    Floor {
        index,
        building: Some(Building {
            building_type: BuildingType::Fletcher,
            tier: ProductionTier::T1,
            output_buffer: ResourceBuffer {
                resource: ResourceType::Arrows,
                current: 8,
                max: FLETCHER_BUFFER_MAX,
            },
            input_buffers: Vec::new(),
            production_rate: FLETCHER_RATE,
            operating_cost: FLETCHER_OPERATING_COST,
            is_active: true,
        }),
        cache: None,
        panel: WallPanel {
            current_hp: WOOD_PANEL_HP,
            max_hp: WOOD_PANEL_HP,
            is_breached: false,
        },
        material: FloorMaterial::Wood,
        transport_segments: Vec::new(),
        floor_width_used: 0.5,
        floor_width_max: 1.0,
    }
}

fn make_busy_engine() -> GameEngine {
    let mut engine = GameEngine::new(42, HeroClass::Archer);
    engine.state.phase = GamePhase::Encounter;
    engine.state.tower.floors = vec![make_floor(0), make_floor(1), make_floor(2)];
    engine.state.tower.hero.personal_ammo = HERO_PERSONAL_AMMO;

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
                RUNNER_ENEMY_SPEED
            } else {
                GRUNT_SPEED
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
            velocity: Vec2::new(SHORTBOW_PROJ_SPEED, 0.0),
            gravity: 0.0,
            damage: SHORTBOW_DAMAGE,
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
