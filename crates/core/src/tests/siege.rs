//! Creatures, damage, defence, and repair.
//!
//! M2's sprint question is whether combat-as-logistics-stress produces
//! drama without an aimed weapon. The two things that have to be true
//! for the answer to be yes are pinned at the bottom of this file: a
//! severed shaft forces a live reroute, and a night without banked
//! charge is meaningfully worse than one with.

use crate::command::GameCommand;
use crate::content::Approach;
use crate::ids::ShaftId;
use crate::state::CrewState;
use crate::state::siege::{DamageTarget, EnemyState};
use crate::tests::{content, engine, item};

/// Wind provocation to the ceiling so waves arrive promptly. Tests
/// about damage should not spend five simulated minutes waiting to be
/// noticed by the jungle.
fn provoke_fully(game: &mut crate::engine::GameEngine) {
    let max = content().balance.siege.provocation_max;
    game.state_mut_for_test().siege.provocation = max;
}

/// Wait until something has its teeth into the tower, and return which
/// one it is together with how long that kind holds on.
///
/// Tests follow one named creature rather than "is anything attached",
/// because at high provocation a fresh wave lands every 2,400 ticks and
/// the answer to "is anything attached" is always yes.
fn wait_for_contact(game: &mut crate::engine::GameEngine) -> (crate::ids::EnemyId, u32) {
    for _ in 0..20_000 {
        game.step(1);
        let found = game
            .state()
            .siege
            .enemies
            .iter()
            .find(|enemy| matches!(enemy.state, EnemyState::Attacking { .. }))
            .map(|enemy| (enemy.id, content().enemy(enemy.def).cling_ticks));
        if let Some(found) = found {
            return found;
        }
    }
    panic!("nothing reached the tower");
}

/// Is this particular creature still holding on?
fn still_attached(game: &crate::engine::GameEngine, id: crate::ids::EnemyId) -> bool {
    game.state()
        .siege
        .enemies
        .iter()
        .any(|enemy| enemy.id == id && matches!(enemy.state, EnemyState::Attacking { .. }))
}

#[test]
fn a_walking_tower_carries_itself_out_from_under_a_wave() {
    // The answer to a wave that costs nothing: keep walking. Without
    // this a tower with no emplacements is guaranteed to be eaten,
    // which would make defences mandatory rather than a decision.
    let mut game = engine(1020);
    provoke_fully(&mut game);
    let (id, cling) = wait_for_contact(&mut game);
    assert!(
        game.state().strode,
        "the tower was not walking, so this tests nothing"
    );

    game.step(cling + 100);
    assert!(
        !still_attached(&game, id),
        "a creature was still holding on {cling} ticks after it took hold of a walking tower"
    );
}

#[test]
fn a_tower_that_has_stopped_shakes_nothing_off() {
    // The other half of the same rule, and the reason stopping to work
    // is a decision rather than a free action.
    let mut game = engine(1021);
    provoke_fully(&mut game);
    let (id, cling) = wait_for_contact(&mut game);
    game.try_send(GameCommand::SetStriding { walking: false })
        .expect("a tower can always be told to stand still");

    // Twice as long as it would have needed on a walking tower.
    game.step(cling * 2);
    assert!(
        still_attached(&game, id),
        "a tower standing still shook a creature off anyway"
    );
}

#[test]
fn walking_away_from_something_does_not_count_as_seeing_it_off() {
    // `repelled` is a readout the player trusts. A tower with no
    // emplacements at all must never be able to raise it.
    let mut game = engine(1022);
    provoke_fully(&mut game);
    game.step(20_000);

    assert!(
        crate::systems::siege::tower_integrity_permille(game.state()) < 1000,
        "nothing ever actually attacked, so this proves nothing"
    );
    assert_eq!(
        game.state().siege.repelled,
        0,
        "a tower with no way to shoot back claimed to have seen creatures off"
    );
}

