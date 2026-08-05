//! Does a lift ever pay, and at what height?
//!
//! **Read the merge first** (`SYSTEMS.md` §6.18). Everything below dated
//! from when there were two built shafts that went up: a dumbwaiter and
//! an elevator. There is one now — the lift fetches stock itself
//! whenever nobody is calling it — so any figure below quoting a
//! dumbwaiter is the pre-merge pair, kept because the argument for the
//! merge *is* those figures. The current sweep says the merged shaft is
//! worth +119% at five floors, +256% at eight, +493% at eleven and
//! +913% at fourteen, and holds +251% to +271% across every hull width,
//! where the old elevator fell from +91% to +48%.
//!
//! ```text
//! cargo run --release -p understory-core --example lift
//! ```
//!
//! `throughput.rs` asks whether an elevator earns its slot in the tower
//! the harness happens to build, and answers **no** — a four-floor lift
//! in a five-floor tower costs about thirty per cent of its hauls and
//! crafts while genuinely cutting the queueing. That is a strange enough
//! answer to be worth a second instrument rather than a tuning change,
//! because "the fast transport makes the tower slower" is either a real
//! trap in the design or a fact about one tower.
//!
//! So this sweeps the one variable `throughput.rs` holds fixed: **how
//! tall the tower is.** An elevator's advantage is per floor travelled
//! (8 ticks against the stairs' 30) and its costs are per *trip* —
//! `elevator_base_wait_ticks` 45 before a car arrives, then
//! `dwell_base_ticks` 10 plus 6 a unit at each end. Fixed costs against
//! a per-floor gain is a break-even, and the only question is where it
//! falls relative to the towers people actually build.
//!
//! It also spends the crew's day rather than counting deliveries, since
//! a haul count cannot tell "fewer trips" from "slower trips", and those
//! want opposite fixes.
//!
//! **What it does not measure:** whether an elevator is *worth building*
//! — that is 18 poles and 6 rope against everything else those buy, and
//! it belongs to `journey.rs`, which plays the whole run. This is the
//! narrower question of whether the thing works once it is up.
//!
//! ## What it found, and how far to trust it
//!
//! **One result held through every version of this harness, and it is
//! the important one: a tower with only stairs gets worse the taller it
//! is.** 118 hauls at five floors, 88 at eight, 58 at eleven, 33 at
//! fourteen — a 72% collapse — while crew-ticks spent queueing at a
//! shaft go from 3,700 to 30,100. Height without vertical transport is
//! ruinous, which is the design working exactly as `DESIGN.md` pillar 2
//! intends: a shaft is the belt, and a tower that grows without one is
//! a factory with no belts.
//!
//! **The elevator's value is enormous but arrives late.** At fourteen
//! floors it is +166% hauls and +268% crafts. At five it is +1%. In
//! between, on two seeds of three, it was built and *never once used* —
//! the crew kept queueing on the stairs with a working, powered lift
//! standing empty in the same tower.
//!
//! **The likeliest explanation is horizontal, not vertical, and it is
//! not yet measured.** The stairs are built in at slot 0; this harness
//! puts the lift in the last free column. Reaching it means crossing the
//! floor and crossing back at the other end, and a crew member's shaft
//! choice is an estimate that includes that walk. At five floors the
//! far-edge lift spent about ten thousand more crew-ticks *walking* than
//! the stairs-only tower did. If that is the whole story then the
//! elevator is not mistuned at all — **where you put it is the
//! decision**, and the game currently teaches nothing about it.
//!
//! Measuring it needs two candidate columns reserved on every row, and
//! reserving two costs a quarter of the tower's width: the eight-floor
//! plan then loses a sail and browns out for three quarters of the
//! window, which measures the power budget instead. That is the next
//! piece of work here, and it wants a wider harness tower rather than a
//! cleverer one.
//!
//! **Two tuning passes that did not work**, both run through
//! `UNDERSTORY_PACK` in about forty seconds each:
//!
//! - `dispatch_max_wait_ticks` 90 → 30 made it *worse* everywhere
//!   (five floors -15% → -26%, eight floors +16% → -21%). Batching is
//!   worth more than promptness: a car that leaves early makes more
//!   trips with fewer aboard, and everyone else waits for it to come
//!   back.
//! - `dwell_base_ticks` 10 → 4 and `dwell_per_unit_ticks` 6 → 2 barely
//!   moved the break-even (eight floors +16% → +18%), despite dwell
//!   being 68% of the time a car is busy. The fixed cost that matters
//!   is not the one inside the shaft.
//!
//! ## Making a shaft necessary: four levers, measured
//!
//! All four run through `UNDERSTORY_PACK` in under a minute. The figure
//! quoted is the elevator's haul delta against stairs at each height.
//!
//! | lever | 5 floors | 8 | 11 | 14 | 5-floor queueing |
//! |---|---|---|---|---|---|
//! | *(shipped)* | +1% | unusable | unusable | +166% | 3,720 |
//! | `climb_ticks_per_floor` 30 → 45 | **+30%** | +100% | +171% | +200% | 5,442 |
//! | `climb_ticks_per_floor` 30 → 60 | -44% | +277% | +621% | +337% | 5,680 |
//! | `carry_capacity` 3 → 2 | +0% | +24% | +93% | +42% | 3,546 |
//! | `starting_crew` 3 → 6 | -27% | +57% | +164% | +571% | **30,849** |
//!
//! **`climb_ticks_per_floor` 30 → 45 is the one that works, and it is a
//! single number.** It moves the elevator's break-even from fourteen
//! floors to five, and it makes the dumbwaiter clearly worth building at
//! five (+26% hauls, +55% crafts) — so the ladder the pack describes,
//! dumbwaiter first and elevator when tall, starts existing. It costs
//! the five-floor tower about a third of its hauls, and that loss is the
//! pressure: a shaft has to be relief from something.
//!
//! It is also the *targeted* nerf. `carry_capacity` taxes horizontal
//! hauling just as hard and barely moves the break-even, which is the
//! measured argument against a general throughput nerf: what has to get
//! expensive is **height**, not work.
//!
//! Sixty is too far. The five-floor tower halves its output and the lift
//! still loses there, so the valley moves rather than closing.
//!
//! **Crew are the congestion lever, and they are not a slider.** Going
//! from three to six crew multiplies five-floor queueing by eight —
//! 3,720 crew-ticks to 30,849 — which is `DESIGN.md` insight 1 exactly:
//! transport is shared, so every body added loads the same staircase.
//! But crew arrive as enclave recruits, so this is a reward that creates
//! the problem the shaft solves, and it wants pairing with the climb
//! change rather than using alone: at six crew the lift is still -27% at
//! five floors, because the trips are short whatever the traffic.
//!
//! ## Width: the placement decision, finally measured
//!
//! The width sweep (`does_width_undo_the_shaft`, eight floors, four
//! widths) was added to answer whether growing sideways undoes the
//! climb. **It does not** — a stairs-only tower hauls 56, 56, 56, 58 at
//! ten, twelve, fourteen and sixteen slots, with crew-ticks climbing
//! flat at ~18,500. The chain spans the tower whatever the floors are.
//!
//! What it found instead is the hypothesis two sections up, measured at
//! last. **The elevator's value halves as the hull widens** — +91%,
//! +77%, +57%, +48% — and climbing does not move (~4,000 crew-ticks
//! throughout). What moves is *walking*: 21,309 to 28,361, a third
//! more. The lift stands in one column and a wider floor is further to
//! cross to reach it. So "where you put it is the decision" is not a
//! guess any more, and the cost of getting it wrong scales with width.
//!
//! **The dumbwaiter behaved in the opposite direction, and the reason is
//! structural.** +93% at ten slots, +284% at twelve, holding there —
//! while its walking *fell*, 9,469 to 4,864. Nothing rides a dumbwaiter,
//! so nobody walks to one. **Width hurts the shaft you have to reach and
//! helps the shaft that comes to you.**
//!
//! That is the finding the merge was built on, and the merge answered
//! it: the lift now does the fetching, and its value across width went
//! from +91%..+48% to +256%..+251% — flat, because the half of its work
//! that used to need a walk no longer does.
//!
//! ## The placement decision does not exist
//!
//! Worth knowing before anyone tunes `floor_slots`: on the shipped pack
//! a full-height shaft has **exactly one column it can go in**. The
//! starting tower's rooms and its built-in staircase occupy columns 0–6
//! across its floors, leaving slot 7 and nothing else. So the far-edge
//! placement this harness uses is not a choice it made — it is the only
//! one available, and the "where you put it" hypothesis above cannot be
//! a player decision until the tower has spare width.
//!
//! Narrowing the tower therefore does the opposite of what it looks
//! like: at `floor_slots` 7 *or* 6 there is no free column at all and a
//! full-height lift cannot be built. Run with `UNDERSTORY_COLUMNS=1` to
//! print what is free.

