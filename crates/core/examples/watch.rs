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
//! ## The specialists, which were supposed to be where it paid
//!
//! §6.22 gave each emplacement an approach it answers, so a tower of a
//! lantern mast, a root ward and a tanglenet should be where
//! nearest-in-range is wrong most often — a mast's nearest is not a
//! burrower. **It is not.** At provocation 1000 the five policies land
//! on 4231..4265 against a 4016..4396 seed spread: within 1%, the same
//! answer as the generalists.
//!
//! The reason was predictable from `defence.rs` and is written here
//! before the numbers deliberately: `answers()` lives *inside*
//! `in_reach`, so a specialist's default target is already filtered to
//! what it can answer. Specialising **narrows** the set a focus can
//! reorder rather than widening it.
//!
//! ## What it does not cover
//!
//! Three days, one seed family, provocation held rather than earned.
//!
//! **And the two tower shapes must not be compared with each other.**
//! The specialist tower is six floors and the generalist four, so it has
//! more to lose by construction and loses more (4241 against 2566) for
//! reasons that have nothing to do with weapons. That is the trap
//! `siege_run.rs` sprang four times, in a milder form: only the policies
//! *within* a shape are a comparison here. The rows are printed together
//! because they answer the same question, not because they are a
//! ladder.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::harness;
use understory_core::state::SimSpeed;
use understory_core::state::power::PowerUse;

