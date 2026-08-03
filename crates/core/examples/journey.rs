//! Play region 1 across many seeds and print what differed.
//!
//! ```text
//! cargo run --release -p understory-core --example journey
//! ```
//!
//! M3's sprint question is run pacing, and its second exit criterion —
//! *two seeds feel meaningfully different* — is not something a test
//! can assert. What a harness can do is put the two kinds of variation
//! side by side: how much a region changes when you change the seed,
//! against how much it changes when you keep the seed and play
//! differently. If the first is not visibly larger than the second, the
//! seed is not buying anything and the criterion is not met, whatever
//! the design intended.
//!
//! The third criterion — *a 45–60 minute session reaches the drowned
//! city* — gets the same numbers in a different column: ticks to the
//! boundary, in minutes, at each speed the player can pick. That rules
//! out the region being wrong by a factor, which is the half of the
//! question a harness can answer. The other half is somebody playing it
//! and writing the number down (`v2-plan.md` §10 rule 3).

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;

const SEEDS: u64 = 12;
const TICKS_PER_DAY: u32 = 14_400;
/// Long enough for the longest region-1 roll at a walking pace, with
/// room for a policy that stops a lot.
const PATIENCE: u32 = 400_000;

fn main() {
    println!("=== one policy, {SEEDS} seeds ===\n");
    let across: Vec<Run> = (1..=SEEDS).map(|seed| play(seed, Policy::Walker)).collect();
    table(&across);

    println!("\n=== one seed, three ways of playing it ===\n");
    let within: Vec<Run> = [Policy::Walker, Policy::Dawdler, Policy::Prepared]
        .into_iter()
        .map(|policy| play(1, policy))
        .collect();
    table(&within);

    println!();
    verdict(&across, &within);
}

/// How the tower is played. Every policy answers forks — a harness that
/// walks into one and stops measures a parked tower with total
/// confidence, which this project has done twice.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Policy {
    /// Never stops. The baseline: pure walking, nothing salvaged.
    Walker,
    /// Buys a rig, a battery and a thornwright, stops at every ruin it
    /// can reach, and leaves when it has been hurt enough — keeping
    /// whatever it pulled out. Walking away is the answer to what a
    /// berth wakes up (`DECISIONS.md` §11), and a policy that never
    /// uses it is not playing the mechanic but standing in front of it:
    /// an earlier version of this one berthed until the ruin was empty
    /// and lost the Heartseed inside a day, every time.
    Prepared,
    /// Walks, but takes the quieter branch every time it is offered.
    Dawdler,
}

impl Policy {
    fn name(self) -> &'static str {
        match self {
            Self::Walker => "walker",
            Self::Prepared => "prepared",
            Self::Dawdler => "dawdler",
        }
    }
}

struct Run {
    label: String,
    length: i64,
    richness: i64,
    ticks: u32,
    halted: u32,
    /// Percentage of distance spent in each terrain kind, by index.
    mix: Vec<i64>,
    harvested: u64,
    salvaged: i64,
    forks: usize,
    branches: Vec<String>,
    ruins_in_reach: usize,
    salvage_seen: i64,
    provocation: i64,
    repelled: u64,
    days: u32,
    died: bool,
}