use std::sync::Arc;

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::content::Content;
use understory_core::state::CrewState;

/// The pack this run measures, so a tuning pass costs seconds.
///
/// ```text
/// cp -r assets/data /tmp/pack && $EDITOR /tmp/pack/balance.ron
/// UNDERSTORY_PACK=/tmp/pack cargo run --release -p understory-core --example lift
/// ```
///
/// **Editing `assets/data` in place does not do this**, and that is the
/// point of the variable rather than a caveat: `include_dir!` makes
/// cargo rebuild the crate whenever a shipped `.ron` changes — correct,
/// never stale, and about twenty seconds of compile for a one-character
/// edit. Pointing at a copy leaves the embedded pack untouched, so the
/// loop is edit, run, read, with no build in it.
///
/// Unset, this is the shipped pack and the numbers are the game's.
fn content() -> Arc<Content> {
    let Ok(dir) = std::env::var("UNDERSTORY_PACK") else {
        return Arc::new(Content::load_embedded().expect("the shipped pack should load"));
    };
    println!("  (reading the pack from {dir})\n");
    Arc::new(
        Content::load(&understory_core::content::DirSource::new(&dir)).unwrap_or_else(|errors| {
            for error in &errors {
                eprintln!("  {error}");
            }
            panic!("{dir} is not a loadable content pack");
        }),
    )
}

/// What the tower has for going up, besides the stairs it always has.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lift {
    /// The baseline. 30 ticks a floor, one body at a time, free.
    None,
    /// 8 ticks a floor, four seats, 5 charge a floor, and a wait for the
    /// car at each end — **put at the far edge of the tower.**
    Elevator,
}

impl Lift {
    fn label(self) -> &'static str {
        match self {
            Self::None => "stairs",
            Self::Elevator => "lift",
        }
    }

    /// Which column the shaft takes: the last one still free.
    ///
    /// **Found, not assumed.** The first version hardcoded the far edge,
    /// and a three-wide room placed at slot 5 spilled across it — which
    /// surfaced two layers later as "could not raise the dumbwaiter:
    /// slot 7 is already occupied".
    /// The outermost free column a *shaft* may have.
    ///
    /// **Not simply the last one.** M6 widened the floor to ten and
    /// made weapons `front_only` (`SYSTEMS.md` §6.13), so the last
    /// columns are where a cutter arm and a gun have to stand — and
    /// this harness's own ladder puts an arm on one of them. Taking the
    /// last free column then meant asking for a slot the harness had
    /// just filled: "could not raise shaft.dumbwaiter 0-2: floor 1 slot
    /// 8 is already occupied".
    ///
    /// **Derived from the tower's width, not from 8.** `WidenTower`
    /// slides everything forward (§6.16), so the weapons' edge moves
    /// with the hull and a hardcoded 8 points into the middle of a
    /// fourteen-wide tower. Worse, it then disagrees with the column
    /// `harness::place_anywhere` reserves — which is `slots - 3` — so
    /// the ladder fills the column this harness was about to raise a
    /// shaft in. That surfaced as "could not raise shaft.elevator 0-7:
    /// floor 7 slot 3 is already occupied", three layers from the
    /// mismatch that caused it.
    ///
    /// So: the same column the harness reserves, when it is free, and
    /// the outermost free one behind the weapons otherwise.
    fn slot(free: &[u8], slots: u8) -> u8 {
        let reserved = slots.saturating_sub(3);
        let weapon_edge = slots.saturating_sub(2);
        free.iter()
            .find(|slot| **slot == reserved)
            .or_else(|| free.iter().rfind(|slot| **slot < weapon_edge))
            .or_else(|| free.last())
            .copied()
            .expect("no free column for a shaft")
    }
}

/// Whole days, both of them. A window that is not a whole number of
/// days measures what time it started at — the crew sleep through the
/// night band and a tower whose crew are in bed does not queue on its
/// staircase. `throughput.rs` was fooled by exactly this once.
const DAY: u32 = 14_400;
const WARMUP: u32 = DAY;
const WINDOW: u32 = DAY * 2;

/// Three seeds, because one run of a stochastic world is an anecdote,
/// and the same three for both towers, because the comparison is only
/// meaningful if the ground underfoot was identical.
const SEEDS: [u64; 3] = [0x00C0_FFEE, 0x0BAD_F00D, 0x00DE_FACE];

/// The heights to try. Five is where a run starts; the top of the range
/// is past anything the capture stills have ever shown a tower reach,
/// which is itself part of the finding.
const HEIGHTS: [u8; 4] = [5, 8, 11, 14];

/// The hull widths to try, in slots. Ten is what the tower sets out
/// with; sixteen is `max_slots`, or three widenings.
///
/// **Held at one height on purpose.** Width and height are the same
/// question asked twice — how far is it from where the thing is made to
/// where it is wanted — and sweeping both at once produces a table
/// nobody can read. Eight floors is where this instrument's own sweep
/// says the stairs hurt and a shaft is clearly worth having, so it is
/// the height at which "does width undo the shaft" can have an answer
/// other than "there was nothing to undo".
const WIDTHS: [u8; 4] = [10, 12, 14, 16];

/// The height the width sweep runs at. See `WIDTHS`.
const WIDTH_SWEEP_HEIGHT: u8 = 8;

/// Crew counts for the car sweep: the tower's starting three, a middling
/// five, and `crew_cap`.
const CREWS: [usize; 3] = [3, 5, 8];

/// Cars for the car sweep, up to the lift's `max_cars`.
const CARS: [u8; 3] = [1, 2, 3];

/// Ticks the affordability run will wait before giving up: twenty
/// in-game days, comfortably past a whole session.
const PATIENCE: u32 = DAY * 20;

/// How tall the affordability run lets its tower get.
///
/// **Capped, because an uncapped one answers the wrong question.** Left
/// to grow whenever nothing else could be placed, it reached ten floors
/// with **zero poles and forty-three rope on the shelves** and reported
/// the elevator as never affordable — which is a fact about a tower that
/// spends every pole on floors, not about a price. Eight is where
/// `lift.rs`'s own sweep says the stairs hurt badly and a lift is clearly
/// worth having.
const GROW_TO: usize = 8;

#[derive(Default, Clone, Copy)]
struct Sample {
    hauls: u64,
    crafts: u64,
    harvested: u64,
    meals: u64,
    /// Crew-ticks spent standing at a shaft waiting for a way up.
    boarding: u64,
    /// Crew-ticks spent on the stairs under their own power.
    climbing: u64,
    /// Crew-ticks spent aboard a car, which is not free either.
    riding: u64,
    /// Crew-ticks spent walking along a floor.
    walking: u64,
    /// Crew-ticks spent doing none of the above: working, or idle for
    /// want of anything to do. The number the other four are stolen
    /// from.
    ashore: u64,
    /// Ticks on which the tower could not power something it wanted to.
    brownout: u32,
    /// Seeds in this row where the shaft was built and never once used.
    /// A row with any of these is not a comparison.
    dead: u32,
}

impl Sample {
    /// Every crew-tick that went into moving rather than into working.
    /// The elevator's whole promise is that this number falls.
    fn in_transit(self) -> u64 {
        self.boarding + self.climbing + self.riding + self.walking
    }