#[test]
fn a_battery_shoots_what_is_in_reach_and_nothing_further() {
    // `range_paces` is what makes a battery start work on an
    // approaching creature before it has hold of anything, and it is
    // the only thing stopping one from picking creatures off the
    // horizon. Both halves have to be pinned: a range check with its
    // comparison inverted still fires, still spends darts, and still
    // kills things, so every other test here passes with it backwards.
    let content = content();
    let darts = item(&content, "item.darts");
    let range = content
        .rooms
        .iter()
        .find_map(|room| room.defence.as_ref().map(|d| d.range_paces))
        .expect("the pack defines an emplacement");

    let mut game = engine(1032);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.dart_battery".into(),
        floor: 1,
        slot: 1,
    })
    .expect("affordable");
    top_up(&mut game, darts);

    // One creature, parked well beyond reach, and the tower held still
    // so it cannot close the gap by walking into it.
    game.try_send(GameCommand::SetStriding { walking: false })
        .expect("always legal");
    let far = place_one_creature(&mut game, range * 3);
    // Held at that distance rather than left to walk in, or it closes
    // the gap on its own and gets shot for entirely proper reasons.
    for _ in 0..600 {
        game.step(1);
        park_creature(&mut game, range * 3);
    }
    assert_eq!(
        creature_hp(&game),
        Some(far),
        "a battery shot something three times its own range away"
    );

    // Same creature, now inside reach.
    let near = {
        let state = game.state_mut_for_test();
        let here = state.world.distance;
        let enemy = state.siege.enemies.first_mut().expect("still there");
        enemy.at = here + crate::fx::paces_from_int(range / 2);
        enemy.hp
    };
    let damage = content
        .rooms
        .iter()
        .find_map(|room| room.defence.as_ref().map(|d| d.damage))
        .expect("the pack defines an emplacement");
    game.step(1);
    assert_eq!(
        creature_hp(&game),
        Some(near - damage),
        "one dart did not take exactly one dart's worth off"
    );
    assert!(
        game.state()
            .siege
            .enemies
            .iter()
            .all(|enemy| !enemy.state.is_going()),
        "a creature with hit points left was written off as finished"
    );
}

#[test]
fn a_battery_with_nothing_to_shoot_at_still_reloads() {
    // Otherwise a battery that fired its last shot at a departing wave
    // is caught with an empty chamber by the next one, for no reason a
    // player could see or plan around. The reload runs on the clock,
    // not on having a target.
    let content = content();
    let darts = item(&content, "item.darts");
    let reload = content
        .rooms
        .iter()
        .find_map(|room| room.defence.as_ref().map(|d| d.reload_ticks))
        .expect("the pack defines an emplacement");
    let range = content
        .rooms
        .iter()
        .find_map(|room| room.defence.as_ref().map(|d| d.range_paces))
        .expect("the pack defines an emplacement");

    let mut game = engine(1033);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.dart_battery".into(),
        floor: 1,
        slot: 1,
    })
    .expect("affordable");
    game.try_send(GameCommand::SetStriding { walking: false })
        .expect("always legal");
    top_up(&mut game, darts);

    // Fire once, at something close enough to hit.
    place_one_creature(&mut game, range / 2);
    game.step(1);
    assert!(
        creature_hp(&game).is_some_and(|hp| hp < content.enemy(crate::ids::EnemyIdx(0)).hp),
        "the battery did not fire at all"
    );

    // Empty field for exactly one reload, then something arrives.
    game.state_mut_for_test().siege.enemies.clear();
    game.step(reload);
    let fresh = place_one_creature(&mut game, range / 2);
    game.step(1);
    assert!(
        creature_hp(&game).is_some_and(|hp| hp < fresh),
        "a battery that spent its reload with nothing in range was still not ready"
    );
}

