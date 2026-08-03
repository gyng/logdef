//! Creatures, damage, defence, and repair.
//!
//! M2's sprint question is whether combat-as-logistics-stress produces
//! drama without an aimed weapon. The two things that have to be true
//! for the answer to be yes are pinned at the bottom of this file: a
//! severed shaft forces a live reroute, and a night without banked
//! charge is meaningfully worse than one with.

use crate::command::GameCommand;
use crate::content::Approach;
use crate::ids::{EnemyIdx, FloorIdx, ShaftId, SlotIdx};
use crate::state::CrewState;
use crate::state::siege::{DamageTarget, EnemyState};
use crate::systems::SoundEvent;
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
fn only_the_creatures_the_darts_killed_are_counted_as_seen_off() {
    // The test above is the zero case, and zero is where a miscount
    // hides: with nothing shooting back there is nothing dying, so a
    // counter that tallies the wrong exit and one that tallies the
    // right one both read nothing. This is the same rule with both
    // exits on the field at once, and it pins the counter to the kills
    // exactly rather than to "more than none".
    //
    // The field is staged rather than fought for. A run that produces
    // both exits cleanly is a narrow window — at low provocation the
    // battery kills the whole wave and nothing is ever left behind, and
    // at high provocation the wave wrecks the battery before it can
    // kill anything — and a test of what a counter counts should not
    // also be a test of whether that window was hit.
    let content = content();
    let skitter = content
        .enemy_idx("enemy.skitter")
        .expect("the pack defines a skitter");
    let fade = content.balance.siege.enemy_fade_ticks;

    let mut game = engine(1044);
    {
        let state = game.state_mut_for_test();
        state.siege.provocation = 0;
        state.siege.next_wave_tick = u64::MAX;
        let here = state.world.distance;
        // Far enough out that it is still walking in when the last fade
        // finishes, so the field also holds something that is on its way
        // nowhere — a creature with no fade left to run is exactly what
        // a counter reading the wrong side of this comparison would
        // start tallying.
        let horizon = here + crate::fx::paces_from_int(500);
        let hp = content.enemy(skitter).hp;
        state.siege.enemies = vec![
            staged_creature(9101, skitter, hp, here, EnemyState::Dying, fade),
            staged_creature(9102, skitter, hp, here, EnemyState::Dying, fade),
            staged_creature(9103, skitter, hp, here, EnemyState::Leaving, fade),
            staged_creature(9104, skitter, hp, here, EnemyState::Leaving, fade),
            staged_creature(9105, skitter, hp, here, EnemyState::Leaving, fade),
            staged_creature(9106, skitter, hp, horizon, EnemyState::Approaching, 0),
        ];
    }

    // Long enough for every fade to run out, and then some.
    game.step(fade + 2);

    assert_eq!(
        game.state().siege.repelled,
        2,
        "two were shot down and three were walked away from; the tower claims {}",
        game.state().siege.repelled
    );
    assert_eq!(
        game.state().siege.enemies.len(),
        1,
        "the field should hold only the creature that was still walking in"
    );
}

/// One creature on the field in a chosen state, so a test can stage a
/// departure of each kind rather than arrange a fight that happens to
/// produce both.
fn staged_creature(
    id: u32,
    def: EnemyIdx,
    hp: i64,
    at: crate::fx::Paces,
    state: EnemyState,
    fade_left: u32,
) -> crate::state::siege::Enemy {
    crate::state::siege::Enemy {
        id: crate::ids::EnemyId(id),
        def,
        at,
        hp,
        state,
        attack_cooldown: 0,
        // Long enough that the tower's stride cannot take it off the
        // field for a reason this test is not asking about.
        cling_left: u32::MAX,
        fade_left,
    }
}

