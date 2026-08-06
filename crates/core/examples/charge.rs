//! Does the charge economy add up? Measured, not derived.
//!
//! ```text
//! cargo run --release -p understory-core --example charge
//! ```
//!
//! **`BALANCE.md`'s Power section is arithmetic all the way down**, and
//! none of it had ever been checked against the simulation. Its rows
//! claim that continuous striding costs "2,880 charge across a full
//! day-cycle", that lighting an 8-floor tower through the night costs
//! "~760". Every one of those is a multiplication somebody did by hand
//! from the constant next to it, and every one of them silently stops
//! being true the moment a system around it changes — which is exactly
//! how M1's throughput harness came to spend a milestone panicking on
//! startup without anybody noticing.
//!
//! So this runs the tower and reads the meter.
//!
//! **M6 changed the question this instrument answers.** It cut the
//! canopy sails, so income no longer arrives from the sky and the old
//! sun-versus-shade rows measure a system that is gone. What is left is
//! one source — burners — and one thing worth knowing about it: *a day
//! of charge costs this many stalks of bamboo*. That is now the whole
//! of the power economy and it is denominated in the same material the
//! mill wants, so the `fuel` column below is the real price of every
//! lamp, every lift and every pace.
//!
//! **Burners are fuelled by hand here.** Whether crew can keep one full
//! is `haul.rs`'s question and `lift.rs` measures it directly; mixing
//! the two is how you get a power measurement that is really a
//! transport measurement. Every tower below has as much fuel as it can
//! burn, so the figures are the budget rather than the shortfall.
//!
//! **Whole days, always.** A window that is not a whole number of days
//! measures what time it started at — the same trap `throughput.rs`
//! documents at the top of itself, which cost that instrument a correct
//! answer once already. Crew sleep through the night band, so this is
//! not merely tidy.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;

const DAY: u32 = 14_400;

fn main() {
    // **A hardcoded day goes stale silently.** `journey.rs` reported a
    // 57,372-tick run as "3 days" when the pack said 7,200 and the
    // answer was eight, because it kept its own copy of the day length.
    // Every instrument here windows on whole days, so a stale copy makes
    // the window a measurement of what time it started at — the trap
    // `throughput.rs` documents at the top of itself. Fail loudly rather
    // than quietly measure a different game.
    let content = understory_core::content::Content::load_embedded().expect("the shipped pack");
    assert_eq!(
        DAY, content.balance.clock.ticks_per_day,
        "the pack's day length has moved; update this file's day constant"
    );
    println!("=== what does a day of charge cost? ===\n");
    println!(
        "  One day-cycle each, measured off the meter rather than multiplied out.\n\
         Burners are handed all the fuel they can burn, so `fuel` is the budget\n\
         rather than what the crew managed to carry.\n"
    );

    println!(
        "{:<26} {:>8} {:>8} {:>8} {:>6} {:>9} {:>8}  {:<12}",
        "tower", "income", "draw", "net", "fuel", "brownout", "dark", "net range"
    );
    let mut rows = Vec::new();
    for (label, floors, walking, burners) in [
        ("starting, parked", 0u8, false, 1u8),
        ("starting, striding", 0, true, 1),
        ("+4 floors, parked", 4, false, 1),
        ("+4 floors, striding", 4, true, 1),
        ("+10 floors, parked", 10, false, 1),
        ("+10 floors, striding", 10, true, 1),
        // The same tall towers with the burners a tall tower would
        // actually build. If one is not enough, the rows above are
        // measuring a brown-out rather than a lighting bill.
        ("+10, 3 burners, parked", 10, false, 3),
        ("+10, 3 burners, striding", 10, true, 3),
        // **No burner at all.** The floor the Heartseed puts under the
        // economy, which is the only reason a tower that runs dry is
        // recoverable rather than dead — see `heartseed_charge_per_100_ticks`.
        ("no burner, striding", 0, true, 0),
    ] {
        let day = measure(floors, walking, burners);
        // **The net's range, and a marker when it changes sign.** The
        // mean alone said "+10 floors, striding" nets +368; one seed of
        // eight says -264. A column that straddles zero is two different
        // towers averaged, and thirteen `BALANCE.md` rows quote this.
        let flips = day.net_low < 0 && day.net_high > 0;
        let net_range = format!("{}..{}", day.net_low, day.net_high);
        println!(
            "{label:<26} {:>8} {:>8} {:>8} {:>6} {:>8}% {:>7}%  {}{}",
            day.income,
            day.draw,
            day.income - day.draw,
            day.fuel,
            day.brownout_ticks * 100 / i64::from(DAY),
            day.dark_ticks * 100 / i64::from(DAY),
            &net_range,
            if flips { "  SIGN FLIPS" } else { "" },
        );
        rows.push((label, day));
    }

    let parked = measure(0, false, 1);
    let striding = measure(0, true, 1);
    let tall = measure(10, false, 3);
    println!(
        "\n  measured stride cost over one day : {} (the row derives 2,880)",
        striding.draw - parked.draw
    );
    println!(
        "  measured lamp cost, 4 floors      : {} (the row derives ~380)",
        parked.draw
    );
    println!(
        "  measured lamp cost, 14 floors     : {} (the row derives ~1,330)",
        tall.draw
    );

    let burner = content
        .rooms
        .iter()
        .find(|room| room.id == "room.burner")
        .and_then(|room| room.burner.as_ref())
        .expect("the pack defines a burner");
    println!(
        "\n  charge per stalk of bamboo        : {}",
        burner.charge_per_burn / burner.fuel_per_burn
    );
    if striding.fuel > 0 {
        println!(
            "  a striding starting tower costs   : {} stalks a day",
            striding.fuel
        );
    }
    println!(
        "  the Heartseed alone, over a day   : {} (striding wants {})",
        rows.iter()
            .find(|(label, _)| *label == "no burner, striding")
            .map_or(0, |(_, day)| day.income),
        striding.draw - parked.draw + parked.draw,
    );
}

