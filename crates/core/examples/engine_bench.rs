use std::hint::black_box;
use std::time::Instant;

use supply_line_core::command::{CommandResult, GameCommand};
use supply_line_core::engine::GameEngine;
use supply_line_core::state::HeroClass;
use supply_line_core::types::{FIXED_DT, NodeId};

fn start_encounter() -> GameEngine {
    let mut engine = GameEngine::new(42, HeroClass::Archer);

    assert!(matches!(
        engine.send_command(GameCommand::SelectNode { node: NodeId(1) }),
        CommandResult::Ok
    ));
    assert!(matches!(
        engine.send_command(GameCommand::March),
        CommandResult::Ok
    ));

    engine
}

fn bench<F>(label: &str, iterations: u32, mut f: F)
where
    F: FnMut(),
{
    for _ in 0..1_000 {
        f();
        black_box(());
    }

    let start = Instant::now();
    for _ in 0..iterations {
        f();
        black_box(());
    }
    let elapsed = start.elapsed();
    let elapsed_ns = elapsed.as_nanos();
    let ns_per_op = elapsed_ns / u128::from(iterations);

    println!(
        "{label:20} total={:>8.3} ms  ns/op={ns_per_op}",
        elapsed.as_secs_f64() * 1000.0
    );
}

fn main() {
    const ITERATIONS: u32 = 200_000;

    println!("supply-line-core engine microbenchmarks");
    println!("iterations per case: {ITERATIONS}");

    bench("new_game", ITERATIONS, || {
        let engine = GameEngine::new(42, HeroClass::Archer);
        black_box(engine);
    });

    bench("tick_idle", ITERATIONS, || {
        let mut engine = GameEngine::new(42, HeroClass::Archer);
        black_box(engine.tick(FIXED_DT));
    });

    bench("tick_encounter", ITERATIONS, || {
        let mut engine = start_encounter();
        black_box(engine.tick(FIXED_DT));
    });

    bench("get_hud_state", ITERATIONS, || {
        let engine = start_encounter();
        black_box(engine.get_hud_state());
    });

    bench("get_economy_state", ITERATIONS, || {
        let engine = start_encounter();
        black_box(engine.get_economy_state());
    });

    bench("get_gold", ITERATIONS, || {
        let engine = start_encounter();
        black_box(engine.get_gold());
    });

    bench("serialize_hud_state", ITERATIONS, || {
        let engine = start_encounter();
        let hud = engine.get_hud_state();
        black_box(serde_json::to_vec(&hud).expect("hud snapshot should serialize"));
    });

    bench("serialize_economy_state", ITERATIONS, || {
        let engine = start_encounter();
        let economy = engine.get_economy_state();
        black_box(serde_json::to_vec(&economy).expect("economy snapshot should serialize"));
    });
}
