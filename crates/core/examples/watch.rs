//! Do the wave verbs earn their place?
//!
//! M6 is *entirely* about what a player has to do while a wave is
//! landing (`SYSTEMS.md` §6), and the central one is naming a creature
//! for every emplacement to prefer. Nothing has ever measured whether
//! that beats leaving them to their own judgement. The tone gate shaped
//! the verb — it is a preference, not an order, and a gun with a better
//! shot in front of it still takes that shot (§6.4) — so it is entirely
//! possible the answer is "no", and that would be worth knowing before
//! anybody balances around it.
//!
//! **This file walks deliberately around the four traps the siege
//! section has already sprung**, because they are all live here:
//!
//! 1. *Give the mechanism something to do.* `siege_run.rs` once ran its
//!    whole plating comparison at a provocation where **nothing reached
//!    a room on either tower** — eight seeds, zero damage, and a
//!    confident conclusion. So this asserts damage was taken somewhere
//!    before it reports anything, and sweeps provocation rather than
//!    sitting on one level.
//! 2. *Never compare on a quantity whose maximum is the thing under
//!    test.* Focus does not change any maximum, so panels would be safe
//!    here — but rooms and shafts are counted anyway, for the same
//!    reason the other file does: it keeps the two comparable.
//! 3. *Assert your setup.* A tower with no weapon measures nothing at
//!    all, and this harness's ancestor shipped a "battery + darts" tower
//!    that was byte-for-byte the bare one. So the guns are built, then
//!    insisted on, and shots fired is a reported column — **if it is
//!    zero the run said nothing**.
//! 4. *A policy is not a mechanism.* Which creature to prefer is a
//!    choice, and a bad choice would measure worse than none. Four
//!    policies are run: leave them alone, finish the weakest, stop the
//!    nearest, and shoot whatever is still walking in.
//!
//! ## What it says
//!
//! **At provocation 1000 — the only level where the seed spread is tight
//! enough to see a small effect — no policy moves damage taken by more
//! than 1%.** Left alone loses 2583 hit points across twelve seeds
//! (2348..2732); the best policy loses 2581 and the worst 2617. At 600
//! the spread is 0..2316 and nothing there means anything; at 300
//! nothing reaches a room at all under any policy, which is trap 1 shown
//! rather than fallen into.
//!
//! **`systems/defence.rs` explains it, and the explanation is the
//! finding.** An emplacement's default target is the *nearest live
//! creature inside its range*, and a focus only overrides that when the
//! named creature is **also** inside range — a focus out of reach falls
//! back to nearest, deliberately, because "a battery that sat idle while
//! something chewed on the tower would be a trap rather than a
//! decision". So naming a creature can only ever reorder among things
//! that emplacement could already shoot, and the default is already the
//! nearest of those. **The room for it to matter is small by
//! construction.**
//!
//! That is not the same as saying the verb is wrong. §6.4's own comment
//! says it "gives the player a second moment to supply their judgement"
//! and "costs attention during a wave to use" — a claim about attention,
//! which is what the tone gate left standing (`AGENTS.md`). What this
//! file establishes is narrower and worth having: **do not balance
//! around focus reducing damage, because it does not.**
//!
//! **The validity check that makes the rest believable**: "stop nearest"
//! is the engine's own default expressed as a policy, and it reproduces
//! the untouched tower almost exactly — 2581 against 2583, on an
//! identical 2348..2732 spread. A harness whose control and whose
//! reproduction of the control disagreed would be measuring itself.
//!
//! ## What it does not cover
//!
//! One tower shape, three days, and **two weapons that answer every
//! approach** — a thorn gun and a dart battery both carry an empty
//! `targets`. §6.22 gave each emplacement an approach it answers, and a
//! tower of *specialists* is exactly where nearest-in-range should be
//! wrong most often, because a mast's nearest is not a burrower. If
//! focus pays anywhere, it pays there, and that is the run to do next.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::harness;
use understory_core::state::SimSpeed;

const TICKS_PER_DAY: u32 = 14_400;
const DAYS: u32 = 3;
const SEEDS: u64 = 12;
/// Held rather than earned, so the comparison is of the answer and not
/// of how much attention each tower happened to draw.
const LEVELS: [i64; 3] = [300, 600, 1000];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Policy {
    /// The emplacements are left to themselves. What M6 replaced.
    Ignore,
    /// Prefer whatever is closest to leaving — the shortest path to one
    /// fewer thing chewing on the tower.
    Weakest,
    /// Prefer whatever is nearest the tower, hurt or not.
    Nearest,
    /// Prefer whatever is still walking in — shoot it down before it
    /// arrives.
    ///
    /// **Added because the first three could not have worked.** All of
    /// them only named creatures already in contact, and by then the
    /// bite is landing whatever the guns do; "the verb makes no
    /// difference" measured that way is a fact about the policy, not
    /// about the verb. This is the reading a player would actually
    /// have — the thing walking at you is the thing to shoot.
    Incoming,
}

