//! Do the crew eat and sleep the way the constants say they do?
//!
//! ```text
//! cargo run --release -p understory-core --example needs
//! ```
//!
//! M4's Crew rows make arithmetic claims about a person's day —
//! `hungry_ticks` 4,800 is "a third of a day, so three meals a day a
//! person"; `rest_gain_per_tick` 2 means "a night off shift refills
//! 12,096"; `rested_max_ticks` 8,640 is "a shade over the 8,352-tick day
//! shift, so a day worker ends their shift just about spent". None of it
//! had been checked against a tower actually running, and the charge
//! section had exactly the same shape until `charge.rs` found a row 70%
//! out (see `BALANCE.md`'s Power section).
//!
//! So this feeds a tower, houses it, and watches the crew for a week.
//!
//! **Whole days, and several of them.** One day measures the opening
//! position: everybody starts fed and rested, so a single day reports a
//! tower that has not yet had to sustain anything.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;

const DAY: u32 = 14_400;
const DAYS: u32 = 7;

/// One seed's answer. **Swept, because this instrument fed ten
/// `BALANCE.md` rows and `PLAYTEST.md` criterion 3 off a single seed.**
/// The numbers turned out stable — but the published figures were the
/// worst seed of five rather than the figure, and nobody could have
/// known that from one run.
struct Run {
    crew: u64,
    meals: f64,
    asleep_hours: f64,
    hungry: f64,
    starving: f64,
    tired: f64,
    slowed: f64,
}

/// Every seed this sweeps. Twelve costs seconds — `lift.rs` spent a
/// milestone on three for no reason anybody had checked.
const SEEDS: [u64; 8] = [0x_FED, 1, 2, 3, 4, 5, 6, 7];

