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
/// Just over seven and a half days — long enough for a tower to finish
/// its shopping list, jam if it is going to, and settle into a steady
/// state, and short enough that every seed's region-1 roll outlasts it.
/// The route comparison runs to this rather than to the region edge.
const FIXED_BUDGET: u32 = 109_520;

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

    route_pays();

    println!("\n=== the whole way ===\n");
    whole_run(1);
}

/// What the tower is carrying when the route comparison runs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tower {
    /// A kitchen and a bunk and nothing else. What M3 measured.
    Bare,
    /// M5's chain with no way to get rid of what it makes.
    Clogged,
    /// The same, plus the chute.
    Full,
    /// The same again, plus a burner — the one room in the pack that
    /// turns a harvested material into something consumed for ever.
    Burning,
    /// A bare tower with every stalk of bamboo teleported out of it ten
    /// times a second, so the cutter arm can never stall for want of
    /// somewhere to put one.
    ///
    /// **Not a tower anybody could play: a ceiling.** Whatever a
    /// standing buyer, a second chute, a bigger storeroom or any other
    /// sink could be worth, it is worth no more than this, because
    /// nothing here ever waited for anywhere to put anything. It is the
    /// only honest way to ask what `yield_pct` is worth, and it settles
    /// `SYSTEMS.md` §5.11 open question 0 without needing to build the
    /// answer first.
    Uncapped,
}

