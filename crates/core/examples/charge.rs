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
//! **The budget table is not the shipped opening.** It uses
//! `harness::chain_tower`: four floors, the opening ladder, a mill, a
//! storeroom and a cell bank, then installs a full-height Busbar Riser.
//! The riser is part of the fixture, not free power: without it the new
//! floor-local network leaves sources, storage and loads on separate
//! islands and a table of zeroes measures wiring failure rather than a
//! charge budget. Every bamboo input is kept full by hand.
//! Whether crew can keep that tower supplied is `haul.rs`'s question;
//! mixing the two is how a power measurement becomes a transport
//! measurement. These rows are therefore the developed tower's charge
//! budget, not the opening and not a claim about what three crew carry.
//!
//! The second table runs the actual two-floor opening for three days,
//! issues no build commands and supplies nothing. It measures the
//! Heartseed's recovery floor and the starting bank honestly, without
//! quietly replacing the opening with a mature fixture.
//!
//! **Whole days, always.** A window that is not a whole number of days
//! measures what time it started at — the same trap `throughput.rs`
//! documents at the top of itself, which cost that instrument a correct
//! answer once already. Crew sleep through the night band, so this is
//! not merely tidy.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;
use understory_core::state::power::PowerUse;

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
    println!("=== developed, hand-fed charge budget ===\n");
    println!(
        "  One day-cycle each, measured off the meter rather than multiplied out.\n\
         Four-floor `chain_tower` baseline plus a full-height busbar trunk.\n\
         Inputs are serviced by hand, so `fuel` is the budget rather than what\n\
         the crew managed to carry. This is not the shipped opening.\n"
    );

    println!(
        "{:<26} {:>8} {:>8} {:>8} {:>6} {:>9} {:>8}  {:<12} {:<18}",
        "tower",
        "income",
        "draw",
        "net",
        "fuel",
        "brownout",
        "dark",
        "net range",
        "refused L/W/G/Lm/Lg"
    );
    let mut rows = Vec::new();
    for (label, floors, walking, burners, forge) in [
        ("developed 4F, parked", 0u8, false, 1u8, false),
        ("developed 4F, striding", 0, true, 1, false),
        ("developed 8F, parked", 4, false, 1, false),
        ("developed 8F, striding", 4, true, 1, false),
        ("developed 14F, parked", 10, false, 1, false),
        ("developed 14F, striding", 10, true, 1, false),
        // The same tall towers with the burners a tall tower would
        // actually build. If one is not enough, the rows above are
        // measuring a brown-out rather than a lighting bill.
        ("developed 14F, 3 burners", 10, false, 3, false),
        ("developed 14F, 3 + stride", 10, true, 3, false),
        // Scrap is supplied and alloy removed directly, so this is a
        // continuously loaded electrical service test rather than a
        // claim about whether the crew can feed or clear the forge.
        ("developed 4F + forge", 0, true, 1, true),
        // Same ready load with the burner removed. This is the service
        // failure probe: per-circuit refusal must identify what the
        // Heartseed floor cannot keep alive.
        ("4F forge, Heartseed only", 0, true, 0, true),
        // **No burner at all.** The floor the Heartseed puts under the
        // economy, which is the only reason a tower that runs dry is
        // recoverable rather than dead — see `heartseed_charge_per_100_ticks`.
        ("developed 4F, no burner", 0, true, 0, false),
    ] {
        let day = measure(floors, walking, burners, forge);
        // **The net's range, and a marker when it changes sign.** The
        // mean alone said "+10 floors, striding" nets +368; one seed of
        // eight says -264. A column that straddles zero is two different
        // towers averaged, and thirteen `BALANCE.md` rows quote this.
        let flips = day.net_low < 0 && day.net_high > 0;
        let net_range = format!("{}..{}", day.net_low, day.net_high);
        println!(
            "{label:<26} {:>8} {:>8} {:>8} {:>6} {:>8}% {:>7}%  {:<12} {:>4}/{:>4}/{:>4}/{:>4}/{:>4}{}",
            day.income,
            day.draw,
            day.income - day.draw,
            day.fuel,
            day.brownout_ticks * 100 / i64::from(DAY),
            day.dark_ticks * 100 / i64::from(DAY),
            &net_range,
            day.refused[PowerUse::Lifts.index()],
            day.refused[PowerUse::Works.index()],
            day.refused[PowerUse::Guns.index()],
            day.refused[PowerUse::Lamps.index()],
            day.refused[PowerUse::Legs.index()],
            if flips { "  SIGN FLIPS" } else { "" },
        );
        rows.push((label, day));
    }

    let parked = measure(0, false, 1, false);
    let striding = measure(0, true, 1, false);
    let tall = measure(10, false, 3, false);
    println!(
        "\n  measured stride cost over one day : {} (the row derives 2,880)",
        striding.draw - parked.draw
    );
    println!(
        "  measured lamp cost, 4 floors      : {} (2 × 4 floors × 43.6% of 14,400 / 100 = ~502)",
        parked.draw
    );
    println!(
        "  measured lamp cost, 14 floors     : {} (2 × 14 floors × 43.6% of 14,400 / 100 = ~1,758)",
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
            "  the developed 4F fixture costs   : {} stalks a day",
            striding.fuel
        );
    }
    println!(
        "  the Heartseed alone, over a day   : {} (striding wants {})",
        rows.iter()
            .find(|(label, _)| *label == "developed 4F, no burner")
            .map_or(0, |(_, day)| day.income),
        striding.draw - parked.draw + parked.draw,
    );

    println!("\n=== untouched shipped opening: no builds, no supplied fuel ===\n");
    println!(
        "{:<8} {:>8} {:>8} {:>9} {:>10} {:<18}",
        "day", "income", "draw", "brownout", "paces", "refused L/W/G/Lm/Lg"
    );
    for (day, opening) in measure_opening(3).iter().enumerate() {
        println!(
            "{:<8} {:>8} {:>8} {:>8}% {:>10} {:>4}/{:>4}/{:>4}/{:>4}/{:>4}",
            day + 1,
            opening.income,
            opening.draw,
            opening.brownout_ticks * 100 / i64::from(DAY),
            opening.paces,
            opening.refused[PowerUse::Lifts.index()],
            opening.refused[PowerUse::Works.index()],
            opening.refused[PowerUse::Guns.index()],
            opening.refused[PowerUse::Lamps.index()],
            opening.refused[PowerUse::Legs.index()],
        );
    }
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
    /// Ticks on which each circuit was actually refused, in
    /// `PowerUse::index` order. Unlike aggregate brownout, this says
    /// which service failed.
    refused: [i64; PowerUse::ALL.len()],
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
fn measure(extra_floors: u8, walking: bool, burners: u8, forge: bool) -> Day {
    let each: Vec<Day> = SEEDS
        .iter()
        .map(|&seed| measure_seed(seed, extra_floors, walking, burners, forge))
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
        refused: std::array::from_fn(|i| each.iter().map(|d| d.refused[i]).sum::<i64>() / n),
    }
}