    fn mean(of: &[Sample]) -> Self {
        let n = of.len() as u64;
        let sum = of.iter().fold(Self::default(), |a, b| Self {
            hauls: a.hauls + b.hauls,
            crafts: a.crafts + b.crafts,
            harvested: a.harvested + b.harvested,
            meals: a.meals + b.meals,
            boarding: a.boarding + b.boarding,
            climbing: a.climbing + b.climbing,
            riding: a.riding + b.riding,
            walking: a.walking + b.walking,
            ashore: a.ashore + b.ashore,
            brownout: a.brownout + b.brownout,
            dead: a.dead + b.dead,
        });
        Self {
            hauls: sum.hauls / n,
            crafts: sum.crafts / n,
            harvested: sum.harvested / n,
            meals: sum.meals / n,
            boarding: sum.boarding / n,
            climbing: sum.climbing / n,
            riding: sum.riding / n,
            walking: sum.walking / n,
            ashore: sum.ashore / n,
            brownout: sum.brownout / u32::try_from(of.len()).unwrap_or(1),
            // Summed, not averaged: one dead seed spoils the row.
            dead: sum.dead,
        }
    }
}

fn main() {
    // **A hardcoded day goes stale silently.** `journey.rs` reported a
    // 57,372-tick run as "3 days" when the pack said 7,200 and the
    // answer was eight, because it kept its own copy of the day length.
    // Every instrument here windows on whole days, so a stale copy makes
    // the window a measurement of what time it started at — the trap
    // `throughput.rs` documents at the top of itself. Fail loudly rather
    // than quietly measure a different game.
    assert_eq!(
        DAY,
        understory_core::content::Content::load_embedded()
            .expect("the shipped pack should load")
            .balance
            .clock
            .ticks_per_day,
        "the pack's day length has moved; update this file's day constant"
    );
    println!("=== does an elevator ever pay? ===\n");
    let pack = content();
    println!(
        "  {} seeds a row, {WINDOW} ticks ({} whole days) after a {WARMUP}-tick warm-up.\n\
         Both towers are handed the elevator's price; only one spends it. Crew-ticks are\n\
         summed across everybody aboard, so they scale with the roster and only the\n\
         stairs/lift comparison within a row is meaningful.\n",
        SEEDS.len(),
        WINDOW / DAY,
    );

    println!(
        "{:<7} {:>6} {:>16} {:>16} {:>10} {:>10}",
        "floors", "shaft", "hauls", "crafts", "transit", "boarding"
    );

    let mut verdicts = Vec::new();
    for height in HEIGHTS {
        let stairs = Sample::mean(&SEEDS.map(|seed| measure(&pack, seed, height, Lift::None)));
        row(height, Lift::None, stairs, None);
        // **One built shaft, since §6.18.** This was a loop over two
        // and the shape is kept — `best` still means "the best shaft
        // for this height", and a pack that adds a second one gets its
        // comparison back for free.
        let mut best = (Lift::None, stairs);
        let s = Sample::mean(&SEEDS.map(|seed| measure(&pack, seed, height, Lift::Elevator)));
        row(height, Lift::Elevator, s, Some(stairs));
        if s.hauls > best.1.hauls && s.dead == 0 {
            best = (Lift::Elevator, s);
        }
        verdicts.push((height, stairs, best));
        println!();
    }

    verdict(&verdicts);
    does_width_undo_the_shaft(&pack);
    does_a_second_car_pay(&pack);
    when_can_you_afford_one(&pack);
}

/// **Does a wider hull undo the shaft?** (`SYSTEMS.md` §6.9 question 2.)
///
/// M6 spent a milestone establishing that a shaft has to earn its column
/// — `climb_ticks_per_floor` 30 → 45 is the change that made one worth
/// building at all — and then M6 turned round and let the hull grow
/// sideways. A wider floor is more room per storey, which is *less*
/// reason to climb. If widening is cheaper relief than a lift, the
/// milestone's whole argument has a side door in it.
///
/// One height, four widths, the same three seeds. What to read:
///
/// - **`hauls` across a row of stairs-only towers.** If a wider tower
///   hauls more with no shaft at all, width is relief and the side door
///   is open.
/// - **the lift's delta at each width.** If it shrinks as the hull
///   widens, a wide tower does not want a lift and the ladder the pack
///   describes stops existing at the top end.
/// - **`walking` against `climbing`.** This is the one that decides what
///   the fix is. Width trades vertical distance for horizontal: the same
///   chain on fewer floors, but every floor longer to cross. If walking
///   rises as fast as climbing falls, widening is not relief at all —
///   it is the same journey rotated, and the price is the only thing
///   that needs to be right.
fn does_width_undo_the_shaft(pack: &Arc<Content>) {
    println!("\n=== does a wider hull undo the shaft? ===\n");
    let shipped = pack.balance.tower.floor_slots;
    println!(
        "  {WIDTH_SWEEP_HEIGHT} floors throughout, {} seeds a row. Width {shipped} is what a\n         tower sets out with; {} is `max_slots`. `walk` and `climb` are crew-ticks — the\n         two halves width trades against each other.\n",
        SEEDS.len(),
        pack.balance.tower.max_slots,
    );
    println!(
        "{:<7} {:>6} {:>16} {:>10} {:>10} {:>10}",
        "slots", "shaft", "hauls", "walk", "climb", "boarding"
    );

    let mut narrow_gain = None;
    let mut wide_gain = None;
    let mut stairs_by_width = Vec::new();
    for width in WIDTHS {
        if width > pack.balance.tower.max_slots.max(shipped) {
            println!("  ({width} is past this pack's max_slots; skipped)");
            continue;
        }
        let stairs = Sample::mean(
            &SEEDS.map(|seed| measure_at(pack, seed, WIDTH_SWEEP_HEIGHT, Lift::None, width)),
        );
        width_row(width, Lift::None, stairs, None);
        stairs_by_width.push((width, stairs));
        let s = Sample::mean(
            &SEEDS.map(|seed| measure_at(pack, seed, WIDTH_SWEEP_HEIGHT, Lift::Elevator, width)),
        );
        width_row(width, Lift::Elevator, s, Some(stairs));
        if s.dead == 0 && stairs.hauls > 0 {
            let gain = (s.hauls as f64 - stairs.hauls as f64) * 100.0 / stairs.hauls as f64;
            if width == WIDTHS[0] {
                narrow_gain = Some(gain);
            } else {
                wide_gain = Some(gain);
            }
        }
        println!();
    }

    // The stairs-only column is the one that answers the question asked.
    if let (Some(&(_, narrow)), Some(&(_, wide))) =
        (stairs_by_width.first(), stairs_by_width.last())
        && narrow.hauls > 0
    {
        let relief = (wide.hauls as f64 - narrow.hauls as f64) * 100.0 / narrow.hauls as f64;
        println!(
            "  With no shaft at all, the widest hull hauls {relief:+.0}% against the narrowest.\n             Crew-ticks walking {:+.0}%, climbing {:+.0}%.",
            pct(narrow.walking, wide.walking),
            pct(narrow.climbing, wide.climbing),
        );
        if relief > 15.0 {
            println!(
                "  **Width is relief.** A tower can buy its way out of the climb by growing\n                 sideways, which is the side door in M6's argument — reprice it, or make it\n                 buy less than a whole hull at a time."
            );
        } else {
            println!(
                "  **Width is not relief.** Growing sideways does not answer the climb, so a\n                 shaft still has to be bought for the reason M6 said it did."
            );
        }
    }
    if let (Some(narrow), Some(wide)) = (narrow_gain, wide_gain) {
        println!(
            "  The lift is worth {narrow:+.0}% on the narrowest hull and {wide:+.0}% on the\n             widest. If that has collapsed, a wide tower does not want one."
        );
    }
}

