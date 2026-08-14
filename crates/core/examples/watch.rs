//! Measures whether the automatic defence loadouts create different
//! outcomes. The retired per-enemy Focus control is deliberately absent:
//! this file now asks about the visible build decision that replaced it.
//!
//! Generalists and specialists are different tower shapes, so their raw
//! hit-point totals are not a ladder. Read each row as a validity and
//! pressure check: the weapons must fire, creatures must be repelled,
//! and high attention must reach the tower. Route/approach instruments
//! decide which specialist is appropriate; this one proves the complete
//! packages are alive rather than decorative.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::harness;
use understory_core::state::SimSpeed;
use understory_core::state::power::PowerUse;
use understory_core::state::world::ActiveBranch;

const TICKS_PER_DAY: u32 = 14_400;
const DAYS: u32 = 3;
const SEEDS: u64 = 12;
/// Held rather than earned, so the comparison is of the answer and not
/// of how much attention each tower happened to draw.
const LEVELS: [i64; 3] = [300, 600, 1000];
/// Seeds for the charge question. Fewer than the siege's twelve because
/// two tower shapes at twelve seeds each put this file over ten minutes.
///
/// **This used to say a spread of zero needs no sample, and that reason
/// expired with the rail** (`SYSTEMS.md` §6.36): the no-bank shape now
/// spreads 13% on paces and 8% on crafts across the orders, so there is
/// a real quantity here and four seeds is a thin sample of it. Widen this
/// before quoting the spread twice — `AGENTS.md` on checking an
/// instrument's seed count before believing it.
const CHARGE_SEEDS: u64 = 4;

struct Cell {
    lost_hp: i64,
    repelled: u64,
    shots: u64,
    deaths: u64,
}

fn main() {
    println!("Do the defence packages produce live, distinct outcomes?");
    println!(
        "  {DAYS} days a run, {SEEDS} seeds a cell, provocation held. Target selection is\n  \
         automatic; the player decision is which physical package to build.\n"
    );
    println!(
        "  {:<42} {:>10} {:>15} {:>10} {:>8} {:>6}",
        "loadout / attention", "hp lost", "seed spread", "repelled", "shots", "lost"
    );

    let mut said_something = false;
    for (arms, arms_name) in [
        (Arms::Generalists, "generalists (thorn gun + dart battery)"),
        (Arms::Mast, "dart + lantern mast (canopy route)"),
        (Arms::Ward, "dart + root ward (burrow route)"),
        (Arms::Net, "dart + tanglenet (ground route)"),
    ] {
        println!(
            "
  == {arms_name} =="
        );
        for level in LEVELS {
            let mut lost_hp = 0i64;
            let mut repelled = 0u64;
            let mut shots = 0u64;
            let mut deaths = 0u64;
            let mut low = i64::MAX;
            let mut high = i64::MIN;
            for seed in 1..=SEEDS {
                let cell = press(arms, level, seed);
                lost_hp += cell.lost_hp;
                repelled += cell.repelled;
                shots += cell.shots;
                deaths += cell.deaths;
                low = low.min(cell.lost_hp);
                high = high.max(cell.lost_hp);
            }
            let n = i64::try_from(SEEDS).unwrap_or(1);
            said_something |= lost_hp > 0;
            assert!(shots > 0, "the loadout never fired at attention {level}");
            println!(
                "  {:<42} {:>10} {:>15} {:>10} {:>8} {:>6}",
                format!("attention {level}"),
                lost_hp / n,
                format!("{low}..{high}"),
                repelled / SEEDS,
                shots / SEEDS,
                deaths,
            );
        }
    }

    assert!(
        said_something,
        "no tower took any damage at any level: this run gave the mechanism nothing to do, and \
         that is the exact failure `siege_run.rs` shipped four times"
    );

    if std::env::var("UNDERSTORY_WATCH_WAVES_ONLY").is_err() {
        charge_question();
    }
}

/// Which weapons the tower gets.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Arms {
    /// The baseline thorn gun plus dart battery workhorse.
    Generalists,
    /// One specialist added to the workhorse on its forecast route.
    Mast,
    Ward,
    Net,
}