fn measure_seed(seed: u64, extra_floors: u8, walking: bool, burners: u8, forge: bool) -> Day {
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
    install_busbar(&mut game);
    let _ = poles;

    let burner_idx = game
        .content()
        .room_idx("room.burner")
        .expect("the pack defines a burner");
    // Build the forge while its unlock rung is still standing. The
    // Heartseed-only stress row removes the burner afterwards; removing
    // it first would make `place_anywhere` correctly return Locked and
    // the row would measure nothing.
    let forge_idx = if forge {
        understory_core::harness::give(&mut game, "item.poles", 20);
        assert!(
            understory_core::harness::place_anywhere(&mut game, "room.sun_forge"),
            "the serviced-forge row has no forge"
        );
        game.content().room_idx("room.sun_forge")
    } else {
        None
    };
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

    step(&mut game, DAY, bamboo, forge_idx);

    let (mut served_rate, mut brownout_ticks, mut dark_ticks) = (0i64, 0i64, 0i64);
    let mut refused = [0i64; PowerUse::ALL.len()];
    let fuel_before = game.state().stats.fuel_burned;
    let charge_before = game.state().power.charge;
    for _ in 0..DAY {
        step(&mut game, 1, bamboo, forge_idx);
        let power = &game.state().power;
        served_rate += PowerUse::ALL
            .iter()
            .map(|use_| {
                power.demand[use_.index()] * power.served(*use_)
                    / understory_core::state::power::FULL
            })
            .sum::<i64>();
        brownout_ticks += i64::from(power.brownout);
        // Ticks on which a circuit got less than it asked for. Under
        // satisfaction that is a rate rather than an outage, so this
        // counts "went short" rather than "was refused".
        for (i, use_) in PowerUse::ALL.iter().enumerate() {
            refused[i] += i64::from(power.short(*use_));
        }
        let content = game.content().clone();
        if understory_core::systems::power::exposure_pct(game.state(), &content)
            < content.balance.clock.night_light_threshold
        {
            dark_ticks += 1;
        }
    }
    // Demand is authored in charge per 100 ticks. Summing the
    // presentation `spent_last` loses every sub-unit tick in the local
    // network (a 0.2/tick leg draw printed as zero forever), which is
    // how this instrument became a table of zeroes. Carry the rates
    // across the whole window, then divide once.
    let draw = served_rate / understory_core::state::power::PER_TICK;
    let income = draw + game.state().power.charge - charge_before;
    Day {
        income,
        draw,
        fuel: (game.state().stats.fuel_burned - fuel_before) as i64,
        brownout_ticks,
        dark_ticks,
        // A single seed has no range; `measure` fills these in.
        net_low: income - draw,
        net_high: income - draw,
        refused,
    }
}

