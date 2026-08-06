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

/// Stand up whatever feeds this weapon, so it can be built.
///
/// **Every emplacement is unlocked by the room that supplies it**
/// (`SYSTEMS.md` §6.27) — a weapon you cannot feed is a weapon that
/// never fires, so the menu offers it once the tower can make its
/// ammunition. A fixture that places a battery therefore has to place a
/// thornwright, which is the same sentence a player reads.
///
/// Does nothing for a weapon with no gate, which is the thorn gun: it
/// eats raw bamboo and is the one a tower can always feed.
pub fn open_the_armoury(game: &mut GameEngine, weapon: &str) {
    let needs = {
        let content = game.content();
        content
            .room_idx(weapon)
            .and_then(|idx| content.room_rt(idx).unlocked_by)
            .map(|idx| game.content().room(idx).id.clone())
    };
    let Some(needs) = needs else { return };
    if game
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .any(|room| game.content().room(room.def).id == needs)
    {
        return;
    }
    open_the_armoury(game, &needs);
    let cost: Vec<(String, i64)> = {
        let content = game.content();
        let idx = content
            .room_idx(&needs)
            .unwrap_or_else(|| panic!("the pack should define {needs}"));
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
        place_anywhere(game, &needs),
        "could not place {needs}, which is what feeds {weapon}"
    );
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
    let width = {
        let content = game.content();
        content
            .room_idx(room)
            .map_or(1, |idx| content.room(idx).width)
    };
    // **The tower's width, not the pack's.** These were the same number
    // until `WidenTower` (`SYSTEMS.md` §6.16), and reading the constant
    // on a widened hull puts the whole front of the tower out of reach:
    // the front is `slots - width`, so a `front_only` cutter arm on a
    // twelve-wide tower wants column 10 and this helper never offered
    // one past 8. It failed as "could not place room.cutter_arm: the
    // opening ladder is stuck at it", which reads as a content problem
    // and was an arithmetic one.
    let slots = game.state().tower.floors.first().map_or_else(
        || game.content().balance.tower.floor_slots,
        |floor| floor.slots,
    );
    // **Column 7 is the shaft's, and the edge is the weapons'.**
    //
    // This used to reserve the *last* column, which was the same thing
    // while a floor was eight wide. M6 widened it to ten and made
    // weapons `front_only` (`SYSTEMS.md` §6.13), so the two are now
    // different columns: 8 and 9 are where a weapon has to go, and a
    // harness that refuses to use them cannot build a cutter arm at
    // all. Reserving one named column instead keeps shafts buildable
    // and lets weapons reach their edge.
    //
    // Derived rather than named, for the same reason: widening slides
    // everything forward, so the third-from-the-front column *is* the
    // convention and 7 was only ever what that came to on a ten-wide
    // floor.
    let shaft_column = slots.saturating_sub(3);
    let last = slots.saturating_sub(width);
    let floors = game.state().tower.floors.len() as u8;
    // **From the top down.** The scarce floors are the low ones: a
    // cutter arm, a fiber comb and a salvage rig all carry `max_floor`
    // 1 because they reach the ground, and floor 0 already has three of
    // its slots under the Heartseed. Filling upward from the bottom
    // spends that scarcity on rooms that could have gone anywhere, and
    // the next ground-reaching room then has nowhere to stand.
    for floor in (0..floors).rev() {
        for slot in 0..=last {
            if slot <= shaft_column && slot + width > shaft_column {
                continue;
            }
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

/// Mean, low and high of one column across a swept set of runs.
///
/// **Every instrument that quotes a single seed has been wrong at least
/// once** (`SYSTEMS.md` §6.34, and the three commits after it). The ones
/// swept so far all published the *worst* seed as though it were the
/// figure, `haulcycle.rs`'s idle share turned out to run 0.0-9.1%, and
/// four of `charge.rs`'s nine rows change **sign** depending on the
/// terrain the tower happened to walk.
///
/// So this lives here rather than being written a fourth time. A mean
/// on its own is the thing that went wrong; the range is what makes it
/// readable, and a caller that prints both cannot quietly publish a
/// point estimate.
///
/// Returns `(mean, low, high)`. An empty slice gives all zeroes rather
/// than a panic — a sweep with no seeds is a caller bug, but not one
/// worth taking an instrument down for.
pub fn span<T>(each: &[T], get: impl Fn(&T) -> f64) -> (f64, f64, f64) {
    if each.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let vals: Vec<f64> = each.iter().map(get).collect();
    let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (vals.iter().sum::<f64>() / vals.len() as f64, lo, hi)
}
