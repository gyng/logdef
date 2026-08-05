//! Test suite, organised by the thing under test rather than by file.
//!
//! Everything here goes through the public surface: commands in, state
//! and snapshots out. Tests that reach into private internals rot the
//! moment the internals move, and the internals are going to move a
//! lot between here and M5.

mod balance_doc;
mod commands;
mod determinism;
mod haul;
mod journey;
mod needs;
mod power;
mod production;
mod replay;
mod siege;
mod snapshot;
mod transport;
mod world;

use std::sync::Arc;

use crate::content::Content;
use crate::engine::GameEngine;
use crate::ids::ItemIdx;
use crate::state::GameState;

/// Shared content pack. Loading parses every RON file, so tests share
/// one rather than each paying for it.
pub(crate) fn content() -> Arc<Content> {
    Arc::new(Content::load_embedded().expect("embedded pack must load"))
}

/// **The shipped opening tower: two floors, three crew, a bed and a
/// shelf.** What a new player actually starts with.
///
/// Almost nothing wants this. Use [`engine`].
pub(crate) fn opening(seed: u64) -> GameEngine {
    GameEngine::with_content(seed, content())
}

/// A tower past the opening — the fixture nearly every test wants.
///
/// **M6 cut the starting tower down to a bed and a shelf** (§6.11), so
/// what used to arrive pre-placed is now something the player builds
/// through the farm → cutter → burner ladder. That is the right opening
/// and the wrong fixture: a test about hauling, production, sieges or
/// crew needs a tower that *works*, and rebuilding one in ninety test
/// bodies would have said nothing about any of them.
///
/// So this walks the ladder once, through the real commands and the
/// real gate, and hands back the tower the old `engine()` used to
/// return: four floors, a chain, and power. Stock is granted rather
/// than earned, because how long a tower takes to afford a mill is
/// `examples/prices.rs`'s question.
///
/// Tests that are *about* the opening use [`opening`] instead.
pub(crate) fn engine(seed: u64) -> GameEngine {
    let mut game = GameEngine::with_content(seed, content());
    // **Three more floors, not two.** Four was the old starting
    // height and the obvious target, but the garden is `top_floor_only`
    // and has to live on the roof, and a dozen tests place rooms on
    // floor 3 by name. Growing one higher gives the fixture a roof of
    // its own and hands floor 3 back to them.
    for _ in 0..3 {
        stock_item(&mut game, "item.poles", 12);
        game.try_send(crate::command::GameCommand::BuildFloor)
            .expect("a paid-for floor should go up");
    }
    // In ladder order, because the gate is real: the garden opens the
    // cutter arm, the cutter arm opens the burner, and the burner opens
    // everything else. A fixture that could skip that would not be
    // exercising the rule the game ships.
    // **Floor 1 is left entirely clear**, which is not tidiness: a
    // salvage rig is three wide and `max_floor` 1, and floor 0 has the
    // Heartseed across slots 1-3, so floor 1 is the only place in a
    // four-floor tower one can stand. Seven tests place one there.
    //
    // Floor 3 is left clear for the same reason: it is where tests put
    // storerooms, bunks and second gardens by name, and the fixture's
    // own roof-only room lives a floor above it.
    //
    // **The burner is up there too, and that is not tidiness either.**
    // A burner's inbox competes for bamboo with the mill's, and
    // `find_destination` feeds the emptiest — so a burner inside a
    // dumbwaiter's span quietly wins every load and the shaft test that
    // was about *which destination a shaft prefers* is decided by a
    // third room nobody mentioned. Switching it off is not the answer:
    // a dumbwaiter draws `charge_per_floor`, so a tower with no burner
    // runs the shaft flat instead.
    //
    // The storeroom cannot join the rig on floor 1: the shipped bunk
    // holds slots 1-2 there, and a two-wide storeroom would then have
    // to take 3-4 and break the rig's gap. It goes on floor 2, and the
    // dumbwaiter tests span floor 0 to floor 2 to reach it.
    for (room, floor, slot) in [
        ("room.garden", 4u8, 1u8),
        ("room.cutter_arm", 1, 8),
        ("room.burner", 3, 5),
        ("room.mill", 2, 3),
        ("room.storeroom", 2, 1),
        ("room.cell_bank", 4, 3),
    ] {
        // **Exactly this room's cost, not a pile of poles.** A shelf
        // holds one kind and the opening tower has four of them, so
        // stocking generously claims every shelf and the next build
        // fails for want of a shelf rather than for want of money —
        // which is how a cell bank came back `InsufficientStock` on
        // charge cells with poles stacked to the ceiling.
        stock_for(&mut game, room, 1);
        let placed = game.try_send(crate::command::GameCommand::PlaceRoom {
            room: room.into(),
            floor,
            slot,
        });
        assert!(placed.is_ok(), "{room} at {floor}.{slot}: {placed:?}");
    }
    // **Nobody is posted to the farm, so the farm does not run.** It
    // is here because the gate needs it standing before a cutter arm
    // can be built, not because this fixture wants produce — and
    // posting two of three crew to it would quietly take two thirds of
    // the tower's hands away from hauling, which is the thing most of
    // these tests are actually measuring. A test that wants a working
    // garden stations somebody itself.
    game
}