fn play(seed: u64, policy: Policy) -> Run {
    let mut engine = GameEngine::new(seed);
    engine.set_speed(SimSpeed::X1);
    let content = engine.content().clone();

    let boundary = engine.state().world.journey[0].end;
    let richness = engine.state().world.journey[0].ruin_richness_pct;
    let rig_reach = content
        .rooms
        .iter()
        .find_map(|room| match room.intake.as_ref()?.source {
            understory_core::content::IntakeSource::Ruin { range_paces, .. } => Some(range_paces),
            understory_core::content::IntakeSource::Terrain { .. } => None,
        })
        .unwrap_or(60);

    let salvages = policy == Policy::Prepared;
    if salvages {
        // Buy the rig out of the starting poles, or there is nothing to
        // measure. A prepared tower also buys something to shoot with,
        // because berthing wakes what lives in the ruin and a berthed
        // tower cannot walk away from it.
        let mut list = vec!["room.salvage_rig"];
        if policy == Policy::Prepared {
            list.push("room.dart_battery");
            list.push("room.thornwright");
        }
        for room in list {
            for floor in 0..content.balance.tower.starting_floors {
                let placed = (0..content.balance.tower.floor_slots).any(|slot| {
                    engine
                        .try_send(GameCommand::PlaceRoom {
                            room: room.into(),
                            floor,
                            slot,
                        })
                        .is_ok()
                });
                if placed {
                    break;
                }
            }
        }
    }

    let mut mix = vec![0i64; content.terrain.len()];
    let mut branches = Vec::new();
    let mut ruins_in_reach = 0usize;
    // Every ruin the region put in the tower's path, counted as it goes
    // by — a property of the world rather than of how it was played, so
    // that "was this city worth stopping at" is comparable between a
    // seed that stopped and one that did not.
    let mut salvage_seen = 0i64;
    let mut counted_to = 0i64;
    let mut seen_ruin: Option<i64> = None;
    let mut halted = 0u32;
    let mut ticks = 0u32;

    while ticks < PATIENCE && engine.state().world.distance < boundary && !engine.state().siege.lost
    {
        // Answer any fork, by policy.
        if let Some(fork) = engine.state().world.fork
            && fork.answer.is_none()
        {
            let pick = match policy {
                Policy::Dawdler => quieter(&content, fork.branches),
                _ => 0,
            };
            for idx in fork.branches {
                branches.push(content.branch(idx).id.clone());
            }
            let _ = engine.try_send(GameCommand::TakeFork { branch: pick });
        }

        // Stop at a ruin, if that is the policy and there is one.
        if salvages {
            let here = engine.state().world.ruin_in_reach(rig_reach);
            let walking = engine.state().walking;
            match here {
                Some(i) => {
                    let held = engine.state().world.features[i].salvage;
                    if seen_ruin != Some(held) {
                        if seen_ruin.is_none() {
                            ruins_in_reach += 1;
                        }
                        seen_ruin = Some(held);
                    }
                    // Walking away is the answer to what a berth wakes
                    // up (`DECISIONS.md` §11), and a policy that never
                    // uses it is not playing the mechanic, it is
                    // standing in front of it. A prepared tower leaves
                    // when it has been hurt enough, keeping whatever it
                    // has already pulled out — which is the decision
                    // the whole of §3.4 is built to pose.
                    let hurt =
                        understory_core::systems::siege::tower_integrity_permille(engine.state())
                            < 880;
                    let leave = policy == Policy::Prepared && hurt;
                    if walking && !leave {
                        let _ = engine.try_send(GameCommand::SetStriding { walking: false });
                    } else if !walking && leave {
                        let _ = engine.try_send(GameCommand::SetStriding { walking: true });
                    }
                }
                None => {
                    seen_ruin = None;
                    if !walking {
                        let _ = engine.try_send(GameCommand::SetStriding { walking: true });
                    }
                }
            }
        }

        let before = engine.state().world.distance;
        engine.step(1);
        ticks += 1;
        let state = engine.state();

        for feature in &state.world.features {
            if feature.salvage > 0 && feature.at > counted_to && feature.at <= state.world.distance
            {
                salvage_seen += feature.salvage;
            }
        }
        counted_to = counted_to.max(state.world.distance);
        if state.world.distance == before {
            halted += 1;
        } else if let Some(band) = state.world.band_at(state.world.distance) {
            mix[band.kind.get()] += 1;
        }
    }

    let state = engine.state();
    let walked = mix.iter().sum::<i64>().max(1);
    Run {
        label: format!("{seed}/{}", policy.name()),
        length: boundary >> 8,
        richness,
        ticks,
        halted,
        mix: mix.iter().map(|n| n * 100 / walked).collect(),
        harvested: state.stats.items_harvested,
        salvaged: state.stock_of(
            content
                .item_idx("item.scrap")
                .expect("the pack defines scrap"),
        ),
        forks: branches.len() / 2,
        branches,
        ruins_in_reach,
        salvage_seen,
        provocation: state.siege.provocation,
        repelled: state.siege.repelled,
        days: ticks / TICKS_PER_DAY,
        died: state.siege.lost,
    }
}