struct Day {
    income: i64,
    draw: i64,
    /// Stalks of bamboo put up the chimney over the day. **The price of
    /// the whole power economy since M6**, denominated in the material
    /// the mill also wants.
    fuel: i64,
    brownout_ticks: i64,
    /// Ticks the tower needed its lamps: exposure below the threshold.
    ///
    /// Not `power.lit`, which is *true in daylight* — it means "the
    /// place is lit", not "the lamps are burning", and reading it as
    /// the second thing says a tower runs its lamps 100% of the day.
    dark_ticks: i64,
    /// The lowest and highest net across `SEEDS`. **A mean that
    /// straddles zero is not a tower that breaks even**, it is two
    /// different towers averaged, and this is what says which.
    net_low: i64,
    net_high: i64,
}

/// One whole day-cycle, after a whole-day warm-up so the bank and the
/// clock are both in a steady state rather than in their opening
/// positions.
///
/// `burners` counts the burners the tower ends up with, *including* the
/// one the starting tower already owns. Zero strips it out, which is the
/// only way to see what the Heartseed's trickle does on its own.
/// Every seed this averages over.
///
/// **It ran on one until 2026-08-06, and one was not enough.** A
/// ten-floor striding tower nets **+368** charge on one seed and
/// **-264** on another — a surplus or a deficit depending on nothing but
/// the terrain it happened to walk. Thirteen `BALANCE.md` rows quote
/// this instrument, and any of them saying a tall tower runs a surplus
/// was true on the seed it was measured on and false on the next.
const SEEDS: [u64; 8] = [0x_5A_11, 1, 2, 3, 4, 5, 6, 7];

/// The mean across `SEEDS`, plus the net's range — because the net is
/// the column that changes sign, and a mean that straddles zero is a
/// different statement from one that does not.
fn measure(extra_floors: u8, walking: bool, burners: u8) -> Day {
    let each: Vec<Day> = SEEDS
        .iter()
        .map(|&seed| measure_seed(seed, extra_floors, walking, burners))
        .collect();
    let n = each.len() as i64;
    let nets: Vec<i64> = each.iter().map(|d| d.income - d.draw).collect();
    Day {
        income: each.iter().map(|d| d.income).sum::<i64>() / n,
        draw: each.iter().map(|d| d.draw).sum::<i64>() / n,
        fuel: each.iter().map(|d| d.fuel).sum::<i64>() / n,
        brownout_ticks: each.iter().map(|d| d.brownout_ticks).sum::<i64>() / n,
        dark_ticks: each.iter().map(|d| d.dark_ticks).sum::<i64>() / n,
        net_low: nets.iter().copied().min().unwrap_or(0),
        net_high: nets.iter().copied().max().unwrap_or(0),
    }
}

