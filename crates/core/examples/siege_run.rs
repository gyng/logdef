//! Play a run and print what the jungle did to it.
//!
//! ```text
//! cargo run -p understory-core --example siege_run
//! ```
//!
//! M2's sprint question — does combat-as-logistics-stress produce drama
//! without an aimed weapon? — is a question about a curve, not about a
//! single tick, and it is not one the unit tests can answer. This is
//! the instrument for it, the way `throughput.rs` was the instrument
//! for M1's elevator question.
//!
//! It plays three towers side by side over five in-game days, all from
//! the same seed and the same starting position, differing only in how
//! their owner answers the jungle:
//!
//! - **subsistence** — one arm, never expands. Should barely be noticed.
//! - **greedy** — a second cutter arm and nothing else. Provokes, and
//!   has to live with it.
//! - **answered** — the second arm, plus a mill, a thornwright and a
//!   dart battery to shoot back with.
//!
//! What the numbers have to show for the answer to be yes: subsistence
//! is quiet, greedy is under real and increasing pressure but is not
//! simply destroyed, and answered is meaningfully better off than
//! greedy for the poles it spent.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;
use understory_core::systems::siege::tower_integrity_permille;

const SEED: u64 = 0x0000_5EED_0000_0002;
const TICKS_PER_DAY: u32 = 14_400;
const DAYS: u32 = 5;

fn main() {
    for plan in [Plan::Subsistence, Plan::Greedy, Plan::Answered] {
        run(plan);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Plan {
    Subsistence,
    Greedy,
    Answered,
}

impl Plan {
    fn name(self) -> &'static str {
        match self {
            Self::Subsistence => "subsistence",
            Self::Greedy => "greedy",
            Self::Answered => "answered",
        }
    }
}

fn run(plan: Plan) {
    let mut engine = GameEngine::new(SEED);
    engine.set_speed(SimSpeed::X1);

    // A minute to get the chain turning before anyone builds anything.
    engine.step(1800);

    // A shopping list, worked through in order as poles allow. Buying
    // the whole plan at tick 1800 is not something a player could do —
    // the first attempt at this harness tried, silently could not
    // afford the third item, and spent five days comparing a tower with
    // a dart battery against a tower that had failed to build one.
    let mut list: Vec<&str> = match plan {
        Plan::Subsistence => vec![],
        Plan::Greedy => vec!["room.cutter_arm"],
        Plan::Answered => vec![
            "room.cutter_arm",
            "room.dart_battery",
            "room.mill",
            "room.thornwright",
        ],
    };
    // Two storerooms at the end of every list: somewhere for the chain
    // to put things, and a standing reason to want poles for something
    // other than repairs.
    list.push("room.storeroom");
    list.push("room.storeroom");
    list.reverse();

    println!("\n=== {} ===", plan.name());
    println!(" day  prov  standing  poles  mended  spent  repelled  bill  built");

    // Look the index up rather than assuming one. Items are interned
    // in sorted-id order, so `ItemIdx(0)` is bamboo, and a harness that
    // guesses reports the wrong column with total confidence.
    let poles = engine
        .content()
        .item_idx("item.poles")
        .expect("the pack defines poles");
    let darts = engine.content().item_idx("item.darts");
    let mut last_mended = 0;
    let mut last_spent = 0;
    let mut last_repelled = 0;
    let mut deferred = 0;

    for day in 1..=DAYS {
        for _ in 0..12 {
            engine.step(TICKS_PER_DAY / 12);
            let Some(next) = list.last().copied() else {
                continue;
            };
            if build_anywhere(&mut engine, next) {
                list.pop();
            } else {
                deferred += 1;
            }
        }

        {
            let content = engine.content().clone();
            let state = engine.state();
            if let Some(darts) = darts {
                let on_rack: i64 = state
                    .tower
                    .floors
                    .iter()
                    .flat_map(|f| f.rooms.iter())
                    .filter(|r| content.room(r.def).defence.is_some())
                    .flat_map(|r| r.inputs.iter())
                    .filter(|s| s.item == darts)
                    .map(|s| s.count)
                    .sum();
                let made = state.stock_of(darts);
                let busy: Vec<String> = state
                    .crew
                    .iter()
                    .map(|c| {
                        format!(
                            "{:?}/{}",
                            c.state,
                            if c.repair.is_some() {
                                "job"
                            } else if c.task.is_some() {
                                "task"
                            } else {
                                "-"
                            }
                        )
                    })
                    .collect();
                let mut hurt = Vec::new();
                for floor in &state.tower.floors {
                    if floor.panel.is_hurt() {
                        hurt.push(format!(
                            "panel{}={}/{}",
                            floor.index, floor.panel.hp, floor.panel.max
                        ));
                    }
                    for room in &floor.rooms {
                        if room.health.is_hurt() {
                            hurt.push(format!(
                                "{}@{}.{}={}/{}",
                                content.room(room.def).short,
                                floor.index,
                                room.slot,
                                room.health.hp,
                                room.health.max
                            ));
                        }
                    }
                }
                for shaft in &state.tower.shafts {
                    if shaft.health.is_hurt() {
                        hurt.push(format!(
                            "shaft{}={}/{}",
                            shaft.id.0, shaft.health.hp, shaft.health.max
                        ));
                    }
                }
                println!("        darts {on_rack} on the rack, {made} in store");
                println!("        crew {busy:?}");
                if !hurt.is_empty() {
                    println!("        hurt {hurt:?}");
                }
            }
        }
        let state = engine.state();
        let mended = state.stats.hp_repaired;
        let spent = state.stats.repair_poles_spent;
        let repelled = state.siege.repelled;
        println!(
            "{day:4}  {:4}  {:7}‰  {:5}  {:6}  {:5}  {:8}  {:4}  {:5}",
            state.siege.provocation,
            tower_integrity_permille(state),
            state.stock_of(poles),
            mended - last_mended,
            spent - last_spent,
            repelled - last_repelled,
            understory_core::systems::repair::outstanding_repair_cost(state, engine.content()),
            rooms(&engine),
        );
        last_mended = mended;
        last_spent = spent;
        last_repelled = repelled;
    }

    let state = engine.state();
    println!(
        "  ended {} ‰ standing, {} creature(s) seen off, {} pole(s) banked, \
         {} of 60 checks could not afford the next thing on the list",
        tower_integrity_permille(state),
        state.siege.repelled,
        state.stock_of(poles),
        deferred,
    );
    if state.siege.lost {
        println!("  the Heartseed is gone");
    }
}

fn try_build(engine: &mut GameEngine, room: &str, floor: u8, slot: u8) -> bool {
    engine
        .try_send(GameCommand::PlaceRoom {
            room: room.into(),
            floor,
            slot,
        })
        .is_ok()
}

/// Put a room in the first place it will go.
///
/// Hard-coded coordinates made this harness lie: the thornwright it
/// thought it had built had in fact been refused for a slot clash, so
/// the "answered" tower had a dart battery and no dart supply, and the
/// comparison it was drawing was meaningless.
fn build_anywhere(engine: &mut GameEngine, room: &str) -> bool {
    let floors = engine.state().tower.floors.len() as u8;
    let slots = engine.content().balance.tower.floor_slots;
    for floor in 0..floors {
        for slot in 0..slots {
            if try_build(engine, room, floor, slot) {
                return true;
            }
        }
    }
    false
}

fn rooms(engine: &GameEngine) -> usize {
    engine
        .state()
        .tower
        .floors
        .iter()
        .map(|floor| floor.rooms.len())
        .sum()
}
