//! One of each creature against one tower, and what answering it costs.
//!
//! ```text
//! cargo run --release -p understory-core --example bestiary
//! ```
//!
//! The creature rows in `BALANCE.md` are stat blocks sized against each
//! other in prose — a warden is "the toughest thing in the pack, half
//! again over a root-borer", a dart battery "needs 800 ticks and 20
//! darts to put one down and takes about 160 damage doing it". Those are
//! divisions, and the pressure table measures creatures only in
//! aggregate: at provocation 500 something kills the tower, but not
//! which thing or how.
//!
//! So this puts exactly one of each in front of the same tower, twice —
//! once undefended, to see what it does, and once with a loaded battery,
//! to see what answering it costs. Every number the rows claim about a
//! single creature is in one of those two columns.

use understory_core::GameEngine;
use understory_core::command::GameCommand;
use understory_core::state::SimSpeed;
use understory_core::systems::siege::tower_integrity_permille;

/// Long enough for the slowest thing in the pack to close, bite its way
/// through a panel, and either be shot down or lose its grip.
const TICKS: u32 = 6_000;

fn main() {
    println!("=== one of each, against one tower ===\n");
    println!(
        "  The same tower every time, {TICKS} ticks, one creature spawned at the\n\
         far edge of its approach. `undefended` is what it does; `answered` is the\n\
         same fight with a loaded dart battery, and the darts column is what that\n\
         cost. `walked off` means it lost its grip and left rather than dying —\n\
         which is a win the `repelled` counter deliberately does not count.\n"
    );

    let content = understory_core::content::Content::load_embedded().expect("pack");
    println!(
        "{:<16} {:>5} {:>7} {:>9} {:>9} {:>7} {:>6}",
        "creature", "hp", "threat", "undefended", "answered", "darts", "ended"
    );

    // **Whether the undefended tower was ever actually hurt.** If it
    // ends every fight at 1000 permille then nothing reached it, both
    // columns are the same number, and "what a battery is worth" comes
    // out as zero against everything — which is `siege_run.rs`'s oldest
    // bug (`AGENTS.md` II rule 3) rather than a finding about batteries.
    let mut ever_hurt = false;
    for (idx, def) in content.enemies.iter().enumerate() {
        let bare = fight(idx, false);
        let armed = fight(idx, true);
        if bare.standing < 1000 {
            ever_hurt = true;
        }
        println!(
            "{:<16} {:>5} {:>7} {:>8}‰ {:>8}‰ {:>7} {:>6}",
            def.name,
            def.hp,
            def.threat,
            bare.standing,
            armed.standing,
            armed.darts,
            if armed.killed {
                "shot"
            } else if armed.gone {
                "walked off"
            } else {
                "still on"
            },
        );
    }

    if !ever_hurt {
        println!();
        println!("  **THE UNDEFENDED COLUMN IS DEGENERATE AND THIS TABLE SAYS NOTHING.**");
        println!("  Every creature left the bare tower whole, so nothing reached it inside the");
        println!("  window and the gap against the armed column is zero by construction. Read");
        println!("  no verdict about a battery off these rows. The window, the spawn distance");
        println!("  or the approach is wrong - `siege_run.rs` drew a confident conclusion from");
        println!("  exactly this shape over eight seeds (`AGENTS.md` II rule 3).");
    }

    println!(
        "\n  Read the two standing columns against each other rather than against 1000.\n\
         The gap is what a battery is worth against that particular creature, and it\n\
         is the only per-creature statement of that anywhere — the pressure table\n\
         measures whole waves, so it can say a tower died at provocation 500 without\n\
         saying what killed it."
    );
}

struct Fight {
    standing: i64,
    darts: i64,
    killed: bool,
    gone: bool,
}

