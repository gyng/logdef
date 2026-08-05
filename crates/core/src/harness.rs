//! Helpers the instruments in `examples/` share.
//!
//! **Nothing that ships to a player may call this.** It exists for the
//! same reason `GameEngine::state_mut_for_test` is `pub`: `examples/`
//! are separate crates, so a helper every instrument needs cannot live
//! beside them without being copied six times — and six copies of a
//! setup step is how three of those instruments came to be measuring
//! nothing at all (`AGENTS.md` §II).

use crate::command::{CommandError, GameCommand};
use crate::engine::GameEngine;

/// Put `amount` of an item onto whatever shelves will take it.
pub fn give(game: &mut GameEngine, id: &str, amount: i64) {
    let Some(idx) = game.content().item_idx(id) else {
        return;
    };
    let state = game.state_mut_for_test();
    let mut left = amount;
    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            left -= room.shelve(idx, left);
            if left <= 0 {
                return;
            }
        }
    }
}

/// Walk the opening ladder, so the rest of the build menu exists.
///
/// **M6 cut the starting tower to a Heartseed and a bed** (`SYSTEMS.md`
/// §6.11): the farm opens the cutter arm, the cutter arm opens the
/// burner, and the burner opens everything else. A harness that placed
/// a canteen on turn one used to get a tower; it now gets
/// `CommandError::Locked`, which is the rule working and the instrument
/// measuring nothing.
///
/// Grows the tower to `floors` first, then places the three ladder
/// rooms wherever they fit, paying from granted stock — how long a
/// tower takes to *afford* the ladder is `examples/prices.rs`'s
/// question and not something every other instrument should re-answer.
///
/// Panics if a rung cannot be placed, because an instrument that
/// silently skipped one would be measuring the tower it failed to
/// build. That exact silence is the repo's most-repeated bug.
pub fn open_the_ladder(game: &mut GameEngine, floors: u8) {
    while (game.state().tower.floors.len() as u8) < floors {
        give(game, "item.poles", 12);
        game.try_send(GameCommand::BuildFloor)
            .expect("a paid-for floor should go up");
    }

    for room in ["room.garden", "room.cutter_arm", "room.burner"] {
        let cost: Vec<(String, i64)> = {
            let content = game.content();
            let idx = content
                .room_idx(room)
                .unwrap_or_else(|| panic!("the pack should define {room}"));
            content
                .room_rt(idx)
                .build_cost
                .iter()
                .map(|(item, n)| (content.item(*item).id.clone(), *n))
                .collect()
        };
        for (item, n) in cost {
            give(game, &item, n * 2);
        }
        if !place_anywhere(game, room) {
            panic!("could not place {room}: the opening ladder is stuck at it");
        }
    }
}

/// The ladder, plus the chain the old starting tower used to arrive
/// with: a mill, a storeroom and a cell bank.
///
/// Most instruments were written against a tower that already had all
/// of this in it. They are measuring throughput, needs, prices or
/// waves, not the opening, so they want it handed over rather than
/// earned — and every one of them would otherwise have to grow the same
/// six rooms in the same order.
pub fn chain_tower(game: &mut GameEngine, floors: u8) {
    open_the_ladder(game, floors);
    for room in ["room.mill", "room.storeroom", "room.cell_bank"] {
        let cost: Vec<(String, i64)> = {
            let content = game.content();
            let idx = content
                .room_idx(room)
                .unwrap_or_else(|| panic!("the pack should define {room}"));
            content
                .room_rt(idx)
                .build_cost
                .iter()
                .map(|(item, n)| (content.item(*item).id.clone(), *n))
                .collect()
        };
        for (item, n) in cost {
            give(game, &item, n * 2);
        }
        assert!(
            place_anywhere(game, room),
            "could not place {room}: this harness has no working chain"
        );
    }
}

/// Try every floor and slot until one takes the room.
///
/// Reports whether it landed, and every caller here checks — a
/// place-a-room helper returning a silent `false` is what made
/// `siege_run.rs` compare a tower against a byte-identical copy of
/// itself twice.
///
/// **Leaves the outboard column alone**, which is the same convention
/// the shipped starting layout keeps: a shaft needs one free slot on
/// every floor it spans, the outer edge is where one naturally goes,
/// and a harness that fills it makes its own elevator unbuildable.
/// `throughput.rs` died exactly that way — "could not raise the
/// elevator: floor 0 slot 7 is already occupied" — with the room in the
/// way put there by this function.
pub fn place_anywhere(game: &mut GameEngine, room: &str) -> bool {
    // The room's own width has to come into this, not just the starting
    // slot: a two-wide room put down at slot 6 of eight covers 6 *and*
    // 7, so skipping "slot 7" alone reserved nothing at all and the
    // elevator was still blocked.
    let (slots, width) = {
        let content = game.content();
        let width = content
            .room_idx(room)
            .map_or(1, |idx| content.room(idx).width);
        (content.balance.tower.floor_slots, width)
    };
    let last = slots.saturating_sub(width).saturating_sub(1);
    let floors = game.state().tower.floors.len() as u8;
    // **From the top down.** The scarce floors are the low ones: a
    // cutter arm, a fiber comb and a salvage rig all carry `max_floor`
    // 1 because they reach the ground, and floor 0 already has three of
    // its slots under the Heartseed. Filling upward from the bottom
    // spends that scarcity on rooms that could have gone anywhere, and
    // the next ground-reaching room then has nowhere to stand.
    for floor in (0..floors).rev() {
        for slot in 0..=last {
            match game.try_send(GameCommand::PlaceRoom {
                room: room.into(),
                floor,
                slot,
            }) {
                Ok(()) => return true,
                // Out of money is worth stopping for; a slot that does
                // not suit is not.
                Err(CommandError::InsufficientStock { .. }) => return false,
                Err(_) => {}
            }
        }
    }
    false
}