impl Policy {
    fn name(self) -> &'static str {
        match self {
            Self::Ignore => "left alone",
            Self::Weakest => "finish weakest",
            Self::Nearest => "stop nearest",
            Self::Incoming => "shoot it walking in",
        }
    }
}

struct Cell {
    lost_hp: i64,
    repelled: u64,
    shots: u64,
    deaths: u64,
}

fn main() {
    println!("Do the wave verbs earn their place?");
    println!(
        "  {DAYS} days a run, {SEEDS} seeds a cell, provocation held. The question is whether\n  \
         naming a creature beats leaving the emplacements to themselves.\n"
    );
    println!(
        "  {:<10} {:<16} {:>10} {:>15} {:>10} {:>8} {:>6}",
        "provocation", "policy", "hp lost", "seed spread", "repelled", "shots", "lost"
    );

    let mut said_something = false;
    for level in LEVELS {
        let mut cells = Vec::new();
        for policy in [
            Policy::Ignore,
            Policy::Weakest,
            Policy::Nearest,
            Policy::Incoming,
        ] {
            let mut lost_hp = 0i64;
            let mut repelled = 0u64;
            let mut shots = 0u64;
            let mut deaths = 0u64;
            let mut low = i64::MAX;
            let mut high = i64::MIN;
            for seed in 1..=SEEDS {
                let cell = press(policy, level, seed);
                lost_hp += cell.lost_hp;
                repelled += cell.repelled;
                shots += cell.shots;
                deaths += cell.deaths;
                low = low.min(cell.lost_hp);
                high = high.max(cell.lost_hp);
            }
            let n = i64::try_from(SEEDS).unwrap_or(1);
            if lost_hp > 0 {
                said_something = true;
            }
            println!(
                "  {:<10} {:<16} {:>10} {:>15} {:>10} {:>8} {:>6}",
                if policy == Policy::Ignore {
                    level.to_string()
                } else {
                    String::new()
                },
                policy.name(),
                lost_hp / n,
                format!("{low}..{high}"),
                repelled / SEEDS,
                shots / SEEDS,
                deaths,
            );
            cells.push((policy, lost_hp / n));
        }
        // The verdict for this level, stated in the same breath as the
        // numbers so nobody has to do the subtraction themselves.
        let alone = cells[0].1;
        for (policy, hp) in cells.iter().skip(1) {
            let delta = alone - hp;
            let pct = if alone == 0 {
                0.0
            } else {
                (delta * 100) as f64 / alone as f64
            };
            println!(
                "      {} vs left alone: {}{:.0}% of the damage {}",
                policy.name(),
                if delta > 0 { "−" } else { "+" },
                pct.abs(),
                if delta > 0 { "avoided" } else { "ADDED" },
            );
        }
        println!();
    }

    assert!(
        said_something,
        "no tower took any damage at any level: this run gave the mechanism nothing to do, and \
         that is the exact failure `siege_run.rs` shipped four times"
    );
}

fn press(policy: Policy, level: i64, seed: u64) -> Cell {
    let mut engine = GameEngine::new(seed);
    engine.set_speed(SimSpeed::X1);
    harness::chain_tower(&mut engine, 4);

    // **Guns, then insist on them.** A tower with nothing that shoots
    // measures the same under every policy by construction, and this
    // harness's ancestor shipped exactly that comparison twice.
    let _ = stock(&mut engine);
    // `open_the_armoury` stands up whatever supplies the weapon, so
    // naming the thornwright here as well built two of them and filled
    // the tower — the battery then had nowhere to go and the assert
    // below caught it, which is the assert doing its job.
    let weapon = "room.dart_battery";
    harness::open_the_armoury(&mut engine, weapon);
    let _ = stock(&mut engine);
    assert!(
        harness::place_anywhere(&mut engine, weapon),
        "could not build {weapon}: this is not a defended tower and it would measure          nothing. The tower is {}",
        describe(&engine)
    );

    let mut shots = 0u64;

    let ticks = DAYS * TICKS_PER_DAY;
    for tick in 0..ticks {
        if tick % 60 == 0 {
            shots += u64::try_from(stock(&mut engine).max(0)).unwrap_or(0);
            let siege = &mut engine.state_mut_for_test().siege;
            siege.provocation = level;
            siege.provocation_acc = 0;
        }

        // The verb, applied the way a player would: once a tick, on
        // whatever the policy prefers, cleared when there is nothing to
        // prefer.
        if policy != Policy::Ignore && tick % 30 == 0 {
            let want = pick(&engine, policy);
            let _ = engine.send(GameCommand::FocusEnemy { enemy: want });
        }

        engine.step(1);
        if engine.state().siege.lost {
            break;
        }
    }

    let state = engine.state();
    Cell {
        // **Rooms and shafts, not panels.** Nothing here raises a
        // maximum, so panels would be honest — but the sibling file's
        // four-times-repeated false finding came from counting a total
        // whose ceiling was under test, and keeping the same column
        // keeps the two files comparable.
        lost_hp: behind_the_skin_lost(state),
        repelled: state.siege.repelled,
        shots,
        deaths: u64::from(state.siege.lost),
    }
}