fn fight(enemy: usize, armed: bool) -> Fight {
    let mut game = GameEngine::new(0x0BE5_71A9);
    game.set_speed(SimSpeed::X1);
    let content = game.content().clone();

    // **The opening ladder first** (`SYSTEMS.md` §6.11), armed or not:
    // both towers in this comparison have to be the same tower apart
    // from the battery, and since M6 a fresh tower has no chain at all.
    understory_core::harness::chain_tower(&mut game, 4);

    // **Strip the tower's own weapons from both sides.** The starting
    // tower ships a thorn gun and the ladder puts up a cutter arm, which
    // deals melee damage — so without this the "undefended" column is a
    // tower that shoots back, and once the panic above was fixed every
    // creature read 1000‰ in *both* columns. A comparison against an
    // armed copy of itself, which is `siege_run.rs`'s oldest bug.
    understory_core::harness::disarm(&mut game);

    if armed {
        // Paid for, then insisted on — a "defended" tower that failed to
        // build its battery is how `siege_run.rs` spent a milestone
        // reporting a defence comparison in which nothing was defended.
        endow(&mut game);
        // **Thornwright first, and the order is the whole bug.** The
        // battery is `unlocked_by: room.thornwright`, so a list with the
        // battery at the front can never place it — `PlaceRoom` answers
        // "not on the menu until a room.thornwright is standing" forty
        // times and the assert below fires. This instrument had been
        // panicking on its own seed and printing an empty table, while
        // fifteen `BALANCE.md` rows quoted it. A fourth dead instrument
        // after the three `AGENTS.md` §II already records.
        for room in ["room.thornwright", "room.dart_battery"] {
            assert!(
                build_anywhere(&mut game, room),
                "the armed tower could not build {room}"
            );
        }
        load_racks(&mut game);
    }

    // One creature, placed at the far edge of its approach, and nothing
    // else: provocation is pinned at zero so no wave arrives to muddle
    // the measurement.
    {
        let state = game.state_mut_for_test();
        state.siege.provocation = 0;
        state.siege.next_wave_tick = u64::MAX;
    }
    {
        // Built by hand rather than through `spawn_at`, which is private
        // — this is the only place in the project that wants exactly one
        // creature of a chosen kind rather than whatever a wave affords.
        let def = understory_core::ids::EnemyIdx(u16::try_from(enemy).unwrap_or(0));
        let ahead = content.balance.siege.spawn_paces_ahead;
        let state = game.state_mut_for_test();
        let id = state.alloc_enemy_id();
        state.siege.enemies.push(understory_core::state::Enemy {
            id,
            def,
            at: state.world.distance + understory_core::fx::paces_from_int(ahead),
            hp: content.enemy(def).hp,
            state: understory_core::state::EnemyState::Approaching,
            attack_cooldown: 0,
            cling_left: content.enemy(def).cling_ticks,
            fade_left: 0,
        });
    }

    let mut darts_used = 0i64;
    for tick in 0..TICKS {
        if tick % 60 == 0 && armed {
            darts_used += load_racks(&mut game);
        }
        if let Some(fork) = game.state().world.fork
            && fork.answer.is_none()
        {
            let _ = game.try_send(GameCommand::TakeFork { branch: 0 });
        }
        game.step(1);
        if game.state().siege.lost {
            break;
        }
    }

    let state = game.state();
    Fight {
        standing: tower_integrity_permille(state),
        darts: darts_used,
        killed: state.siege.repelled > 0,
        gone: state.siege.enemies.is_empty(),
    }
}

fn endow(game: &mut GameEngine) {
    let content = game.content().clone();
    let state = game.state_mut_for_test();
    for id in ["item.poles", "item.rope"] {
        let Some(item) = content.item_idx(id) else {
            continue;
        };
        let mut left = 40;
        'floors: for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                left -= room.shelve(item, left);
                if left <= 0 {
                    break 'floors;
                }
            }
        }
    }
}

/// Top every dart rack up, and report how many it took — which is
/// exactly how many were fired since the last time.
fn load_racks(game: &mut GameEngine) -> i64 {
    let content = game.content().clone();
    let Some(darts) = content.item_idx("item.darts") else {
        return 0;
    };
    let mut reloaded = 0;
    let state = game.state_mut_for_test();
    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            if let Some(rack) = room.inputs.iter_mut().find(|s| s.item == darts) {
                let space = rack.space();
                reloaded += rack.deposit(space);
            }
        }
    }
    reloaded
}

fn build_anywhere(game: &mut GameEngine, room: &str) -> bool {
    let floors = game.state().tower.floors.len() as u8;
    let slots = game.content().balance.tower.floor_slots;
    for floor in 0..floors {
        for slot in 0..slots {
            if game
                .try_send(GameCommand::PlaceRoom {
                    room: room.into(),
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