/// **Does a second car pay, and does a crowd make it necessary?**
///
/// The late-game question `AddCar` exists for (`SYSTEMS.md` §6.20). A
/// shaft is a *column* and a column is the scarcest thing a tower owns,
/// so the answer to a queue late on should be another car rather than
/// another shaft — but only if a car actually buys anything.
///
/// Capacity is applied per car (`service_stop` boards up to `capacity`
/// for each), so a second car is a second **carload**, not a faster one.
/// That means it should do nothing at all when nobody is queueing and
/// everything when the tower is crowded, which is exactly what the sweep
/// is checking: crew across, cars down.
///
/// Read the diagonal. If the three-crew row is flat and the eight-crew
/// row is not, cars are a late-game purchase and the design works. If
/// every row is flat, a car buys nothing and `AddCar` is a button. If
/// the three-crew row *rises*, cars are just good and there is no
/// decision in them.
fn does_a_second_car_pay(pack: &Arc<Content>) {
    println!("\n=== does a second car pay, and when? ===\n");
    println!(
        "  {WIDTH_SWEEP_HEIGHT} floors, {} seeds a cell. Hauls completed, and in brackets the\n         crew-ticks spent standing at the shaft — which is what a car is bought to reduce.\n         Every tower is handed every car's price; only some spend it.\n",
        SEEDS.len(),
    );
    print!("{:<6}", "crew");
    for cars in CARS {
        print!("{:>22}", format!("{cars} car(s)"));
    }
    println!();

    for crew in CREWS {
        print!("{crew:<6}");
        let mut first = None;
        for cars in CARS {
            let s = Sample::mean(&SEEDS.map(|seed| measure_cars(pack, seed, crew, cars)));
            let delta = match first {
                None => {
                    first = Some(s.hauls);
                    String::new()
                }
                Some(base) if base > 0 => format!(
                    " {:+.0}%",
                    (s.hauls as f64 - base as f64) * 100.0 / base as f64
                ),
                Some(_) => String::new(),
            };
            print!("{:>22}", format!("{}{delta} ({})", s.hauls, s.boarding));
        }
        println!();
    }
    println!(
        "\n  A car is a *turn*, not speed. Flat at three crew and rising at eight is the\n         shape `AddCar` is for; flat everywhere means it buys nothing."
    );
    println!(
        "
  **What it says is not that shape.** A second car does exactly what a car
         is for -- queueing falls 60-75% at every crew count -- but it pays *less* at
         eight crew (+7%) than at three (+11%). The reason is the first column: three
         crew to eight buys +4% hauls. **This tower is not crew-bound.** Read the
         plateau rather than the deltas: hauls stop at ~225 whatever is thrown at the
         transport, so the binding constraint sits downstream of the shaft and more
         bodies cannot make a car more necessary. Growing the *tower* alongside the
         crew is the sweep this wants next; it holds the room plan fixed."
    );
}

/// Percentage change from `from` to `to`, guarding a zero baseline.
fn pct(from: u64, to: u64) -> f64 {
    if from == 0 {
        return 0.0;
    }
    (to as f64 - from as f64) * 100.0 / from as f64
}

/// A row of the width sweep. Shaped like `row` but printing the two
/// crew-tick columns the question turns on rather than the totals.
fn width_row(width: u8, kind: Lift, s: Sample, against: Option<Sample>) {
    let delta = |now: u64, was: u64| -> String {
        if was == 0 {
            return String::new();
        }
        format!(" ({:+.0}%)", (now as f64 - was as f64) * 100.0 / was as f64)
    };
    let hauls = match against {
        None => format!("{}", s.hauls),
        Some(base) => format!("{}{}", s.hauls, delta(s.hauls, base.hauls)),
    };
    let dead = if s.dead > 0 {
        format!("  <- built and never used on {} seed(s)", s.dead)
    } else {
        String::new()
    };
    println!(
        "{width:<7} {:>6} {hauls:>16} {:>10} {:>10} {:>10}{dead}",
        kind.label(),
        s.walking,
        s.climbing,
        s.boarding,
    );
}

/// Is the cure available before the pain?
///
/// **The question re-pricing actually turns on, and it is not about
/// poles.** Every built shaft in the pack costs rope — a dumbwaiter 3, an
/// elevator 6 — and rope needs a fiber comb feeding a ropery. Until that
/// chain runs, a tower's only shaft is the chute, which goes one way,
/// downward, and carries nobody. So the elevator's 18 poles is not the
/// gate; the 6 rope is, and the same material gates the cheaper rung
/// too.
///
/// This plays a tower that buys the chain in a sensible order and never
/// builds a shaft, and reports two things against each other: **the
/// first tick it could have paid for each shaft**, and **the tick its
/// crew started queueing in earnest**.
///
/// It never builds what it can afford, deliberately: a tower that bought
/// a dumbwaiter would stop queueing, and the second column would then be
/// measuring the fix rather than the need.
///
/// # Trust the second column, not the first
///
/// **`queueing` is solid.** It has come out at 19,000-21,000 ticks —
/// about twelve minutes at 1x — through every version of this harness,
/// including the broken ones. A tower's crew accumulate a full crew-day
/// of standing at a shaft inside the first fifth of a run.
///
/// **`affordable` is not, yet, and the honest thing is to say so.**
/// Three shopping policies have given three answers for the same
/// question — the dumbwaiter first affordable at 26, 51 and 91 thousand
/// ticks depending on whether the tower grew, whether it owned a chute,
/// and in what order it bought things. That is not a fact about a price;
/// it is a fact about a tower that spends every pole the moment it has
/// one, so it almost never *holds* a surplus, and "the first tick its
/// shelves held the price" is then a measurement of its spending habits.
/// Removing the dumbwaiter's rope entirely moved the number not at all,
/// which is the tell: rope was never what was binding.
///
/// What the column would need to mean something is a tower that stops
/// buying and starts saving — a policy this does not model — or a
/// cumulative measure of what it produced rather than what it held. Both
/// are real work, and until one exists these figures should not be used
/// to re-price anything.
///
/// What *is* safe to take from it: **the chute is affordable from tick
/// zero and both built shafts are behind the fiber chain**, because a
/// dumbwaiter needs 3 rope and an elevator 6, and rope needs a comb
/// feeding a ropery. Whatever the exact timing, a tower's first twelve
/// minutes of queueing have no answer in them but a chute, which goes
/// one way, downward, and carries nobody.
fn when_can_you_afford_one(pack: &Arc<Content>) {
    println!(
        "
=== is the cure available before the pain? ===
"
    );
    println!(
        "  A tower buying the chain in order and never building a shaft, {} seeds.
         `affordable` is the first tick its shelves held the price. `queueing` is the
         first tick crew had spent a whole crew-day standing at a shaft — pain that has
         accumulated rather than a bad moment.
",
        SEEDS.len(),
    );
    println!(
        "{:<14} {:>12} {:>12} {:>10}",
        "", "affordable", "in minutes", "at 1x"
    );

    let shafts = ["shaft.chute", "shaft.elevator"];
    let mut first = [u32::MAX; 2];
    let mut queueing = u32::MAX;
    for seed in SEEDS {
        let (afford, queued) = afford_run(pack, seed, &shafts);
        // **Zipped, not indexed.** `afford_run` returns a fixed-size
        // array and the caller decides how many shafts to ask about;
        // §6.18 took the list from three to two and this kept indexing
        // three, which is an out-of-bounds panic that only fires once
        // somebody runs the instrument.
        for (slot, tick) in first.iter_mut().zip(afford.iter()) {
            *slot = (*slot).min(*tick);
        }
        queueing = queueing.min(queued);
    }

    let say = |tick: u32| -> String {
        if tick == u32::MAX {
            "never".into()
        } else {
            format!("{tick}")
        }
    };
    let mins = |tick: u32| -> String {
        if tick == u32::MAX {
            "-".into()
        } else {
            format!("{:.0}m", f64::from(tick) / 30.0 / 60.0)
        }
    };
    for (i, id) in shafts.iter().enumerate() {
        println!(
            "{:<14} {:>12} {:>12}",
            id.trim_start_matches("shaft."),
            say(first[i]),
            mins(first[i]),
        );
    }
    println!(
        "{:<14} {:>12} {:>12}",
        "queueing",
        say(queueing),
        mins(queueing)
    );

    println!(
        "
  Read the gap. If a shaft becomes affordable before `queueing`, the ladder is
         paced and the price is not the problem; if after, a tower spends that gap with a
         bottleneck it can see and cannot answer, which is the one shape of difficulty
         this game should not have."
    );
}

