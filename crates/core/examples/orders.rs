//! Does the work order move the numbers?
//!
//! **It does not, and that is the third verb in a row.** `watch.rs`
//! found focus moves damage by <=1% across five policies and two tower
//! shapes, and charge priority moves nothing at all — identical to the
//! digit across five orders. This one joins them: five seeds, 60,000
//! ticks, a five-floor chain tower.
//!
//! ```text
//!   default        poles  119  hauls  152  hp mended  456  poles spent  48  whole 99%
//!   mend last      poles  112  hauls  157  hp mended  566  poles spent  61  whole 99%
//!   mend first     poles  119  hauls  152  hp mended  456  poles spent  48  whole 99%
//! ```
//!
//! Two things in there are worth more than the verdict.
//!
//! **"Mend first" is byte-identical to the default**, because the
//! default already *is* `Answer, Mend, Man, Haul` — Mend is second, and
//! `Answer` fires only when something is being stolen. A sweep that
//! includes the control without noticing is a sweep with one fewer arm
//! than it thinks, which is the validity check `watch.rs` had to invent
//! and this one got for free by printing all three.
//!
//! **Demoting Mend below Haul makes the tower mend *more*, not less.**
//! 566 hit points against 456, and 61 poles spent against 48. The
//! intuition it kills is a good one: mending outranks hauling, a repair
//! shift costs its poles whether it mends twenty points or one
//! (`balance.ron`, `repair_hp_per_shift`), so a barely-scratched tower
//! looks like it should be wasting poles on a backlog it cannot clear.
//! What actually happens is that hauling first *funds* the repairs —
//! more poles reach the shelves, so more repair shifts can be paid for.
//! The order does not choose how much mending happens; the poles do.
//!
//! **This is why the tool surface does not offer the advice.** A `look`
//! that said "demote mending" would be telling a player to do a thing
//! that measurably does the opposite, and the fact that reordering feels
//! like it should work is exactly why it needed measuring first.
//!
//! What remains true and is worth saying: `repair_cost` routinely
//! exceeds the poles on the shelves, so mending and building draw on the
//! same pool. That is a fact about the tower's income, not a scheduling
//! mistake, and §6.19 already owns it.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::harness;
use understory_core::state::Job;

fn run(order: &[Job], label: &str, seeds: &[u64]) {
    let mut poles = 0i64;
    let mut hauls = 0u64;
    let mut mended = 0u64;
    let mut spent = 0u64;
    let mut whole = 0i64;
    for &seed in seeds {
        let mut game = GameEngine::new(seed);
        harness::chain_tower(&mut game, 5);
        game.try_send(GameCommand::SetWorkOrder {
            order: order.to_vec(),
        })
        .expect("a permutation of every job is a legal work order");
        for _ in 0..60_000 {
            game.step(1);
            let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
        }
        let content = game.content();
        let idx = content.item_idx("item.poles").expect("poles exist");
        poles += game.state().stock_of(idx);
        let s = &game.state().stats;
        hauls += s.hauls_completed;
        mended += s.hp_repaired;
        spent += s.repair_poles_spent;
        whole += game.view().siege.integrity_permille;
    }
    let n = seeds.len() as i64;
    println!(
        "  {label:<14} poles {:>4}  hauls {:>5}  hp mended {:>5}  poles spent {:>4}  whole {:>3}%",
        poles / n,
        hauls / seeds.len() as u64,
        mended / seeds.len() as u64,
        spent / seeds.len() as u64,
        whole / n / 10,
    );
}

fn main() {
    let seeds = [4242u64, 7, 101, 2718, 31337];
    println!(
        "Does demoting Mend below Haul help a poor tower? {} seeds, 60k ticks",
        seeds.len()
    );
    run(&Job::ALL, "default", &seeds);
    run(
        &[Job::Answer, Job::Man, Job::Haul, Job::Mend],
        "mend last",
        &seeds,
    );
    run(
        &[Job::Mend, Job::Answer, Job::Man, Job::Haul],
        "mend first",
        &seeds,
    );
}