#[test]
fn a_battery_shoots_the_nearest_thing_first() {
    // With a wave strung out along the approach, which one a battery
    // picks decides whether it thins the front of the wave or plinks
    // at the back while the front walks in. Nearest first is the only
    // choice that reads as sensible from outside, and nothing else in
    // the simulation depends on the ordering, so it needs pinning.
    let content = content();
    let darts = item(&content, "item.darts");
    let range = content
        .rooms
        .iter()
        .find_map(|room| room.defence.as_ref().map(|d| d.range_paces))
        .expect("the pack defines an emplacement");

    let mut game = engine(1042);
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

    // Two creatures, both within reach, at opposite ends of it.
    let full = place_one_creature(&mut game, range / 4);
    {
        let state = game.state_mut_for_test();
        let far_at = state.world.distance + crate::fx::paces_from_int(range * 3 / 4);
        let mut far = state.siege.enemies[0].clone();
        far.id = crate::ids::EnemyId(9002);
        far.at = far_at;
        state.siege.enemies.push(far);
    }

    game.step(1);
    let near_hp = creature_hp(&game).expect("the near one is still there");
    let far_hp = game
        .state()
        .siege
        .enemies
        .iter()
        .find(|enemy| enemy.id == crate::ids::EnemyId(9002))
        .map(|enemy| enemy.hp)
        .expect("the far one is still there");

    assert!(
        near_hp < full,
        "the battery did not shoot the creature at a quarter of its range"
    );
    assert_eq!(
        far_hp, full,
        "the battery shot past the near creature at the far one"
    );
}

#[test]
fn equally_hurt_things_are_mended_nearest_first() {
    // The other half of triage. Once two wounds are equally bad, the
    // tie goes to whichever is fewer floors away, so a crew member does
    // not walk the length of the tower past an identical job.
    let mut game = engine(1043);
    crate::tests::stock_poles(&mut game, 200);
    {
        let state = game.state_mut_for_test();
        state.siege.provocation = 0;
        state.siege.enemies.clear();
        state.siege.next_wave_tick = u64::MAX;
        state.crew.truncate(1);
        // Identical wounds, one right where the crew member is
        // standing and one at the top of the tower.
        let top = state.tower.top_floor();
        state.tower.floor_mut(0).expect("ground floor").panel.hp -= 100;
        state.tower.floor_mut(top).expect("top floor").panel.hp -= 100;
        let member = state.crew.first_mut().expect("one crew member");
        member.state = crate::state::CrewState::Idle;
    }

    let mut went_to = None;
    for _ in 0..600 {
        game.step(1);
        if let Some(job) = game.state().crew.first().and_then(|member| member.repair) {
            went_to = Some(job.target);
            break;
        }
    }

    assert_eq!(
        went_to,
        Some(DamageTarget::Panel { floor: 0 }),
        "the crew climbed the tower past an identical job on the way"
    );
}

#[test]
fn the_repair_bill_is_what_the_repairs_actually_cost() {
    // "TO MEND 34 poles" is the number the player triages against, and
    // a bill that is not the true cost is worse than no bill at all —
    // they would put off a repair they could afford, or bank for one
    // they could already pay for. Nothing else in the simulation reads
    // this figure, so it needs pinning against the thing it predicts:
    // the poles actually spent putting the same damage right.
    let content = content();
    let mut game = engine(1040);
    crate::tests::stock_poles(&mut game, 200);

    // A quiet tower with one known wound, so the bill has exactly one
    // thing in it and repair is not racing fresh damage.
    let hurt_by = 100;
    {
        let state = game.state_mut_for_test();
        state.siege.provocation = 0;
        state.siege.enemies.clear();
        state.siege.next_wave_tick = u64::MAX;
        let floor = state.tower.floor_mut(0).expect("ground floor");
        floor.panel.hp -= hurt_by;
    }

    let bill = crate::systems::repair::outstanding_repair_cost(game.state(), &content);
    assert_eq!(
        bill,
        hurt_by * content.balance.siege.repair_poles_per_10_hp / 10,
        "the bill is not the damage priced at the going rate"
    );

    // Now let the crew settle it, and see the two figures agree.
    let before = game.state().stats.repair_poles_spent;
    for _ in 0..200 {
        game.step(60);
        let state = game.state_mut_for_test();
        state.siege.provocation = 0;
        state.siege.enemies.clear();
        if crate::systems::repair::outstanding_repair_cost(state, &content) == 0 {
            break;
        }
    }

    assert_eq!(
        crate::systems::repair::outstanding_repair_cost(game.state(), &content),
        0,
        "the crew never finished the one repair on the list"
    );
    assert_eq!(
        (game.state().stats.repair_poles_spent - before) as i64,
        bill,
        "the crew spent a different number of poles than the bill quoted"
    );
}