/// Clear the field and put a single creature `ahead` paces in front.
/// Returns its hit points.
fn place_one_creature(game: &mut crate::engine::GameEngine, ahead: i64) -> i64 {
    let def = crate::ids::EnemyIdx(0);
    let hp = game.content().enemy(def).hp;
    let state = game.state_mut_for_test();
    let at = state.world.distance + crate::fx::paces_from_int(ahead);
    state.siege.enemies.clear();
    state.siege.enemies.push(crate::state::siege::Enemy {
        id: crate::ids::EnemyId(9001),
        def,
        at,
        hp,
        state: EnemyState::Approaching,
        attack_cooldown: 0,
        // Long enough that it cannot expire during the test and take
        // the creature off the field for the wrong reason.
        cling_left: u32::MAX,
        fade_left: 0,
    });
    // Stop anything else wandering in and muddying the measurement.
    state.siege.provocation = 0;
    state.siege.next_wave_tick = u64::MAX;
    hp
}

/// Put the creature back where it was, undoing its approach.
fn park_creature(game: &mut crate::engine::GameEngine, ahead: i64) {
    let state = game.state_mut_for_test();
    let at = state.world.distance + crate::fx::paces_from_int(ahead);
    if let Some(enemy) = state.siege.enemies.first_mut() {
        enemy.at = at;
    }
}

fn creature_hp(game: &crate::engine::GameEngine) -> Option<i64> {
    game.state()
        .siege
        .enemies
        .iter()
        .find(|enemy| enemy.id == crate::ids::EnemyId(9001))
        .map(|enemy| enemy.hp)
}

#[test]
fn a_battery_fires_no_faster_than_it_reloads() {
    // Ammo drain is the economics the whole emplacement design rests
    // on — a battery is a load on the chain, and how heavy a load is
    // decided by `reload_ticks`. Nothing else in the simulation reads
    // the reload counter, so an off-by-anything here would let a
    // battery empty its rack in a second and no other test would
    // notice. Mutation testing found exactly that hole.
    let content = content();
    let darts = item(&content, "item.darts");
    let reload = content
        .rooms
        .iter()
        .find_map(|room| room.defence.as_ref().map(|d| d.reload_ticks))
        .expect("the pack defines an emplacement");

    let mut game = engine(1031);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.dart_battery".into(),
        floor: 1,
        slot: 1,
    })
    .expect("affordable");
    provoke_fully(&mut game);

    // Keep the rack topped up by hand, so this measures the reload and
    // not the haul system's ability to keep up.
    const WINDOW: u32 = 3000;
    let mut fired = 0i64;
    for _ in 0..WINDOW {
        let before = rack_count(&game, darts);
        game.step(1);
        fired += (before - rack_count(&game, darts)).max(0);
        top_up(&mut game, darts);
    }

    let ceiling = i64::from(WINDOW / reload) + 1;
    assert!(fired > 0, "a stocked battery under attack never fired");
    assert!(
        fired <= ceiling,
        "a battery fired {fired} times in {WINDOW} ticks;          {reload}-tick reload allows at most {ceiling}"
    );
}

/// Refill every emplacement's rack to full.
fn top_up(game: &mut crate::engine::GameEngine, darts: crate::ids::ItemIdx) {
    let state = game.state_mut_for_test();
    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            if let Some(rack) = room.inputs.iter_mut().find(|stack| stack.item == darts) {
                let space = rack.space();
                rack.deposit(space);
            }
        }
    }
}

fn rack_count(game: &crate::engine::GameEngine, darts: crate::ids::ItemIdx) -> i64 {
    game.state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .flat_map(|room| room.inputs.iter())
        .filter(|stack| stack.item == darts)
        .map(|stack| stack.count)
        .sum()
}