/// Which of a fork's two branches is the quieter one.
fn quieter(
    content: &understory_core::content::Content,
    pair: [understory_core::ids::BranchIdx; 2],
) -> u8 {
    let a = content.branch(pair[0]).threat_pct;
    let b = content.branch(pair[1]).threat_pct;
    u8::from(b < a)
}

fn table(runs: &[Run]) {
    let content = GameEngine::new(1).content().clone();
    let names: Vec<&str> = content
        .terrain
        .iter()
        .map(|t| t.id.trim_start_matches("terrain."))
        .collect();

    print!(
        "{:<14} {:>7} {:>5} {:>8} {:>7} {:>5}",
        "run", "length", "rich", "ticks", "halted", "days"
    );
    for name in &names {
        print!(" {:>7}", &name[..name.len().min(7)]);
    }
    println!(
        " {:>7} {:>6} {:>5} {:>6} {:>7} {:>5} {:>4}  {}",
        "bamboo", "scrap", "forks", "ruins", "salvage", "prov", "off", "end"
    );

    for run in runs {
        print!(
            "{:<14} {:>7} {:>4}% {:>8} {:>7} {:>5}",
            run.label, run.length, run.richness, run.ticks, run.halted, run.days
        );
        for pct in &run.mix {
            print!(" {pct:>6}%");
        }
        println!(
            " {:>7} {:>6} {:>5} {:>6} {:>7} {:>5} {:>4}  {}",
            run.harvested,
            run.salvaged,
            run.forks,
            run.ruins_in_reach,
            run.salvage_seen,
            run.provocation,
            run.repelled,
            if run.died { "LOST" } else { "reached" }
        );
    }

    println!("\n  minutes to the boundary, at each speed the player can pick:");
    for run in runs {
        let minutes = |mult: u32| (run.ticks as f64) / 30.0 / 60.0 / f64::from(mult);
        println!(
            "    {:<14} 1x {:>5.1}   2x {:>5.1}   4x {:>5.1}   ({:.0}% of it standing still)",
            run.label,
            minutes(1),
            minutes(2),
            minutes(4),
            f64::from(run.halted) * 100.0 / f64::from(run.ticks.max(1))
        );
    }

    if runs.len() > 2 {
        println!("\n  branch archetypes drawn:");
        for run in runs {
            println!("    {:<14} {}", run.label, run.branches.join(", "));
        }
    }
}

/// The criterion, decided on screen rather than in somebody's head.
fn verdict(across: &[Run], within: &[Run]) {
    // A run that ended in the Heartseed going is not a way of playing
    // the region, it is a way of not finishing it — and its truncated
    // numbers would swamp the comparison with noise that says nothing
    // about whether seeds differ.
    let spread = |runs: &[Run], of: &dyn Fn(&Run) -> i64| -> i64 {
        let mut values: Vec<i64> = runs.iter().filter(|r| !r.died).map(of).collect();
        values.sort_unstable();
        values.last().copied().unwrap_or(0) - values.first().copied().unwrap_or(0)
    };

    let measures: [(&str, &dyn Fn(&Run) -> i64); 3] = [
        ("forks offered", &|r: &Run| r.forks as i64),
        ("terrain mix (widest band)", &|r: &Run| {
            r.mix.iter().copied().max().unwrap_or(0)
        }),
        ("salvage in reach", &|r: &Run| r.salvage_seen),
    ];

    println!("=== do two seeds differ by more than two ways of playing one? ===\n");
    println!(
        "{:<28} {:>12} {:>12}   {}",
        "measure", "across seeds", "within one", "verdict"
    );
    let mut met = 0;
    for (name, of) in measures {
        let a = spread(across, of);
        let w = spread(within, of);
        let ok = a > w;
        met += i32::from(ok);
        println!(
            "{name:<28} {a:>12} {w:>12}   {}",
            if ok { "seed wins" } else { "PLAY WINS" }
        );
    }
    println!(
        "\n{met} of 3. The criterion asks for all three; anything less means the \n\
         seed is not buying what the design thinks it is."
    );
}