fn measure_seed(seed: u64, extra_floors: u8, walking: bool, burners: u8) -> Day {
    let mut game = GameEngine::new(seed);
    game.set_speed(SimSpeed::X1);

    let poles = game
        .content()
        .item_idx("item.poles")
        .expect("the pack defines poles");
    let bamboo = game
        .content()
        .item_idx("item.bamboo")
        .expect("the pack defines bamboo");

    // **The opening ladder, then the height** (`SYSTEMS.md` §6.11). M6
    // cut the starting tower to a Heartseed and a bed, so the burner
    // whose fuel bill this instrument exists to measure is now
    // something the tower has to build. Four floors is the height the
    // old starting tower arrived at, which is what every row below is
    // written relative to.
    //
    // Floors and rooms are paid for from granted stock: this is a
    // measurement of a tall tower's *draw*, not of how long it takes to
    // become one.
    understory_core::harness::chain_tower(&mut game, 4 + extra_floors);
    let _ = poles;

    let burner_idx = game
        .content()
        .room_idx("room.burner")
        .expect("the pack defines a burner");
    // The ladder above put exactly one up.
    if burners == 0 {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            floor.rooms.retain(|room| room.def != burner_idx);
        }
    } else {
        // **Asserted, because a burner that silently failed to go up
        // would make this row identical to the one above it and read as
        // "height buys nothing".** That exact failure — a helper
        // returning `false` into nothing — is what
        // `siege_run.rs` recorded twice and `AGENTS.md` §II now warns
        // about by name.
        let already = 1;
        for n in already..burners {
            give(&mut game, bamboo, 0);
            understory_core::harness::give(&mut game, "item.poles", 20);
            let up = understory_core::harness::place_anywhere(&mut game, "room.burner");
            assert!(up, "burner {n} did not go up; this row measures nothing");
        }
    }

    let _ = game.try_send(GameCommand::SetStriding { walking });

    step(&mut game, DAY, bamboo);

    let (mut income, mut draw, mut brownout_ticks, mut dark_ticks) = (0i64, 0i64, 0i64, 0i64);
    let fuel_before = game.state().stats.fuel_burned;
    for _ in 0..DAY {
        step(&mut game, 1, bamboo);
        let power = &game.state().power;
        income += power.income_last;
        draw += power.spent_last;
        brownout_ticks += i64::from(power.brownout);
        let content = game.content().clone();
        if understory_core::systems::power::exposure_pct(game.state(), &content)
            < content.balance.clock.night_light_threshold
        {
            dark_ticks += 1;
        }
    }
    Day {
        income,
        draw,
        fuel: (game.state().stats.fuel_burned - fuel_before) as i64,
        brownout_ticks,
        dark_ticks,
        // A single seed has no range; `measure` fills these in.
        net_low: income - draw,
        net_high: income - draw,
    }
}

/// Put an item straight onto the shelves.
fn give(game: &mut GameEngine, item: understory_core::ids::ItemIdx, mut amount: i64) {
    let state = game.state_mut_for_test();
    'floors: for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            amount -= room.shelve(item, amount);
            if amount <= 0 {
                break 'floors;
            }
        }
    }
}

/// Step, answering any fork and topping every burner up.
///
/// An unanswered fork halts the tower, and a halted tower pays no stride
/// charge, which is the measurement. Topping the burners up is what
/// makes this a measurement of the power budget rather than of whether
/// three crew can carry bamboo up ten floors — see the module header.
fn step(game: &mut GameEngine, ticks: u32, fuel: understory_core::ids::ItemIdx) {
    for _ in 0..ticks {
        if let Some(fork) = game.state().world.fork
            && fork.answer.is_none()
        {
            let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
        }
        {
            let state = game.state_mut_for_test();
            for floor in &mut state.tower.floors {
                for room in &mut floor.rooms {
                    if let Some(stack) = room.inputs.iter_mut().find(|s| s.item == fuel) {
                        let space = stack.space();
                        stack.deposit(space);
                    }
                }
            }
        }
        game.step(1);
    }
}
