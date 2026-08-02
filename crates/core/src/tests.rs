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
mod power;
mod production;
mod replay;
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