/// Wire the developed fixture with the authored fixed and per-boundary
/// costs. This instrument grants those materials because it compares
/// electrical budgets after construction; `prices.rs` and `journey.rs`
/// measure when the tower can pay for them.
fn install_busbar(game: &mut GameEngine) {
    let high = game.state().tower.floors.len() as u8 - 1;
    let slot = game.state().tower.floors[0].slots - 3;
    let runtime = game
        .content()
        .shaft_rt(game.content().shaft_idx("shaft.busbar").expect("busbar"));
    let mut cost = runtime.build_cost.clone();
    for &(item, per_boundary) in &runtime.span_cost {
        let amount = per_boundary * i64::from(high);
        if let Some((_, held)) = cost.iter_mut().find(|(had, _)| *had == item) {
            *held += amount;
        } else {
            cost.push((item, amount));
        }
    }
    for (item, amount) in cost {
        let id = game.content().item(item).id.clone();
        understory_core::harness::give(game, &id, amount);
    }
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.busbar".into(),
        low: 0,
        high,
        slot,
    })
    .unwrap_or_else(|error| panic!("developed charge fixture has no busbar: {error}"));
}

#[derive(Default)]
struct OpeningDay {
    income: i64,
    draw: i64,
    brownout_ticks: i64,
    paces: i64,
    refused: [i64; PowerUse::ALL.len()],
}

/// The actual shipped tower from tick zero: two floors, no burner and
/// no fixture-granted stock or rooms. This intentionally issues no
/// build commands. It is a recovery-floor probe, not a model player.
fn measure_opening(days: usize) -> Vec<OpeningDay> {
    let mut totals: Vec<OpeningDay> = (0..days).map(|_| OpeningDay::default()).collect();
    for seed in SEEDS {
        let mut game = GameEngine::new(seed);
        game.set_speed(SimSpeed::X1);
        let _ = game.try_send(GameCommand::SetStriding { walking: true });
        let mut last_distance = game.state().world.distance;
        for total in &mut totals {
            let charge_before = game.state().power.charge;
            let mut served_rate = 0i64;
            for _ in 0..DAY {
                if let Some(fork) = game.state().world.fork
                    && fork.answer.is_none()
                {
                    let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
                }
                game.step(1);
                let power = &game.state().power;
                served_rate += PowerUse::ALL
                    .iter()
                    .map(|use_| {
                        power.demand[use_.index()] * power.served(*use_)
                            / understory_core::state::power::FULL
                    })
                    .sum::<i64>();
                total.brownout_ticks += i64::from(game.state().power.brownout);
                for (i, use_) in PowerUse::ALL.iter().enumerate() {
                    total.refused[i] += i64::from(game.state().power.short(*use_));
                }
            }
            let draw = served_rate / understory_core::state::power::PER_TICK;
            total.draw += draw;
            total.income += draw + game.state().power.charge - charge_before;
            let distance = game.state().world.distance;
            total.paces += understory_core::fx::paces_to_int(distance - last_distance);
            last_distance = distance;
        }
    }
    let seeds = SEEDS.len() as i64;
    for total in &mut totals {
        total.income /= seeds;
        total.draw /= seeds;
        total.brownout_ticks /= seeds;
        total.paces /= seeds;
        for refused in &mut total.refused {
            *refused /= seeds;
        }
    }
    totals
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
fn step(
    game: &mut GameEngine,
    ticks: u32,
    fuel: understory_core::ids::ItemIdx,
    serviced_forge: Option<understory_core::ids::RoomIdx>,
) {
    let scrap = game
        .content()
        .item_idx("item.scrap")
        .expect("the pack defines scrap");
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
                    if Some(room.def) == serviced_forge {
                        if let Some(stack) = room.inputs.iter_mut().find(|s| s.item == scrap) {
                            stack.deposit(stack.space());
                        }
                        for stack in &mut room.outputs {
                            stack.withdraw(stack.count);
                        }
                    }
                }
            }
        }
        game.step(1);
    }
}