fn measure(seed: u64) -> Run {
    let content = understory_core::content::Content::load_embedded().expect("pack");
    let mut game = GameEngine::new(seed);
    game.set_speed(SimSpeed::X1);
    // **The opening ladder first** (`SYSTEMS.md` §6.11). M6 cut the
    // starting tower to a Heartseed and a bed, so a harness that places
    // a canteen on turn one gets `CommandError::Locked` rather than a
    // tower. `chain_tower` grows to four floors and walks farm → cutter
    // arm → burner → mill, storeroom, cell bank, which is the tower
    // every instrument here was written against.
    understory_core::harness::chain_tower(&mut game, 4);

    // A kitchen and beds, bought when affordable. Without them this
    // measures neglect rather than a working rota.
    // **Wherever they fit, rather than at named slots.** These used to
    // pin the canteen to floor 1 slot 4 and the bunk to floor 3 slot 1,
    // which were free in the old pre-built starting tower and are not
    // free in a tower this harness has just grown for itself. A slot
    // number is not what any of these instruments is measuring.
    for room in ["room.canteen", "room.bunk"] {
        let cost: Vec<(String, i64)> = {
            let content = game.content();
            let idx = content.room_idx(room).expect("the pack defines it");
            content
                .room_rt(idx)
                .build_cost
                .iter()
                .map(|(item, n)| (content.item(*item).id.clone(), *n))
                .collect()
        };
        for (item, n) in cost {
            understory_core::harness::give(&mut game, &item, n * 2);
        }
        assert!(
            understory_core::harness::place_anywhere(&mut game, room),
            "could not place {room}: this harness is measuring a tower it failed to build"
        );
    }

    step(&mut game, DAY);
    let crew = game.state().crew.len() as u64;
    let meals_at_start = game.state().stats.meals_eaten;
    let asleep_at_start = game.state().stats.crew_ticks_asleep;

    let (mut hungry, mut starving, mut tired, mut slowed, mut samples) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    for _ in 0..DAY * (DAYS - 1) {
        step(&mut game, 1);
        let state = game.state();
        for member in &state.crew {
            samples += 1;
            if member.hunger >= content.balance.crew.hungry_ticks {
                hungry += 1;
            }
            if member.hunger >= content.balance.crew.starving_ticks {
                starving += 1;
            }
            if member.rested <= content.balance.crew.tired_ticks {
                tired += 1;
            }
            if member.hunger >= content.balance.crew.starving_ticks
                || member.rested <= content.balance.crew.tired_ticks
            {
                slowed += 1;
            }
        }
    }

    let days = u64::from(DAYS - 1);
    let state = game.state();
    let meals = state.stats.meals_eaten - meals_at_start;
    let asleep = state.stats.crew_ticks_asleep - asleep_at_start;

    Run {
        crew,
        meals: meals as f64 / crew as f64 / days as f64,
        asleep_hours: asleep as f64 / crew as f64 / days as f64 / f64::from(DAY) * 24.0,
        hungry: pct(hungry, samples),
        starving: pct(starving, samples),
        tired: pct(tired, samples),
        slowed: pct(slowed, samples),
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
    println!("=== do the crew eat and sleep as designed? ===\n");
    println!(
        "  A fed, housed tower over {DAYS} whole days. The first day is the warm-up:\n\
         everybody starts full and rested, so day one measures the opening position\n\
         rather than a tower sustaining anything.\n"
    );

    let each: Vec<Run> = SEEDS.iter().map(|&seed| measure(seed)).collect();
    let show = |name: &str, get: fn(&Run) -> f64, unit: &str| {
        let (mean, lo, hi) = understory_core::harness::span(&each, get);
        println!(
            "{name:<30}{mean:>6.2}{unit}   (range {lo:.2}-{hi:.2} across {} seeds)",
            SEEDS.len()
        );
    };

    println!("crew                          {}", each[0].crew);
    show("meals a person a day", |r| r.meals, " ");
    show("hours asleep a person a day", |r| r.asleep_hours, "h");
    println!();
    println!("share of a person's time spent:");
    show("  hungry (past 4,800)", |r| r.hungry, "%");
    show("  starving (past 7,200)", |r| r.starving, "%");
    show("  tired (under 1,440 rest)", |r| r.tired, "%");
    show("  **slowed** (either)", |r| r.slowed, "%");
    for line in [
        "",
        "  **The shortfall is structural, and cutting the rota took most of it away.**",
        "",
        "`hungry_ticks` 4,800 is a third of a 14,400-tick day, so the row reads as three meals",
        "a person a day. Hunger rises around the clock — you do not stop needing to eat because",
        "you are in bed — but *eating* does not, so anybody who sleeps through a whole 4,800-tick",
        "cycle wakes a meal in debt. That much is unchanged and always will be.",
        "",
        "What changed is how long they are under. **Under the rota this instrument read 2.00",
        "meals, 33.3% hungry, 13.4% starving and 22.0% tired — a crew member spent 35.5% of their",
        "life working two-thirds as fast.** Everybody slept the same 6,048-tick band whether they",
        "needed it or not, and hunger ran the whole way through it. Sleep is need-driven since M6",
        "(`SYSTEMS.md` §6.32): people are under for shorter stretches, at staggered times, and the",
        "numbers above are what that did to the needs economy.",
        "",
        "`hungry_work_pct` and `tired_work_pct` are both 60, so **slowed** is the share of a crew",
        "member's life spent working two-thirds as fast. Read it against 35.5%.",
        "",
        "Whether what is left is too gentle rather than too harsh is now the open judgement, and",
        "it is the opposite of the one this instrument used to pose. `docs/PLAYTEST.md` criterion",
        "3 carries it.",
    ] {
        println!("{line}");
    }
}

fn pct(n: u64, of: u64) -> f64 {
    n as f64 * 100.0 / of.max(1) as f64
}

fn step(game: &mut GameEngine, ticks: u32) {
    for _ in 0..ticks {
        if let Some(fork) = game.state().world.fork
            && fork.answer.is_none()
        {
            let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
        }
        game.step(1);
    }
}
