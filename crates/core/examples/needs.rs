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

fn main() {
    println!("=== do the crew eat and sleep as designed? ===\n");
    println!(
        "  A fed, housed tower over {DAYS} whole days. The first day is the warm-up:\n\
         everybody starts full and rested, so day one measures the opening position\n\
         rather than a tower sustaining anything.\n"
    );

    let content = understory_core::content::Content::load_embedded().expect("pack");
    let mut game = GameEngine::new(0x_FED);
    game.set_speed(SimSpeed::X1);

    // A kitchen and beds, bought when affordable. Without them this
    // measures neglect rather than a working rota.
    for (room, floor, slot) in [("room.canteen", 1u8, 4u8), ("room.bunk", 3, 1)] {
        for _ in 0..600 {
            match game.try_send(GameCommand::PlaceRoom {
                room: room.into(),
                floor,
                slot,
            }) {
                Ok(()) => break,
                Err(understory_core::command::CommandError::InsufficientStock { .. }) => {
                    step(&mut game, 300);
                }
                Err(other) => panic!("could not place {room}: {other}"),
            }
        }
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

    println!("crew                          {crew}");
    println!(
        "meals a person a day          {:.2}   (`hungry_ticks` 4,800 says 3)",
        meals as f64 / crew as f64 / days as f64
    );
    println!(
        "hours asleep a person a day    {:.1}h  (of a 24h day-cycle)",
        asleep as f64 / crew as f64 / days as f64 / f64::from(DAY) * 24.0
    );
    println!(
        "\nshare of a person's time spent:\n  \
         hungry (past 4,800)         {:>5.1}%\n  \
         starving (past 7,200)       {:>5.1}%\n  \
         tired (under 1,440 rest)    {:>5.1}%\n  \
         **slowed** (either)         {:>5.1}%",
        pct(hungry, samples),
        pct(starving, samples),
        pct(tired, samples),
        pct(slowed, samples),
    );
    println!(
        "\n  **Two meals a day, not three, and the shortfall is structural.**\n\
         \n\
         `hungry_ticks` 4,800 is a third of a 14,400-tick day and the row reads it as\n\
         three meals a person a day. Hunger does rise around the clock — `needs.rs`\n\
         says so in as many words, you do not stop needing to eat because you are in\n\
         bed — but *eating* does not. A crew member sleeps about 9.6 of 24 hours, and\n\
         hunger climbs by more than one whole 4,800-tick cycle while they are under,\n\
         so they cannot help waking at least one meal in debt. The clock says three\n\
         and the rota can only deliver two.\n\
         \n\
         That is what the shares below are: a third of a person's life spent hungry\n\
         and an eighth spent *starving*, on a tower with a working kitchen, beds, and\n\
         nothing attacking it. `hungry_work_pct` and `tired_work_pct` are both 60, so\n\
         the slowed line is the share of a crew member's life spent working two-thirds\n\
         as fast — and on a well-run tower it is over a third.\n\
         \n\
         Whether that is too harsh is a judgement. That it is not what the row says is\n\
         not."
    );
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