#[test]
fn the_moment_something_gives_way_has_its_own_sound() {
    // `Health::hurt` returns whether that blow was the one that
    // finished the thing off, and the only consumer of that answer is
    // the audio layer: a bite that lands is an Impact, the bite that
    // takes the last hit point is a Breach, a Wrecked or a Severed.
    // Nothing else in the simulation branches on it, so without this
    // the distinction could be inverted and every test would still
    // pass — which is exactly what mutation testing found.
    let mut game = engine(1030);
    provoke_fully(&mut game);

    // Thin the ground floor's skin to one hit point, so the next bite
    // that lands on it is the one that opens it up.
    {
        let floor = game
            .state_mut_for_test()
            .tower
            .floor_mut(0)
            .expect("the tower has a ground floor");
        floor.panel.hp = 1;
    }

    let mut ever_breached = false;
    for _ in 0..20_000 {
        let intact_before = !panel_broken(&game);
        let breached = game.step(1).contains(&crate::systems::SoundEvent::Breach);
        if breached {
            // The bite that made that sound has to be the bite that
            // took the last hit point. Anything else means the two
            // cues are telling the player the wrong thing.
            assert!(
                intact_before && panel_broken(&game),
                "a Breach sounded while the panel was still standing"
            );
            ever_breached = true;
            break;
        }
        assert!(
            intact_before || !breached,
            "a Breach sounded for a panel that had already fallen in"
        );
    }

    assert!(ever_breached, "a panel fell in without a sound");
}

fn panel_broken(game: &crate::engine::GameEngine) -> bool {
    game.state()
        .tower
        .floor(0)
        .expect("the tower has a ground floor")
        .panel
        .is_broken()
}

#[test]
fn the_pack_defines_creatures_that_each_teach_something() {
    let content = content();
    assert!(!content.enemies.is_empty(), "no creatures in the pack");

    // Every approach has to be represented, because each one is a
    // different lesson: ammo economics, height as exposure, transport
    // redundancy. A missing approach is a missing lesson.
    for approach in [Approach::Ground, Approach::Canopy, Approach::Burrow] {
        assert!(
            content.enemies.iter().any(|e| e.approach == approach),
            "nothing in the pack approaches by {approach:?}"
        );
    }
    for enemy in &content.enemies {
        assert!(enemy.hp > 0, "{} has no hit points", enemy.id);
        assert!(enemy.threat > 0, "{} costs nothing to field", enemy.id);
        assert!(
            enemy.attack_ticks > 0,
            "{} attacks infinitely fast",
            enemy.id
        );
    }
}

#[test]
fn a_quiet_tower_is_left_alone_for_a_while() {
    // The opening of a run should be gentle without a difficulty
    // setting. Provocation starts at zero and only the cheapest
    // creatures qualify.
    let mut game = engine(1000);
    game.step(1800);
    assert!(
        game.state().siege.enemies.is_empty(),
        "a minute into a fresh run, something had already come looking"
    );
}

#[test]
fn harvesting_hard_draws_attention_and_walking_quietly_sheds_it() {
    // One cutter arm is deliberately break-even against decay: a tower
    // living within its means is left alone indefinitely (BALANCE.md,
    // provocation_per_100_harvested). It is the *second* arm that
    // provokes, so that is what this test builds.
    let mut game = engine(1001);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.cutter_arm".into(),
        floor: 0,
        slot: 6,
    })
    .expect("room for a second arm on the ground floor");

    game.step(12_000);
    let provoked = game.state().siege.provocation;
    assert!(
        provoked > 0,
        "stripping the terrain with two arms drew no attention at all"
    );

    // Tear both arms out and the attention should bleed off.
    for slot in [4, 6] {
        game.try_send(GameCommand::RemoveRoom { floor: 0, slot })
            .expect("a cutter arm is removable");
    }
    game.step(12_000);
    assert!(
        game.state().siege.provocation < provoked,
        "attention did not decay once the tower stopped harvesting: {provoked} then {}",
        game.state().siege.provocation
    );
}

#[test]
fn provocation_never_leaves_its_bounds() {
    let max = content().balance.siege.provocation_max;
    let mut game = engine(1002);
    for _ in 0..400 {
        game.step(60);
        let provocation = game.state().siege.provocation;
        assert!(
            (0..=max).contains(&provocation),
            "provocation escaped its range at {provocation}"
        );
    }
}