/// What is standing where, for an assert that has to explain itself.
fn describe(engine: &GameEngine) -> String {
    let content = engine.content();
    engine
        .state()
        .tower
        .floors
        .iter()
        .map(|floor| {
            let rooms: Vec<String> = floor
                .rooms
                .iter()
                .map(|room| format!("{}@{}", content.room(room.def).id, room.slot))
                .collect();
            format!(
                "F{} [{} slots] {}",
                floor.index,
                floor.slots,
                rooms.join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Whichever creature this policy would name, if any.
fn pick(engine: &GameEngine, policy: Policy) -> Option<understory_core::ids::EnemyId> {
    let state = engine.state();
    let mut best: Option<(understory_core::ids::EnemyId, i64)> = None;
    for enemy in &state.siege.enemies {
        use understory_core::state::siege::EnemyState;
        let arrived = matches!(enemy.state, EnemyState::Attacking { .. });
        let walking = matches!(enemy.state, EnemyState::Approaching);
        let score = match policy {
            // The first three name only what is already in contact.
            Policy::Weakest if arrived => enemy.hp,
            Policy::Nearest if arrived => (enemy.at - state.world.distance).abs(),
            // This one names only what has not arrived yet.
            Policy::Incoming if walking => (enemy.at - state.world.distance).abs(),
            _ => continue,
        };
        if best.is_none_or(|(_, seen)| score < seen) {
            best = Some((enemy.id, score));
        }
    }
    best.map(|(id, _)| id)
}

/// Hit points gone from rooms and shafts — the things a creature has to
/// get through the skin to reach.
fn behind_the_skin_lost(state: &understory_core::state::GameState) -> i64 {
    let mut lost = 0;
    for floor in &state.tower.floors {
        for room in &floor.rooms {
            lost += room.health.max - room.health.hp;
        }
    }
    for shaft in &state.tower.shafts {
        lost += shaft.health.max - shaft.health.hp;
    }
    lost
}

/// Keep the shelves stocked, so this measures the siege rather than the
/// economy — the same choice `siege_run.rs` makes and for the same
/// reason.
///
/// **Topped up rather than dumped.** The first version handed
/// `harness::give` forty of eight different items; a tower has one
/// storeroom, `give` shelves what fits and silently drops the rest, and
/// the items late in the list got nothing at all. The dart battery was
/// then refused for the rope it could not be given — and because
/// `place_anywhere` returns `false` on `InsufficientStock`, that
/// surfaced as "could not build room.dart_battery" with two empty front
/// slots sitting right there in the tower dump.
fn stock(engine: &mut GameEngine) -> i64 {
    let content = engine.content().clone();
    let state = engine.state_mut_for_test();
    for id in [
        "item.bamboo",
        "item.poles",
        "item.fiber",
        "item.rope",
        "item.darts",
        "item.thorns",
        "item.produce",
        "item.meals",
    ] {
        let Some(item) = content.item_idx(id) else {
            continue;
        };
        let held = state.stock_of(item);
        if held < 12 {
            state.shelve(item, 12 - held);
        }
    }
    // And the racks, so a battery is measured on its reload rather than
    // on its supply line.
    let Some(darts) = content.item_idx("item.darts") else {
        return 0;
    };
    // **What the racks swallow is what the guns fired.** The first
    // version counted darts put back on the *shelves*, which is a
    // different quantity entirely — the racks are refilled here too, so
    // the shelf level barely moved and the column read a flat 12 in
    // every single cell at every level under every policy. A number
    // that never varies is not a measurement, and this column exists
    // precisely to catch a run where nothing shot.
    let mut reloaded = 0;
    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            if let Some(rack) = room.inputs.iter_mut().find(|s| s.item == darts) {
                let space = rack.space();
                reloaded += rack.deposit(space);
            }
        }
    }
    reloaded
}
