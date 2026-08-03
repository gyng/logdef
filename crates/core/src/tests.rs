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

pub(crate) fn engine(seed: u64) -> GameEngine {
    GameEngine::with_content(seed, content())
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
        let chunk = left.min(100);
        game.step(chunk);
        let siege = &mut game.state_mut_for_test().siege;
        siege.provocation = 0;
        siege.provocation_acc = 0;
        siege.enemies.clear();
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