const TICKS_PER_DAY: u32 = 14_400;
const DAYS: u32 = 3;
const SEEDS: u64 = 12;
/// Held rather than earned, so the comparison is of the answer and not
/// of how much attention each tower happened to draw.
const LEVELS: [i64; 3] = [300, 600, 1000];
/// Seeds for the charge question. Fewer than the siege's twelve because
/// the rows come out **identical to the digit** — a spread of zero needs
/// no sample to establish, and two tower shapes at twelve seeds each put
/// this file over ten minutes.
const CHARGE_SEEDS: u64 = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Policy {
    /// The emplacements are left to themselves. What M6 replaced.
    Ignore,
    /// Prefer whatever is closest to leaving — the shortest path to one
    /// fewer thing chewing on the tower.
    Weakest,
    /// Prefer whatever is nearest the tower, hurt or not.
    Nearest,
    /// Prefer whatever has got **past the skin** — something chewing a
    /// room, a shaft or the Heartseed outranks something scraping a
    /// panel.
    ///
    /// **This is the case the verb exists for and the first four
    /// missed.** An emplacement's default is nearest-in-range, and
    /// nearest cannot tell a creature scratching the hull from one
    /// inside a mill. That distinction is exactly a player's judgement
    /// and exactly what `DamageTarget` already carries. If focus pays
    /// anywhere, it should pay here.
    Inside,
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
            Self::Inside => "stop what is inside",
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
    for (arms, arms_name) in [
        (Arms::Generalists, "generalists (thorn gun + dart battery)"),
        (Arms::Specialists, "specialists (mast + ward + tanglenet)"),
    ] {
        println!(
            "
  == {arms_name} =="
        );
        for level in LEVELS {
            let mut cells = Vec::new();
            for policy in [
                Policy::Ignore,
                Policy::Weakest,
                Policy::Nearest,
                Policy::Inside,
                Policy::Incoming,
            ] {
                let mut lost_hp = 0i64;
                let mut repelled = 0u64;
                let mut shots = 0u64;
                let mut deaths = 0u64;
                let mut low = i64::MAX;
                let mut high = i64::MIN;
                for seed in 1..=SEEDS {
                    let cell = press(arms, policy, level, seed);
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
    }

    charge_question();

    assert!(
        said_something,
        "no tower took any damage at any level: this run gave the mechanism nothing to do, and \
         that is the exact failure `siege_run.rs` shipped four times"
    );
}

/// Which weapons the tower gets.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Arms {
    /// A thorn gun and a dart battery: both carry an empty `targets`,
    /// so both answer every approach.
    Generalists,
    /// A lantern mast (canopy), a root ward (burrow) and a tanglenet
    /// (ground) — §6.22's specialists, each answering one approach.
    ///
    /// **The shape where nearest-in-range should be wrong most often**,
    /// because a mast's nearest is not a burrower. Note that `answers()`
    /// lives *inside* `in_reach` in `defence.rs`, so a specialist's
    /// default is already filtered to what it can answer — which
    /// predicts, before running anything, that specialising narrows the
    /// room for a focus rather than widening it.
    Specialists,
}

fn press(arms: Arms, policy: Policy, level: i64, seed: u64) -> Cell {
    let mut engine = GameEngine::new(seed);
    engine.set_speed(SimSpeed::X1);
    // Five floors for the specialists: three emplacements all want a
    // leading edge, and `front_only` means one per floor.
    harness::chain_tower(&mut engine, if arms == Arms::Specialists { 6 } else { 4 });

    // **Shelves before stock.** `Tower::shelve` puts what fits and drops
    // the rest in silence, and one storeroom cannot hold eleven items —
    // so the specialist tower was refused its root ward for an alloy it
    // had been "given" and never received. Two more storerooms, then the
    // stock, then an assertion that the stock is actually there.
    for _ in 0..2 {
        let _ = harness::place_anywhere(&mut engine, "room.storeroom");
    }

    // **Guns, then insist on them.** A tower with nothing that shoots
    // measures the same under every policy by construction, and this
    // harness's ancestor shipped exactly that comparison twice.
    let _ = stock(&mut engine);
    // `open_the_armoury` stands up whatever supplies the weapon, so
    // naming the thornwright here as well built two of them and filled
    // the tower — the battery then had nowhere to go and the assert
    // below caught it, which is the assert doing its job.
    let weapons: &[&str] = match arms {
        Arms::Generalists => &["room.dart_battery"],
        Arms::Specialists => &["room.lantern_mast", "room.root_ward", "room.tanglenet"],
    };
    for weapon in weapons {
        harness::open_the_armoury(&mut engine, weapon);
        stock_or_panic(&mut engine);
        assert!(
            harness::place_anywhere(&mut engine, weapon),
            "could not build {weapon}: this is not a defended tower and it would measure              nothing. The tower is {}",
            describe(&engine)
        );
    }

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
        use understory_core::state::siege::{DamageTarget, EnemyState};
        let arrived = matches!(enemy.state, EnemyState::Attacking { .. });
        let walking = matches!(enemy.state, EnemyState::Approaching);
        let score = match policy {
            // The first three name only what is already in contact.
            Policy::Weakest if arrived => enemy.hp,
            Policy::Nearest if arrived => (enemy.at - state.world.distance).abs(),
            // This one names only what has not arrived yet.
            Policy::Incoming if walking => (enemy.at - state.world.distance).abs(),
            // And this one ranks by what it is chewing, nearest first
            // within a rank. The Heartseed is the run; a room or a
            // shaft is something the tower needs; a panel is the skin
            // doing its job.
            Policy::Inside if arrived => {
                let rank = match enemy.state {
                    EnemyState::Attacking {
                        target: DamageTarget::Heart,
                    } => 0,
                    EnemyState::Attacking {
                        target: DamageTarget::Room { .. } | DamageTarget::Shaft { .. },
                    } => 1,
                    _ => 2,
                };
                rank * 100_000 + (enemy.at - state.world.distance).abs()
            }
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
    stock_inner(engine, false)
}

/// Stock, and refuse to continue if any of it failed to land.
fn stock_or_panic(engine: &mut GameEngine) {
    stock_inner(engine, true);
}

fn stock_inner(engine: &mut GameEngine, insist: bool) -> i64 {
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
        // **Every weapon's ammunition, and every weapon's price.** A
        // root ward costs an alloy and fires alloy; a mast fires charge
        // cells. Leaving them out is how the specialist tower failed to
        // build at all — and had it built without them, it would have
        // measured a tower whose guns never fired, which is worse.
        "item.alloy",
        "item.charge_cells",
        "item.mechanisms",
    ] {
        let Some(item) = content.item_idx(id) else {
            continue;
        };
        let held = state.stock_of(item);
        if held < 12 {
            state.shelve(item, 12 - held);
        }
        assert!(
            !insist || state.stock_of(item) >= 6,
            "{id} would not go on a shelf: the tower has nowhere to put it, so anything priced              in {id} is about to be refused for a shortage this harness caused"
        );
    }
    // And the racks, so a weapon is measured on its reload rather than
    // on its supply line.
    //
    // **Every rack, not the dart rack.** A first pass topped up only
    // darts, which is fine for a thorn gun and a battery and reads a
    // flat zero for a tower of masts and wards — a shots column that
    // cannot see the guns firing is the column not doing the one job it
    // has (trap 3).
    let ammo: Vec<_> = [
        "item.darts",
        "item.alloy",
        "item.charge_cells",
        "item.rope",
        "item.bamboo",
    ]
    .iter()
    .filter_map(|id| content.item_idx(id))
    .collect();
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
            for rack in room.inputs.iter_mut().filter(|s| ammo.contains(&s.item)) {
                let space = rack.space();
                reloaded += rack.deposit(space);
            }
        }
    }
    reloaded
}