pub(crate) fn item(content: &Content, id: &str) -> ItemIdx {
    content
        .item_idx(id)
        .unwrap_or_else(|| panic!("content pack must define {id}"))
}

/// Step with the jungle held off.
///
/// A test that measures the economy — throughput, craft rates — is not
/// a test about sieges, and a wave chewing through one of two otherwise
/// identical towers turns a measurement into a coin flip. Provocation
/// is zeroed and anything already attached is cleared, in short chunks
/// so neither can build up in between.
pub(crate) fn step_quietly(game: &mut GameEngine, ticks: u32) {
    let mut left = ticks;
    while left > 0 {
        // **And forks answered**, for the same reason `step_walking`
        // exists: a tower held at a junction is not a quiet tower, it is
        // a stopped one, and this helper's whole job is to measure a
        // running economy without a wave in it.
        if game
            .state()
            .world
            .fork
            .is_some_and(|fork| fork.answer.is_none())
        {
            let _ = game.try_send(crate::command::GameCommand::TakeFork { branch: 0 });
        }
        let chunk = left.min(100);
        game.step(chunk);
        let siege = &mut game.state_mut_for_test().siege;
        siege.provocation = 0;
        siege.provocation_acc = 0;
        siege.enemies.clear();
        left -= chunk;
    }
}

/// Step, answering any fork before it can halt the tower.
///
/// **A parked tower harvests nothing, and a test that parks one is
/// measuring a fork.** `SYSTEMS.md` §3.9 makes an unanswered fork a
/// halt, and every instrument in `examples/` has answered them from the
/// start — but a handful of tests stepped naked for ten or twenty
/// thousand ticks and got away with it, because forks were rare enough
/// to fall outside the window. That was an accident of
/// `fork_interval_paces`, not a property of the tests, and it stopped
/// being true the moment the journey was shortened: three tests went
/// from passing to "drew no attention at all" and "0 crafts without, 0
/// with", each of them describing a tower standing still at a junction.
///
/// Anything stepping far enough to reach one should use this.
pub(crate) fn step_walking(game: &mut GameEngine, ticks: u32) {
    let mut left = ticks;
    while left > 0 {
        if game
            .state()
            .world
            .fork
            .is_some_and(|fork| fork.answer.is_none())
        {
            let _ = game.try_send(crate::command::GameCommand::TakeFork { branch: 0 });
        }
        let chunk = left.min(300);
        game.step(chunk);
        left -= chunk;
    }
}

/// Put poles straight onto the shelves.
///
/// Tests about elevators, cell banks and batteries need money, not a
/// simulated economy. Earning it by stepping six thousand ticks made
/// them slow, and — once the siege landed and repair started competing
/// for the same poles — flaky for reasons that had nothing to do with
/// what they were testing.
pub(crate) fn stock_poles(game: &mut GameEngine, amount: i64) {
    stock_item(game, "item.poles", amount);
    // **And a little rope, from M5.**
    //
    // The name is now a small lie and the job is unchanged: this is how
    // a test says "the tower has money". Until M5 money was poles,
    // because every build cost was poles; now every shaft and every
    // emplacement costs rope as well, and nine tests that stocked poles
    // and placed a dart battery silently stopped placing a dart battery
    // — every one of them about something else entirely.
    //
    // **Eight, not `amount`, and only rope.** A shelf holds one kind and
    // the starting tower has four of them, so stocking four materials at
    // full generosity claims every shelf and deadlocks the chain this
    // helper exists to keep running — which is the `storeroom` row's
    // failure mode, reproduced by the fixture rather than by the game.
    // Eight rope is more than any single build needs and leaves half the
    // storeroom for bamboo. Mechanisms and cells are deliberately absent:
    // only the elevator and the cell bank want them, and those two tests
    // ask for them by name.
    stock_item(game, "item.rope", 8);
}

/// Take every weapon out of the tower.
///
/// **M6 gave the tower a thorn gun to set out with** (`SYSTEMS.md`
/// §6.13), which is right for a player and wrong for a test that is
/// measuring what happens to an *undefended* tower — a creature that is
/// shot before it can chew is a creature that proves nothing about
/// chewing. Four tests started measuring the gun instead of their own
/// subject the moment it existed.
///
/// Also takes the melee out of the cutter arm, which is the same
/// argument: an arm is a blade, and a tower with one is not undefended.
pub(crate) fn disarm(game: &mut GameEngine) {
    let content = content();
    let state = game.state_mut_for_test();
    for floor in &mut state.tower.floors {
        floor.rooms.retain(|room| {
            let def = content.room(room.def);
            def.defence.is_none() && content.room_rt(room.def).melee_damage == 0
        });
    }
}