#[test]
fn creatures_arrive_and_close_on_the_tower() {
    let mut game = engine(1003);
    provoke_fully(&mut game);

    let mut ever_spawned = false;
    let mut ever_contacted = false;
    for _ in 0..12_000 {
        game.step(1);
        if !game.state().siege.enemies.is_empty() {
            ever_spawned = true;
        }
        if game
            .state()
            .siege
            .enemies
            .iter()
            .any(|enemy| matches!(enemy.state, EnemyState::Attacking { .. }))
        {
            ever_contacted = true;
            break;
        }
    }
    assert!(ever_spawned, "no wave ever arrived at full provocation");
    assert!(ever_contacted, "creatures never reached the tower");
}

#[test]
fn creatures_damage_the_things_the_player_built() {
    let mut game = engine(1004);
    provoke_fully(&mut game);
    game.step(18_000);

    let integrity = crate::systems::siege::tower_integrity_permille(game.state());
    assert!(
        integrity < 1000,
        "ten minutes at full provocation left the tower untouched"
    );
}

#[test]
fn a_severed_shaft_is_no_longer_a_route() {
    // The signature emergency. Cutting the stairs must remove them from
    // route-finding — if it does not, damage to circulation is cosmetic
    // and the whole milestone's argument collapses.
    let mut game = engine(1005);
    game.step(600);
    let stairs = game.state().tower.shafts[0].id;

    {
        let state = game.state_mut_for_test();
        let shaft = state.tower.shaft_mut(stairs).expect("the stairs");
        shaft.health.hp = 0;
    }

    let shaft = game.state().tower.shaft(stairs).expect("still standing");
    assert!(shaft.is_severed());
    assert!(!shaft.has_room(), "a severed shaft still accepted a rider");
    let daypart = game.state().clock.daypart(game.content());
    assert!(
        !shaft.serves_trip(0, 2, daypart),
        "route-finding still offered a severed shaft"
    );
}

#[test]
fn a_severed_shaft_puts_everyone_on_it_back_on_their_feet() {
    let mut game = engine(1006);
    game.step(900);
    let stairs = game.state().tower.shafts[0].id;

    crate::systems::siege::evict_riders(game.state_mut_for_test(), stairs);

    let stuck = game.state().crew.iter().any(|member| {
        matches!(member.state,
            CrewState::Boarding { shaft, .. }
            | CrewState::Climbing { shaft, .. }
            | CrewState::Riding { shaft, .. } if shaft == stairs)
    });
    assert!(!stuck, "somebody is still on a shaft that was cut through");
    assert_eq!(
        game.state().tower.shaft(stairs).expect("standing").riders,
        0
    );
}

#[test]
fn a_wrecked_room_stops_working() {
    let content = content();
    let bamboo = item(&content, "item.bamboo");
    let mut game = engine(1007);
    game.step(1800);
    game.state_mut_for_test().siege.enemies.clear();

    // Wreck the mill and stock it, so the only reason it could stall is
    // the damage.
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if let Some(stack) = room.inputs.iter_mut().find(|s| s.item == bamboo) {
                    let space = stack.space();
                    stack.deposit(space);
                }
            }
        }
        if let Some(floor) = state.tower.floor_mut(2)
            && let Some(mill) = floor.rooms.iter_mut().find(|room| room.covers(3))
        {
            mill.health.hp = 0;
        }
    }

    let crafts = game.state().stats.crafts_completed;
    game.step(1200);
    assert_eq!(
        game.state().stats.crafts_completed,
        crafts,
        "a wrecked mill kept milling"
    );
}

#[test]
fn the_heartseed_ends_the_run() {
    let mut game = engine(1008);
    game.step(60);
    assert!(!game.state().siege.lost);

    // Destroy it outright rather than waiting for a creature to chew
    // through the toughest thing in the tower.
    {
        let state = game.state_mut_for_test();
        let heart = state
            .tower
            .floors
            .iter_mut()
            .flat_map(|floor| floor.rooms.iter_mut())
            .find(|room| room.health.max == content().balance.siege.heartseed_hp);
        if let Some(heart) = heart {
            heart.health.hp = 1;
        }
    }
    provoke_fully(&mut game);
    game.step(30_000);

    assert!(
        game.state().siege.lost,
        "the Heartseed fell and the run carried on regardless"
    );
}

