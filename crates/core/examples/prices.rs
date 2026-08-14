//! When does the opening actually unlock, afford, build and use each rung?
//!
//! This instrument starts from the untouched shipped tower. It never
//! grants stock and it buys every prerequisite legally. The previous
//! version began with `harness::chain_tower`, whose helpers grant twice
//! every build cost; after the Cell Bank acquired cell and mechanism
//! costs, that made deep machinery appear affordable at tick zero.
//!
//! Two policies make the Garden's opportunity cost visible: transport
//! first builds the useful four-floor lift before the resin branch;
//! resin first builds the Garden before saving the frame. Each stage is
//! reported as an eight-seed range. `built` without `useful` is a setup
//! failure, not evidence that the purchase works.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::content::Content;
use understory_core::ids::{ItemIdx, RoomIdx};
use understory_core::state::CrewState;

const SEEDS: [u64; 8] = [0x0BE1_9CE5, 1, 2, 3, 4, 5, 6, 7];
const WINDOW: u64 = 36 * 60 * 30;
const STEP: u32 = 30;

#[derive(Clone, Copy)]
enum Kind {
    Room(&'static str),
    Lift,
}

impl Kind {
    fn id(self) -> &'static str {
        match self {
            Self::Room(id) => id,
            Self::Lift => "shaft.elevator",
        }
    }
}

const TARGETS: [Kind; 8] = [
    Kind::Room("room.cutter_arm"),
    Kind::Room("room.burner"),
    Kind::Room("room.mill"),
    Kind::Room("room.fiber_comb"),
    Kind::Room("room.storeroom"),
    Kind::Room("room.ropery"),
    Kind::Room("room.garden"),
    Kind::Lift,
];

#[derive(Clone, Copy, Default)]
struct Stamp {
    unlocked: Option<u64>,
    affordable: Option<u64>,
    built: Option<u64>,
    useful: Option<u64>,
}

#[derive(Clone, Copy)]
enum Policy {
    TransportFirst,
    ResinFirst,
}

impl Policy {
    fn label(self) -> &'static str {
        match self {
            Self::TransportFirst => "transport first",
            Self::ResinFirst => "resin first",
        }
    }

    fn order(self) -> [Kind; 8] {
        let core = [
            Kind::Room("room.cutter_arm"),
            Kind::Room("room.burner"),
            Kind::Room("room.mill"),
            Kind::Room("room.fiber_comb"),
            Kind::Room("room.storeroom"),
            Kind::Room("room.ropery"),
        ];
        match self {
            Self::TransportFirst => [
                core[0],
                core[1],
                core[2],
                core[3],
                core[4],
                core[5],
                Kind::Lift,
                Kind::Room("room.garden"),
            ],
            Self::ResinFirst => [
                core[0],
                core[1],
                core[2],
                core[3],
                core[4],
                core[5],
                Kind::Room("room.garden"),
                Kind::Lift,
            ],
        }
    }
}

fn main() {
    let content = Content::load_embedded().expect("the shipped pack should load");
    println!("=== opening progression: untouched tower, legal purchases, eight seeds ===\n");
    for policy in [Policy::TransportFirst, Policy::ResinFirst] {
        let runs: Vec<Vec<Stamp>> = SEEDS
            .iter()
            .map(|&seed| run(seed, policy, &content))
            .collect();
        println!("{}", policy.label());
        println!(
            "  {:<19} {:>13} {:>13} {:>13} {:>13}",
            "rung", "unlocked", "affordable", "built", "useful"
        );
        for (index, target) in TARGETS.iter().enumerate() {
            println!(
                "  {:<19} {:>13} {:>13} {:>13} {:>13}",
                target
                    .id()
                    .trim_start_matches("room.")
                    .trim_start_matches("shaft."),
                range(&runs, index, |stamp| stamp.unlocked),
                range(&runs, index, |stamp| stamp.affordable),
                range(&runs, index, |stamp| stamp.built),
                range(&runs, index, |stamp| stamp.useful),
            );
        }
        println!();
    }
    println!(
        "  Times are minute ranges. Unlock is a standing prerequisite; affordable includes\n\
         the lift's per-deck frame; built is the canonical policy; useful means the room\n\
         actually handled stock/work or the lift carried a rider/freight. Garden usefulness
         requires a downstream room; this policy deliberately does not count its Ropewalk trade.
         No fixture grants."
    );
}

