//! Does an elevator ever pay, and at what height?
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
    /// Item-only, no wait, `batch` 4, spans two or three floors. The
    /// pack calls it "the inserter", and the ladder the design intends
    /// is dumbwaiter first, elevator when the tower is tall.
    Dumb,
    /// 8 ticks a floor, four seats, 5 charge a floor, and a wait for the
    /// car at each end — **put at the far edge of the tower.**
    Elevator,
}

impl Lift {
    fn label(self) -> &'static str {
        match self {
            Self::None => "stairs",
            Self::Dumb => "dumb",
            Self::Elevator => "lift",
        }
    }

    /// Which column the shaft takes: the last one still free.
    ///
    /// **Found, not assumed.** The first version hardcoded the far edge,
    /// and a three-wide room placed at slot 5 spilled across it — which
    /// surfaced two layers later as "could not raise the dumbwaiter:
    /// slot 7 is already occupied".
    fn slot(free: &[u8]) -> u8 {
        *free.last().expect("no free column for a shaft")
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
        let mut best = (Lift::None, stairs);
        for kind in [Lift::Dumb, Lift::Elevator] {
            let s = Sample::mean(&SEEDS.map(|seed| measure(&pack, seed, height, kind)));
            row(height, kind, s, Some(stairs));
            if s.hauls > best.1.hauls && s.dead == 0 {
                best = (kind, s);
            }
        }
        verdicts.push((height, stairs, best));
        println!();
    }

    verdict(&verdicts);
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
    let mut game = GameEngine::with_content(seed, Arc::clone(pack));
    let slots = game.content().balance.tower.floor_slots;
    grow(&mut game, height);
    // **Every row reserves every candidate column**, not just the one it
    // uses. Reserving only the column this row needs would let the rows
    // differ by which rooms fitted as well as by the shaft, and then the
    // comparison is about floor plans.
    let free = free_columns(&game, slots);
    let reserved = [Lift::slot(&free)];

    // A chain that crosses the whole tower, top to bottom, so a haul has
    // somewhere to go — an elevator in a tower whose chain sits on two
    // adjacent floors has nothing to do, and would measure as worthless
    // however it were tuned.
    //
    // **Sails first, and on the roof, or the whole sweep measures a
    // blackout.** `canopy_sails` is `top_floor_only`, so every floor
    // this harness adds shades the ones the starting tower came with —
    // and the first version of this grew towers to fourteen floors with
    // no sail above them at all. Result: charge 0 of 2300, brown-out on
    // every one of 28,800 ticks, and **zero hauls and zero crafts on
    // both towers at every height**. A perfectly symmetrical comparison
    // of two towers that were doing nothing.
    // **Most-constrained first.** Reserving two candidate columns costs
    // a quarter of the tower's width, and in that order the cutter arm —
    // two slots wide and capped by the pack at floor 1, so it has
    // exactly two floors to land on — kept losing its place to a
    // storeroom that could have gone anywhere.
    let plan = [
        // Reaches the ground, so `max_floor` is 1. Nowhere else to go.
        ("room.cutter_arm", 1),
        // `top_floor_only`, and without it the tower browns out.
        ("room.canopy_sails", height - 1),
        ("room.mill", height / 2),
        ("room.canopy_sails", height - 1),
        ("room.canopy_sails", height - 1),
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
        // **Except a sail, which has exactly one floor it works on.**
        // `top_floor_only` rooms placed anywhere else are shaded and
        // earn nothing, and letting the search settle one floor down
        // put an eight-floor tower into brown-out for three quarters of
        // the window — a measurement of the power budget wearing a
        // transport instrument's clothes.
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
    // **Named checks, not a count.** A threshold on "how many of the
    // plan landed" says nothing about *which* — the first version of
    // this demanded seven of nine and a five-floor tower took six, so it
    // died without saying that what it was missing was the second sail,
    // which does not matter. What matters is that the tower can power
    // itself, harvest, craft, and has reason to move things between
    // distinct floors.
    let has = |id: &str| got.iter().any(|(room, _)| *room == id);
    let mut floors: Vec<u8> = got.iter().map(|(_, floor)| *floor).collect();
    floors.dedup();
    floors.sort_unstable();
    floors.dedup();
    assert!(
        has("room.canopy_sails") && has("room.cutter_arm") && has("room.mill") && floors.len() >= 4,
        "a {height}-floor tower is not a fair test of a shaft: sails {}, cutter arm {}, \
         mill {}, occupied floors {floors:?}",
        has("room.canopy_sails"),
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
            Lift::slot(&free),
            height,
        ),
        // **A dumbwaiter cannot span a tall tower** — `max_span` 3 — so
        // it goes where the traffic is heaviest: from the cutter arm's
        // floor up toward the mill. On an eleven-floor tower it
        // therefore relieves the bottom three floors and nothing else,
        // which is the answer rather than a limitation of the harness.
        Lift::Dumb => {
            let span = game
                .content()
                .shaft(
                    game.content()
                        .shaft_idx("shaft.dumbwaiter")
                        .expect("the pack has no dumbwaiter"),
                )
                .max_span;
            // `span` counts floors, `high` is an index, so the top of a
            // three-floor dumbwaiter starting at 0 is floor 2.
            let high = (height - 1).min(span - 1);
            raise(
                &mut game,
                "shaft.dumbwaiter",
                0,
                high,
                Lift::slot(&free),
                height,
            );
        }
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
    assert!(
        s.brownout < WINDOW / 4,
        "{height} floors, seed {seed:#x}: browned out on {} of {WINDOW} ticks — this row \
         would be a measurement of the power budget, not of the shaft",
        s.brownout,
    );
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
    (0..slots)
        .filter(|slot| !reserved.iter().any(|r| (*slot..slot + width).contains(r)))
        .any(|slot| {
            game.try_send(GameCommand::PlaceRoom {
                room: room.into(),
                floor,
                slot,
            })
            .is_ok()
        })
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
/// first. Slot 0 is the built-in staircase and never appears.
fn free_columns(game: &GameEngine, slots: u8) -> Vec<u8> {
    (1..slots)
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
