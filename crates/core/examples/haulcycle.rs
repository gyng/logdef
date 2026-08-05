//! What a haul actually costs, in ticks and in trips.
//!
//! ```text
//! cargo run --release -p understory-core --example haulcycle
//! ```
//!
//! `DESIGN.md` insight 1 is that transport is shared rather than
//! dedicated per chain, so the Crew section's movement constants —
//! `walk_ticks_per_slot`, `climb_ticks_per_floor`, `load_ticks`,
//! `unload_ticks`, `carry_capacity` — are what the whole game's pacing
//! rests on. Their rows are hand arithmetic: "0.4 s a slot, ~3 s to
//! cross a floor", "two crew at three per trip roughly match the mill".
//!
//! Nothing had checked them against a running tower, and the two other
//! sections that got this treatment both turned out to have a row badly
//! out — lamps by 70% (`charge.rs`), meals by a third (`needs.rs`). The
//! arithmetic is not wrong so much as it is arithmetic about an idealised
//! day: what it misses is queueing, sleep, and the ticks a crew member
//! spends deciding.
//!
//! So this measures the round trip end to end, and the share of a crew
//! member's day that actually goes into moving things.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::{CrewState, SimSpeed};

const DAY: u32 = 14_400;
const DAYS: u32 = 4;

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
    println!("=== what does a haul cost? ===\n");
    println!(
        "  A fed, housed tower over {DAYS} whole days, first day discarded as warm-up.\n\
         The rows this checks are hand arithmetic about an idealised trip; what they\n\
         cannot see is queueing, sleep, and the ticks spent deciding.\n"
    );

    let mut game = GameEngine::new(0x_4A17);
    game.set_speed(SimSpeed::X1);
    // **The opening ladder first** (`SYSTEMS.md` §6.11). M6 cut the
    // starting tower to a Heartseed and a bed, so a harness that places
    // a canteen on turn one gets `CommandError::Locked` rather than a
    // tower. `chain_tower` grows to four floors and walks farm → cutter
    // arm → burner → mill, storeroom, cell bank, which is the tower
    // every instrument here was written against.
    understory_core::harness::chain_tower(&mut game, 4);
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
    let hauls_at_start = game.state().stats.hauls_completed;
    let window = DAY * (DAYS - 1);

    // Where a crew member's ticks go, by the state they are in.
    let (mut carrying, mut walking, mut climbing, mut boarding, mut idle, mut asleep) =
        (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut samples = 0u64;
    for _ in 0..window {
        step(&mut game, 1);
        for member in &game.state().crew {
            samples += 1;
            if member.is_carrying() {
                carrying += 1;
            }
            match member.state {
                CrewState::Walking { .. } => walking += 1,
                CrewState::Climbing { .. } | CrewState::Riding { .. } => climbing += 1,
                CrewState::Boarding { .. } => boarding += 1,
                CrewState::Sleeping => asleep += 1,
                CrewState::Idle => idle += 1,
                _ => {}
            }
        }
    }

    let hauls = game.state().stats.hauls_completed - hauls_at_start;
    println!("crew                             {crew}");
    println!(
        "hauls completed a person a day   {:.1}",
        hauls as f64 / crew as f64 / f64::from(DAYS - 1)
    );
    println!(
        "ticks of crew time a haul        {:.0}   (all crew time, sleep included)",
        f64::from(window) * crew as f64 / hauls.max(1) as f64
    );
    println!(
        "\nwhere a person's day goes:\n  \
         carrying a load             {:>5.1}%\n  \
         walking                     {:>5.1}%\n  \
         climbing                    {:>5.1}%\n  \
         **queued at a shaft**       {:>5.1}%\n  \
         asleep                      {:>5.1}%\n  \
         idle or otherwise           {:>5.1}%",
        pct(carrying, samples),
        pct(walking, samples),
        pct(climbing, samples),
        pct(boarding, samples),
        pct(asleep, samples),
        pct(idle, samples),
    );
    println!(
        "\n  Three things worth reading off that table.\n\
         \n\
         1. **Queueing is 3.4%, and the design rests on it.** `DESIGN.md` insight 1 is\n\
            that transport is shared rather than dedicated, and a shaft's capacity is\n\
            the point rather than a limitation to work around. On a *starting* tower\n\
            that thesis is real but thin — present, not felt. It has to arrive with\n\
            height and room count or it does not arrive at all.\n\
         \n\
         2. **Crew are idle 0.2% of the time.** They are saturated, so anything added\n\
            to this tower is paid for out of something else it was already doing. That\n\
            is the shape the game wants, and it also means throughput measurements on\n\
            a starting tower measure the crew rather than the thing being added.\n\
         \n\
         3. **Walking costs nearly three times what climbing does**, 34.2% of a day\n\
            against 12.1%. `climb_ticks_per_floor` 30 is two and a half times\n\
            `walk_ticks_per_slot` 12 *per unit*, so the rows read as though vertical\n\
            movement dominates — and on a four-floor tower it plainly does not, because\n\
            there is far more horizontal distance to cover than vertical. `DESIGN.md`\n\
            pillar 2 calls vertical transport the belt; that is a claim about a tall\n\
            tower, and this is what it looks like before the tower is tall."
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