#[test]
fn crew_go_to_the_worst_damage_first() {
    // Triage is the whole point of repair being neither automatic nor
    // free: with more damage than crew, which one they walk to has to
    // be the one that most needs them. Worst first, ties to nearest.
    let mut game = engine(1041);
    crate::tests::stock_poles(&mut game, 200);
    {
        let state = game.state_mut_for_test();
        state.siege.provocation = 0;
        state.siege.enemies.clear();
        state.siege.next_wave_tick = u64::MAX;
        state.crew.truncate(1);
        // Two wounds on the same floor, so distance cannot be the
        // reason for the choice — only how bad they are.
        state.tower.floor_mut(0).expect("ground floor").panel.hp -= 20;
        state.tower.floor_mut(1).expect("first floor").panel.hp -= 120;
    }

    let mut went_to = None;
    for _ in 0..600 {
        game.step(1);
        if let Some(job) = game.state().crew.first().and_then(|member| member.repair) {
            went_to = Some(job.target);
            break;
        }
    }

    assert_eq!(
        went_to,
        Some(DamageTarget::Panel { floor: 1 }),
        "the crew went to the scratch and left the hole"
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
    place_creature(game, EnemyIdx(0), ahead)
}

/// The same, for a named kind. The targeting tests each need a
/// particular approach, and which index the pack happens to intern a
/// creature at is not something a test should know.
fn place_creature(game: &mut crate::engine::GameEngine, def: EnemyIdx, ahead: i64) -> i64 {
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

/// Wind the clock to `tick_of_day`, wind provocation to the ceiling, and
/// run the one tick on which a wave is due. Leaves the field holding
/// exactly what that wave brought and nothing else.
fn force_a_wave(game: &mut crate::engine::GameEngine, tick_of_day: u32) {
    let ceiling = game.content().balance.siege.provocation_max;
    {
        let state = game.state_mut_for_test();
        state.siege.enemies.clear();
        state.siege.provocation = ceiling;
        state.siege.next_wave_tick = state.tick;
        // The clock advances at the top of the tick, so wind it to one
        // short of where the wave should see it.
        state.clock.tick_of_day = tick_of_day.saturating_sub(1);
    }
    game.step(1);
}

#[test]
fn a_wave_announces_itself_once_however_many_creatures_it_brings() {
    // The horizon cue is the entire warning the player gets, and the
    // spawn counter behind it is read by nothing else in the
    // simulation. One cue per wave: silence would mean nothing is
    // coming, and a cue per creature would turn a large wave into a
    // stutter that says nothing about how large it is. A wave that
    // brought its creatures in silence, or brought none at all, looks
    // identical to every other test in this file.
    let mut game = engine(1055);
    provoke_fully(&mut game);

    // Tick zero of a run is already a wave check.
    let sounds = game.step(1);
    let announced = sounds
        .iter()
        .filter(|sound| **sound == SoundEvent::WaveArrives)
        .count();
    assert_eq!(announced, 1, "a wave announced itself {announced} times");
    assert!(
        game.state().siege.enemies.len() > 1,
        "the wave that announced itself was a single creature, or none"
    );

    // And nothing announces itself when no wave is due.
    game.state_mut_for_test().siege.next_wave_tick = u64::MAX;
    let quiet = game.step(1200);
    assert!(
        !quiet.contains(&SoundEvent::WaveArrives),
        "a wave announced itself with no wave behind it"
    );
}

#[test]
fn the_night_prowler_only_comes_out_after_dark() {
    // `night_only` is the mechanism behind the milestone's second exit
    // criterion — a night without banked charge being meaningfully
    // worse than one with — and one comparison against the sun decides
    // whether it is night at all. Backwards, the fastest creature in
    // the pack turns up at noon and never after dark, and the whole
    // reason to bank charge quietly evaporates while every other test
    // here carries on passing.
    let content = content();
    let prowler = content
        .enemy_idx("enemy.night_prowler")
        .expect("the pack defines a night prowler");
    let ticks_per_day = content.balance.clock.ticks_per_day;

    let mut game = engine(1056);

    // Midday, sun at its highest.
    for _ in 0..6 {
        force_a_wave(&mut game, ticks_per_day / 2);
        assert!(
            game.state()
                .siege
                .enemies
                .iter()
                .all(|enemy| enemy.def != prowler),
            "a night prowler turned up at midday"
        );
    }

    // The small hours, when it is supposed to.
    let mut ever_prowled = false;
    for _ in 0..6 {
        force_a_wave(&mut game, 100);
        ever_prowled |= game
            .state()
            .siege
            .enemies
            .iter()
            .any(|enemy| enemy.def == prowler);
    }
    assert!(
        ever_prowled,
        "six fully-provoked waves after dark and nothing nocturnal came"
    );
}

#[test]
fn a_wave_never_fields_more_than_its_budget_can_pay_for() {
    // Provocation is the only difficulty dial in the game and the
    // threat budget is the whole of what it buys, so a wave that
    // overspent it would make the dial mean nothing — a tower walking
    // quietly and one stripping the terrain bare would meet the same
    // jungle, and every figure in BALANCE.md's siege section would be
    // describing something the simulation does not do.
    let content = content();
    let balance = &content.balance.siege;

    let mut game = engine(1057);
    for provocation in [20, 100, 400, balance.provocation_max] {
        {
            let state = game.state_mut_for_test();
            state.siege.enemies.clear();
            state.siege.provocation = provocation;
            state.siege.next_wave_tick = state.tick;
        }
        game.step(1);

        let budget =
            (provocation * balance.threat_per_100_provocation / 100).max(balance.base_threat);
        let fielded: i64 = game
            .state()
            .siege
            .enemies
            .iter()
            .map(|enemy| content.enemy(enemy.def).threat)
            .sum();
        assert!(
            fielded > 0,
            "provocation {provocation} bought a wave with nothing in it"
        );
        assert!(
            fielded <= budget,
            "provocation {provocation} fielded {fielded} threat against a budget of {budget}"
        );
    }
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

// ---------------------------------------------------------------------------
// What a creature goes for
// ---------------------------------------------------------------------------

/// Take the poles away and shut the rooms down, so nothing the tower
/// owns is mended or replaced while a measurement is running.
///
/// The targeting tests below each knock one specific thing down and then
/// watch what a creature turns to next. If the crew patch the hole
/// mid-measurement the creature's choice legitimately changes and the
/// test is quietly measuring the repair system instead. Same trick, and
/// the same reason, as `repair_without_poles_does_not_happen`.
fn hold_the_repairs_off(game: &mut crate::engine::GameEngine) {
    let poles = item(&content(), "item.poles");
    let state = game.state_mut_for_test();
    let held = state.stock_of(poles);
    state.take_stock(poles, held);
    for floor in &mut state.tower.floors {
        for room in &mut floor.rooms {
            room.active = false;
        }
    }
}

fn room_hp(game: &crate::engine::GameEngine, floor: FloorIdx, slot: SlotIdx) -> i64 {
    game.state()
        .tower
        .find_room(floor, slot)
        .expect("the room is still standing")
        .health
        .hp
}

#[test]
fn a_leaper_moves_on_to_whatever_is_still_standing_on_the_roof() {
    // Every branch of the targeting rule reaches for the first thing
    // that is still *intact*, and the intactness check is the whole of
    // what makes that true. Inverted, a creature goes back to the hole
    // it has already made, the bite lands on nothing, and it re-picks
    // the same hole forever. From outside that reads as a wave that has
    // arrived and then stopped doing anything — and it means the top
    // deck, the thing height is supposed to expose, is never actually
    // at risk once its panel is gone.
    let content = content();
    let leaper = content
        .enemy_idx("enemy.canopy_leaper")
        .expect("the pack defines a canopy leaper");

    let mut game = engine(1053);
    game.try_send(GameCommand::SetStriding { walking: false })
        .expect("always legal");
    hold_the_repairs_off(&mut game);

    // The roof panel is already gone. The sails behind it are not.
    let top = game.state().tower.top_floor();
    let slot = {
        let floor = game
            .state_mut_for_test()
            .tower
            .floor_mut(top)
            .expect("the top floor");
        floor.panel.hp = 0;
        floor.rooms.first().expect("the canopy sails").slot
    };
    let whole = room_hp(&game, top, slot);
    place_creature(&mut game, leaper, 0);

    let mut bitten = false;
    for _ in 0..600 {
        game.step(1);
        if room_hp(&game, top, slot) < whole {
            bitten = true;
            break;
        }
    }
    assert!(
        bitten,
        "a leaper on a roof whose panel was already gone never touched the sails behind it"
    );
}

#[test]
fn a_creature_whose_target_is_torn_out_from_under_it_finds_another() {
    // What a creature is chewing is a coordinate rather than a handle
    // precisely so the player can demolish a room mid-bite
    // (`siege.rs`, `apply_damage`). The other half of that decision is
    // that the creature has to notice and move on. One left gnawing at
    // a coordinate that resolves to nothing has quietly retired, and a
    // player who worked that out could clear a wave by pulling down one
    // room per attacker — a demolition button that doubles as a weapon,
    // which is not a verb this game has.
    let content = content();
    let leaper = content
        .enemy_idx("enemy.canopy_leaper")
        .expect("the pack defines a canopy leaper");

    let mut game = engine(1058);
    crate::tests::stock_poles(&mut game, 20);
    // A second room on the roof, so there is somewhere for the leaper
    // to go once the first one is pulled out from under it.
    game.try_send(GameCommand::PlaceRoom {
        room: "room.storeroom".into(),
        floor: 3,
        slot: 1,
    })
    .expect("affordable, and the roof has room");
    game.try_send(GameCommand::SetStriding { walking: false })
        .expect("always legal");
    hold_the_repairs_off(&mut game);

    let top = game.state().tower.top_floor();
    game.state_mut_for_test()
        .tower
        .floor_mut(top)
        .expect("the top floor")
        .panel
        .hp = 0;
    let store_whole = room_hp(&game, top, 1);
    let sails_whole = room_hp(&game, top, 3);
    place_creature(&mut game, leaper, 0);

    // Wait until it has its teeth into the nearer of the two rooms.
    let mut chewing = false;
    for _ in 0..600 {
        game.step(1);
        if room_hp(&game, top, 1) < store_whole {
            chewing = true;
            break;
        }
    }
    assert!(chewing, "the leaper never started on the storeroom");

    // Now pull it down around them.
    game.try_send(GameCommand::RemoveRoom {
        floor: top,
        slot: 1,
    })
    .expect("a storeroom is demolishable");

    let mut moved_on = false;
    for _ in 0..600 {
        game.step(1);
        if room_hp(&game, top, 3) < sails_whole {
            moved_on = true;
            break;
        }
    }
    assert!(
        moved_on,
        "the room a leaper was chewing was demolished and it never looked for another"
    );
}

#[test]
fn a_borer_moves_on_to_the_column_that_is_still_whole() {
    // The same rule, on the branch that matters most. A borer that
    // keeps working a column it has already severed is a borer that can
    // never take the second one, which would make transport redundancy
    // free rather than a decision — build one spare shaft and the
    // jungle can never cut your circulation again.
    let content = content();
    let borer = content
        .enemy_idx("enemy.root_borer")
        .expect("the pack defines a root borer");

    let mut game = engine(1054);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.elevator".into(),
        low: 0,
        high: 3,
        slot: 7,
    })
    .expect("affordable");
    game.try_send(GameCommand::SetStriding { walking: false })
        .expect("always legal");
    hold_the_repairs_off(&mut game);

    let stairs = game.state().tower.shafts[0].id;
    let elevator = game.state().tower.shafts[1].id;
    let whole = {
        let state = game.state_mut_for_test();
        state.tower.shaft_mut(stairs).expect("the stairs").health.hp = 0;
        state.tower.shaft(elevator).expect("the elevator").health.hp
    };
    place_creature(&mut game, borer, 0);

    let mut bitten = false;
    for _ in 0..900 {
        game.step(1);
        if game
            .state()
            .tower
            .shaft(elevator)
            .expect("still standing")
            .health
            .hp
            < whole
        {
            bitten = true;
            break;
        }
    }
    assert!(
        bitten,
        "a borer at a tower with one column already cut through never started on the other"
    );
}

// ---------------------------------------------------------------------------
// The standing figure
// ---------------------------------------------------------------------------

/// Every hit point the opening tower is built from: four panels at
/// `panel_hp`, a Heartseed at `heartseed_hp`, five ordinary rooms at
/// `room_hp`, and one staircase at `shaft_hp`.
///
/// Written out rather than summed off the tower, because the test below
/// exists to check the readout against arithmetic done somewhere other
/// than the function that produces it.
const OPENING_TOWER_HP: i64 = 6500;

#[test]
fn the_standing_figure_weighs_panels_rooms_and_shafts_together() {
    // "STANDING 95%" is the player's one at-a-glance read on how the
    // tower is holding up, and nothing else in the simulation consumes
    // it — no system branches on it, no crew decision reads it. That
    // makes it exactly the kind of figure that can quietly become
    // nonsense and have every other test in this file still pass, which
    // is what mutation testing found: the whole function could be
    // replaced with a constant and nobody noticed.
    //
    // Each of the three kinds of thing the tower is made of has its own
    // accumulator, and each is damaged separately below, because a
    // readout that only counted its panels would still say the tower
    // was whole with a wrecked mill and a severed spine in it. That is
    // the reassuring lie the top bar must never tell.
    let content = content();
    let siege = &content.balance.siege;
    assert_eq!(
        4 * siege.panel_hp + siege.heartseed_hp + 5 * siege.room_hp + siege.shaft_hp,
        OPENING_TOWER_HP,
        "the opening tower is not the one the arithmetic below is written against"
    );

    let mut game = engine(1050);
    assert_eq!(
        crate::systems::siege::tower_integrity_permille(game.state()),
        1000,
        "a tower nothing has touched yet did not read as whole"
    );

    // A panel first. 65 hit points off 6,500 is one per cent of the
    // tower, so the figure has to land on exactly 990.
    game.state_mut_for_test()
        .tower
        .floor_mut(0)
        .expect("ground floor")
        .panel
        .hp -= 65;
    assert_eq!(
        crate::systems::siege::tower_integrity_permille(game.state()),
        990,
        "a breached panel did not move the standing figure by its own share of the tower"
    );

    // Then the mill: 130 more off, two per cent, down to 970.
    {
        let mill = game
            .state_mut_for_test()
            .tower
            .floor_mut(2)
            .expect("the mill's floor")
            .rooms
            .iter_mut()
            .find(|room| room.covers(3))
            .expect("the mill");
        mill.health.hp -= 130;
    }
    assert_eq!(
        crate::systems::siege::tower_integrity_permille(game.state()),
        970,
        "a chewed-up room left the standing figure where it was"
    );

    // Then the stairs: 260 more, four per cent, down to 930.
    {
        let state = game.state_mut_for_test();
        let stairs = state.tower.shafts[0].id;
        state.tower.shaft_mut(stairs).expect("the stairs").health.hp -= 260;
    }
    assert_eq!(
        crate::systems::siege::tower_integrity_permille(game.state()),
        930,
        "a half-cut staircase left the standing figure where it was"
    );
}

#[test]
fn the_standing_figure_stays_on_its_scale() {
    // The frontend draws this as a percentage and as a bar. A figure
    // that can run past full or below empty does not draw as anything a
    // player can read, and it would do it at exactly the moment they
    // most need the readout — during the wave that is taking the tower
    // apart.
    let mut game = engine(1051);
    provoke_fully(&mut game);

    let mut ever_hurt = false;
    for _ in 0..300 {
        game.step(60);
        let standing = crate::systems::siege::tower_integrity_permille(game.state());
        assert!(
            (0..=1000).contains(&standing),
            "the standing figure left its scale at {standing}"
        );
        ever_hurt |= standing < 1000;
    }
    assert!(
        ever_hurt,
        "nothing was ever damaged over ten minutes at full provocation, so this proves nothing"
    );

    // The bottom of the scale, which no survivable run reaches: a tower
    // with nothing left standing reads as nothing left standing.
    {
        let state = game.state_mut_for_test();
        for floor in &mut state.tower.floors {
            floor.panel.hp = 0;
            for room in &mut floor.rooms {
                room.health.hp = 0;
            }
        }
        for shaft in &mut state.tower.shafts {
            shaft.health.hp = 0;
        }
    }
    assert_eq!(
        crate::systems::siege::tower_integrity_permille(game.state()),
        0,
        "a tower with no hit points anywhere in it still claimed to be standing"
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
    // One crew member in each of the three states that has somebody
    // committed to a column, because a sever has to reach all three:
    // queueing at the foot of it, climbing it, and riding it. Somebody
    // left `Climbing` a column that no longer exists never arrives
    // anywhere and never takes another job — they are simply gone from
    // the tower, which reads as a bug and is one.
    //
    // The states are set by hand rather than waited for. Whether any of
    // three crew happens to be on the stairs at tick 900 is a fact
    // about the haul system's scheduling, and the version of this test
    // that took its chances on that passed for a long time while
    // proving nothing: at tick 900 the stairs were empty, so cutting
    // them evicted nobody and the assertions below were all vacuous.
    let mut game = engine(1006);
    game.step(900);
    let stairs = game.state().tower.shafts[0].id;

    {
        let state = game.state_mut_for_test();
        assert_eq!(
            state.crew.len(),
            3,
            "this test wants one crew member per boarding state"
        );
        state.crew[0].state = CrewState::Boarding {
            shaft: stairs,
            to_floor: 2,
        };
        state.crew[1].state = CrewState::Climbing {
            shaft: stairs,
            to_floor: 3,
        };
        // Caught mid-flight, between the first and second floors.
        state.crew[1].floor_fx = crate::fx::Fx::ratio(3, 2);
        state.crew[2].state = CrewState::Riding {
            shaft: stairs,
            car: 0,
            to_floor: 1,
        };
        state.tower.shaft_mut(stairs).expect("the stairs").riders = 1;
    }

    crate::systems::siege::evict_riders(game.state_mut_for_test(), stairs);

    let stuck = game.state().crew.iter().any(|member| {
        matches!(member.state,
            CrewState::Boarding { shaft, .. }
            | CrewState::Climbing { shaft, .. }
            | CrewState::Riding { shaft, .. } if shaft == stairs)
    });
    assert!(!stuck, "somebody is still on a shaft that was cut through");
    assert!(
        game.state()
            .crew
            .iter()
            .all(|member| member.state == CrewState::Idle),
        "somebody came off a severed shaft still mid-errand"
    );
    assert!(
        game.state()
            .crew
            .iter()
            .all(|member| member.floor_fx.frac_raw() == 0),
        "somebody came off a severed shaft standing between two floors"
    );
    assert_eq!(
        game.state().tower.shaft(stairs).expect("standing").riders,
        0
    );
}

#[test]
fn cutting_one_shaft_leaves_the_crew_on_the_other_one_alone() {
    // Eviction is per column, and it has to be. The whole reason to
    // spend eighteen poles on a second way up is that losing the first
    // one is survivable; a sever that emptied every shaft in the tower
    // would turn that redundancy into a liability, and the reroute this
    // milestone exists to produce would have nobody left to make it.
    let mut game = engine(1052);
    crate::tests::stock_poles(&mut game, 20);
    game.try_send(GameCommand::BuildShaft {
        shaft: "shaft.elevator".into(),
        low: 0,
        high: 3,
        slot: 7,
    })
    .expect("affordable");

    let stairs = game.state().tower.shafts[0].id;
    let elevator = game.state().tower.shafts[1].id;
    assert_ne!(stairs, elevator, "the second shaft was never built");

    let aboard = CrewState::Riding {
        shaft: elevator,
        car: 0,
        to_floor: 3,
    };
    {
        let state = game.state_mut_for_test();
        state.crew[0].state = CrewState::Climbing {
            shaft: stairs,
            to_floor: 2,
        };
        state.crew[1].state = aboard.clone();
        state.tower.shaft_mut(stairs).expect("the stairs").riders = 1;
        state
            .tower
            .shaft_mut(elevator)
            .expect("the elevator")
            .riders = 1;
    }

    crate::systems::siege::evict_riders(game.state_mut_for_test(), stairs);

    assert_eq!(
        game.state().crew[0].state,
        CrewState::Idle,
        "the crew member on the cut staircase was left climbing it"
    );
    assert_eq!(
        game.state().crew[1].state,
        aboard,
        "cutting the stairs threw somebody out of the elevator seven slots away"
    );
    assert_eq!(
        game.state().tower.shaft(elevator).expect("standing").riders,
        1,
        "the elevator's own rider count was cleared by damage to the stairs"
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