// ---------------------------------------------------------------------------
// The second verb: what the bank pays for first
// ---------------------------------------------------------------------------

/// Does saying what the charge pays for first change anything?
///
/// **Asked because the first verb did not.** Focus turned out to move
/// damage by under 1% and `defence.rs` explained why — it can only
/// reorder among things a gun could already shoot. Charge priority is a
/// different shape of verb entirely: the bank is a hard constraint, not
/// a preference, and `PowerBank::draw` refuses a spender outright when
/// taking its share would leave less than higher-ranked uses that have
/// not spent yet are owed. So there is real room for it to matter, and
/// the honest thing is to check rather than assume the milestone's other
/// verb is fine because this one was not.
///
/// **The tower has to actually be short.** A tower that can pay for
/// everything measures the same under every order by construction —
/// that is trap 1 again, and it is why this asserts a brown-out happened
/// before reporting anything.
///
/// ## What it says
///
/// **Five orders, two tower shapes, and every row identical to the
/// digit.** A fourteen-floor tower walking, with a lift, two works rooms
/// and one burner:
///
/// | shape | income | spent | brown-out ticks | paces | crafts |
/// |---|---|---|---|---|---|
/// | with a cell bank | 14,331 | 13,274 | 0 | 25,818 | 166 |
/// | no cell bank | 8,116 | 8,809 | 13,478 of 43,200 | 19,142 | 143 |
///
/// Within each shape, all five orders produce the same paces, the same
/// crafts, the same hauls, the same dark ticks and the same brown-outs.
///
/// **Three structural reasons, all readable in the code**, and they are
/// the useful half:
///
/// 1. **`PowerBank::draw` refuses on `charge < amount` before it ever
///    consults the reservation.** A flat-broke tower never reaches the
///    ranking — everything is refused at the first check. A comfortable
///    tower never reaches it either, because nothing is refused. The
///    ranking can only bite in a middle band where there is enough for
///    *this* spender but not enough to also cover higher-ranked uses
///    that have not spent yet.
/// 2. **`estimate_demand` reports zero for Lamps and Legs on 99 ticks
///    out of 100**, and says so in its own comment: they buy in
///    hundred-tick blocks, "so what they want on any given tick is
///    either a whole block or nothing at all". `reserved_against` sums
///    `demand`, so ranking either of them above anything reserves
///    nothing almost all of the time.
/// 3. **Capacity is income.** A burner idles unless the bank can take
///    the whole burn (`BALANCE.md`'s burner row), so removing the cell
///    bank to make the tower poor cut its income from 14,331 to 8,116.
///    The tower does not pass through the middle band on the way down —
///    it jumps from comfortable to broke.
///
/// ## What this does and does not establish
///
/// Measured: **no order changed any outcome in either shape.** Inferred
/// from the code: *why*, and that the band where it could bite is narrow
/// by construction. **Not established: that no tower can reach that
/// band** — three shapes were tried and none did, which is evidence but
/// not proof.
///
/// This is a question for the difficulty pass rather than a bug to fix
/// here: the ranking is wired correctly end to end, and every piece of
/// it does what its comments say. What has not been shown is a tower on
/// which a player turning that dial would see anything happen.
fn charge_question() {
    println!(
        "

What does the bank pay for first?"
    );
    println!(
        "  {DAYS} days a run, {CHARGE_SEEDS} seeds an order, fourteen floors of lamps against one
           burner, walking. Only the order differs.
"
    );

    let orders: [[PowerUse; 4]; 5] = [
        [
            PowerUse::Lifts,
            PowerUse::Works,
            PowerUse::Lamps,
            PowerUse::Legs,
        ],
        [
            PowerUse::Legs,
            PowerUse::Lifts,
            PowerUse::Works,
            PowerUse::Lamps,
        ],
        [
            PowerUse::Works,
            PowerUse::Lifts,
            PowerUse::Lamps,
            PowerUse::Legs,
        ],
        [
            PowerUse::Lamps,
            PowerUse::Lifts,
            PowerUse::Works,
            PowerUse::Legs,
        ],
        [
            PowerUse::Lifts,
            PowerUse::Legs,
            PowerUse::Lamps,
            PowerUse::Works,
        ],
    ];

    let mut browned = false;
    for (banked, shape) in [(true, "with a cell bank"), (false, "no cell bank")] {
        println!("  -- {shape} --");
        println!(
            "  {:<34} {:>8} {:>7} {:>6} {:>7} {:>10} {:>8} {:>8}",
            "charge goes to", "paces", "crafts", "hauls", "dark", "brownouts", "income", "spent"
        );
        let mut rows = Vec::new();
        for order in orders {
            let (mut paces, mut crafts, mut hauls, mut lit, mut outs) =
                (0i64, 0u64, 0u64, 0u64, 0u64);
            let (mut income, mut spent) = (0i64, 0i64);
            for seed in 1..=CHARGE_SEEDS {
                let run = press_charge(banked, &order, seed);
                paces += run.0;
                crafts += run.1;
                hauls += run.2;
                lit += run.3;
                outs += run.4;
                income += run.5;
                spent += run.6;
            }
            if outs > 0 {
                browned = true;
            }
            let n = i64::try_from(CHARGE_SEEDS).unwrap_or(1);
            let label = order
                .iter()
                .map(|use_| format!("{use_:?}"))
                .collect::<Vec<_>>()
                .join(" > ");
            println!(
                "  {:<34} {:>8} {:>7} {:>6} {:>7} {:>10} {:>8} {:>8}",
                label,
                paces / n,
                crafts / CHARGE_SEEDS,
                hauls / CHARGE_SEEDS,
                lit / CHARGE_SEEDS,
                outs / CHARGE_SEEDS,
                income / n,
                spent / n,
            );
            rows.push((label, paces / n, crafts / CHARGE_SEEDS));
        }

        // The spread across orders, which is the whole answer: a verb that
        // does nothing produces five identical rows.
        let (mut most_paces, mut least_paces) = (i64::MIN, i64::MAX);
        let (mut most_crafts, mut least_crafts) = (u64::MIN, u64::MAX);
        for (_, paces, crafts) in &rows {
            most_paces = most_paces.max(*paces);
            least_paces = least_paces.min(*paces);
            most_crafts = most_crafts.max(*crafts);
            least_crafts = least_crafts.min(*crafts);
        }
        println!(
        "
  Across the five orders: paces {least_paces}..{most_paces} ({:+.0}%), crafts          {least_crafts}..{most_crafts} ({:+.0}%).",
        if least_paces == 0 {
            0.0
        } else {
            ((most_paces - least_paces) * 100) as f64 / least_paces as f64
        },
        if least_crafts == 0 {
            0.0
        } else {
            ((most_crafts - least_crafts) * 100) as f64 / least_crafts as f64
        },
    );

        println!();
    }

    assert!(
        browned,
        "neither shape ever ran the bank dry, so every use was paid in full and the order could          not matter: this measured two rich towers, not a choice"
    );
}