/// Post `count` crew to the room at `floor`.`slot`.
///
/// The farm is the one room with `crew_required`, so a test that wants
/// produce has to staff it — see `SYSTEMS.md` §6.11. Returns how many
/// were actually posted.
pub(crate) fn staff(game: &mut GameEngine, floor: u8, slot: u8, count: usize) -> usize {
    let Some(room) = game
        .state()
        .tower
        .find_room(floor, slot)
        .map(|room| room.id)
    else {
        return 0;
    };
    let crew: Vec<crate::ids::CrewId> = game.state().crew.iter().map(|member| member.id).collect();
    let mut posted = 0;
    for who in crew.into_iter().take(count) {
        if game
            .try_send(crate::command::GameCommand::StationCrew {
                crew: who,
                room: Some(room),
                until_tired: false,
            })
            .is_ok()
        {
            posted += 1;
        }
    }
    posted
}

/// Put `amount` of any item on whatever shelves will take it.
///
/// **`stock_poles` stopped being enough at M5.** Build costs have taken
/// a list of materials since M0 and in practice everything cost poles,
/// so every test that wanted a room stocked poles and placed it. From
/// M5 a shaft costs rope, an emplacement costs rope, and the elevator
/// costs mechanics as well — so a test that stocks poles and places a
/// dart battery is a test that silently stopped placing a dart battery.
/// Four of them did exactly that, which is why this exists.
pub(crate) fn stock_item(game: &mut GameEngine, id: &str, amount: i64) {
    let idx = item(game.content(), id);
    let state = game.state_mut_for_test();
    let mut remaining = amount;
    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            remaining -= room.shelve(idx, remaining);
            if remaining == 0 {
                return;
            }
        }
    }
}

/// Stock everything a shaft's build cost names, with slack.
///
/// The elevator became a tier-two building at M5 — 18 poles, 6 rope and
/// 2 mechanisms — so every test that builds one has to be handed parts
/// it has no chain for.
/// Can the shelves pay for this room right now?
pub(crate) fn can_afford(game: &GameEngine, room: &str) -> bool {
    let content = game.content();
    let Some(idx) = content.room_idx(room) else {
        return false;
    };
    content
        .room_rt(idx)
        .build_cost
        .iter()
        .all(|(item, amount)| game.state().stock_of(*item) >= *amount)
}

pub(crate) fn stock_for_shaft(game: &mut GameEngine, shaft: &str, times: i64) {
    let costs: Vec<(String, i64)> = game
        .content()
        .shafts
        .iter()
        .find(|def| def.id == shaft)
        .map(|def| {
            def.build_cost
                .iter()
                .map(|entry| (entry.item.clone(), entry.amount))
                .collect()
        })
        .unwrap_or_default();
    // **Scarcest first, because a shelf holds one kind.** The starting
    // tower has four shelves and a build cost can name three materials;
    // stocking the bulky one first spreads it across two or three
    // shelves and leaves the two-unit item nowhere to go. Reversing puts
    // the small, rare things on a shelf while there is still a shelf.
    for (id, amount) in costs.into_iter().rev() {
        stock_item(game, &id, amount * times.max(1));
    }
}

/// Stock everything a room's build cost names, with slack.
///
/// The honest replacement for "stock some poles and hope": it reads the
/// cost off the content pack, so a test keeps working when a designer
/// adds a material to a room it was never about.
pub(crate) fn stock_for(game: &mut GameEngine, room: &str, times: i64) {
    let costs: Vec<(String, i64)> = game
        .content()
        .rooms
        .iter()
        .find(|def| def.id == room)
        .map(|def| {
            def.build_cost
                .iter()
                .map(|entry| (entry.item.clone(), entry.amount))
                .collect()
        })
        .unwrap_or_default();
    for (id, amount) in costs {
        stock_item(game, &id, amount * times.max(1));
    }
}

/// Total count of an item anywhere in the tower or in a crew member's
/// hands. Used to assert conservation: hauling never creates or
/// destroys.
pub(crate) fn total_in_flight(state: &GameState, target: ItemIdx) -> i64 {
    let in_rooms: i64 = state
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .map(|room| {
            let inputs: i64 = room
                .inputs
                .iter()
                .filter(|s| s.item == target)
                .map(|s| s.count)
                .sum();
            let outputs: i64 = room
                .outputs
                .iter()
                .filter(|s| s.item == target)
                .map(|s| s.count)
                .sum();
            let shelves: i64 = room
                .shelves
                .iter()
                .filter(|s| s.item == Some(target))
                .map(|s| s.count)
                .sum();
            inputs + outputs + shelves
        })
        .sum();

    let carried: i64 = state
        .crew
        .iter()
        .filter_map(|member| member.carrying)
        .filter(|(held, _)| *held == target)
        .map(|(_, count)| count)
        .sum();

    in_rooms + carried
}