fn run(seed: u64, policy: Policy, content: &Content) -> Vec<Stamp> {
    let mut game = GameEngine::new(seed);
    let order = policy.order();
    let mut stamps = vec![Stamp::default(); TARGETS.len()];
    let mut next = 0usize;

    while game.state().tick < WINDOW && next < order.len() {
        answer_fork(&mut game);
        game.step(STEP);
        mark_stages(&game, content, &mut stamps);

        let target = order[next];
        let built = match target {
            Kind::Room(id) => place_reserving_lift_column(&mut game, id),
            Kind::Lift => {
                // A two-floor lift is a cheap object with no useful job.
                // Height is part of the point at which it is wanted.
                if game.state().tower.floors.len() < 4 {
                    let _ = game.try_send(GameCommand::BuildFloor);
                    false
                } else {
                    let high = game.state().tower.floors.len() as u8 - 1;
                    build_elevator_in_clear_column(&mut game, high)
                }
            }
        };
        if built {
            let tick = game.state().tick;
            if let Some(index) = target_index(target) {
                stamps[index].built.get_or_insert(tick);
            }
            next += 1;
            continue;
        }

        // This policy is explicitly measuring a useful four-floor lift,
        // so it grows those decks before widening the old ones. Widening
        // only shifts floors that already exist; widening first and then
        // adding narrow floors can leave no column shared by the whole
        // tower, which is a self-inflicted layout failure rather than an
        // acquisition price.
        if matches!(target, Kind::Room(_)) {
            if game.state().tower.floors.len() < 4 {
                let _ = game.try_send(GameCommand::BuildFloor);
            } else {
                let _ = game.try_send(GameCommand::WidenTower);
            }
        }
    }

    // Give the last purchase a minute to demonstrate that it has a job.
    for _ in 0..60 {
        answer_fork(&mut game);
        game.step(STEP);
        mark_stages(&game, content, &mut stamps);
    }
    stamps
}

fn mark_stages(game: &GameEngine, content: &Content, stamps: &mut [Stamp]) {
    let tick = game.state().tick;
    for (index, target) in TARGETS.iter().copied().enumerate() {
        let unlocked = match target {
            Kind::Room(id) => {
                room_unlocked(game, content.room_idx(id).expect("target room"), content)
            }
            Kind::Lift => game.state().tower.floors.len() >= 4,
        };
        if unlocked {
            stamps[index].unlocked.get_or_insert(tick);
        }
        if unlocked && affordable(game, target, content) {
            stamps[index].affordable.get_or_insert(tick);
        }
        if stamps[index].built.is_some() && useful(game, target, content) {
            stamps[index].useful.get_or_insert(tick);
        }
    }
}

fn room_unlocked(game: &GameEngine, room: RoomIdx, content: &Content) -> bool {
    content.room_rt(room).unlocked_by.is_none_or(|needed| {
        game.state()
            .tower
            .floors
            .iter()
            .flat_map(|floor| &floor.rooms)
            .any(|standing| standing.def == needed && !standing.is_wrecked(content))
    })
}

fn affordable(game: &GameEngine, target: Kind, content: &Content) -> bool {
    let cost: Vec<(ItemIdx, i64)> = match target {
        Kind::Room(id) => content
            .room_rt(content.room_idx(id).expect("target room"))
            .build_cost
            .clone(),
        Kind::Lift => {
            let runtime = content.shaft_rt(content.shaft_idx("shaft.elevator").expect("lift"));
            let mut cost = runtime.build_cost.clone();
            let boundaries = game.state().tower.floors.len().saturating_sub(1) as i64;
            for &(item, per) in &runtime.span_cost {
                if let Some((_, amount)) = cost.iter_mut().find(|(had, _)| *had == item) {
                    *amount += per * boundaries;
                } else {
                    cost.push((item, per * boundaries));
                }
            }
            cost
        }
    };
    cost.iter()
        .all(|(item, amount)| game.state().stock_of(*item) >= *amount)
}

fn useful(game: &GameEngine, target: Kind, content: &Content) -> bool {
    match target {
        Kind::Room("room.garden") => resin_reached_a_consumer(game, content),
        Kind::Room(id) => {
            let def = content.room_idx(id).expect("target room");
            game.state()
                .tower
                .floors
                .iter()
                .flat_map(|floor| &floor.rooms)
                .filter(|room| room.def == def)
                .any(|room| {
                    room.progress > 0
                        || room.work_acc > 0
                        || room.intake_acc != understory_core::fx::Fx::ZERO
                        || room.burning
                        || room.inputs.iter().any(|stack| stack.count > 0)
                        || room.outputs.iter().any(|stack| stack.count > 0)
                        || room.shelves.iter().any(|shelf| shelf.count > 0)
                })
        }
        Kind::Lift => {
            game.state()
                .crew
                .iter()
                .any(|crew| matches!(crew.state, CrewState::Riding { .. }))
                || game
                    .state()
                    .tower
                    .shafts
                    .iter()
                    .filter(|shaft| content.shaft(shaft.def).id == "shaft.elevator")
                    .flat_map(|shaft| &shaft.cars)
                    .any(|car| !car.riders.is_empty() || !car.freight.is_empty())
        }
    }
}