fn press(arms: Arms, level: i64, seed: u64) -> Cell {
    let mut engine = GameEngine::new(seed);
    engine.set_speed(SimSpeed::X1);
    // Same six-floor hull for both packages. Comparing four floors of
    // vulnerable surface with six made the specialist package look
    // worse before its weapons were considered—the exact denominator
    // trap this repository warns about.
    harness::chain_tower(&mut engine, 6);

    // Put each specialist on the route whose forecast makes it a
    // candidate. Measuring a mast against a generic mixed wave asks why
    // it is not a dart battery; the decision the game shows is whether
    // to commit to a canopy, burrow, or ground-heavy road.
    let route = match arms {
        Arms::Generalists => None,
        Arms::Mast => Some("branch.canopy_passage"),
        Arms::Ward => Some("branch.driftwood_shelf"),
        Arms::Net => Some("branch.tide_road"),
    };
    if let Some(route) = route {
        let def = engine
            .content()
            .branch_idx(route)
            .unwrap_or_else(|| panic!("missing route {route}"));
        let here = engine.state().world.distance;
        engine.state_mut_for_test().world.branch = Some(ActiveBranch {
            def,
            from: here,
            to: here + understory_core::fx::paces_from_int(1_000_000),
        });
    }

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
    if arms == Arms::Ward {
        // Give the ward a visible shaft to guard. The starting stairs'
        // inlet is buried behind the Heartseed/bunk on both low decks;
        // without a second riser the supposedly legal ward package can
        // never be staged by this fixture.
        engine
            .try_send(GameCommand::BuildShaft {
                shaft: "shaft.busbar".into(),
                low: 0,
                high: 5,
                slot: 7,
            })
            .expect("the specialist fixture needs a ward-adjacent busbar");
    }
    if arms == Arms::Mast {
        // The ladder's cutter occupies the current roof weapon mount.
        // This is a held-pressure defence fixture with replenished ammo,
        // not an economy run, so clear that mount rather than silently
        // measuring a canopy package with no mast.
        let state = engine.state_mut_for_test();
        let roof = state.tower.floors.len() - 1;
        if let Some(at) = state.tower.floors[roof]
            .rooms
            .iter()
            .position(|room| room.slot >= 8)
        {
            let mut displaced = state.tower.floors[roof].rooms.remove(at);
            displaced.slot = 8;
            state.tower.floors[2].rooms.push(displaced);
            state.tower.floors[2].rooms.sort_by_key(|room| room.slot);
        }
    }
    let base_rooms = engine
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .map(|room| room.id)
        .collect::<Vec<_>>();
    // `open_the_armoury` stands up whatever supplies the weapon, so
    // naming the thornwright here as well built two of them and filled
    // the tower — the battery then had nowhere to go and the assert
    // below caught it, which is the assert doing its job.
    let weapons: &[&str] = match arms {
        Arms::Generalists => &["room.dart_battery"],
        Arms::Mast => &["room.lantern_mast", "room.dart_battery"],
        Arms::Ward => &["room.dart_battery", "room.root_ward"],
        Arms::Net => &["room.dart_battery", "room.tanglenet"],
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

    // This table measures the emplacement package, not the vulnerable
    // surface area of every factory needed to manufacture its ammo.
    // `open_the_armoury` builds that unlock chain honestly; after the
    // weapon is validated, remove only those temporary producers and
    // keep the same base tower plus the named weapons. Economy/prices
    // instruments own the acquisition cost.
    let weapon_defs = weapons
        .iter()
        .filter_map(|weapon| engine.content().room_idx(weapon))
        .collect::<Vec<_>>();
    for floor in &mut engine.state_mut_for_test().tower.floors {
        floor
            .rooms
            .retain(|room| base_rooms.contains(&room.id) || weapon_defs.contains(&room.def));
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
        "item.resin_feedstock",
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
/// **Asked because the first verb did not.** The retired Focus verb moved
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
/// **It says something different since the rail** (`SYSTEMS.md` §6.36),
/// and the before/after is the most useful thing in this file.
///
/// **Before:** five orders, two tower shapes, every row identical to the
/// digit. The reason was structural and is worth keeping, because it is
/// what the rail was built to fix: `Power::draw` refused on
/// `charge < amount` *before* it ever consulted the reservation, so a
/// flat-broke tower never reached the ranking and a comfortable one
/// never reached it either. The ranking could only bite in a middle band
/// where there was enough for *this* spender but not enough to also
/// cover higher-ranked uses — and **capacity was income**, because a
/// burner idles unless the bank can take the whole burn, so removing the
/// cell bank to make a tower poor cut its income too. The tower did not
/// pass through the middle band on the way down; it jumped from
/// comfortable to broke.
///
/// **After:** the rail is a second refusal reason that does not depend
/// on the bank being empty, so the middle band is no longer a knife
/// edge. On the tower with no cell bank, the seven orders now spread
/// **paces 19,350..21,830 (+13%) and crafts 179..193 (+8%)** — measured
/// at four seeds an order. Ranking the lamps first costs the tower a
/// quarter of its crafts and gives it three times the dark ticks;
/// ranking the works first is the best row on the board.
///
/// **The banked shape is almost flat rather than identical:** 3% paces
/// and 4% crafts. The no-bank shape is the run-shaping posture; the bank
/// makes priority insurance. Two shapes on opposite sides of that line
/// are the comparison working rather than half of it failing.
///
/// **`estimate_demand` still reports zero for Lamps and Legs on 99 ticks
/// out of 100**, because they buy in hundred-tick blocks. That has not
/// changed and it still blunts any ranking that puts them high — which
/// is visible in the table as the lamps-first row being bad for reasons
/// of *shedding*, not of reservation.
///
/// ## What this does and does not establish
///
/// Measured: **the order now changes the outcome on a tower that is
/// genuinely short**, and does not on one that is not. **Not
/// established: that the spread is a good spread.** Nothing here says
/// the best order is discoverable by a player, or that the worst one is
/// a trap they would fall into rather than a mistake they would fix in
/// ten seconds. That is the difficulty pass's question and it needs a
/// person, not a harness.
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

    // **The first row must reproduce the engine's own default**, so the
    // table carries its own control: if the shipped order measures
    // differently from the others for any reason other than the order,
    // the comparison is broken and every row below is noise.
    //
    // Guns joined at M6's rail (`SYSTEMS.md` §6.36), which is why there
    // are seven rows rather than five: the last two are the decision the
    // whole change exists to create — *shed the mill, keep the guns*,
    // and its reverse.
    let orders: [[PowerUse; 5]; 7] = [
        [
            PowerUse::Lifts,
            PowerUse::Works,
            PowerUse::Guns,
            PowerUse::Lamps,
            PowerUse::Legs,
        ],
        [
            PowerUse::Legs,
            PowerUse::Lifts,
            PowerUse::Works,
            PowerUse::Guns,
            PowerUse::Lamps,
        ],
        [
            PowerUse::Works,
            PowerUse::Lifts,
            PowerUse::Guns,
            PowerUse::Lamps,
            PowerUse::Legs,
        ],
        [
            PowerUse::Lamps,
            PowerUse::Lifts,
            PowerUse::Works,
            PowerUse::Guns,
            PowerUse::Legs,
        ],
        [
            PowerUse::Lifts,
            PowerUse::Legs,
            PowerUse::Guns,
            PowerUse::Lamps,
            PowerUse::Works,
        ],
        [
            PowerUse::Guns,
            PowerUse::Lifts,
            PowerUse::Works,
            PowerUse::Lamps,
            PowerUse::Legs,
        ],
        [
            PowerUse::Lifts,
            PowerUse::Works,
            PowerUse::Lamps,
            PowerUse::Legs,
            PowerUse::Guns,
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
  Across the {} orders: paces {least_paces}..{most_paces} ({:+.0}%), crafts          {least_crafts}..{most_crafts} ({:+.0}%).",
        rows.len(),
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
    order: &[PowerUse; 5],
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
    // Removing the cell bank used to overshoot straight past that band:
    // income 8,248 against 8,908 spent, 32% of ticks browned out, and
    // five orders identical to the digit. **The rail changed that** —
    // a draw can now be refused for want of supply rather than of
    // savings, so this shape sits *inside* the band instead of past it,
    // and it is the shape that produces the spread this function
    // reports.
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
    // **A gun, and then insist on it** (`SYSTEMS.md` §6.36). Guns is a
    // circuit now, and a tower with nothing that shoots measures the
    // same under every order by construction — the same argument this
    // file already makes two hundred lines up about a missing shaft, and
    // the same trap `siege_run.rs` shipped twice.
    harness::open_the_armoury(&mut engine, "room.dart_battery");
    stock_or_panic(&mut engine);
    assert!(
        harness::place_anywhere(&mut engine, "room.dart_battery"),
        "no emplacement: Guns would never spend and two of the seven orders would be           decoration. The tower is {}",
        describe(&engine)
    );
    engine
        .try_send(GameCommand::SetPowerPriority {
            order: order.to_vec(),
        })
        .expect("the order is every legal use, exactly once");
    assert_eq!(
        engine.state().power.priority,
        order.to_vec(),
        "the order did not take: this would compare identical towers"
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
