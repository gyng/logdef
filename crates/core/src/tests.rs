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
    let poles = item(game.content(), "item.poles");
    let state = game.state_mut_for_test();
    let mut remaining = amount;
    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            remaining -= room.shelve(poles, remaining);
            if remaining == 0 {
                return;
            }
        }
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