/// One seed: the tick each shaft first became affordable, and the tick
/// crew had queued a whole crew-day between them.
fn afford_run(pack: &Arc<Content>, seed: u64, shafts: &[&str]) -> ([u32; 3], u32) {
    let mut game = GameEngine::with_content(seed, Arc::clone(pack));
    let content = pack.clone();
    // The order a player would: harvest, then somewhere to put it, then
    // the two rooms that make a tower liveable, then the fiber chain
    // that rope comes from. Nothing here is a shaft.
    // **Least constrained first, because the search takes the *last*
    // placeable item.** A fiber comb reaches the ground, so `max_floor`
    // is 1 and it has exactly two floors it can ever stand on — and the
    // first version of this list put storerooms ahead of it, watched
    // them take the low slots, and then reported that rope was never
    // affordable on a tower that had simply never been able to put its
    // fiber comb anywhere. The same ordering mistake `measure` above
    // records making with the cutter arm.
    // **The opening ladder, before the shopping list** (`SYSTEMS.md`
    // §6.11). Every room in the list below is gated behind a farm, a
    // cutter arm and a burner, and a list that cannot buy its first
    // item measures nothing: this ran three seeds and reported
    // "heartseed bunk bunk, 3 poles" on every one of them.
    //
    // Handed over rather than earned, because the question here is when
    // a *shaft* becomes affordable and the ladder is a fixed cost every
    // tower pays before that question starts.
    understory_core::harness::open_the_ladder(&mut game, 4);

    let mut list = vec![
        "room.storeroom",
        "room.burner",
        "room.canteen",
        "room.bunk",
        "room.mill",
        "room.storeroom",
        "room.ropery",
        // **And a chute, or the tower jams and the answer is a jam.**
        // Without one it ends the run holding 43 rope, no bamboo
        // anywhere, a mill with nothing to mill and *zero poles* — so
        // it can never save the elevator's 18, and the instrument
        // reports a price problem where there is a shelf-typing problem.
        // The chute is the one shaft it can afford from tick zero, which
        // is exactly what `shafts/chute.ron` argues it is for.
        "shaft.chute",
        // Floor-capped, so highest priority: tried first.
        "room.fiber_comb",
    ];
    let costs: Vec<Vec<(understory_core::ids::ItemIdx, i64)>> = shafts
        .iter()
        .map(|id| {
            content
                .shaft_rt(content.shaft_idx(id).expect("the pack has this shaft"))
                .build_cost
                .clone()
        })
        .collect();

    let mut first = [u32::MAX; 3];
    let mut queued_crew_ticks = 0u64;
    let mut queueing = u32::MAX;
    let slots = content.balance.tower.floor_slots;
    for tick in 0..PATIENCE {
        walk(&mut game, 1);

        // Buy what it can, latest first — a strict queue stalls on the
        // first thing it cannot pay for, and then nothing behind that is
        // "built later", it is not built at all.
        if tick.is_multiple_of(60) {
            let floors = game.state().tower.floors.len() as u8;
            if let Some(at) = (0..list.len())
                .rev()
                .find(|&at| buy(&mut game, list[at], floors, slots))
            {
                list.remove(at);
            } else if game.state().tower.floors.len() < GROW_TO
                && game.try_send(GameCommand::BuildFloor).is_ok()
            {
                // **Nothing could be placed, so make somewhere.** Gating
                // growth on an empty shopping list was wrong twice over
                // in this file: a room that cannot be placed never
                // leaves the list, so the list is never empty, so the
                // tower never grows — and it then sits on 240 poles at
                // four floors with a fiber comb it has nowhere to put.
                // Which reported as "rope is never affordable", a fact
                // about the harness wearing the clothes of a fact about
                // the economy.
                //
                // A burner rather than the sail deck this used to
                // re-buy: since M6 cut the sails, a new floor is a new
                // floor to light rather than a roof to re-cover, and
                // what a growing tower needs is more fuel-burning
                // capacity.
                list.push("room.burner");
            }
        }

        for (i, cost) in costs.iter().enumerate() {
            if first[i] == u32::MAX && affordable(&game, cost) {
                first[i] = tick;
            }
        }

        queued_crew_ticks += game
            .state()
            .crew
            .iter()
            .filter(|member| matches!(member.state, CrewState::Boarding { .. }))
            .count() as u64;
        if queueing == u32::MAX && queued_crew_ticks >= u64::from(DAY) {
            queueing = tick;
        }
        if first.iter().all(|t| *t != u32::MAX) && queueing != u32::MAX {
            break;
        }
    }

    // **Say what the tower actually managed**, because "never
    // affordable" is exactly the shape of answer that is usually a
    // harness that failed to build the chain rather than a game that
    // cannot pay for a shaft. `AGENTS.md` §II rule 1.
    let state = game.state();
    let rooms: Vec<&str> = state
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .map(|room| content.room(room.def).id.trim_start_matches("room."))
        .collect();
    let held = |id: &str| -> i64 {
        let Some(item) = content.item_idx(id) else {
            return -1;
        };
        state
            .tower
            .floors
            .iter()
            .flat_map(|floor| floor.rooms.iter())
            .flat_map(|room| room.shelves.iter().map(move |s| (s, room)))
            .filter(|(shelf, _)| shelf.item == Some(item))
            .map(|(shelf, _)| shelf.count)
            .sum::<i64>()
    };
    println!(
        "  seed {seed:#x}: {} floors, unbought [{}], shelves: {} poles, {} fiber, {} rope
             built: {}",
        state.tower.floors.len(),
        list.join(" "),
        held("item.poles"),
        held("item.fiber"),
        held("item.rope"),
        rooms.join(" "),
    );
    (first, queueing)
}

/// Do the tower's shelves hold this price right now?
fn affordable(game: &GameEngine, cost: &[(understory_core::ids::ItemIdx, i64)]) -> bool {
    cost.iter().all(|(item, amount)| {
        let held: i64 = game
            .state()
            .tower
            .floors
            .iter()
            .flat_map(|floor| floor.rooms.iter())
            .flat_map(|room| room.shelves.iter())
            .filter(|shelf| shelf.item == Some(*item))
            .map(|shelf| shelf.count)
            .sum();
        held >= *amount
    })
}

fn row(height: u8, kind: Lift, s: Sample, against: Option<Sample>) {
    let label = kind.label();
    let delta = |now: u64, then: Option<u64>| -> String {
        match then {
            None => String::new(),
            Some(before) => {
                let d = now as i64 - before as i64;
                let pct = if before == 0 {
                    0
                } else {
                    d * 100 / before as i64
                };
                format!("  {}{pct}%", if d >= 0 { "+" } else { "" })
            }
        }
    };
    println!(
        "{height:<7} {label:>6} {:>10}{:<6} {:>10}{:<6} {:>10} {:>10}",
        s.hauls,
        delta(s.hauls, against.map(|a| a.hauls)),
        s.crafts,
        delta(s.crafts, against.map(|a| a.crafts)),
        s.in_transit(),
        s.boarding,
    );
    if s.dead > 0 {
        println!(
            "        ^ unusable: on {} of {} seeds the shaft was built and never once used",
            s.dead,
            SEEDS.len(),
        );
    }
}

fn verdict(rows: &[(u8, Sample, (Lift, Sample))]) {
    println!("where the crew-ticks went, best shaft against stairs:\n");
    println!(
        "{:<7} {:>10} {:>10} {:>10} {:>10} {:>12}",
        "floors", "boarding", "climbing", "riding", "walking", "transit"
    );
    for (height, stairs, (_, lift)) in rows {
        let d = |a: u64, b: u64| -> i64 { b as i64 - a as i64 };
        println!(
            "{height:<7} {:>+10} {:>+10} {:>+10} {:>+10} {:>+12}",
            d(stairs.boarding, lift.boarding),
            d(stairs.climbing, lift.climbing),
            d(stairs.riding, lift.riding),
            d(stairs.walking, lift.walking),
            d(stairs.in_transit(), lift.in_transit()),
        );
    }

    println!("\nwhat a tower of each height should build:\n");
    for (height, stairs, (best, sample)) in rows {
        let gain = sample.hauls as i64 - stairs.hauls as i64;
        let pct = if stairs.hauls == 0 {
            0
        } else {
            gain * 100 / stairs.hauls as i64
        };
        match best {
            Lift::None => {
                println!("  {height:>2} floors — nothing. Neither shaft beats the stairs here.");
            }
            other => println!(
                "  {height:>2} floors — the {} ({gain:+} hauls, {pct:+}%)",
                other.label(),
            ),
        }
    }

    // **The second finding, and the larger one.** Read down the stairs
    // column: what a tower loses by growing without solving transport.
    // The shaft comparison is the question this instrument was written
    // to answer; this is the one it turned up on the way.
    println!("\nwhat height costs a tower with only stairs:\n");
    let (base_height, base_hauls) = rows
        .first()
        .map_or((0, 0), |(height, stairs, _)| (*height, stairs.hauls));
    for (height, stairs, _) in rows {
        let pct = if base_hauls == 0 {
            0
        } else {
            (stairs.hauls as i64 - base_hauls as i64) * 100 / base_hauls as i64
        };
        println!(
            "  {height:>2} floors — {:>4} hauls ({pct:+}% against {base_height} floors), \
             {:>6} crew-ticks queued at a shaft",
            stairs.hauls, stairs.boarding,
        );
    }

    println!(
        "\n  Read `transit` first. If it fell and the hauls fell too, the crew are making\n\
         fewer, longer trips rather than slower ones, and the shaft is working — a haul\n\
         count cannot tell those apart and they want opposite fixes."
    );
}