/// Does the route pay? **It does not, in bamboo, and that is the design
/// working rather than failing.**
///
/// The section above this one cannot answer the question, and it took
/// two milestones to notice. It compares two policies on *one seed*, and
/// per seed the shade-against-sun gap on one unchanged tower ranges from
/// **-3.9% to +71.9%**. Every finding this instrument ever reported
/// about the route — "exactly 100 each", "219 against 219", "579 against
/// 530" — was a coin flip written down as a result, and two of them
/// became open questions in `SYSTEMS.md`. So this totals both policies
/// across every seed, which is the least it can do and still be
/// measuring anything.
///
/// The last row is the one that settles it. **`Uncapped` is a tower
/// whose buffers are bottomless** — nothing in it can ever be full, so
/// the cutter arm never stalls and its harvest is whatever the ground
/// and the legs allowed. It is not playable and it is not meant to be;
/// it is a ceiling. Whatever a standing buyer, a second chute, a bigger
/// storeroom or any other sink could ever be worth, it is worth no more
/// than that row.
///
/// What the rows say, in order:
///
/// - **Uncapping is worth about a quarter.** 6,501 to 7,988 on a bare
///   tower. So the cap is real, and a sink is worth building if more
///   harvest is what you want.
/// - **It buys nothing at all on the route.** The gap at the ceiling is
///   **-1.4% — the sun route ahead** — so no sink will make terrain
///   yield legible, because the yield was never what was binding.
/// - **Because yield and sun cancel, on purpose.** `TerrainDef.sun_pct`
///   says it outright: "deliberately opposed to `yield_pct`… it only
///   works if no band is good at both." Yield scales harvest per pace;
///   charge decides how many paces you get, because a browned-out tower
///   stops walking and terrain intake is paid in ground covered. Shade
///   gives richer ground and less power to cross it. Measured, those two
///   cancel to within two percent — which is a dead heat, and a dead
///   heat is what "no band is good at both" asks for.
///
/// So route choice is legible in what it *costs* — charge, threat, what
/// there is to salvage — and not in total bamboo, and `SYSTEMS.md` §5.11
/// open question 0 closes on that rather than on a new mechanic.
fn route_pays() {
    println!("\n=== does the route pay? (every seed, five towers) ===\n");
    println!(
        "  Bamboo harvested in {FIXED_BUDGET} ticks, shade route against sun.\n\
         One seed cannot answer this: the per-seed gap swings -4% to +72%.\n\
         The last row is a ceiling — bottomless buffers, so nothing can jam.\n"
    );
    println!("tower                 shade      sun      gap");
    for tower in [
        Tower::Bare,
        Tower::Clogged,
        Tower::Full,
        Tower::Burning,
        Tower::Uncapped,
    ] {
        let (mut shade, mut sun) = (0u64, 0u64);
        let (mut deaths, mut ticks) = (0u32, 0u32);
        for seed in 1..=SEEDS {
            let a = play_tower(seed, Policy::Forager, tower, true);
            let b = play_tower(seed, Policy::Sunseeker, tower, true);
            shade += a.bamboo;
            sun += b.bamboo;
            deaths += u32::from(a.died) + u32::from(b.died);
            ticks += a.ticks + b.ticks;
        }
        let ticks = ticks / (SEEDS as u32 * 2);
        println!(
            "{:<19} {shade:>6}   {sun:>6}   {:>+6.1}%   ({deaths} died, {ticks} ticks)",
            match tower {
                Tower::Bare => "bare",
                Tower::Clogged => "+chain, no chute",
                Tower::Full => "+chain +chute",
                Tower::Burning => "+chain +chute +burner",
                Tower::Uncapped => "bare, cannot jam",
            },
            (shade as i64 - sun as i64) as f64 * 100.0 / sun.max(1) as f64
        );
    }
    println!(
        "\n  Read down, not across. Uncapping a tower is worth about a quarter of\n\
         its harvest; the route is worth nothing, because shade's richer ground\n\
         and sun's extra paces cancel — which is what `sun_pct` was written to do."
    );
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
    // **Every settlement, not "the enclave".** `enclave_at` answers "the
    // next one you have not passed", which was the same thing as "the
    // one in the drowned city" while the pack had a single settlement
    // and stopped being so twice — once when M5 added the coast, and
    // again when region 1 got a board of its own. A run that stops at
    // the first one and walks past two is not measuring the journey.
    let mut stops: Vec<i64> = content
        .regions
        .iter()
        .enumerate()
        .filter_map(|(region, def)| {
            let enclave = def.enclave.as_ref()?;
            Some(
                engine
                    .state()
                    .world
                    .region_start_of(understory_core::ids::RegionIdx(region as u16))
                    + understory_core::fx::paces_from_int(enclave.at_paces),
            )
        })
        .collect();
    stops.sort_unstable();

    let mut traded = 0;
    let mut recruited = 0;
    let mut crossed_at = None;
    let mut berthed = 0usize;

    for tick in 0..600_000u32 {
        if let Some(fork) = engine.state().world.fork
            && fork.answer.is_none()
        {
            let _ = engine.try_send(GameCommand::TakeFork { branch: 0 });
        }

        // Stop at each settlement in turn, take what it offers, move on.
        let here = engine.state().world.distance;
        if stops.get(berthed).is_some_and(|at| here >= *at) {
            berthed += 1;
            let _ = engine.try_send(GameCommand::SetStriding { walking: false });
            engine.step(2);
            traded += (0..4u8)
                .filter(|offer| {
                    engine
                        .try_send(GameCommand::Trade { offer: *offer })
                        .is_ok()
                })
                .count();
            recruited += usize::from(engine.try_send(GameCommand::Recruit).is_ok());
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
        "  crossed into the drowned city at {:.0} minutes; berthed at {} of {} settlement(s), \
         {traded} trade(s) and {recruited} recruit(s)",
        crossed_at.map_or(0.0, minutes),
        berthed,
        stops.len(),
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
    /// Bamboo and produce separately — the two ends of the route axis.
    /// The total cannot see a change in the *mix*, which is the whole of
    /// what M5 added, so it stopped being the number this instrument is
    /// about (`SYSTEMS.md` §5.2).
    bamboo: u64,
    produce: u64,
    /// Meals eaten over the run. The kitchen chain's own throughput,
    /// and the number that says whether the biomass axis is alive: a
    /// tower that ate nothing has nothing to do with bamboo, which is
    /// the exact condition `SYSTEMS.md` §3.10 recorded.
    meals: u64,
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

/// Put `room` in the first slot on the lowest floor that will take it.
/// False if the tower cannot pay for it or has nowhere to put it.
/// How much of one material a run actually pulled out of the ground.
fn harvested_of(
    content: &understory_core::content::Content,
    state: &understory_core::state::GameState,
    id: &str,
) -> u64 {
    content
        .item_idx(id)
        .and_then(|idx| state.stats.harvested_by_item.get(idx.0 as usize).copied())
        .unwrap_or(0)
}

/// Put up a room *or* a shaft, whichever the id names.
fn build_anywhere(
    engine: &mut GameEngine,
    content: &understory_core::content::Content,
    id: &str,
) -> bool {
    if id.starts_with("shaft.") {
        let slots = content.balance.tower.floor_slots;
        let high = (engine.state().tower.floors.len() as u8).saturating_sub(1);
        return (0..slots).any(|slot| {
            engine
                .try_send(GameCommand::BuildShaft {
                    shaft: id.into(),
                    low: 0,
                    high,
                    slot,
                })
                .is_ok()
        });
    }
    place_anywhere(engine, content, id)
}

fn place_anywhere(
    engine: &mut GameEngine,
    content: &understory_core::content::Content,
    room: &str,
) -> bool {
    let floors = engine.state().tower.floors.len() as u8;
    (0..floors).any(|floor| {
        (0..content.balance.tower.floor_slots).any(|slot| {
            engine
                .try_send(GameCommand::PlaceRoom {
                    room: room.into(),
                    floor,
                    slot,
                })
                .is_ok()
        })
    })
}

fn play(seed: u64, policy: Policy) -> Run {
    play_tower(seed, policy, Tower::Full, false)
}

/// `full` puts up M5's chain — a garden, a comb, a ropery and the chute
/// that keeps them from strangling the mill. `false` leaves a bare
/// tower with a kitchen, which is the tower M3 measured.
///
/// `fixed_ticks` stops on the clock rather than at the region edge; see
/// the loop condition for why a route comparison needs that and nothing
/// else does.
fn play_tower(seed: u64, policy: Policy, tower: Tower, fixed_ticks: bool) -> Run {
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
            understory_core::content::IntakeSource::Terrain { .. }
            // A garden takes from the sky rather than from a ruin, so
            // it is not what this is looking for.
            | understory_core::content::IntakeSource::Sun { .. } => None,
        })
        .unwrap_or(60);

    let salvages = policy == Policy::Prepared;

    // **A shopping list worked through as poles allow, rather than a
    // one-shot purchase at tick zero.**
    //
    // Every policy buys a canteen, and that is the point of M4's change
    // to this harness. `SYSTEMS.md` §3.10 recorded that a shade-seeking
    // route and a sun-seeking one harvested *exactly* 100 bamboo each
    // over a whole region, because a tower had nothing to do with
    // bamboo: the shelves filled, the mill's outbox backed up, and the
    // arm stalled, identically on both routes. Meals are a demand that
    // scales with the crew rather than with shelf space and cannot be
    // satisfied by stockpiling, so a policy without a canteen is still
    // measuring the old, dead axis. A bunk comes with it, because a
    // tower whose crew never rest measures exhaustion.
    //
    // The rig comes first for a prepared tower, because it is what that
    // policy *is*: buying the home rooms first left it two poles short
    // of a rig, and a berthing policy with nothing to berth with stops
    // at every ruin and extracts nothing — measured, 99% of the run
    // standing still. Order matters when the starting stock is ten
    // poles and the list costs more than that.
    let mut list: Vec<&str> = Vec::new();
    if salvages {
        list.push("room.salvage_rig");
        list.push("room.dart_battery");
        list.push("room.thornwright");
    }
    // **The garden first, because it is the only room that needs the
    // roof.** Everything else fits anywhere, so anything bought before
    // it can take the one deck that sees the sky — and a garden that
    // never got built reports produce 0, which reads as "the sun axis
    // buys nothing" and means "the harness never tested it".
    // **The chute first of all**, ahead even of the garden, and the
    // ordering is the finding rather than a detail.
    //
    // A second harvested material with no consumer claims shelf after
    // shelf, bamboo runs out of anywhere to go, the cutter arm stalls,
    // and harvest collapses to whatever the tower's buffers hold — a
    // property of the *tower*, and so identical whichever way it walked.
    // That is how this instrument spent a milestone reporting that the
    // route did not matter. A chute is the cure, and it was in this list
    // for a whole session doing nothing, because it sat behind a seed
    // thrower that costs mechanisms the tower can never make: **this
    // loop stops at the first thing it cannot afford, so anything behind
    // a blocker is not "built later", it is not built at all.** Cheap
    // and load-bearing first; anything that can block goes last.
    // **Defences before the burner, or there is no measurement.**
    // Burner smoke is provocation, provocation is waves, and an
    // undefended three-crew tower dies: measured, 24 runs out of 24, at
    // a mean of 25,384 ticks. That is the design working — burning is
    // supposed to cost something — but it means the tower that would
    // burn has to be the tower that can afford to.
    if tower == Tower::Burning {
        list.push("room.thornwright");
        list.push("room.dart_battery");
    }
    if tower == Tower::Full || tower == Tower::Burning {
        list.push("shaft.chute");
    }
    if !matches!(tower, Tower::Bare | Tower::Uncapped) {
        list.push("room.garden");
    }
    list.push("room.canteen");
    list.push("room.bunk");
    // **And M5's chain, because a harness without a milestone's rooms in
    // it measures the previous milestone with total confidence.** Third
    // time this has been written down (`SYSTEMS.md` §3.9, §4.8, §5.9).
    // The garden in particular changes what this instrument is *for*: it
    // is the first intake that runs while the tower is stopped, so a
    // policy that berths is no longer paying for it with its whole
    // harvest.
    if !matches!(tower, Tower::Bare | Tower::Uncapped) {
        list.push("room.fiber_comb");
        list.push("room.ropery");
    }
    // Last, because it is the thing being measured and everything else
    // has to be standing before it starts drawing attention.
    if tower == Tower::Burning {
        list.push("room.burner");
    }
    // **No bombary and no thrower**, deliberately. A thrower costs
    // mechanisms, mechanisms cost a fitter, a fitter costs a forge and a
    // rig — none of which a route-comparison tower has any business
    // building — so a list containing one simply *stops there*, and
    // every item behind it goes unbuilt. That is how the chute ended up
    // never being built at all on the first attempt, which is the item
    // this whole comparison turns on. **Put the cheap, load-bearing
    // things first; anything that can block belongs at the end.**

    // **And then storerooms, indefinitely — which is what finally moved
    // this instrument's headline number.**
    //
    // `SYSTEMS.md` §3.10 recorded that a shade-seeking route and a
    // sun-seeking one harvested *exactly* the same over a whole region,
    // and read it as the biomass axis being unfeelable. That was true,
    // and it was not the whole of it. A tower that has worked through a
    // finite shopping list stops wanting poles; with nothing being
    // built, the mill's outbox fills, the shelves fill, the cutter arm's
    // buffer fills, and harvest stops dead at the tower's total buffer
    // capacity — which is a property of the *tower*, identical on both
    // routes whatever the ground underfoot was. The two policies were
    // reporting the size of their own shelves.
    //
    // `siege_run.rs` diagnosed exactly this for provocation and fixed
    // it there ("a player does not stop wanting things on day two, so
    // neither does the harness"); this harness never got the same
    // treatment. A storeroom is the cheapest standing reason to want
    // poles, so it is what the list keeps buying.
    list.extend(std::iter::repeat_n("room.storeroom", 40));
    list.reverse();

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

    // **A route comparison stops on the clock; everything else stops at
    // the boundary.**
    //
    // Running each route until it reaches the region edge lets the two
    // routes walk for *different lengths of time* — a branch draw is not
    // the same distance as its alternative — so the shade tower and the
    // sun tower get different numbers of ticks in which to harvest, and
    // the difference between them is then partly a difference in how
    // long they ran. That is a confound sitting directly on the axis
    // being measured, and it is worth about seven points: the same
    // comparison reads +9.7% on a fixed clock and +2.1% run to the
    // boundary. Everything else here is *about* the boundary — minutes
    // to it, terrain mix on the way — so only the route comparison takes
    // the fixed budget.
    let deadline = if fixed_ticks { FIXED_BUDGET } else { PATIENCE };
    while ticks < deadline
        && (fixed_ticks || engine.state().world.distance < boundary)
        && !engine.state().siege.lost
    {
        // Work the shopping list whenever the poles are there. Checked
        // every tick and cheap when the list is empty, which it is for
        // most of a run.
        //
        // **Every item is tried, not just the front one, and that is a
        // scar rather than a refinement.** A strict queue stops dead at
        // the first thing it cannot afford, so anything behind a blocker
        // is never built at all — and the blocker is usually something
        // whose cost is *made by a room further down the same list*. It
        // has cost three separate measurements now: a seed thrower
        // needing mechanisms hid the chute for a whole session and this
        // harness reported the route as flat; a dart battery needing two
        // rope sat in front of the ropery that makes rope, and the tower
        // harvested 725 bamboo instead of 4,000. Both looked like
        // findings about the game and were findings about this loop.
        // Order should be a hint here, never a gate.
        // Once a sim-second rather than every tick: scanning a whole
        // list across every floor and slot is ~1,500 rejected commands,
        // and at 30 Hz over twelve seeds and four towers that is the
        // difference between a minute and an hour. A build arriving up
        // to 29 ticks late changes nothing being measured here.
        if ticks.is_multiple_of(30)
            && let Some(at) = (0..list.len())
                .rev()
                .find(|&at| build_anywhere(&mut engine, &content, list[at]))
        {
            list.remove(at);
            // **Deliberately no `BuildFloor` here**, unlike
            // `siege_run.rs`, which does grow its towers when they run
            // out of slots. A new top floor displaces the canopy sail
            // deck (`v2-plan.md` §6.3): only the roof's sails see the
            // sun, so growing taller adds a floor's worth of lighting
            // cost and no income at all. Left to build upward whenever
            // it ran out of space, this harness bankrupted both route
            // policies — measured, permanent brown-out from about day
            // 27, 97% of the run standing still, and both routes
            // reporting the same 80 stalks because neither was moving.
            // The tower that measures a route is one that can still
            // afford to walk it.
        }

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

        // **The ceiling experiment**: give every buffer in the tower a
        // bottomless capacity, so nothing anywhere is ever full and the
        // cutter arm never once stalls for want of somewhere to put a
        // stalk.
        //
        // Enlarging the buffers rather than emptying them, and the
        // difference is the whole experiment. Deleting the bamboo *also*
        // starves the mill that turns it into poles, so the tower cannot
        // afford a kitchen, the crew starve, the legs brown out and it
        // stops walking — measured, 1,531 stalks against a bare tower's
        // 6,501. That is a measurement of starvation, not of the ground.
        if tower == Tower::Uncapped && ticks.is_multiple_of(30) {
            const BOTTOMLESS: i64 = 1_000_000;
            let state = engine.state_mut_for_test();
            for floor in &mut state.tower.floors {
                for room in &mut floor.rooms {
                    for stack in room.outputs.iter_mut().chain(room.inputs.iter_mut()) {
                        stack.max = BOTTOMLESS;
                    }
                    for shelf in &mut room.shelves {
                        shelf.max = BOTTOMLESS;
                    }
                }
            }
        }

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
        bamboo: harvested_of(&content, state, "item.bamboo"),
        produce: harvested_of(&content, state, "item.produce"),
        meals: state.stats.meals_eaten,
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
        " {:>4} {:>6} {:>5} {:>7} {:>8} {:>6} {:>6} {:>5} {:>7} {:>4}  end",
        "sun", "brown", "all", "bamboo", "produce", "meals", "scrap", "forks", "salvage", "off"
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
            " {:>3}% {:>6} {:>5} {:>7} {:>8} {:>6} {:>6} {:>5} {:>7} {:>4}  {}",
            run.exposure,
            run.brownout,
            run.harvested,
            run.bamboo,
            run.produce,
            run.meals,
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