// ---------------------------------------------------------------------------
// Emplacements
// ---------------------------------------------------------------------------

#[test]
fn a_fed_battery_sees_creatures_off() {
    let content = content();
    let darts = item(&content, "item.darts");
    let mut game = engine(1009);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.dart_battery".into(),
        floor: 1,
        slot: 1,
    })
    .expect("affordable after 200 seconds");

    // Hand it ammo directly; whether the crew keep it fed is the haul
    // system's business, not this test's.
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                if let Some(magazine) = room.inputs.iter_mut().find(|s| s.item == darts) {
                    let space = magazine.space();
                    magazine.deposit(space);
                }
            }
        }
    }
    provoke_fully(&mut game);

    let mut ever_repelled = false;
    for _ in 0..30_000 {
        game.step(1);
        if game.state().siege.repelled > 0 {
            ever_repelled = true;
            break;
        }
    }
    assert!(ever_repelled, "a fully loaded battery saw nothing off");
}

#[test]
fn a_dry_battery_is_as_quiet_as_a_starved_mill() {
    // The equivalence the whole design rests on: an emplacement out of
    // ammo fails exactly the way a room out of inputs fails, through
    // the same stack and the same haul system.
    let mut game = engine(1010);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.dart_battery".into(),
        floor: 1,
        slot: 1,
    })
    .expect("affordable");
    provoke_fully(&mut game);

    // No thornwright, so no darts are ever made and the magazine can
    // never fill.
    game.step(18_000);
    assert_eq!(
        game.state().siege.repelled,
        0,
        "a battery with no ammo supply somehow shot something"
    );

    // And it has to *look* the way a starved mill looks. Failing
    // quietly is only half of the equivalence — the other half is that
    // the cross-section says so, because that is the only warning the
    // player gets (`DECISIONS.md` §8, diegetic first).
    let view = game.view();
    let battery = view
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .find(|room| room.slot == 1 && room.def == battery_def(&game))
        .expect("the battery is still standing");
    assert!(
        battery.stalled,
        "a battery with an empty rack did not read as stalled"
    );
}

fn battery_def(game: &crate::engine::GameEngine) -> u16 {
    game.content()
        .rooms
        .iter()
        .position(|room| room.id == "room.dart_battery")
        .expect("the pack defines a dart battery") as u16
}

// ---------------------------------------------------------------------------
// Repair
// ---------------------------------------------------------------------------

#[test]
fn crew_mend_what_is_broken_and_it_costs_poles() {
    let content = content();
    let poles = item(&content, "item.poles");
    let mut game = engine(1011);
    game.step(6000);

    // Break a panel, then leave the tower alone to fix it.
    {
        let state = game.state_mut_for_test();
        if let Some(floor) = state.tower.floor_mut(0) {
            floor.panel.hp = floor.panel.max / 4;
        }
    }
    let poles_before = game.state().stock_of(poles);
    let hp_before = game.state().tower.floor(0).expect("ground floor").panel.hp;

    game.step(9000);
    let hp_after = game.state().tower.floor(0).expect("ground floor").panel.hp;

    assert!(
        hp_after > hp_before,
        "nobody mended the panel: {hp_before} then {hp_after}"
    );
    assert!(game.state().stats.hp_repaired > 0, "repair was not counted");
    // Poles were spent on it. The chain pays for the repairs as well as
    // for the tower.
    assert!(
        game.state().stock_of(poles) < poles_before + 40,
        "repairing appears to have been free"
    );
}

