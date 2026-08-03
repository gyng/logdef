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
    println!(
        "  forager takes the shadiest branch each time, sunseeker the most open.\n\
         If the route is the power mix, those two towers should not look alike.\n"
    );
    let within: Vec<Run> = [Policy::Forager, Policy::Sunseeker, Policy::Prepared]
        .into_iter()
        .map(|policy| play(1, policy))
        .collect();
    table(&within);

    println!();
    verdict(&across, &within);

    println!("\n=== the whole way ===\n");
    whole_run(1);
}

/// One seed, start to finish, through both regions and the enclave.
///
/// Everything above stops at the region-1 boundary, which measures the
/// half of the run that has been played most. This walks the rest: into
/// the drowned city, past the settlement in it, and out to the far edge
/// where the run ends. It is the only thing that exercises a region
/// change, a second region's palette, and the enclave in one go, and it
/// is how "seeded start to region 2" stops being a claim.
fn whole_run(seed: u64) {
    let mut engine = GameEngine::new(seed);
    engine.set_speed(SimSpeed::X1);
    let content = engine.content().clone();
    let enclave_at = engine
        .state()
        .world
        .enclave_at(&content)
        .expect("the pack puts an enclave somewhere");

    let mut traded = 0;
    let mut recruited = false;
    let mut crossed_at = None;
    let mut berthed = false;

    for tick in 0..600_000u32 {
        if let Some(fork) = engine.state().world.fork
            && fork.answer.is_none()
        {
            let _ = engine.try_send(GameCommand::TakeFork { branch: 0 });
        }

        // Stop at the settlement, take what it offers, move on.
        let here = engine.state().world.distance;
        if !berthed && here >= enclave_at {
            berthed = true;
            let _ = engine.try_send(GameCommand::SetStriding { walking: false });
            engine.step(2);
            traded = (0..4u8)
                .filter(|offer| {
                    engine
                        .try_send(GameCommand::Trade { offer: *offer })
                        .is_ok()
                })
                .count();
            recruited = engine.try_send(GameCommand::Recruit).is_ok();
            let _ = engine.try_send(GameCommand::SetStriding { walking: true });
        }

        engine.step(1);
        let state = engine.state();
        if crossed_at.is_none() && state.world.region.get() > 0 {
            crossed_at = Some(tick);
        }
        if state.arrived || state.siege.lost {
            break;
        }
    }

    let state = engine.state();
    let minutes = |t: u32| f64::from(t) / 30.0 / 60.0;
    println!(
        "  seed {seed}: {} after {} ticks ({:.0} minutes at 1x, {:.0} at 4x), {} days",
        if state.arrived {
            "reached the far edge"
        } else if state.siege.lost {
            "lost the Heartseed"
        } else {
            "still going when the harness gave up"
        },
        state.tick,
        minutes(state.tick as u32),
        minutes(state.tick as u32) / 4.0,
        state.tick / u64::from(TICKS_PER_DAY),
    );
    println!(
        "  crossed into the drowned city at {:.0} minutes; {} trade(s) and {} at the settlement",
        crossed_at.map_or(0.0, minutes),
        traded,
        if recruited { "a recruit" } else { "nobody" },
    );
    println!(
        "  ended {} paces on, {} crew, {} standing",
        state.world.distance >> 8,
        state.crew.len(),
        understory_core::systems::siege::tower_integrity_permille(state),
    );
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
    /// Takes the shadiest branch on offer every time: biomass-rich,
    /// sun-poor. One half of "your route is your power mix".
    Forager,
    /// Takes the most open branch every time: sun-rich, biomass-poor.
    /// The other half.
    Sunseeker,
}

impl Policy {
    fn name(self) -> &'static str {
        match self {
            Self::Walker => "walker",
            Self::Prepared => "prepared",
            Self::Forager => "forager",
            Self::Sunseeker => "sunseeker",
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
    /// Mean sunlight reaching the sails, in percent, over the run.
    exposure: i64,
    /// Ticks the tower could not afford to walk.
    brownout: u32,
    forks: usize,
    branches: Vec<String>,
    salvage_seen: i64,
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
    // Every ruin the region put in the tower's path, counted as it goes
    // by — a property of the world rather than of how it was played, so
    // that "was this city worth stopping at" is comparable between a
    // seed that stopped and one that did not.
    let mut salvage_seen = 0i64;
    let mut counted_to = 0i64;
    let mut seen_ruin: Option<i64> = None;
    let mut halted = 0u32;
    let mut ticks = 0u32;
    let mut exposure_total = 0i64;
    let mut brownout = 0u32;

    while ticks < PATIENCE && engine.state().world.distance < boundary && !engine.state().siege.lost
    {
        // Answer any fork, by policy.
        if let Some(fork) = engine.state().world.fork
            && fork.answer.is_none()
        {
            let pick = match policy {
                Policy::Forager => shadier(&content, fork.branches, true),
                Policy::Sunseeker => shadier(&content, fork.branches, false),
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
        exposure_total += understory_core::systems::power::exposure_pct(state, &content);
        if state.walking && !state.strode {
            brownout += 1;
        }

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
        exposure: exposure_total / i64::from(ticks.max(1)),
        brownout,
        salvaged: state.stock_of(
            content
                .item_idx("item.scrap")
                .expect("the pack defines scrap"),
        ),
        forks: branches.len() / 2,
        branches,
        salvage_seen,
        repelled: state.siege.repelled,
        days: ticks / TICKS_PER_DAY,
        died: state.siege.lost,
    }
}

/// Which of a fork's two branches is the shadier one, by the sunlight
/// its palette lets through — or the more open one, if `shade` is
/// false.
///
/// The whole "route is your power mix" argument rests on this choice,
/// so the policies that exercise it pick on exactly the quantity the
/// argument is about, rather than on a branch's name or its threat.
fn shadier(
    content: &understory_core::content::Content,
    pair: [understory_core::ids::BranchIdx; 2],
    shade: bool,
) -> u8 {
    let sun = |idx: understory_core::ids::BranchIdx| -> i64 {
        let palette = &content.branch_rt(idx).palette;
        let total: i64 = palette.iter().map(|(_, w)| *w).sum::<i64>().max(1);
        palette
            .iter()
            .map(|(terrain, weight)| content.terrain(*terrain).sun_pct * weight)
            .sum::<i64>()
            / total
    };
    let (a, b) = (sun(pair[0]), sun(pair[1]));
    if shade {
        u8::from(b < a)
    } else {
        u8::from(b > a)
    }
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
        " {:>4} {:>6} {:>7} {:>6} {:>5} {:>7} {:>4}  end",
        "sun", "brown", "bamboo", "scrap", "forks", "salvage", "off"
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
            " {:>3}% {:>6} {:>7} {:>6} {:>5} {:>7} {:>4}  {}",
            run.exposure,
            run.brownout,
            run.harvested,
            run.salvaged,
            run.forks,
            run.salvage_seen,
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

    type Measure<'a> = (&'a str, &'a dyn Fn(&Run) -> i64);
    let measures: [Measure; 3] = [
        ("forks offered", &|r: &Run| r.forks as i64),
        ("terrain mix (widest band)", &|r: &Run| {
            r.mix.iter().copied().max().unwrap_or(0)
        }),
        ("salvage in reach", &|r: &Run| r.salvage_seen),
    ];

    println!("=== do two seeds differ by more than two ways of playing one? ===\n");
    println!(
        "{:<28} {:>12} {:>12}   verdict",
        "measure", "across seeds", "within one"
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