/// Resin sitting in the Garden's own outbox is inventory, not payoff.
/// The branch becomes useful only when a downstream recipe has accepted
/// it or produced one of its products. This deliberately lets the
/// instrument print `0/8` when a policy buys the optional intake but
/// never reaches a reason to have done so.
fn resin_reached_a_consumer(game: &GameEngine, content: &Content) -> bool {
    let resin = content
        .item_idx("item.resin_feedstock")
        .expect("the pack has resin");
    game.state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| &floor.rooms)
        .filter(|room| {
            content
                .room_rt(room.def)
                .recipe_inputs
                .iter()
                .any(|(item, _, _)| *item == resin)
        })
        .any(|room| {
            room.progress > 0
                || room.work_acc > 0
                || room.inputs.iter().any(|stack| stack.count > 0)
                || room.outputs.iter().any(|stack| stack.count > 0)
        })
}

fn build_elevator_in_clear_column(game: &mut GameEngine, high: u8) -> bool {
    let common_slots = game
        .state()
        .tower
        .floors
        .iter()
        .map(|floor| floor.slots)
        .min()
        .unwrap_or(0);
    for slot in 0..common_slots {
        match game.try_send(GameCommand::BuildShaft {
            shaft: "shaft.elevator".into(),
            low: 0,
            high,
            slot,
        }) {
            Ok(()) => return true,
            Err(understory_core::command::CommandError::SlotOccupied { .. }) => continue,
            Err(understory_core::command::CommandError::InsufficientStock { .. }) => return false,
            Err(error) => panic!(
                "a clear elevator search failed at slot {slot}, tick {}: {error}",
                game.state().tick
            ),
        }
    }
    let columns = (0..common_slots)
        .map(|slot| {
            let blocked = (0..=high)
                .filter(|&floor| game.state().tower.slot_range_blocked(floor, slot, 1))
                .collect::<Vec<_>>();
            format!("{slot}:{blocked:?}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let layout = game
        .state()
        .tower
        .floors
        .iter()
        .map(|floor| {
            let rooms = floor
                .rooms
                .iter()
                .map(|room| {
                    format!(
                        "{}@{}+{}",
                        game.content().room(room.def).id,
                        room.slot,
                        game.content().room(room.def).width
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("F{}[{rooms}]", floor.index)
        })
        .collect::<Vec<_>>()
        .join(" ");
    panic!(
        "the progression policy left no full-height elevator column at tick {}: {layout}; {columns}",
        game.state().tick
    );
}

/// A competent progression policy does not fill the only column it is
/// explicitly saving an elevator for. The old generic helper packed the
/// resin-first Garden through that future shaft line, then reported the
/// branch could afford an elevator but built one on 0/8 seeds. That was a
/// layout mistake in the instrument, not the price of resin.
fn place_reserving_lift_column(game: &mut GameEngine, room_id: &str) -> bool {
    let Some(def) = game.content().room_idx(room_id) else {
        return false;
    };
    let width = game.content().room(def).width;
    let floors: Vec<_> = game
        .state()
        .tower
        .floors
        .iter()
        .map(|floor| (floor.index, floor.slots))
        .collect();
    for (floor, slots) in floors.into_iter().rev() {
        let shaft_slot = slots.saturating_sub(3);
        for slot in 0..=slots.saturating_sub(width) {
            if slot <= shaft_slot && slot + width > shaft_slot {
                continue;
            }
            if game
                .try_send(GameCommand::PlaceRoom {
                    room: room_id.into(),
                    floor,
                    slot,
                })
                .is_ok()
            {
                return true;
            }
        }
    }
    false
}

fn answer_fork(game: &mut GameEngine) {
    if game
        .state()
        .world
        .fork
        .is_some_and(|fork| fork.answer.is_none())
    {
        let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
    }
}

fn target_index(target: Kind) -> Option<usize> {
    TARGETS
        .iter()
        .position(|candidate| candidate.id() == target.id())
}

fn range(runs: &[Vec<Stamp>], index: usize, read: impl Fn(Stamp) -> Option<u64>) -> String {
    let values: Vec<u64> = runs.iter().filter_map(|run| read(run[index])).collect();
    if values.len() != runs.len() {
        return format!("{}/{}", values.len(), runs.len());
    }
    let low = values.iter().min().copied().unwrap_or(0) as f64 / 1_800.0;
    let high = values.iter().max().copied().unwrap_or(0) as f64 / 1_800.0;
    if (high - low).abs() < 0.05 {
        format!("{low:.1}m")
    } else {
        format!("{low:.1}–{high:.1}m")
    }
}