/// One run under one priority order.
///
/// Returns paces, crafts, hauls, dark ticks, brown-out ticks, income
/// and spend — the last two because "the tower is rich" was the answer
/// three times running and only income-against-spend said *by how
/// much*, which is what turned a guessing game into a reading.
fn press_charge(
    banked: bool,
    order: &[PowerUse; 4],
    seed: u64,
) -> (i64, u64, u64, u64, u64, i64, i64) {
    let mut engine = GameEngine::new(seed);
    engine.set_speed(SimSpeed::X1);
    // **Tall on purpose, because lamps scale with height and income
    // does not.** `light_charge_per_100_ticks_per_floor` is per floor,
    // so a fourteen-floor tower's lamps cost 2,268 charge a day against
    // a four-floor tower's 648 and one burner leaves it short
    // (`AGENTS.md`, "growing taller has a real cost"). A four-floor
    // tower was the first attempt here and it never once ran the bank
    // dry — five identical rows and the assert at the end of
    // `charge_question` catching it, which is that assert doing its job.
    harness::chain_tower(&mut engine, 14);

    // **And no cell bank.** `chain_tower` builds one, and 1,500 of
    // capacity on top of the Heartseed's 800 is a buffer deep enough to
    // ride out every night — measured: income 13,731 against 12,480
    // spent over three days, a mean bank of 1,837, and not one
    // brown-out at any height. A tower whose bank never empties cannot
    // be asked what to pay for first. Capacity, not income, was what
    // made this comfortable.
    // **Which is a band, not a direction — and finding that was the
    // whole difficulty here.** `PowerBank::draw` refuses on
    // `charge < amount` *before* it ever consults the reservation, so a
    // tower that is flat broke never invokes priority at all: everything
    // is refused at the first check and the ranking is never reached. A
    // comfortable tower never invokes it either, because nothing is
    // refused. **Priority can only bite in the middle**, where there is
    // enough for this spender but not enough to also cover higher-ranked
    // uses that have not spent yet.
    //
    // Removing the cell bank overshoots straight past that band: income
    // 8,248 against 8,908 spent, 32% of ticks browned out, and five
    // orders identical to the digit. `UNDERSTORY_BANK=0` does that, for
    // anyone who wants to see the far end.
    if !banked {
        let bank = engine
            .content()
            .room_idx("room.cell_bank")
            .expect("the pack has a cell bank");
        let found = engine.state().tower.floors.iter().find_map(|floor| {
            floor
                .rooms
                .iter()
                .find(|room| room.def == bank)
                .map(|room| (floor.index, room.slot))
        });
        if let Some((floor, slot)) = found {
            let _ = engine.try_send(GameCommand::RemoveRoom { floor, slot });
        }
    }

    // And a shaft and a second works room, so there is more than one
    // thing wanting the last of the bank.
    assert!(
        harness::place_anywhere(&mut engine, "room.cellwright"),
        "no second works room: Works would have almost nothing to want"
    );
    let top = (engine.state().tower.floors.len() as u8).saturating_sub(1);
    let slot = engine.state().tower.floors[0].slots.saturating_sub(3);
    // Shelves first — the shaft costs rope this tower has never made.
    for _ in 0..2 {
        let _ = harness::place_anywhere(&mut engine, "room.storeroom");
    }
    stock_or_panic(&mut engine);
    // **Asserted, because a missing shaft is a missing spender.** Lifts
    // is the only use that draws on tick position 0, so without a shaft
    // the whole of Lifts-versus-everything disappears from the
    // comparison — and `let _ =` on a `BuildShaft` is exactly the silent
    // `false` this repo has been bitten by more than any other.
    engine
        .try_send(GameCommand::BuildShaft {
            shaft: "shaft.elevator".into(),
            low: 0,
            high: top,
            slot,
        })
        .expect("the tower needs a lift, or Lifts never spends and the order cannot bite");
    engine
        .try_send(GameCommand::SetPowerPriority {
            order: order.to_vec(),
        })
        .expect("the order is four legal uses");
    assert_eq!(
        engine.state().power.priority,
        order.to_vec(),
        "the order did not take: this would compare five identical towers"
    );
    let _ = engine.send(GameCommand::SetStriding { walking: true });

    let mut lit = 0u64;
    let mut outs = 0u64;
    let (mut income, mut spent, mut charge_sum) = (0i64, 0i64, 0i64);
    let start = engine.state().world.distance;
    for tick in 0..DAYS * TICKS_PER_DAY {
        // **Nothing is topped up here, and that is the point.** The
        // first version handed the tower twelve bamboo every 120 ticks;
        // a burner makes 800 charge from one stalk, so that is over a
        // million charge a day against a twelve-floor tower's ~1,900 of
        // lamps, and it never once ran short. Height was not the
        // constraint — **fuel is** — and the tower has to earn its own
        // for the burner and the mill to be competing for it at all.
        // That competition is `charge_per_burn`'s whole design
        // (`AGENTS.md`), and it is the condition under which an order
        // is a choice.
        // **Answer the fork or measure a parked tower.** Without this
        // the tower walked into the first split and stood there for the
        // rest of the run — 3,700 paces of a possible 25,000, the legs
        // drawing nothing because `pay_for_stride` does not charge a
        // tower that has nowhere to go, and therefore no contention at
        // all. `SYSTEMS.md` §3.3, and the trap `AGENTS.md` names.
        if tick % 300 == 0
            && let Some(fork) = engine.state().world.fork
            && fork.answer.is_none()
        {
            let _ = engine.try_send(GameCommand::TakeFork { branch: 0 });
        }
        engine.step(1);
        let power = &engine.state().power;
        // **Dark ticks, not lit ones.** `lighting` sets `lit` true and
        // returns early in daylight, so counting lit ticks counted the
        // sun — a flat 43,200 out of 43,200 in every cell, which is the
        // same "a number that never varies is not a measurement" the
        // shots column already taught this file.
        if !power.lit {
            lit += 1;
        }
        if power.brownout {
            outs += 1;
        }
        income += power.income_last;
        spent += power.spent_last;
        charge_sum += power.charge;
    }

    let state = engine.state();
    // **Say how rich it is, not just whether it browned out.** Three
    // attempts at making this tower poor failed and the assert only
    // said "rich" — which is true and useless. Income against spend
    // says *by how much*, and it turned a guessing game into a reading.
    let _ = charge_sum;
    (
        // Whole paces. `Paces` is Q8.8, so the raw difference reads as
        // 947,200 where the tower walked 3,700 — a number nobody can
        // sanity-check against the 0.6 paces a tick in `BALANCE.md`.
        (state.world.distance - start) / i64::from(understory_core::fx::FX_ONE),
        state.stats.crafts_completed,
        state.stats.hauls_completed,
        lit,
        outs,
        income,
        spent,
    )
}