#[test]
fn repair_without_poles_does_not_happen() {
    let content = content();
    let poles = item(&content, "item.poles");
    let mut game = engine(1012);
    game.step(600);

    {
        let state = game.state_mut_for_test();
        if let Some(floor) = state.tower.floor_mut(0) {
            floor.panel.hp = 1;
        }
        // Hold the jungle off, or the panel is finished off by
        // something arriving rather than left alone unrepaired.
        state.siege.provocation = 0;
        state.siege.enemies.clear();
        // Strip the shelves bare and stop anything refilling them.
        let held = state.stock_of(poles);
        state.take_stock(poles, held);
        for floor in &mut state.tower.floors {
            for room in &mut floor.rooms {
                room.active = false;
            }
        }
    }

    game.step(3000);
    assert_eq!(
        game.state().tower.floor(0).expect("ground floor").panel.hp,
        1,
        "a panel was mended with no poles on the shelves"
    );
}

#[test]
fn two_crew_never_mend_the_same_thing() {
    let mut game = engine(1013);
    game.step(6000);
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            floor.panel.hp = floor.panel.max / 2;
        }
    }

    for _ in 0..9000 {
        game.step(1);
        let jobs: Vec<DamageTarget> = game
            .state()
            .crew
            .iter()
            .filter_map(|member| member.repair.map(|job| job.target))
            .collect();
        let mut seen = jobs.clone();
        seen.sort_by_key(|target| format!("{target:?}"));
        seen.dedup_by_key(|target| format!("{target:?}"));
        assert_eq!(
            seen.len(),
            jobs.len(),
            "two crew were assigned the same damage at tick {}",
            game.state().tick
        );
    }
}

// ---------------------------------------------------------------------------
// The milestone's exit criteria
// ---------------------------------------------------------------------------

#[test]
fn a_severed_shaft_forces_a_live_reroute() {
    // The moment the whole milestone exists to produce. Build a second
    // way up, cut the first, and the crew must carry on using the one
    // that is left rather than stalling.
    let mut game = engine(1014);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.elevator".into(),
        low: 0,
        high: 3,
        slot: 7,
    })
    .expect("affordable after 200 seconds");
    game.step(1800);

    let stairs: ShaftId = game.state().tower.shafts[0].id;
    let hauls_before = game.state().stats.hauls_completed;

    {
        let state = game.state_mut_for_test();
        state.tower.shaft_mut(stairs).expect("the stairs").health.hp = 0;
    }
    crate::systems::siege::evict_riders(game.state_mut_for_test(), stairs);

    // Watch tick by tick: the crew will eventually mend the stairs and
    // start using them again, which is correct. What must never happen
    // is anyone travelling on them *while* they are cut through.
    let mut used_while_severed = false;
    for _ in 0..9000 {
        game.step(1);
        let severed = game
            .state()
            .tower
            .shaft(stairs)
            .is_some_and(crate::state::Shaft::is_severed);
        if !severed {
            continue;
        }
        if game.state().crew.iter().any(|member| {
            matches!(member.state,
                CrewState::Climbing { shaft, .. } | CrewState::Riding { shaft, .. }
                    if shaft == stairs)
        }) {
            used_while_severed = true;
            break;
        }
    }

    assert!(
        !used_while_severed,
        "somebody travelled on a shaft that was cut through"
    );
    assert!(
        game.state().stats.hauls_completed > hauls_before,
        "the tower stopped hauling entirely when the stairs were cut — no reroute happened"
    );
}

#[test]
fn a_tower_with_one_shaft_stalls_when_it_is_cut() {
    // The other side of the same coin, and the reason redundancy is a
    // decision rather than a formality: with nothing to reroute onto,
    // cutting the only shaft really does stop the chain.
    let mut game = engine(1015);
    game.step(3000);
    let stairs = game.state().tower.shafts[0].id;
    {
        let state = game.state_mut_for_test();
        state.tower.shaft_mut(stairs).expect("the stairs").health.hp = 0;
    }
    crate::systems::siege::evict_riders(game.state_mut_for_test(), stairs);

    // Let whatever is already in the mill's inbox run out first — a
    // buffered craft after the cut is correct, not a leak.
    game.step(3000);
    let crafts_before = game.state().stats.crafts_completed;
    game.step(9000);
    assert_eq!(
        game.state().stats.crafts_completed,
        crafts_before,
        "bamboo reached a mill two floors up with no working shaft"
    );
}