fn measure(pack: &Arc<Content>, seed: u64, height: u8, build_lift: Lift) -> Sample {
    measure_at(pack, seed, height, build_lift, 0)
}

/// As `measure`, with `crew` aboard and `cars` in the lift.
///
/// Crew are cloned from the last one aboard and dropped at the foot of
/// the tower, which is where a recruit arrives anyway. Cloning rather
/// than constructing keeps them fed, rested and on the day shift — a
/// harness that added starving crew would measure hunger.
fn measure_cars(pack: &Arc<Content>, seed: u64, crew: usize, cars: u8) -> Sample {
    measure_with(
        pack,
        seed,
        WIDTH_SWEEP_HEIGHT,
        Lift::Elevator,
        0,
        crew,
        cars,
    )
}

/// As `measure`, with the hull widened to `width` slots first.
///
/// **Widened before anything is placed**, which is not tidiness:
/// `WidenTower` slides every room and shaft forward to keep the leading
/// edge where it was (`SYSTEMS.md` §6.16), so widening a laid-out tower
/// would move the reserved columns out from under the plan. Zero means
/// "whatever the pack ships".
fn measure_at(pack: &Arc<Content>, seed: u64, height: u8, build_lift: Lift, width: u8) -> Sample {
    measure_with(pack, seed, height, build_lift, width, 0, 0)
}

/// The full sweep surface: height, width, crew and cars.
///
/// Zero means "as the pack ships it" for each of the last three, so the
/// older entry points read unchanged.
#[allow(clippy::too_many_arguments)]
fn measure_with(
    pack: &Arc<Content>,
    seed: u64,
    height: u8,
    build_lift: Lift,
    width: u8,
    crew: usize,
    cars: u8,
) -> Sample {
    let mut game = GameEngine::with_content(seed, Arc::clone(pack));
    grow(&mut game, height);
    widen_to(&mut game, width);
    crew_of(&mut game, crew);
    // Read *after* widening: the whole point is that this is no longer
    // the pack's constant.
    let slots = game
        .state()
        .tower
        .floors
        .first()
        .map_or(game.content().balance.tower.floor_slots, |floor| {
            floor.slots
        });
    // **Every row reserves every candidate column**, not just the one it
    // uses. Reserving only the column this row needs would let the rows
    // differ by which rooms fitted as well as by the shaft, and then the
    // comparison is about floor plans.
    let free = free_columns(&game, slots);
    if std::env::var("UNDERSTORY_COLUMNS").is_ok() {
        println!("  {height} floors, {slots} slots: free columns {free:?}");
    }
    let reserved = [Lift::slot(&free, slots)];

    // A chain that crosses the whole tower, top to bottom, so a haul has
    // somewhere to go — an elevator in a tower whose chain sits on two
    // adjacent floors has nothing to do, and would measure as worthless
    // however it were tuned.
    //
    // **Burners, or the whole sweep measures a blackout.** This used
    // to be a note about sails: `canopy_sails` was `top_floor_only`,
    // so every floor this harness added shaded the ones the starting
    // tower came with, and the first version grew towers to fourteen
    // floors with no sail above them. Result: charge 0 of 2300,
    // brown-out on every one of 28,800 ticks, and **zero hauls and
    // zero crafts on both towers at every height** — a perfectly
    // symmetrical comparison of two towers that were doing nothing.
    //
    // M6 cut the sails, which removes the pinning problem and keeps
    // the lesson: a fourteen-floor tower lights fourteen floors, and
    // charge now comes only from burners somebody has to keep fuelled.
    // Three of them, indoors, spread out — one is not enough for the
    // tall end of the sweep, and a harness that browns out at fourteen
    // floors and not at five is measuring the power budget while
    // claiming to measure a shaft.
    // **Most-constrained first.** Reserving two candidate columns costs
    // a quarter of the tower's width, and in that order the cutter arm —
    // two slots wide and capped by the pack at floor 1, so it has
    // exactly two floors to land on — kept losing its place to a
    // storeroom that could have gone anywhere.
    // **The opening ladder first** (`SYSTEMS.md` §6.11): since M6 a
    // fresh tower has a Heartseed and a bed and nothing else, and every
    // room in the plan below is gated behind a farm, a cutter arm and a
    // burner. This puts one of each up — wherever they fit — and the
    // plan then builds the tower this instrument actually measures.
    understory_core::harness::open_the_ladder(&mut game, height);

    let plan = [
        // Reaches the ground, so `max_floor` is 1. Nowhere else to go.
        ("room.cutter_arm", 1),
        ("room.cutter_arm", 0),
        ("room.cutter_arm", 1),
        // `min_floor` 2, and without them the tower browns out.
        ("room.burner", 2),
        ("room.mill", height / 2),
        ("room.mill", height / 2 + 1),
        ("room.mill", height - 3),
        ("room.burner", height / 2),
        ("room.burner", height - 2),
        ("room.canteen", height - 2),
        ("room.bunk", height - 3),
        ("room.storeroom", 0),
        ("room.storeroom", height / 2),
        ("room.storeroom", height - 2),
    ];
    let mut got: Vec<(&str, u8)> = Vec::new();
    for (room, want) in plan {
        // Reserving two candidate columns costs the tower a quarter of
        // its width, and a five-floor plan stopped fitting its mill on
        // the floor it asked for. So the plan states a preference and
        // this settles for near it, the way a player would — searching
        // outward from `want` rather than giving up on the floor.
        // **Except a room the pack pins to one floor**, which used to
        // mean a sail: `top_floor_only` rooms placed anywhere else were
        // shaded and earned nothing, and letting the search settle one
        // floor down put an eight-floor tower into brown-out for three
        // quarters of the window — a measurement of the power budget
        // wearing a transport instrument's clothes.
        //
        // Nothing in the plan is pinned that way since M6. A burner's
        // `min_floor` is a floor it cannot go *below*, which `place`
        // rejects, so the outward search simply keeps going.
        let pinned = game
            .content()
            .room_idx(room)
            .is_some_and(|idx| game.content().room(idx).top_floor_only);
        let reach = if pinned { 1 } else { height };
        let landed = (0..reach).map(|d| [want.saturating_sub(d), (want + d).min(height - 1)]);
        for floor in landed.flatten() {
            if place(&mut game, room, floor, slots, &reserved) {
                got.push((room, floor));
                break;
            }
        }
    }
    if std::env::var("UNDERSTORY_COLUMNS").is_ok() {
        let arms = game
            .state()
            .tower
            .floors
            .iter()
            .flat_map(|floor| floor.rooms.iter())
            .filter(|room| game.content().room(room.def).id == "room.cutter_arm")
            .count();
        println!("  {height} floors: {arms} cutter arm(s); plan landed {got:?}");
    }
    // **Named checks, not a count.** A threshold on "how many of the
    // plan landed" says nothing about *which* — the first version of
    // this demanded seven of nine and a five-floor tower took six, so it
    // died without saying that what it was missing was the second sail,
    // which does not matter. What matters is that the tower can power
    // itself, harvest, craft, and has reason to move things between
    // distinct floors.
    // **Read off the tower, not off the plan's log.** The ladder
    // above already put a cutter arm and a burner somewhere, so a plan
    // entry that failed to find a second slot is not the same thing as
    // the tower lacking one — and what this assertion is about is
    // whether the tower can power itself, harvest and craft.
    let standing = |id: &str| {
        game.content().room_idx(id).is_some_and(|idx| {
            game.state()
                .tower
                .floors
                .iter()
                .flat_map(|floor| floor.rooms.iter())
                .any(|room| room.def == idx)
        })
    };
    let has = |id: &str| standing(id) || got.iter().any(|(room, _)| *room == id);
    // **Read off the tower, for the same reason `has` is.** `got` only
    // records what the plan placed; the ladder above put three rooms up
    // before the plan ran, and counting only the plan's floors reported
    // a five-floor tower as occupying three of them.
    let mut floors: Vec<u8> = game
        .state()
        .tower
        .floors
        .iter()
        .filter(|floor| !floor.rooms.is_empty())
        .map(|floor| floor.index)
        .collect();
    floors.sort_unstable();
    floors.dedup();
    assert!(
        has("room.burner") && has("room.cutter_arm") && has("room.mill") && floors.len() >= 4,
        "a {height}-floor tower is not a fair test of a shaft: burner {}, cutter arm {}, \
         mill {}, occupied floors {floors:?}",
        has("room.burner"),
        has("room.cutter_arm"),
        has("room.mill"),
    );

    // Every tower is handed the *elevator's* price, the dearest of the
    // three, so the three rows differ by the shaft and by nothing else —
    // including how much was left on the shelves for the crew to move.
    endow(&mut game, "shaft.elevator");
    match build_lift {
        Lift::None => {}
        Lift::Elevator => raise(
            &mut game,
            "shaft.elevator",
            0,
            height - 1,
            Lift::slot(&free, slots),
            height,
        ),
    }

    // **Extra cars, and every tower is handed the price whether it
    // spends it or not.** Stocking only the tower that buys is how the
    // first version of `a_second_car_moves_more_than_one_does` came out
    // at 50 hauls against 185: a shelf holds one kind and the tower has
    // eight of them, so 80 granted poles is an economy, not a control.
    if cars > 1 {
        let id = game
            .state()
            .tower
            .shafts
            .last()
            .expect("a lift was just raised")
            .id;
        for _ in 1..cars {
            let cost = {
                let content = game.content();
                content
                    .shaft_idx("shaft.elevator")
                    .map(|idx| content.shaft_rt(idx).car_cost.clone())
                    .unwrap_or_default()
            };
            give(&mut game, &cost);
            game.try_send(GameCommand::AddCar { shaft: id })
                .unwrap_or_else(|err| panic!("could not add a car: {err}"));
        }
        assert_eq!(
            game.state()
                .tower
                .shafts
                .last()
                .expect("still there")
                .cars
                .len(),
            cars as usize,
            "the harness did not end up with the cars it asked for"
        );
    }

    walk(&mut game, WARMUP);

    let before = game.state().stats.clone();
    let mut s = Sample::default();
    for _ in 0..WINDOW {
        walk(&mut game, 1);
        let state = game.state();
        for member in &state.crew {
            match member.state {
                CrewState::Boarding { .. } => s.boarding += 1,
                CrewState::Climbing { .. } => s.climbing += 1,
                CrewState::Riding { .. } => s.riding += 1,
                CrewState::Walking { .. } => s.walking += 1,
                _ => s.ashore += 1,
            }
        }
        if state.power.brownout {
            s.brownout += 1;
        }
    }

    let after = &game.state().stats;
    s.hauls = after.hauls_completed - before.hauls_completed;
    s.crafts = after.crafts_completed - before.crafts_completed;
    s.harvested = after.items_harvested - before.items_harvested;
    s.meals = after.meals_eaten - before.meals_eaten;

    // **Not an assert, deliberately.** `AGENTS.md` §II says to check the
    // run gave the mechanism something to do, and the first version of
    // this stopped the sweep dead on the shortest tower. But "nobody
    // ever rode it" is the answer to the question rather than a broken
    // harness — so it is reported per row and the sweep continues, and
    // the row is marked so it can never be read as a fair comparison.
    if build_lift == Lift::Elevator && s.riding == 0 {
        s.dead = 1;
        println!(
            "  ! {height} floors, seed {seed:#x}: the lift was built and nobody ever rode it \
             (charge {} of {}, brownout on {} ticks)",
            game.state().power.charge,
            game.state().power.capacity,
            s.brownout,
        );
    }
    // The setup check that actually matters, and the one whose absence
    // produced the blackout sweep above: a tower that cannot keep its
    // lamps on is not measuring transport.
    //
    // **Marked, not asserted.** Stopping the sweep dead on it throws
    // away every taller row as well, and the harness tower genuinely
    // cannot buy more sail than one roof holds — so past eight floors it
    // is one busy day from the dark whatever the shaft does. Reported
    // and excluded from the verdict, the same way a lift nobody rode is.
    if s.brownout > WINDOW / 4 {
        s.dead = 1;
        println!(
            "  ! {height} floors, seed {seed:#x}: browned out on {} of {WINDOW} ticks — \
             this row is the power budget, not the shaft",
            s.brownout,
        );
    }
    s
}

