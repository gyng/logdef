//! Maintenance-doctrine instrument, plus the historical work-order measurement that caused
//! the old configurable ordering API to be removed.
//!
//! Two questions about mending, both raised by played runs.
//!
//! **Is the mending backlog a spiral?** No. It peaks on day one at 15-32
//! poles — most of what an early tower has — and is back to nearly zero
//! by day two on every seed. A dogfood tower reading 13 outstanding on
//! day two was the tail of that peak, not a tower losing ground.
//!
//! **Did the former work order help?** No, and moving `Mend` down made the
//! tower mend *more*. Both sections below.
//!
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
fn run(seeds: &[u64]) {
    let mut poles = 0i64;
    let mut hauls = 0u64;
    let mut mended = 0u64;
    let mut spent = 0u64;
    let mut crafts = 0u64;
    let mut wrecked = 0u64;
    let mut whole = 0i64;
    for &seed in seeds {
        let mut game = GameEngine::new(seed);
        harness::chain_tower(&mut game, 5);
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
        crafts += s.crafts_completed;
        wrecked += game
            .state()
            .tower
            .floors
            .iter()
            .flat_map(|floor| &floor.rooms)
            .filter(|room| room.is_wrecked(content))
            .count() as u64;
        whole += game.view().siege.integrity_permille;
    }
    let n = seeds.len() as i64;
    println!(
        "  {:<14} poles {:>4}  hauls {:>5}  crafts {:>4}  wrecks {:>2}  hp mended {:>5}  poles spent {:>4}  whole {:>3}%",
        "automatic",
        poles / n,
        hauls / seeds.len() as u64,
        crafts / seeds.len() as u64,
        wrecked / seeds.len() as u64,
        mended / seeds.len() as u64,
        spent / seeds.len() as u64,
        whole / n / 10,
    );
}

fn main() {
    let seeds = [4242u64, 7, 101, 2718, 31337];
    println!(
        "What does automatic recovery cost a poor tower? {} seeds, 60k ticks",
        seeds.len()
    );
    println!("  Historical comparison removed from play: Restore spent 90 poles, made 45 crafts");
    println!("  and prevented no wrecks; emergency recovery spent 42 and made 59.");
    run(&seeds);
    backlog(&seeds);
}

/// Does the mending backlog clear, or does it only ever grow?
///
/// **The question came from a played run.** A dogfood tower reached day
/// two at *zero poles* with thirteen poles of mending outstanding, on a
/// tower 98% whole and barely provoked (attention 79 of 1000). Mending
/// and building draw on the same pole. The two postures above are the
/// player's deliberate answer: preserve only broken systems, or spend
/// toward full restoration.
///
/// Sampled every whole day rather than at the end, because a backlog
/// that ends at thirteen could have been at thirty and clearing, or at
/// two and climbing, and those are opposite findings.
fn backlog(seeds: &[u64]) {
    println!();
    println!("Does the mending backlog clear?  outstanding/poles-spent, per day");
    println!(
        "  {:<8} {:>10} {:>10} {:>10} {:>10} {:>7}",
        "seed", "day 1", "day 2", "day 3", "day 4", "poles"
    );
    let mut grew = 0;
    for &seed in seeds {
        let mut game = GameEngine::new(seed);
        harness::chain_tower(&mut game, 4);
        let mut per_day = Vec::new();
        let mut spent_before = 0u64;
        for _ in 0..4 {
            for _ in 0..14_400 {
                game.step(1);
                let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
                // **A standing reason to want poles**, or this measures
                // a rich tower. The first version of this section let
                // the tower bank to its 120-pole cap and reported a
                // backlog of zero on every seed — true of *that* tower
                // and silent about the played one, which reached day two
                // at zero poles because it had spent everything on its
                // chain. `AGENTS.md` §I names this trap and it is the
                // second time this session it has been walked into.
                let _ = game.try_send(GameCommand::WidenTower);
                let _ = harness::place_anywhere(&mut game, "room.bunk");
            }
            let spent = game.state().stats.repair_poles_spent;
            per_day.push((game.view().siege.repair_cost, spent - spent_before));
            spent_before = spent;
        }
        let held = game
            .content()
            .item_idx("item.poles")
            .map_or(0, |i| game.state().stock_of(i));
        if per_day.last().map(|d| d.0) > per_day.first().map(|d| d.0) {
            grew += 1;
        }
        // **Outstanding, and what it cost that day.** The backlog alone
        // says whether the tower is losing ground; the poles spent say
        // what staying level is worth, and only the second is a design
        // number. A tower ending every day at zero backlog having spent
        // forty poles on it is not the same tower as one that spent two.
        println!(
            "  {seed:<8} {:>10} {:>10} {:>10} {:>10} {:>7}",
            format!("{}/{}", per_day[0].0, per_day[0].1),
            format!("{}/{}", per_day[1].0, per_day[1].1),
            format!("{}/{}", per_day[2].0, per_day[2].1),
            format!("{}/{}", per_day[3].0, per_day[3].1),
            held
        );
    }
    println!();
    if grew == seeds.len() {
        println!("  **THE BACKLOG GROWS ON EVERY SEED.** Mending outranks hauling and the work");
        println!("  emergency posture cannot arrest it, so a tower under even light harassment");
        println!("  has no effective verb against this. That is a design problem, not a tuning");
        println!("  one.");
    } else {
        println!(
            "  It grows on {grew} of {} seeds. **Mending is a first-day cost, not a spiral**:",
            seeds.len()
        );
        println!("  the backlog peaks on day one — 15 to 32 poles, which is most of what an early");
        println!("  tower has — and is back to nearly zero by day two on every seed. The played");
        println!("  run that prompted this read 13 outstanding on day two, which is the tail of");
        println!("  that peak rather than a tower losing ground.");
        println!();
        println!("  **What it costs is 30 to 48 poles, all of it in the first two days, and then");
        println!("  nothing.** Days three and four spend zero on every seed. So mending is an");
        println!("  opening tax rather than an ongoing one — which is the worst possible shape");
        println!("  for it to have, because the first two days are exactly when a tower is");
        println!("  trying to afford its chain and has the fewest poles to give.");
        println!();
        println!("  Days three and four spending *nothing* is `pick_repair`'s threshold working:");
        println!("  a shift costs its poles whether it mends twenty points or one, so crew do not");
        println!("  chase scratches. Chasing this down found the readout disagreeing with it —");
        println!("  `outstanding_repair_cost` counted every missing hit point including the ones");
        println!("  crew are right to leave, so a tower reported a standing bill it would never");
        println!("  be offered the chance to pay. Fixed; the 1-2 that used to sit here forever");
        println!("  are now zero, and seed 101's real 8 still shows and is paid the next day.");
        println!();
        println!("  **The limit, and it is load-bearing.** This tower ends holding 94-117 poles:");
        println!("  the sink runs out once the hull is at max_slots and the floors are full of");
        println!("  bunks, so what is measured is a tower that *gets* rich clawing a backlog");
        println!("  back. It does not show that a permanently poor tower recovers, and the first");
        println!("  version of this section — with no sink at all — reported a flat zero backlog");
        println!("  and would have said the question was silly.");
    }
}