/// Put a shaft in, and fail loudly if it did not go in.
///
/// `AGENTS.md` §II rule 1: assert your setup. Both of `siege_run.rs`'s
/// false findings were a silent `false` from a place-a-room helper, and
/// a shaft that quietly failed to build here would produce two
/// byte-identical towers and a confident "it makes no difference".
fn raise(game: &mut GameEngine, shaft: &str, low: u8, high: u8, slot: u8, height: u8) {
    game.try_send(GameCommand::BuildShaft {
        shaft: shaft.into(),
        low,
        high,
        slot,
    })
    .unwrap_or_else(|err| panic!("{height} floors: could not raise {shaft} {low}-{high}: {err}"));
}

/// Add floors until the tower is `height` tall, paying for each.
///
/// `floor_cost` is the authored form — item *ids*, not interned indices
/// — because it lives on `balance.ron` rather than on a room. Resolved
/// here rather than in `give`, so `give` stays the one shape every
/// caller uses.
fn grow(game: &mut GameEngine, height: u8) {
    let cost: Vec<(understory_core::ids::ItemIdx, i64)> = game
        .content()
        .balance
        .tower
        .floor_cost
        .iter()
        .map(|entry| {
            (
                game.content()
                    .item_idx(&entry.item)
                    .unwrap_or_else(|| panic!("floor_cost names an unknown {}", entry.item)),
                entry.amount,
            )
        })
        .collect();
    while (game.state().tower.floors.len() as u8) < height {
        give(game, &cost);
        game.try_send(GameCommand::BuildFloor)
            .unwrap_or_else(|err| panic!("could not add a floor: {err}"));
    }
}

/// Put `crew` people aboard, cloning the last one so they arrive fed,
/// rested and on the day shift.
///
/// **Cloned rather than constructed**, because a harness that added
/// starving crew on the night rota would be measuring needs while
/// claiming to measure a shaft. Zero leaves the roster alone.
fn crew_of(game: &mut GameEngine, crew: usize) {
    if crew == 0 {
        return;
    }
    let content = game.content().clone();
    let state = game.state_mut_for_test();
    while state.crew.len() > crew {
        state.crew.pop();
    }
    while state.crew.len() < crew {
        // The game's own recruiter, which names them off the pack, draws
        // `fidget` from the *cosmetic* stream (`DECISIONS.md` §2) and
        // starts them rested. A harness that built its own would have to
        // get all three right and would quietly measure whatever it got
        // wrong.
        state.add_crew(&content);
    }
}

/// Widen the hull until it is at least `width` slots across.
///
/// Paid for out of thin air like every other cost in this harness: what
/// a widening *costs* is `journey.rs`'s question, and this one is about
/// what it does once it is there.
///
/// **The price is per floor, and getting that wrong made this lie.**
/// The first version handed over one floor's worth and the command
/// wanted the whole height's, so every widening after the first failed
/// for want of stock — and `try_send` returning an error was read as
/// "the pack caps here" and swallowed. The rows at twelve, fourteen and
/// sixteen slots came out **byte-identical**, which is this repo's
/// oldest tell and was missed for exactly as long as it took to read
/// the table. It panics rather than returns now.
fn widen_to(game: &mut GameEngine, width: u8) {
    if width == 0 {
        return;
    }
    let slots_of = |game: &GameEngine| {
        game.state()
            .tower
            .floors
            .first()
            .map_or(0, |floor| floor.slots)
    };
    while slots_of(game) < width {
        let floors = i64::try_from(game.state().tower.floors.len().max(1)).unwrap_or(1);
        let cost: Vec<(understory_core::ids::ItemIdx, i64)> = game
            .content()
            .balance
            .tower
            .widen_cost
            .iter()
            .filter_map(|entry| {
                game.content()
                    .item_idx(&entry.item)
                    .map(|item| (item, entry.amount * floors))
            })
            .collect();
        give(game, &cost);
        let before = slots_of(game);
        game.try_send(GameCommand::WidenTower)
            .unwrap_or_else(|err| panic!("could not widen to {width} slots: {err}"));
        assert!(
            slots_of(game) > before,
            "a widening that reported success added no slots"
        );
    }
}

/// Place a room on `floor`, in the first slot that will take it.
fn place(game: &mut GameEngine, room: &str, floor: u8, slots: u8, reserved: &[u8]) -> bool {
    let idx = game
        .content()
        .room_idx(room)
        .unwrap_or_else(|| panic!("the pack has no {room}"));
    let cost = game.content().room_rt(idx).build_cost.clone();
    let width = game.content().room(idx).width;
    give(game, &cost);
    // **A reserved column is a column, not a left edge.** Filtering the
    // starting slot alone let a three-wide room at slot 5 spill across
    // slot 7 and take the shaft's column with it — which showed up as
    // "could not raise shaft.dumbwaiter: floor 1 slot 7 is already
    // occupied", two setup layers away from the cause.
    let mut last_error = None;
    let landed = (0..slots)
        .filter(|slot| !reserved.iter().any(|r| (*slot..slot + width).contains(r)))
        .any(|slot| {
            match game.try_send(GameCommand::PlaceRoom {
                room: room.into(),
                floor,
                slot,
            }) {
                Ok(()) => true,
                Err(err) => {
                    // The *first*, not the last: the last is always
                    // "slots run past the floor width", which is the
                    // search reaching the end rather than the reason.
                    last_error.get_or_insert(err);
                    false
                }
            }
        });
    // **Says why it failed, when asked.** A plan entry that never lands
    // is silent, and the harness then measures a tower it did not
    // build: three cutter arms were added to the plan and none of them
    // appeared, twice, with the numbers coming out unchanged and
    // nothing to read.
    if !landed
        && std::env::var("UNDERSTORY_COLUMNS").is_ok()
        && let Some(err) = last_error
    {
        println!("    {room} floor {floor}: {err}");
    }
    landed
}

/// Hand the tower a shaft's whole price, so the two samples differ by
/// the shaft and by nothing else. See `throughput.rs` for why waiting
/// until it can afford one is the wrong repair — it starts the two
/// samples three in-game days apart, on different ground.
fn endow(game: &mut GameEngine, shaft: &str) {
    let cost = game
        .content()
        .shaft_rt(
            game.content()
                .shaft_idx(shaft)
                .unwrap_or_else(|| panic!("the pack has no {shaft}")),
        )
        .build_cost
        .clone();
    give(game, &cost);
}

/// Put a cost's worth of goods on the shelves.
///
/// **Clears a shelf if it has to, and that is not cheating.** A shelf
/// takes whichever item lands on it first and holds only that until it
/// empties (`shafts/chute.ron` is the whole design note), so handing a
/// tower four bamboo when all four of its shelves hold poles fails —
/// which is the real shelf-typing deadlock, and exactly the thing this
/// instrument is not asking about. Every row is set up the same way, so
/// the comparison is untouched; what would corrupt it is one row
/// starting with a stocked tower and another with a jammed one.
fn give(game: &mut GameEngine, cost: &[(understory_core::ids::ItemIdx, i64)]) {
    let state = game.state_mut_for_test();
    for &(item, amount) in cost {
        let mut left = amount;
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                left -= room.shelve(item, left);
                if left <= 0 {
                    break;
                }
            }
        }
        if left <= 0 {
            continue;
        }
        // Nothing would take it. Commandeer shelves holding something
        // else, biggest first, until the price fits.
        let mut shelves: Vec<&mut understory_core::state::tower::Shelf> = state
            .tower
            .floors
            .iter_mut()
            .flat_map(|floor| floor.rooms.iter_mut())
            .flat_map(|room| room.shelves.iter_mut())
            .filter(|shelf| shelf.item != Some(item))
            .collect();
        shelves.sort_by_key(|shelf| -shelf.max);
        for shelf in shelves {
            if left <= 0 {
                break;
            }
            shelf.item = Some(item);
            shelf.count = shelf.max.min(left);
            left -= shelf.count;
        }
        // Still short: the tower has less shelf than the price. Widen
        // one shelf rather than fail.
        //
        // **A harness-only measure, and the alternative was worse.** A
        // fourteen-floor tower is grown before its storerooms exist, so
        // early on the only shelves in it are the starting room's, and
        // handing it thirteen floors' worth of poles overflows them. The
        // choice is between this and re-ordering the setup so the tower
        // earns its way up, which would start each row at a different
        // point in the run — the exact mistake `throughput.rs` records
        // itself making. Every row gets the identical treatment, and
        // what is being compared is the shaft.
        if left > 0
            && let Some(shelf) = widest(state)
        {
            shelf.max += left;
            shelf.item = Some(item);
            shelf.count += left;
            left = 0;
        }
        assert!(left <= 0, "the tower has no shelves at all");
    }
}

/// Step, answering any fork before it can bring the tower to a halt —
/// nobody is playing, and a tower standing at an unanswered fork
/// measures as a tower with nothing to haul.
fn walk(game: &mut GameEngine, ticks: u32) {
    let mut left = ticks;
    while left > 0 {
        if game
            .state()
            .world
            .fork
            .is_some_and(|fork| fork.answer.is_none())
        {
            let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
        }
        let _ = game.try_send(GameCommand::SetStriding { walking: true });
        let chunk = left.min(300);
        game.step(chunk);
        left -= chunk;
    }
}

/// The biggest shelf in the tower, for `give`'s last resort.
fn widest(
    state: &mut understory_core::state::GameState,
) -> Option<&mut understory_core::state::tower::Shelf> {
    state
        .tower
        .floors
        .iter_mut()
        .flat_map(|floor| floor.rooms.iter_mut())
        .flat_map(|room| room.shelves.iter_mut())
        .max_by_key(|shelf| shelf.max)
}

/// Columns free on every floor of the tower as it now stands, lowest
/// first.
///
/// **Starts at 0, and finds the staircase rather than assuming it.** It
/// used to start at 1 on the grounds that slot 0 is the built-in
/// staircase, which stopped being true the moment a hull could widen:
/// widening slides every shaft forward too, so on a fourteen-wide tower
/// the stairs are at column 4 and columns 0-3 are the new back deck.
fn free_columns(game: &GameEngine, slots: u8) -> Vec<u8> {
    (0..slots)
        .filter(|slot| {
            let taken_by_shaft = game
                .state()
                .tower
                .shafts
                .iter()
                .any(|shaft| shaft.slot == *slot);
            let taken_by_room = game.state().tower.floors.iter().any(|floor| {
                floor
                    .rooms
                    .iter()
                    .any(|room| (room.slot..room.slot + room.width).contains(slot))
            });
            !taken_by_shaft && !taken_by_room
        })
        .collect()
}

/// Place a room, or raise a shaft, wherever it will go.
fn buy(game: &mut GameEngine, id: &str, floors: u8, slots: u8) -> bool {
    if let Some(shaft) = id.strip_prefix("shaft.") {
        let _ = shaft;
        return (0..slots).any(|slot| {
            game.try_send(GameCommand::BuildShaft {
                shaft: id.into(),
                low: 0,
                high: floors - 1,
                slot,
            })
            .is_ok()
        });
    }
    (0..floors).any(|floor| {
        (0..slots).any(|slot| {
            game.try_send(GameCommand::PlaceRoom {
                room: id.into(),
                floor,
                slot,
            })
            .is_ok()
        })
    })
}
