//! The journey as content: regions, palettes, branches, enclaves, and
//! the two intake sources that make walking and stopping opposites.
//!
//! Half of this file asserts what the shipped pack says; the other half
//! feeds the loader a deliberately broken pack and checks it says so.
//! `Content::load_embedded` panics on an invalid shipped pack by design
//! (`AGENTS.md` §IV) — a broken pack is a build error — so the only way
//! to test a rejection is to hand the loader a patched source.

use std::borrow::Cow;

use crate::content::{Content, DataSource, EmbeddedSource, IntakeSource, LoadError, RoomCategory};
use crate::ids::{BranchIdx, EnemyIdx, RegionIdx};
use crate::tests::{content, engine};

// ---------------------------------------------------------------------------
// A patched pack
// ---------------------------------------------------------------------------

/// The shipped pack with one path's bytes replaced, or one path added.
///
/// Everything else loads from the embedded source, so a test changes
/// exactly the thing it is about and inherits the rest.
struct Patched {
    path: String,
    bytes: Vec<u8>,
}

impl Patched {
    fn new(path: &str, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            path: path.to_owned(),
            bytes: bytes.into(),
        }
    }

    /// The load errors this patch produces. Panics if the patch was
    /// somehow legal, since a test that silently stops testing anything
    /// is worse than one that fails.
    fn errors(&self) -> Vec<LoadError> {
        Content::load(self).expect_err("the patched pack should not have loaded")
    }

    /// Whether the patch loads cleanly. The counterpart to `errors`,
    /// for the tests that are about a value being *allowed* — a
    /// validation rule with no test for what it lets through grows
    /// stricter than anyone intended.
    fn loads(&self) -> bool {
        Content::load(self).is_ok()
    }
}

impl DataSource for Patched {
    fn read(&self, path: &str) -> Result<Cow<'_, [u8]>, LoadError> {
        if path == self.path {
            return Ok(Cow::Borrowed(self.bytes.as_slice()));
        }
        EmbeddedSource.read(path)
    }

    fn list(&self, prefix: &str) -> Vec<String> {
        let mut all = EmbeddedSource.list(prefix);
        if self.path.starts_with(prefix) && !all.contains(&self.path) {
            all.push(self.path.clone());
            all.sort();
        }
        all
    }
}

fn reports(errors: &[LoadError], needle: &str) -> bool {
    errors.iter().any(|error| error.message.contains(needle))
}

/// A whole, legal region 2, parameterised on the one thing a test wants
/// to break. Branch-free and enclave-free so a rejection test is about
/// the field it names and nothing else.
fn region_two(order: u8) -> String {
    format!(
        r#"#![enable(implicit_some)]
        RegionDef(
            id: "region.drowned_city",
            name: "The Drowned City",
            order: {order},
            length_min_paces: 34000,
            length_max_paces: 46000,
            ruin_richness_min_pct: 60,
            ruin_richness_max_pct: 140,
            palette: [
                TerrainWeight(terrain: "terrain.drowned_street", weight: 40),
                TerrainWeight(terrain: "terrain.ruin_field", weight: 35),
                TerrainWeight(terrain: "terrain.clearing", weight: 15),
            ],
            threat_pct: 150,
            fork_interval_paces: 0,
        )"#
    )
}

// ---------------------------------------------------------------------------
// What the shipped pack says
// ---------------------------------------------------------------------------

#[test]
fn the_pack_defines_a_journey_and_interns_it() {
    let content = content();
    assert!(
        content.regions.len() >= 2,
        "M3 ships two regions; found {}",
        content.regions.len()
    );
    for (i, region) in content.regions.iter().enumerate() {
        assert_eq!(content.region_idx(&region.id), Some(RegionIdx(i as u16)));
    }
    for (i, branch) in content.branches.iter().enumerate() {
        assert_eq!(content.branch_idx(&branch.id), Some(BranchIdx(i as u16)));
    }
    assert_eq!(content.region_runtime.len(), content.regions.len());
    assert_eq!(content.branch_runtime.len(), content.branches.len());
}

#[test]
fn regions_come_out_in_journey_order_not_alphabetical_order() {
    let content = content();
    // Alphabetically the drowned city comes first; by journey order it
    // does not. If `RegionIdx` ever sorts by id this flips.
    assert_eq!(content.region(RegionIdx(0)).id, "region.deep_jungle");
    assert_eq!(content.region(RegionIdx(1)).id, "region.drowned_city");
    for (i, region) in content.regions.iter().enumerate() {
        assert_eq!(
            usize::from(region.order),
            i,
            "{} sits at journey position {i} carrying order {}",
            region.id,
            region.order
        );
    }
}

#[test]
fn every_region_owns_its_own_slice_of_the_flat_branch_list() {
    let content = content();
    let mut expected_first = 0u16;
    for (i, region) in content.regions.iter().enumerate() {
        let rt = &content.region_runtime[i];
        assert_eq!(rt.branch_first, expected_first, "{} starts late", region.id);
        assert_eq!(
            usize::from(rt.branch_end - rt.branch_first),
            region.branches.len()
        );
        for (n, branch) in region.branches.iter().enumerate() {
            let idx = BranchIdx(rt.branch_first + n as u16);
            assert_eq!(content.branch(idx).id, branch.id);
        }
        expected_first = rt.branch_end;
    }
    assert_eq!(usize::from(expected_first), content.branches.len());
}

#[test]
fn every_shipped_palette_clears_the_three_kind_minimum() {
    let content = content();
    for (i, region) in content.regions.iter().enumerate() {
        let palette = &content.region_runtime[i].palette;
        assert!(
            palette.len() >= 3,
            "{}'s palette has {} kinds; two alternates",
            region.id,
            palette.len()
        );
        assert!(palette.iter().all(|(_, weight)| *weight > 0));
        assert!(palette.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }
    for (i, branch) in content.branches.iter().enumerate() {
        assert!(
            content.branch_runtime[i].palette.len() >= 3,
            "{}'s palette has too few kinds",
            branch.id
        );
    }
}

#[test]
fn the_two_regions_are_opposed_on_yield_and_sun() {
    // The whole point of a region: the opposition M0 made a texture,
    // held long enough that it does not average out.
    let content = content();
    let mean = |idx: RegionIdx, pick: fn(&crate::content::TerrainRuntime) -> i64| {
        let palette = &content.region_rt(idx).palette;
        let total: i64 = palette.iter().map(|(_, weight)| *weight).sum();
        let sum: i64 = palette
            .iter()
            .map(|(kind, weight)| pick(&content.terrain_runtime[kind.get()]) * weight)
            .sum();
        sum / total.max(1)
    };
    let jungle = RegionIdx(0);
    let city = RegionIdx(1);
    assert!(
        mean(jungle, |rt| rt.yield_pct) > mean(city, |rt| rt.yield_pct),
        "the jungle should be the richer place to cut"
    );
    assert!(
        mean(city, |rt| rt.sun_pct) > mean(jungle, |rt| rt.sun_pct),
        "the city should be the richer place to charge"
    );
    assert!(content.region(city).threat_pct > content.region(jungle).threat_pct);
}

#[test]
fn the_enclave_stands_inside_the_region_however_its_length_rolls() {
    let content = content();
    let enclaves = content
        .regions
        .iter()
        .filter(|region| region.enclave.is_some())
        .count();
    assert_eq!(enclaves, 1, "M3 ships exactly one enclave");

    let city = content.region(RegionIdx(1));
    let enclave = city.enclave.as_ref().expect("region 2 has the enclave");
    assert!(enclave.at_paces > 0 && enclave.at_paces < city.length_min_paces);
    assert!(!enclave.offers.is_empty());
    assert!(enclave.recruits > 0);

    // Every offer resolved to a real item on both sides.
    let rt = content
        .region_rt(RegionIdx(1))
        .enclave
        .as_ref()
        .expect("the enclave resolved");
    assert_eq!(rt.offers.len(), enclave.offers.len());
    for (offer, resolved) in enclave.offers.iter().zip(&rt.offers) {
        assert_eq!(content.item(resolved.give.0).id, offer.give.item);
        assert_eq!(content.item(resolved.take.0).id, offer.take.item);
        assert!(resolved.stock > 0);
    }
}

#[test]
fn the_enclaves_offers_are_all_worse_than_the_chain_can_do_itself() {
    // The enclave is where a tower that lacks a room buys around the
    // gap once, not a substitute for building the room. A rate that
    // beat the chain would make it one.
    let content = content();
    let bamboo = content.item_idx("item.bamboo").unwrap();
    let poles = content.item_idx("item.poles").unwrap();
    let darts = content.item_idx("item.darts").unwrap();
    let rt = content
        .region_rt(RegionIdx(1))
        .enclave
        .as_ref()
        .expect("the enclave resolved");

    for offer in &rt.offers {
        if offer.give.0 == bamboo && offer.take.0 == poles {
            // A mill is one for one.
            assert!(
                offer.take.1 < offer.give.1,
                "the board beats a mill: {} bamboo for {} poles",
                offer.give.1,
                offer.take.1
            );
        }
        if offer.give.0 == poles && offer.take.0 == darts {
            // A thornwright is one pole for three darts.
            assert!(
                offer.take.1 < offer.give.1 * 3,
                "the board beats a thornwright: {} poles for {} darts",
                offer.give.1,
                offer.take.1
            );
        }
    }
}

#[test]
fn walking_harvests_and_stopping_salvages() {
    // The two intake sources are exact opposites, and the difference
    // lives in the content rather than in a special case in the system.
    let content = content();
    let cutter = content.room_idx("room.cutter_arm").expect("cutter arm");
    let rig = content.room_idx("room.salvage_rig").expect("salvage rig");

    assert!(matches!(
        content.room_rt(cutter).intake_source,
        Some(IntakeSource::Terrain { paces_per_item }) if paces_per_item > 0
    ));
    assert!(matches!(
        content.room_rt(rig).intake_source,
        Some(IntakeSource::Ruin { ticks_per_item, range_paces })
            if ticks_per_item > 0 && range_paces > 0
    ));

    // Every intake room says where it draws from, and nothing else
    // pretends to.
    for (i, room) in content.rooms.iter().enumerate() {
        let has_source = content.room_runtime[i].intake_source.is_some();
        assert_eq!(
            has_source,
            room.category == RoomCategory::Intake,
            "{} disagrees with its category about harvesting",
            room.id
        );
    }
}

#[test]
fn the_cutter_arm_converts_one_for_one_from_the_old_tick_rate() {
    // §3.6's claim, corrected: a tower that never stops harvests at
    // exactly the rate M2 was *measured* at. That was never the 90
    // ticks the arm was authored with — intake truncated it to 128 —
    // so keeping the authored figure at 54 would have meant shipping a
    // 42% faster arm under a promise that nothing had changed. 78 paces
    // at 0.6 paces a tick is 130 ticks, which is that measured rate
    // written down where a designer can see it.
    let content = content();
    let cutter = content.room_idx("room.cutter_arm").expect("cutter arm");
    let Some(IntakeSource::Terrain { paces_per_item }) = content.room_rt(cutter).intake_source
    else {
        panic!("the cutter arm draws from the terrain");
    };
    let stride = content.balance.world.stride_paces_per_100_ticks;
    assert_eq!(paces_per_item * 100 / stride, 130);
}

#[test]
fn only_ruin_bearing_terrain_holds_salvage() {
    let content = content();
    for (i, band) in content.terrain.iter().enumerate() {
        let rt = &content.terrain_runtime[i];
        assert_eq!(rt.ruin_feature.len(), band.feature_kinds.len());
        let bears_ruins = rt.ruin_feature.iter().any(|is_ruin| *is_ruin);
        assert_eq!(
            bears_ruins,
            !band.ruin_kinds.is_empty(),
            "{} disagrees with itself about holding ruins",
            band.id
        );
        if bears_ruins {
            assert!(rt.salvage_min > 0 && rt.salvage_max >= rt.salvage_min);
        }
    }
    // Both the ruin-field and the city's own band, and nothing else.
    let ruined: Vec<&str> = content
        .terrain
        .iter()
        .filter(|band| !band.ruin_kinds.is_empty())
        .map(|band| band.id.as_str())
        .collect();
    assert_eq!(ruined, ["terrain.drowned_street", "terrain.ruin_field"]);
}

#[test]
fn a_warden_is_not_something_an_ordinary_wave_can_draw() {
    let content = content();
    let warden = content.enemy_idx("enemy.feral_warden").expect("the warden");
    assert!(!content.enemy(warden).wave_eligible);
    for (i, def) in content.enemies.iter().enumerate() {
        assert_eq!(
            def.wave_eligible,
            EnemyIdx(i as u16) != warden,
            "{} disagrees about wave eligibility",
            def.id
        );
    }
}

#[test]
fn a_loud_tower_never_meets_a_warden_by_accident() {
    // The content flag is only worth having if the wave path reads it.
    // Pin provocation at the ceiling and let the jungle come.
    let mut game = engine(3100);
    let warden = game
        .content()
        .enemy_idx("enemy.feral_warden")
        .expect("the warden");
    let max = game.content().balance.siege.provocation_max;

    let mut anything_came = false;
    for _ in 0..60 {
        game.state_mut_for_test().siege.provocation = max;
        game.step(500);
        let creatures = &game.state().siege.enemies;
        anything_came |= !creatures.is_empty();
        assert!(
            creatures.iter().all(|enemy| enemy.def != warden),
            "a warden turned up in an ordinary wave"
        );
    }
    assert!(
        anything_came,
        "the jungle never came at all, so this proved nothing"
    );
}

// ---------------------------------------------------------------------------
// What the loader refuses
// ---------------------------------------------------------------------------

#[test]
fn a_duplicated_region_order_is_a_load_error() {
    let errors = Patched::new("regions/drowned_city.ron", region_two(0)).errors();
    assert!(
        reports(&errors, "contiguous run from zero"),
        "expected an order error, got {errors:?}"
    );
}

#[test]
fn a_gap_in_the_region_order_is_a_load_error() {
    let errors = Patched::new("regions/drowned_city.ron", region_two(2)).errors();
    assert!(
        reports(&errors, "contiguous run from zero"),
        "expected an order error, got {errors:?}"
    );
}

#[test]
fn a_two_kind_palette_is_a_load_error() {
    // Two kinds plus the generator's no-repeat rule is strict ABABAB
    // alternation, which reads as a bug rather than as terrain.
    let region = r#"#![enable(implicit_some)]
        RegionDef(
            id: "region.deep_jungle",
            name: "Deep Jungle",
            order: 0,
            length_min_paces: 52000,
            length_max_paces: 68000,
            ruin_richness_min_pct: 60,
            ruin_richness_max_pct: 140,
            palette: [
                TerrainWeight(terrain: "terrain.canopy", weight: 55),
                TerrainWeight(terrain: "terrain.clearing", weight: 45),
            ],
            threat_pct: 100,
            fork_interval_paces: 0,
        )"#;
    let errors = Patched::new("regions/deep_jungle.ron", region).errors();
    assert!(
        reports(&errors, "at least 3 terrain kinds"),
        "expected a palette error, got {errors:?}"
    );
}

#[test]
fn a_palette_weight_of_zero_does_not_count_toward_the_minimum() {
    // A zero weight is a kind the region cannot produce, so letting it
    // pad the count out to three would defeat the check.
    let region = r#"#![enable(implicit_some)]
        RegionDef(
            id: "region.deep_jungle",
            name: "Deep Jungle",
            order: 0,
            length_min_paces: 52000,
            length_max_paces: 68000,
            ruin_richness_min_pct: 60,
            ruin_richness_max_pct: 140,
            palette: [
                TerrainWeight(terrain: "terrain.canopy", weight: 55),
                TerrainWeight(terrain: "terrain.clearing", weight: 45),
                TerrainWeight(terrain: "terrain.ruin_field", weight: 0),
            ],
            threat_pct: 100,
            fork_interval_paces: 0,
        )"#;
    let errors = Patched::new("regions/deep_jungle.ron", region).errors();
    assert!(
        reports(&errors, "at least 3 terrain kinds"),
        "expected a palette error, got {errors:?}"
    );
}

#[test]
fn a_terrain_no_palette_can_produce_is_a_load_error() {
    // Content with no consumer, which is the v1 failure mode
    // (`DECISIONS.md` §9.1) and exactly what a pack-wide weight of zero
    // used to hide.
    let orphan = r#"#![enable(implicit_some)]
        TerrainDef(
            id: "terrain.orphan",
            name: "Nowhere",
            yield_pct: 100,
            sun_pct: 100,
            features_per_100_paces: 4,
            feature_kinds: ["rock"],
        )"#;
    let errors = Patched::new("terrain/orphan.ron", orphan).errors();
    assert!(
        reports(&errors, "no region or branch palette can produce"),
        "expected an unreferenced-terrain error, got {errors:?}"
    );
}

#[test]
fn a_palette_naming_terrain_that_does_not_exist_is_a_load_error() {
    let region = region_two(1).replace("terrain.clearing", "terrain.nonexistent");
    let errors = Patched::new("regions/drowned_city.ron", region).errors();
    assert!(
        reports(&errors, "unknown terrain terrain.nonexistent"),
        "expected an unknown-terrain error, got {errors:?}"
    );
}

#[test]
fn an_enclave_the_region_might_be_too_short_to_reach_is_a_load_error() {
    let region = region_two(1).replace(
        "fork_interval_paces: 0,",
        r#"fork_interval_paces: 0,
            enclave: EnclaveDef(
                id: "enclave.too_far",
                name: "Too Far",
                at_paces: 40000,
                offers: [],
                recruits: 1,
                recruit_cost: [CostEntryDef(item: "item.poles", amount: 30)],
            ),"#,
    );
    let errors = Patched::new("regions/drowned_city.ron", region).errors();
    assert!(
        reports(&errors, "falls outside the shortest this region can roll"),
        "expected an enclave placement error, got {errors:?}"
    );
}

#[test]
fn a_forking_region_with_nothing_to_choose_between_is_a_load_error() {
    let region = region_two(1).replace("fork_interval_paces: 0,", "fork_interval_paces: 15000,");
    let errors = Patched::new("regions/drowned_city.ron", region).errors();
    assert!(
        reports(&errors, "at least 2 branch archetypes"),
        "expected a branch error, got {errors:?}"
    );
}

#[test]
fn an_intake_rate_too_far_for_fixed_point_is_a_load_error() {
    // Intake accumulates effort against a threshold, so a slow rate is
    // simply a large threshold — there is no longer a rate so fine it
    // rounds away to nothing, which is what this check used to guard.
    // The remaining cliff is the other end: a threshold past what Q8.8
    // can hold clamps, and the room then works at the clamp rather than
    // at what it was authored to. Silently, and forever, which is the
    // one thing a content pack must never do.
    let arm = r#"#![enable(implicit_some)]
        RoomDef(
            id: "room.distant_arm",
            name: "Distant Arm",
            short: "DST",
            category: Intake,
            width: 2,
            intake: IntakeDef(
                item: "item.bamboo",
                source: Terrain(paces_per_item: 9000000),
                buffer_max: 8,
            ),
        )"#;
    let errors = Patched::new("rooms/distant_arm.ron", arm).errors();
    assert!(
        reports(&errors, "further than fixed point can carry"),
        "expected a rate-precision error, got {errors:?}"
    );

    // And the rate that used to be refused now loads, because it works:
    // 400 paces an item is slow, not impossible.
    let slow = r#"#![enable(implicit_some)]
        RoomDef(
            id: "room.slow_arm",
            name: "Slow Arm",
            short: "SLW",
            category: Intake,
            width: 2,
            intake: IntakeDef(
                item: "item.bamboo",
                source: Terrain(paces_per_item: 400),
                buffer_max: 8,
            ),
        )"#;
    assert!(
        Patched::new("rooms/slow_arm.ron", slow).loads(),
        "400 paces an item is slow, not impossible, and should load"
    );
}

#[test]
fn an_intake_room_with_no_rate_is_a_load_error() {
    let room = r#"#![enable(implicit_some)]
        RoomDef(
            id: "room.broken_rig",
            name: "Broken Rig",
            short: "BRK",
            category: Intake,
            width: 2,
            intake: IntakeDef(
                item: "item.scrap",
                source: Ruin(ticks_per_item: 60, range_paces: 0),
                buffer_max: 8,
            ),
        )"#;
    let errors = Patched::new("rooms/broken_rig.ron", room).errors();
    assert!(
        reports(&errors, "a positive rate and a reach"),
        "expected a ruin-intake error, got {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------
//
// Property tests over many seeds rather than example tests over one. A
// generator that satisfies its invariants on seed 1 and violates them
// on seed 400 is a generator that ships a broken run to somebody, and
// the only way to find that out at the desk is to try 400 seeds.

use crate::fx::{FX_SHIFT, paces_from_int};
use crate::rng::RngStreams;
use crate::state::world::World;

/// Enough seeds to catch a once-in-a-hundred violation, few enough to
/// stay fast. Individual tests step through this at a stride.
const SEEDS: std::ops::Range<u64> = 1..300;

fn fresh(seed: u64, content: &Content) -> (RngStreams, World) {
    let mut streams = RngStreams::new(seed);
    let world = World::new(&mut streams.world, content);
    (streams, world)
}

/// Walk a world forward without an engine, answering every fork with
/// its first branch, so generation keeps running.
fn walk(world: &mut World, streams: &mut RngStreams, content: &Content, steps: usize, each: i64) {
    for _ in 0..steps {
        world.distance += paces_from_int(each);
        if world.fork.is_some() {
            world.answer_fork(content, 0);
        }
        world.generate_ahead(&mut streams.world, content);
    }
}

#[test]
fn every_region_is_rolled_inside_the_length_it_was_authored_with() {
    let content = content();
    for seed in SEEDS {
        let (_, world) = fresh(seed, &content);
        let mut start = 0;
        for (i, roll) in world.journey.iter().enumerate() {
            let def = &content.regions[i];
            let length = (roll.end - start) >> FX_SHIFT;
            assert!(
                (def.length_min_paces..=def.length_max_paces).contains(&length),
                "seed {seed}: {} rolled {length} paces, outside {}..={}",
                def.id,
                def.length_min_paces,
                def.length_max_paces
            );
            assert!(
                (def.ruin_richness_min_pct..=def.ruin_richness_max_pct)
                    .contains(&roll.ruin_richness_pct),
                "seed {seed}: {} rolled {}% richness, outside its range",
                def.id,
                roll.ruin_richness_pct
            );
            start = roll.end;
        }
    }
}

#[test]
fn the_journey_is_the_same_every_time_a_seed_is_played() {
    // The whole point of a shared seed: two people comparing notes
    // about a run have to be talking about the same journey.
    let content = content();
    for seed in SEEDS.step_by(7) {
        let (_, first) = fresh(seed, &content);
        let (_, second) = fresh(seed, &content);
        assert_eq!(
            first.journey, second.journey,
            "seed {seed} rolled two different journeys"
        );
        assert_eq!(
            first.bands, second.bands,
            "seed {seed} generated two different opening stretches"
        );
    }
}

#[test]
fn seeds_differ_in_how_many_decisions_a_region_asks() {
    // What the length roll was bought for (`SYSTEMS.md` §3.2). Fork
    // spacing is fixed, so the roll changes how many forks fit — and if
    // every seed came out the same length it would be buying nothing.
    let content = content();
    let mut lengths: Vec<i64> = SEEDS
        .step_by(3)
        .map(|seed| fresh(seed, &content).1.journey[0].end)
        .collect();
    lengths.sort_unstable();
    lengths.dedup();
    assert!(
        lengths.len() > 10,
        "region 1 came out only {} distinct lengths across the seeds tried",
        lengths.len()
    );
}

#[test]
fn no_band_repeats_the_kind_before_it() {
    // Carried forward from M0, now that a palette rather than a
    // pack-wide weight decides what is eligible. The anti-frustration
    // constraint lives in the generator and is never a rule the player
    // can perceive.
    let content = content();
    for seed in SEEDS.step_by(5) {
        let (mut streams, mut world) = fresh(seed, &content);
        walk(&mut world, &mut streams, &content, 40, 300);
        for pair in world.bands.windows(2) {
            assert_ne!(
                pair[0].kind,
                pair[1].kind,
                "seed {seed}: two {} bands ran together",
                content.terrain(pair[0].kind).id
            );
        }
    }
}

#[test]
fn every_band_is_drawn_from_the_palette_covering_it() {
    // A band drawn from the wrong palette is what would make the
    // drowned city look like the deep jungle, and it is invisible in a
    // screenshot of either one on its own.
    let content = content();
    for seed in SEEDS.step_by(11) {
        let (mut streams, mut world) = fresh(seed, &content);
        for _ in 0..60 {
            world.distance += paces_from_int(400);
            if world.fork.is_some() {
                world.answer_fork(&content, 0);
            }
            world.generate_ahead(&mut streams.world, &content);

            let branch = world.branch;
            for band in &world.bands {
                let inside = branch.is_some_and(|b| band.start >= b.from && band.start < b.to);
                let palette = if inside {
                    &content.branch_rt(branch.expect("checked").def).palette
                } else {
                    &content.region_rt(world.region_at(band.start)).palette
                };
                assert!(
                    palette.iter().any(|(idx, _)| *idx == band.kind),
                    "seed {seed}: a {} band at {} came from no palette covering it",
                    content.terrain(band.kind).id,
                    band.start >> FX_SHIFT
                );
            }
        }
    }
}

#[test]
fn the_generator_never_runs_past_a_fork_nobody_has_answered() {
    // The whole basis of the halt: the tower stands at the split
    // because the ground beyond has not been decided, not because a
    // rule says stop. If generation ran on, the halt would be arbitrary
    // and the renderer would be drawing ground nobody may walk onto.
    let content = content();
    for seed in SEEDS.step_by(13) {
        let (mut streams, mut world) = fresh(seed, &content);
        for _ in 0..80 {
            world.distance += paces_from_int(400);
            world.generate_ahead(&mut streams.world, &content);
            if let Some(fork) = world.fork
                && fork.answer.is_none()
            {
                assert!(
                    world.generated_to <= fork.at,
                    "seed {seed}: generated {} paces past an unanswered fork",
                    (world.generated_to - fork.at) >> FX_SHIFT
                );
                break;
            }
        }
    }
}

#[test]
fn a_fork_never_lands_on_top_of_a_boundary_or_a_berth() {
    // A decision competing with a region change or an enclave for the
    // same stretch of horizon is two things at once, and one of them
    // gets missed.
    let content = content();
    let margin = paces_from_int(content.balance.journey.fork_edge_margin_paces);
    for seed in SEEDS.step_by(9) {
        let (mut streams, mut world) = fresh(seed, &content);
        for _ in 0..120 {
            world.distance += paces_from_int(500);
            if let Some(fork) = world.fork {
                let region = world.region_at(fork.at);
                let start = world.region_start_of(region);
                let end = world.journey[region.get()].end;
                assert!(
                    fork.at - start >= margin && end - fork.at >= margin,
                    "seed {seed}: a fork at {} crowds a region boundary",
                    fork.at >> FX_SHIFT
                );
                if let Some(enclave) = content.region(region).enclave.as_ref() {
                    let berth = start + paces_from_int(enclave.at_paces);
                    assert!(
                        (fork.at - berth).abs() >= margin,
                        "seed {seed}: a fork at {} crowds the enclave",
                        fork.at >> FX_SHIFT
                    );
                }
                world.answer_fork(&content, 0);
            }
            world.generate_ahead(&mut streams.world, &content);
        }
    }
}

#[test]
fn how_much_a_run_has_to_salvage_depends_on_the_seed() {
    // Richness is the roll the player actually feels, so it has to
    // reach the ruins: one run's city picked over and grudging, the
    // next one's worth stopping at more than once.
    let content = content();
    let mut totals = Vec::new();
    for seed in SEEDS.step_by(17) {
        let (mut streams, mut world) = fresh(seed, &content);
        walk(&mut world, &mut streams, &content, 200, 500);
        totals.push(world.features.iter().map(|f| f.salvage).sum::<i64>());
    }
    assert!(
        totals.iter().any(|total| *total > 0),
        "no seed generated a single ruin worth berthing at"
    );
    let low = totals.iter().min().copied().unwrap_or(0);
    let high = totals.iter().max().copied().unwrap_or(0);
    assert!(
        high > low,
        "every seed's ruins held exactly {high} scrap between them — \
         richness is not reaching the ruins"
    );
}

#[test]
fn nothing_but_a_ruin_holds_salvage() {
    let content = content();
    for seed in SEEDS.step_by(23) {
        let (mut streams, mut world) = fresh(seed, &content);
        walk(&mut world, &mut streams, &content, 60, 500);
        for feature in &world.features {
            if feature.salvage == 0 {
                continue;
            }
            let band = world
                .band_at(feature.at)
                .expect("every feature stands in a band");
            let rt = &content.terrain_runtime[band.kind.get()];
            assert!(
                rt.ruin_feature
                    .get(usize::from(feature.kind))
                    .copied()
                    .unwrap_or(false),
                "seed {seed}: a {} feature is carrying {} scrap",
                content.terrain(band.kind).id,
                feature.salvage
            );
        }
    }
}

#[test]
fn answering_a_fork_replaces_the_terrain_drawn_for_the_other_branch() {
    // Re-answering is legal right up to the split, so the terrain past
    // it has to match the answer that stands — otherwise the player
    // walks onto ground drawn from a palette they rejected.
    let content = content();
    let (mut streams, mut world) = fresh(4242, &content);
    for _ in 0..200 {
        world.distance += paces_from_int(200);
        world.generate_ahead(&mut streams.world, &content);
        if world.fork.is_some() {
            break;
        }
    }
    let fork = world.fork.expect("a fork inside the first region");

    world.answer_fork(&content, 0);
    world.generate_ahead(&mut streams.world, &content);
    assert!(world.generated_to > fork.at, "generation did not resume");

    world.answer_fork(&content, 1);
    assert!(
        world.generated_to <= fork.at,
        "changing the answer left behind terrain drawn for the branch that was dropped"
    );
    assert_eq!(
        world.branch.map(|b| b.def),
        Some(fork.branches[1]),
        "the second answer did not take"
    );
}

#[test]
fn a_fork_never_offers_the_same_way_twice() {
    let content = content();
    for seed in SEEDS.step_by(3) {
        let (mut streams, mut world) = fresh(seed, &content);
        for _ in 0..120 {
            world.distance += paces_from_int(500);
            if let Some(fork) = world.fork {
                assert_ne!(
                    fork.branches[0], fork.branches[1],
                    "seed {seed}: a fork offered the same branch on both sides"
                );
                world.answer_fork(&content, 0);
            }
            world.generate_ahead(&mut streams.world, &content);
        }
    }
}

#[test]
fn the_tower_stands_at_a_fork_until_it_is_answered_and_then_walks_on() {
    // End to end through the engine rather than the world alone. The
    // halt is a consequence of the generator, but all the player ever
    // sees is the tower stopping and starting again.
    let mut game = engine(7);
    let mut halted_at = None;
    for _ in 0..40_000 {
        game.step(1);
        if game.state().world.is_blocked() {
            halted_at = Some(game.state().world.distance);
            break;
        }
    }
    let halted_at = halted_at.expect("the tower reached a fork inside a region's length");

    game.step(600);
    assert_eq!(
        game.state().world.distance,
        halted_at,
        "a tower with nowhere to walk moved anyway"
    );

    game.try_send(crate::command::GameCommand::TakeFork { branch: 0 })
        .expect("the pending fork offers a branch 0");
    game.step(600);
    assert!(
        game.state().world.distance > halted_at,
        "answering the fork did not set the tower walking again"
    );
}

#[test]
fn standing_at_a_fork_costs_nothing_to_run_the_legs() {
    // A tower with nowhere to walk must not pay for trying. Charging
    // for it would quietly bleed a player who was only thinking.
    let mut game = engine(11);
    for _ in 0..40_000 {
        game.step(1);
        if game.state().world.is_blocked() {
            break;
        }
    }
    assert!(game.state().world.is_blocked(), "never reached a fork");
    assert!(
        game.state().walking,
        "this test is about a tower that still intends to walk"
    );

    let before = game.state().power.charge;
    game.step(300);
    assert!(
        game.state().power.charge >= before,
        "a tower standing at a fork spent charge on its legs: {before} then {}",
        game.state().power.charge
    );
}

// ---------------------------------------------------------------------------
// Walking, stopping, and berthing
// ---------------------------------------------------------------------------
//
// The four opposed forces of `SYSTEMS.md` §3.6 come down to one binary
// choice, and two of the four are what this section is about: walking
// harvests and stopping does not, stopping salvages and walking does
// not. Everything here goes through the engine — command in, state out
// — because "is the tower berthed" is not a flag anybody sets, it is a
// consequence of having stopped somewhere.

use crate::command::GameCommand;
use crate::engine::GameEngine;
use crate::fx::Paces;
use crate::state::{EnemyState, Feature};
use crate::tests::{item, step_quietly, stock_poles, total_in_flight};

/// Build the one rig the pack ships.
///
/// Floor 1 slot 4 is the only three-wide gap the opening tower has, and
/// the rig is three wide — so this is not a convenient corner, it is
/// the single place the room fits without tearing something out. Which
/// is the shape of the real decision.
fn build_rig(game: &mut GameEngine) {
    stock_poles(game, 20);
    game.try_send(GameCommand::PlaceRoom {
        room: "room.salvage_rig".into(),
        floor: 1,
        slot: 4,
    })
    .expect("floor 1 slot 4 is the rig-shaped gap in the opening tower");
}

/// Stop, and let stride report the stop.
///
/// Intake reads `strode` and `paces_last` one tick late (`SYSTEMS.md`
/// §3.8), so the tick a player halts on is still a walking tick as far
/// as the arms are concerned. A test that did not allow for that would
/// be measuring the lag rather than the rule.
fn halt(game: &mut GameEngine) {
    game.try_send(GameCommand::SetStriding { walking: false })
        .expect("halting is always legal");
    game.step(2);
}

/// Put a ruin holding `salvage` exactly where the tower stands, and
/// clear every other feature so nothing else is in reach.
///
/// Walking to a real one takes thousands of ticks and lands the tower
/// on whatever the seed happened to scatter. What a berth *is* has
/// already been property-tested against the generator above; these
/// tests are about what the rig does once there is a ruin beside it.
fn plant_ruin(game: &mut GameEngine, salvage: i64) -> Paces {
    let state = game.state_mut_for_test();
    let at = state.world.distance;
    state.world.features.clear();
    state.world.features.push(Feature {
        at,
        kind: 0,
        scale: 128,
        layer: 1,
        salvage,
        roused: false,
    });
    at
}

fn ruin(game: &GameEngine, at: Paces) -> &Feature {
    game.state()
        .world
        .features
        .iter()
        .find(|feature| feature.at == at)
        .expect("the planted ruin should still be in the window")
}

fn warden(game: &GameEngine) -> crate::ids::EnemyIdx {
    game.content()
        .enemy_idx("enemy.feral_warden")
        .expect("the pack ships a warden")
}

fn wardens_out(game: &GameEngine) -> usize {
    let warden = warden(game);
    game.state()
        .siege
        .enemies
        .iter()
        .filter(|enemy| enemy.def == warden)
        .count()
}

#[test]
fn a_walking_tower_strips_the_terrain_and_a_stopped_one_strips_nothing() {
    // The change that makes the stop/go decision bite (`SYSTEMS.md`
    // §3.6). Per-tick intake meant a parked tower stripped bamboo out
    // of ground it had already stripped, indefinitely.
    let mut game = engine(9001);
    step_quietly(&mut game, 600);

    let before = game.state().stats.items_harvested;
    step_quietly(&mut game, 900);
    let walked = game.state().stats.items_harvested - before;
    assert!(walked > 0, "a walking tower harvested nothing at all");

    halt(&mut game);
    let stopped_from = game.state().stats.items_harvested;
    step_quietly(&mut game, 900);
    assert_eq!(
        game.state().stats.items_harvested,
        stopped_from,
        "a parked tower stripped ground it had already walked over"
    );
}

#[test]
fn a_cutter_arm_harvests_at_the_rate_it_was_authored_to() {
    // 78 paces an item at the shipped stride of 0.6 paces a tick is
    // 130 ticks an item, and that is what the arm now does — the rate
    // the M2 economy was actually measured against, stated in the
    // content instead of arrived at by accident.
    //
    // It used to be authored at 54 (90 ticks) and *run* at 128, because
    // intake accrued a truncated per-tick fraction of an item —
    // `Fx::ratio(1, 90)` is `Fx(2)` in Q8.8 — so every authored rate
    // was quietly rounded to whatever 1/256ths could express.
    //
    // Pinned as a number, because the number is the point: if it moves
    // again, that is a balance change and wants the siege rows
    // re-measured with it.
    let content = content();
    let ticks_to_first_stalk = |terrain: &str| -> u32 {
        let mut game = engine(9002);
        let kind = content.terrain_idx(terrain).expect("a shipped band");
        {
            let state = game.state_mut_for_test();
            state.crew.clear();
            for band in &mut state.world.bands {
                band.kind = kind;
            }
        }
        for tick in 1..=900u32 {
            game.step(1);
            if game.state().stats.items_harvested > 0 {
                return tick;
            }
        }
        panic!("{terrain}: the arm never harvested at all");
    };

    // Two ticks over the authored 90: one is the lag intake spends
    // waiting for stride's first report (`GameState::paces_last`), and
    // one is the tick the crossing itself lands on.
    assert_eq!(ticks_to_first_stalk("terrain.clearing"), 132);
    // Rich ground is faster and poor ground slower, in proportion —
    // 140% of the rate and 50% of it, which is the thread §3.2's region
    // palettes pull on and which the old arithmetic flattened away.
    assert_eq!(ticks_to_first_stalk("terrain.canopy"), 95);
    assert_eq!(ticks_to_first_stalk("terrain.ruin_field"), 263);
}

#[test]
fn a_berthed_rig_draws_a_ruin_down_and_nothing_else_does() {
    let content = content();
    let scrap = item(&content, "item.scrap");
    let mut game = engine(9003);
    build_rig(&mut game);
    halt(&mut game);
    let at = plant_ruin(&mut game, 40);

    game.step(600);
    let taken = 40 - ruin(&game, at).salvage;
    assert!(taken > 0, "a berthed rig extracted nothing");
    assert_eq!(
        total_in_flight(game.state(), scrap),
        taken,
        "scrap appeared from somewhere other than the ruin"
    );
}

#[test]
fn a_tower_that_walks_past_a_ruin_leaves_it_alone() {
    // Range is a property of the rig, and a berth is a stop. A rig on a
    // walking tower is an ornament.
    let content = content();
    let scrap = item(&content, "item.scrap");
    let mut game = engine(9004);
    build_rig(&mut game);
    let at = plant_ruin(&mut game, 40);

    game.step(400);
    assert_eq!(ruin(&game, at).salvage, 40, "a walking tower salvaged");
    assert_eq!(total_in_flight(game.state(), scrap), 0);
    assert_eq!(wardens_out(&game), 0, "walking past woke something");
}

#[test]
fn stopping_beside_a_ruin_with_no_rig_does_nothing_at_all() {
    // There is no `Berth` command, so there is nothing to reject: no
    // extraction, no rousing, and no error. The empty space in the
    // build menu is the affordance.
    let mut game = engine(9005);
    halt(&mut game);
    let at = plant_ruin(&mut game, 40);

    game.step(900);
    assert_eq!(ruin(&game, at).salvage, 40);
    assert!(!ruin(&game, at).roused);
    assert_eq!(wardens_out(&game), 0);
    assert_eq!(game.state().siege.provocation, 0);
}

#[test]
fn a_ruin_rouses_its_wardens_once_and_never_again() {
    // What a ruin has instead of a lock — and `Feature.roused` is what
    // stops it being a toll charged every time you come back to one you
    // half emptied.
    let mut game = engine(9006);
    build_rig(&mut game);
    halt(&mut game);
    let at = plant_ruin(&mut game, 60);

    game.step(300);
    assert!(ruin(&game, at).roused, "the ruin gave up scrap in silence");
    let first = wardens_out(&game);
    assert!(first > 0, "nothing came out of the ruin");
    // 60 units at 120 threat per 100 buys two of a warden's 30.
    assert_eq!(first, 2, "the wave was not sized against what it held");
    let distance = game.state().world.distance;
    assert!(
        game.state()
            .siege
            .enemies
            .iter()
            .all(|enemy| enemy.at > distance),
        "a warden woke on top of the tower rather than out in the ground"
    );

    game.state_mut_for_test().siege.enemies.clear();
    game.step(900);
    assert_eq!(
        wardens_out(&game),
        0,
        "the same ruin roused a second time; coming back to one is a trap"
    );
}

#[test]
fn a_worked_out_ruin_stops_giving() {
    let content = content();
    let scrap = item(&content, "item.scrap");
    let mut game = engine(9007);
    build_rig(&mut game);
    halt(&mut game);
    let at = plant_ruin(&mut game, 3);

    game.step(1200);
    assert_eq!(ruin(&game, at).salvage, 0, "a ruin went into overdraft");
    assert_eq!(total_in_flight(game.state(), scrap), 3);
}

#[test]
fn a_rig_with_a_full_outbox_stalls_and_the_ruin_keeps_the_rest() {
    // The same stall every intake room has (§0.7), and the ruin keeps
    // whatever it has not given up rather than being drained into
    // nowhere.
    let content = content();
    let scrap = item(&content, "item.scrap");
    let mut game = engine(9008);
    build_rig(&mut game);
    game.state_mut_for_test().crew.clear();
    halt(&mut game);
    let at = plant_ruin(&mut game, 40);

    game.step(3000);
    let held = total_in_flight(game.state(), scrap);
    let buffer = game
        .state()
        .tower
        .floors
        .iter()
        .flat_map(|floor| floor.rooms.iter())
        .flat_map(|room| room.outputs.iter())
        .find(|stack| stack.item == scrap)
        .expect("the rig has an outbox")
        .max;
    assert_eq!(held, buffer, "the rig did not stall at a full outbox");
    assert_eq!(
        ruin(&game, at).salvage,
        40 - buffer,
        "the ruin gave up more than the rig could hold"
    );
}

#[test]
fn salvaging_draws_attention_the_way_cutting_does() {
    // Closing M2's forward reference in §2.6. A stopped tower harvests
    // nothing, so anything the dial reads here came out of the ruin.
    let mut game = engine(9009);
    build_rig(&mut game);
    halt(&mut game);
    plant_ruin(&mut game, 60);
    game.step(1200);
    let with_a_rig = game.state().siege.provocation;
    assert!(
        with_a_rig > 0,
        "taking a ruin apart went completely unnoticed"
    );

    let mut quiet = engine(9009);
    halt(&mut quiet);
    plant_ruin(&mut quiet, 60);
    quiet.step(1200);
    assert_eq!(
        quiet.state().siege.provocation,
        0,
        "a stopped tower with no rig provoked something"
    );
}

#[test]
fn a_wardens_grip_only_runs_out_once_the_tower_walks_away() {
    // The best thing in the milestone, asserted (`SYSTEMS.md` §3.4,
    // `DECISIONS.md` §11). A creature's grip counts down only while the
    // legs are running, and berthing is the one time the tower cannot
    // walk away — not because a rule forbids it, but because walking
    // away is what ends the salvage. No new mechanic produces this.
    let mut game = engine(9010);
    build_rig(&mut game);
    halt(&mut game);
    plant_ruin(&mut game, 60);

    let warden = warden(&game);
    let mut arrived = false;
    for _ in 0..3000 {
        game.step(1);
        arrived = game.state().siege.enemies.iter().any(|enemy| {
            enemy.def == warden && matches!(enemy.state, EnemyState::Attacking { .. })
        });
        if arrived {
            break;
        }
    }
    assert!(arrived, "no warden ever reached the berthed tower");

    let grip = |game: &GameEngine| {
        game.state()
            .siege
            .enemies
            .iter()
            .filter(|enemy| enemy.def == warden)
            .map(|enemy| enemy.cling_left)
            .max()
            .unwrap_or(0)
    };
    let held = grip(&game);
    game.step(600);
    assert_eq!(
        grip(&game),
        held,
        "a warden lost its grip on a tower that was standing still"
    );

    game.try_send(GameCommand::SetStriding { walking: true })
        .expect("setting off again is always legal");
    game.step(600);
    assert!(
        grip(&game) < held,
        "walking away did not start shaking the warden off"
    );
}

// ---------------------------------------------------------------------------
// The enclave, and the end of the run
// ---------------------------------------------------------------------------

use crate::command::CommandError;

/// Park the tower at the enclave, stopped and in reach.
///
/// Walking there honestly would take the better part of an in-game
/// week; these tests are about what happens once you arrive.
fn berth_at_the_enclave(game: &mut crate::engine::GameEngine) {
    let content = content();
    let at = game
        .state()
        .world
        .enclave_at(&content)
        .expect("the pack puts an enclave somewhere");
    {
        let state = game.state_mut_for_test();
        state.world.distance = at;
        state.walking = false;
        state.strode = false;
        // No wave mid-trade; these tests are about the transaction.
        state.siege.provocation = 0;
        state.siege.enemies.clear();
        state.siege.next_wave_tick = u64::MAX;
    }
}

#[test]
fn trading_needs_the_tower_to_be_standing_there() {
    // An enclave is a place, not a menu. A tower that walked past it
    // has lost it, because there is no going back down the axis, and
    // that is what makes stopping cost something.
    let mut game = engine(3000);
    let error = game
        .try_send(GameCommand::Trade { offer: 0 })
        .expect_err("the tower starts a region away from anyone");
    assert!(matches!(error, CommandError::NotBerthedAtAnEnclave));

    berth_at_the_enclave(&mut game);
    crate::tests::stock_poles(&mut game, 40);
    game.try_send(GameCommand::Trade { offer: 2 })
        .expect("offer 2 takes poles");
}

#[test]
fn an_offer_runs_out() {
    // Finite stock is what stops the enclave being a converter the
    // player can run in a loop until the numbers say yes.
    let content = content();
    let mut game = engine(3001);
    berth_at_the_enclave(&mut game);
    crate::tests::stock_poles(&mut game, 200);

    let stock = game.state().enclave_stock[2];
    for _ in 0..stock {
        game.try_send(GameCommand::Trade { offer: 2 })
            .expect("stock remains");
    }
    let error = game
        .try_send(GameCommand::Trade { offer: 2 })
        .expect_err("the offer is spent");
    assert!(matches!(error, CommandError::OfferExhausted { offer: 2 }));
    assert_eq!(game.state().enclave_stock[2], 0);

    let darts = item(&content, "item.darts");
    assert!(
        game.state().stock_of(darts) > 0,
        "the trades paid out nothing"
    );
}

#[test]
fn a_trade_the_tower_cannot_pay_for_changes_nothing() {
    // `DECISIONS.md` §4 in its narrowest form: a rejected command is a
    // no-op, down to the enclave's own bookkeeping.
    let mut game = engine(3002);
    berth_at_the_enclave(&mut game);

    // The tower ships with a few poles; spend them so the refusal is
    // about affording the trade rather than about anything else.
    {
        let poles = item(&content(), "item.poles");
        let state = game.state_mut_for_test();
        let held = state.stock_of(poles);
        state.take_stock(poles, held);
    }
    let before = game.state().enclave_stock.clone();
    let error = game
        .try_send(GameCommand::Trade { offer: 2 })
        .expect_err("the tower has no poles to spare");
    assert!(matches!(error, CommandError::InsufficientStock { .. }));
    assert_eq!(
        game.state().enclave_stock,
        before,
        "a refused trade still spent the enclave's stock"
    );
}

#[test]
fn nobody_is_recruited_twice_and_nobody_for_free() {
    let mut game = engine(3003);
    berth_at_the_enclave(&mut game);

    let broke = game
        .try_send(GameCommand::Recruit)
        .expect_err("recruiting is not free");
    assert!(matches!(broke, CommandError::InsufficientStock { .. }));

    crate::tests::stock_poles(&mut game, 200);
    let before = game.state().crew.len();
    game.try_send(GameCommand::Recruit)
        .expect("somebody here will come");
    assert_eq!(game.state().crew.len(), before + 1);

    let again = game
        .try_send(GameCommand::Recruit)
        .expect_err("only so many people live here");
    assert!(matches!(again, CommandError::NobodyToRecruit));
}

#[test]
fn a_recruit_is_the_same_kind_of_person_as_the_starting_crew() {
    // One construction path, so a hired hand cannot drift into being a
    // different sort of thing from one the tower set out with.
    let mut game = engine(3004);
    berth_at_the_enclave(&mut game);
    crate::tests::stock_poles(&mut game, 200);
    game.try_send(GameCommand::Recruit).expect("affordable");

    let newcomer = game.state().crew.last().expect("just recruited");
    assert!(!newcomer.name.is_empty(), "a nameless crew member");
    assert!(
        game.state()
            .crew
            .iter()
            .filter(|other| other.id == newcomer.id)
            .count()
            == 1,
        "the newcomer reused somebody else's id"
    );

    // And they work: given a few hundred ticks they pick something up.
    game.step(600);
    assert!(
        game.state().crew.len() > 3,
        "the recruit vanished on the next tick"
    );
}

#[test]
fn the_far_edge_of_the_journey_ends_the_run() {
    // The second of the two endings. Reaching it is an arrival, not a
    // victory — the tower stops because the world has run out, exactly
    // the way it stops at an unanswered fork.
    let mut game = engine(3005);
    {
        let state = game.state_mut_for_test();
        // A stride short of the end, with the jungle held off so the
        // other ending cannot happen first.
        state.world.distance = state.world.journey_end() - crate::fx::paces_from_int(2);
        state.siege.provocation = 0;
        state.siege.enemies.clear();
        state.siege.next_wave_tick = u64::MAX;
    }
    assert!(!game.state().arrived);

    game.step(120);
    assert!(
        game.state().arrived,
        "the tower never noticed it had arrived"
    );
    assert_eq!(
        game.state().world.distance,
        game.state().world.journey_end(),
        "the tower walked off the end of the world"
    );

    let held = game.state().world.distance;
    game.step(600);
    assert_eq!(
        game.state().world.distance,
        held,
        "an arrived tower kept walking"
    );
    assert!(!game.state().siege.lost, "arriving is not losing");
}

#[test]
fn the_snapshot_says_which_kind_of_standing_still_this_is() {
    // Four reasons, one silhouette. The renderer cannot tell them apart
    // from the geometry, and a tower waiting at a fork that reads as a
    // frozen game is the one failure that would make the whole
    // halt-at-a-fork argument worthless (`SYSTEMS.md` §3.3).
    use crate::snapshot::HaltView;

    let mut game = engine(3006);
    game.step(2);
    assert_eq!(game.view().journey.halt, HaltView::Walking);

    game.try_send(GameCommand::SetStriding { walking: false })
        .expect("always legal");
    game.step(2);
    assert_eq!(game.view().journey.halt, HaltView::Stopped);

    game.try_send(GameCommand::SetStriding { walking: true })
        .expect("always legal");
    {
        // Empty the banks and take the sails off the roof, so nothing
        // refills them.
        let content = content();
        let state = game.state_mut_for_test();
        state.power.charge = 0;
        for floor in &mut state.tower.floors {
            floor
                .rooms
                .retain(|room| content.room(room.def).solar.is_none());
        }
    }
    // Long enough for the prepaid stride block to run out: charge is
    // bought a hundred ticks at a time, so a tower that has just paid
    // keeps walking on credit for the rest of the block.
    game.step(150);
    assert_eq!(
        game.view().journey.halt,
        HaltView::Brownout,
        "a tower that asked for its legs and did not get them is not merely stopped"
    );

    let mut game = engine(3007);
    for _ in 0..40_000 {
        game.step(1);
        if game.state().world.is_blocked() {
            break;
        }
    }
    // One more tick: the step that lands the tower on the fork line is
    // still a step, so `strode` is true on the tick it arrives.
    game.step(1);
    assert_eq!(
        game.view().journey.halt,
        HaltView::Fork,
        "the tower is standing at a fork and the snapshot does not say so"
    );
}

#[test]
fn a_ruins_worth_crosses_the_bridge() {
    // The renderer draws a ruin still worth berthing at differently
    // from one already stripped, so the figure has to be in the
    // snapshot rather than inferred from the terrain kind.
    let mut game = engine(3008);
    for _ in 0..400 {
        game.step(30);
        if game.view().world.features.iter().any(|f| f.salvage > 0) {
            return;
        }
    }
    panic!("no ruin with anything in it ever reached the snapshot");
}

#[test]
fn a_run_can_be_played_from_the_first_pace_to_the_last() {
    // The whole of M3 in one test: two regions, a boundary crossing, a
    // second palette, the settlement in the middle of the drowned city,
    // and the far edge where the run ends.
    //
    // Every other test here holds something still to measure one thing.
    // This one holds nothing still, which is the only way to catch the
    // failures that live between systems — a fork scheduled behind the
    // tower after a region change, an enclave that never becomes
    // reachable, a generator that stops producing past a boundary.
    // Slower than the rest by a wide margin and worth every tick.
    let content = content();
    let mut game = engine(1);
    let enclave_at = game
        .state()
        .world
        .enclave_at(&content)
        .expect("the pack puts an enclave somewhere");

    let mut crossed = false;
    let mut traded = false;
    let mut berthed = false;

    for _ in 0..600_000 {
        if let Some(fork) = game.state().world.fork
            && fork.answer.is_none()
        {
            game.try_send(crate::command::GameCommand::TakeFork { branch: 0 })
                .expect("a pending fork always offers a branch 0");
        }

        if !berthed && game.state().world.distance >= enclave_at {
            berthed = true;
            game.try_send(crate::command::GameCommand::SetStriding { walking: false })
                .expect("always legal");
            game.step(2);
            traded = (0..4u8).any(|offer| {
                game.try_send(crate::command::GameCommand::Trade { offer })
                    .is_ok()
            });
            game.try_send(crate::command::GameCommand::SetStriding { walking: true })
                .expect("always legal");
        }

        game.step(1);
        crossed |= game.state().world.region.get() > 0;
        if game.state().arrived || game.state().siege.lost {
            break;
        }
    }

    let state = game.state();
    assert!(!state.siege.lost, "the run ended in the Heartseed going");
    assert!(state.arrived, "the tower never reached the far edge");
    assert!(crossed, "the tower never entered the second region");
    assert!(berthed, "the enclave never came within reach");
    assert!(traded, "nothing could be traded at the settlement");
    assert_eq!(
        state.world.region.get(),
        content.regions.len() - 1,
        "the run finished somewhere other than the last region"
    );
}

#[test]
fn a_region_boundary_belongs_to_the_region_after_it() {
    // Off by one here is a tower standing on the line and being told it
    // is still in the region behind it, which means the wrong palette
    // generated ahead of it and the crossing event never firing. It is
    // one comparison and nothing else in the simulation would notice.
    let content = content();
    let (_, world) = fresh(9, &content);
    let first_end = world.journey[0].end;

    assert_eq!(world.region_at(first_end - 1).get(), 0, "just short of it");
    assert_eq!(
        world.region_at(first_end).get(),
        1,
        "the first pace of a region is the region's own"
    );
    assert_eq!(world.region_at(0).get(), 0, "the very first pace");

    // Past the end of the last region — the finish line — clamps rather
    // than running off the end of the journey.
    let beyond = world.journey_end() + paces_from_int(10_000);
    assert_eq!(
        world.region_at(beyond).get(),
        content.regions.len() - 1,
        "past the far edge should clamp to the last region"
    );
}

#[test]
fn a_region_starts_where_the_one_before_it_ended() {
    // `region_start_of` is what every piece of fork arithmetic is
    // measured from, so a version of it that always answered zero would
    // put region 2's forks at region 1's distances — and only in region
    // 2, which is the half of the journey least often played.
    let content = content();
    let (_, world) = fresh(10, &content);

    assert_eq!(world.region_start_of(RegionIdx(0)), 0);
    for i in 1..content.regions.len() {
        assert_eq!(
            world.region_start_of(RegionIdx(i as u16)),
            world.journey[i - 1].end,
            "region {i} does not start where region {} ended",
            i - 1
        );
    }
    assert!(
        world.region_start_of(RegionIdx(1)) > 0,
        "the second region starts at zero, so nothing downstream can be right"
    );
}

#[test]
fn a_branch_covers_exactly_the_ground_it_was_authored_for() {
    // A branch is a palette override for a stretch, and the stretch has
    // to end where it says. One pace either way is terrain drawn from
    // the wrong side of a decision.
    let content = content();
    let (mut streams, mut world) = fresh(4242, &content);
    for _ in 0..300 {
        world.distance += paces_from_int(200);
        world.generate_ahead(&mut streams.world, &content);
        if world.fork.is_some() {
            break;
        }
    }
    world.answer_fork(&content, 0);
    let branch = world.branch.expect("answering sets the branch");
    let authored = paces_from_int(content.branch(branch.def).length_paces);
    assert_eq!(
        branch.to - branch.from,
        authored,
        "the branch does not span its authored length"
    );

    // The palette switches back on the pace the branch ends, not after.
    let region_palette = &content.region_rt(world.region_at(branch.to)).palette;
    let inside = world.palette_at_for_test(&content, branch.to - 1);
    let outside = world.palette_at_for_test(&content, branch.to);
    assert_ne!(
        inside, region_palette,
        "the last pace of a branch is already using the region's palette"
    );
    assert_eq!(
        outside, region_palette,
        "the first pace past a branch is still using the branch's palette"
    );
}

#[test]
fn forks_fall_on_the_interval_they_were_authored_with() {
    // Fork spacing is content, and deliberately so — predictable
    // punctuation is what lets a player see one coming. If the
    // arithmetic that places them drifted, they would still appear, and
    // still be answerable, and be somewhere else entirely.
    let content = content();
    let interval = content.region(RegionIdx(0)).fork_interval_paces;
    assert!(interval > 0, "region 1 is supposed to fork");

    let (mut streams, mut world) = fresh(21, &content);
    let mut seen = 0;
    for _ in 0..400 {
        world.distance += paces_from_int(300);
        if let Some(fork) = world.fork {
            let from_start =
                (fork.at - world.region_start_of(world.region_at(fork.at))) >> FX_SHIFT;
            assert_eq!(
                from_start % interval,
                0,
                "a fork at {from_start} paces into its region is not on the {interval}-pace \
                 interval"
            );
            assert!(from_start > 0, "a fork landed on the region's first pace");
            seen += 1;
            world.answer_fork(&content, 0);
        }
        world.generate_ahead(&mut streams.world, &content);
    }
    assert!(seen >= 2, "only saw {seen} fork(s) in a whole region");
}

#[test]
fn a_palettes_weights_are_the_proportions_it_gets() {
    // A palette says canopy 55, clearing 30, ruin-field 15, and that is
    // the whole of how a region is characterised — "biomass-rich and
    // sun-poor" is not written down anywhere except as those three
    // numbers. Nothing checked that the generator honoured them, so the
    // weighted pick could have drifted to nearly uniform and every
    // region would still have produced plausible-looking terrain.
    //
    // Measured by count over a long stretch rather than by distance,
    // because the weights govern which kind is drawn and band lengths
    // are rolled separately.
    let content = content();
    let palette = &content.region_rt(RegionIdx(0)).palette;

    let mut counts = vec![0i64; content.terrain.len()];
    let mut bands = 0i64;
    for seed in 1..40u64 {
        let (mut streams, mut world) = fresh(seed, &content);
        // Region 1 only, and no branches — a branch is a different
        // palette and would muddy the measurement.
        for _ in 0..30 {
            world.distance += paces_from_int(400);
            if world.fork.is_some() {
                break;
            }
            world.generate_ahead(&mut streams.world, &content);
        }
        for band in &world.bands {
            counts[band.kind.get()] += 1;
            bands += 1;
        }
    }

    assert!(
        bands > 500,
        "only {bands} bands to measure — too few to tell"
    );
    // Asserted as an ordering rather than as percentages, because the
    // no-repeat rule bends the distribution away from the raw weights
    // by design: it forbids whatever came last, which suppresses the
    // commonest kind and lifts the rarest. Canopy is authored at 55%
    // and settles near 42% for exactly that reason, and pinning 55
    // would be pinning a number the generator is not trying to hit.
    //
    // What must hold is that heavier means commoner, strictly, and that
    // the spread survives — a weighted pick that drifted to uniform, or
    // that inverted, fails here while any amount of no-repeat bending
    // passes.
    let mut ranked: Vec<(i64, i64, &str)> = palette
        .iter()
        .map(|(terrain, weight)| {
            (
                *weight,
                counts[terrain.get()] * 100 / bands,
                content.terrain(*terrain).id.as_str(),
            )
        })
        .collect();
    ranked.sort_by_key(|(weight, _, _)| -weight);
    for pair in ranked.windows(2) {
        let (heavy_w, heavy, heavy_id) = pair[0];
        let (light_w, light, light_id) = pair[1];
        assert!(
            heavy > light,
            "{heavy_id} is authored heavier than {light_id} ({heavy_w} against {light_w})              and came out rarer: {heavy}% against {light}%"
        );
    }
    let (_, most, _) = ranked.first().copied().expect("a palette has entries");
    let (_, least, _) = ranked.last().copied().expect("a palette has entries");
    assert!(
        most * 10 >= least * 15,
        "the heaviest kind came out {most}% and the lightest {least}% — that is nearly          uniform, so the weights are not being read"
    );

    // And nothing outside the palette got in at all.
    for (i, count) in counts.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let idx = crate::ids::TerrainIdx(i as u16);
        assert!(
            palette.iter().any(|(terrain, _)| *terrain == idx),
            "{} appeared in region 1 and is not in its palette",
            content.terrain(idx).id
        );
    }
}

#[test]
fn the_streaming_window_stays_the_same_size_however_far_the_tower_walks() {
    // The premise of the whole world model: terrain is a stream, not a
    // level, so a run of any length costs the same memory. It is also
    // the property that quietly stops holding if a boundary comparison
    // in `generate_ahead` or `prune_behind` drifts by one — the window
    // would creep, and nothing would fail until a long run.
    let content = content();
    let balance = &content.balance.world;
    let (mut streams, mut world) = fresh(31, &content);

    let mut widest = 0usize;
    for step in 0..600 {
        world.distance += paces_from_int(500);
        if world.fork.is_some() {
            world.answer_fork(&content, 0);
        }
        world.generate_ahead(&mut streams.world, &content);
        world.prune_behind(&content);

        // Always somewhere to stand.
        assert!(
            world.band_at(world.distance).is_some(),
            "step {step}: the tower is standing on nothing at {} paces",
            world.distance >> FX_SHIFT
        );
        // Always something ahead, unless a fork is holding generation.
        assert!(
            world.generated_to >= world.distance,
            "step {step}: the generator fell behind the tower"
        );
        // And nothing kept from far behind.
        let oldest = world.bands.first().map_or(world.distance, |b| b.start);
        let behind = (world.distance - oldest) >> FX_SHIFT;
        assert!(
            behind <= balance.stream_behind_paces + balance.band_max_paces,
            "step {step}: keeping {behind} paces of history, past the \
             {} the window allows",
            balance.stream_behind_paces
        );
        widest = widest.max(world.bands.len());
    }

    assert!(
        widest < 40,
        "the band window grew to {widest}, so it is not a window"
    );
}

#[test]
fn a_fork_that_would_land_on_the_enclave_is_moved_along() {
    // `fork_edge_margin_paces` keeps a decision from competing with a
    // boundary or a berth for the same stretch of horizon. Nothing in
    // the shipped pack triggers it — region 2's enclave sits 8,000 in
    // and its first fork 15,000 in, which clears the 5,000 margin — so
    // the guard has never once run in anger, and mutation testing found
    // every comparison in it alive.
    //
    // Content changes. A pack where a fork *does* land on the enclave
    // is one edit away, and the failure would be two things happening
    // at the same place with no error anywhere. So this puts the
    // enclave exactly on the first fork and insists the fork moves.
    let region = r#"#![enable(implicit_some)]
        RegionDef(
            id: "region.drowned_city",
            name: "The Drowned City",
            order: 1,
            length_min_paces: 40000,
            length_max_paces: 40000,
            ruin_richness_min_pct: 100,
            ruin_richness_max_pct: 100,
            palette: [
                TerrainWeight(terrain: "terrain.drowned_street", weight: 40),
                TerrainWeight(terrain: "terrain.ruin_field", weight: 35),
                TerrainWeight(terrain: "terrain.clearing", weight: 25),
            ],
            threat_pct: 150,
            fork_interval_paces: 10000,
            branches: [
                BranchDef(
                    id: "branch.flooded_boulevard",
                    name: "Flooded Boulevard",
                    length_paces: 6000,
                    palette: [
                        TerrainWeight(terrain: "terrain.drowned_street", weight: 60),
                        TerrainWeight(terrain: "terrain.ruin_field", weight: 25),
                        TerrainWeight(terrain: "terrain.clearing", weight: 15),
                    ],
                    threat_pct: 100,
                ),
                BranchDef(
                    id: "branch.green_terraces",
                    name: "Green Terraces",
                    length_paces: 6000,
                    palette: [
                        TerrainWeight(terrain: "terrain.canopy", weight: 50),
                        TerrainWeight(terrain: "terrain.clearing", weight: 25),
                        TerrainWeight(terrain: "terrain.drowned_street", weight: 25),
                    ],
                    threat_pct: 80,
                ),
            ],
            enclave: EnclaveDef(
                id: "enclave.high_water",
                name: "High Water",
                at_paces: 10000,
                offers: [
                    OfferDef(
                        give: CostEntryDef(item: "item.scrap", amount: 4),
                        take: CostEntryDef(item: "item.poles", amount: 3),
                        stock: 20,
                    ),
                ],
                recruits: 1,
                recruit_cost: [CostEntryDef(item: "item.poles", amount: 30)],
            ),
        )"#;

    let patched = Patched::new("regions/drowned_city.ron", region);
    let content = std::sync::Arc::new(
        Content::load(&patched).expect("the patched pack is legal, just awkwardly placed"),
    );
    let margin = paces_from_int(content.balance.journey.fork_edge_margin_paces);

    let mut streams = RngStreams::new(77);
    let mut world = World::new(&mut streams.world, &content);
    let city = RegionIdx(1);
    let city_start = world.region_start_of(city);
    let berth = city_start + paces_from_int(10_000);

    // Walk into the drowned city and past its far side, answering
    // everything, and check no fork ever crowds the settlement.
    world.distance = city_start;
    world.enter_region(&content, city);
    let mut seen = 0;
    for _ in 0..200 {
        world.distance += paces_from_int(400);
        if let Some(fork) = world.fork {
            assert!(
                (fork.at - berth).abs() >= margin,
                "a fork landed {} paces from the settlement, inside the {} margin",
                ((fork.at - berth).abs()) >> FX_SHIFT,
                content.balance.journey.fork_edge_margin_paces
            );
            seen += 1;
            world.answer_fork(&content, 0);
        }
        world.generate_ahead(&mut streams.world, &content);
    }
    assert!(
        seen > 0,
        "the region never forked at all, so nothing was skipped past — the test proves nothing"
    );
}

#[test]
fn every_region_that_says_it_forks_actually_does() {
    // `schedule_first_fork` is one addition, and an addition that went
    // wrong would not fail loudly: a region would simply never split,
    // and the only sign would be a run that felt oddly featureless.
    // Region 2 is the one this would go unnoticed in, since almost
    // nothing routinely plays that far.
    let content = content();
    for (i, def) in content.regions.iter().enumerate() {
        if def.fork_interval_paces <= 0 {
            continue;
        }
        let region = RegionIdx(i as u16);
        let mut streams = RngStreams::new(500 + i as u64);
        let mut world = World::new(&mut streams.world, &content);
        world.distance = world.region_start_of(region);
        world.enter_region(&content, region);

        let mut forked = false;
        for _ in 0..300 {
            world.distance += paces_from_int(400);
            if world.fork.is_some() {
                forked = true;
                world.answer_fork(&content, 0);
            }
            world.generate_ahead(&mut streams.world, &content);
            if world.region_at(world.distance) != region {
                break;
            }
        }
        assert!(
            forked,
            "{} authors a {}-pace fork interval and never split",
            def.id, def.fork_interval_paces
        );
    }
}

#[test]
fn a_settlement_plates_the_whole_tower_and_what_gets_built_after() {
    // Shell work is the only permanent upgrade in the game, and the
    // only thing scrap can become besides a trade. It has to apply to
    // every floor, including ones that do not exist yet — otherwise
    // growing taller means growing a soft spot, and the player has to
    // remember which storeys were done.
    let content = content();
    let scrap = item(&content, "item.scrap");
    let mut game = engine(4200);
    berth_at_the_enclave(&mut game);
    // Scrap first: shelves hold one kind each, and a tower full of
    // poles has nowhere to put metal.
    game.state_mut_for_test().shelve(scrap, 40);
    crate::tests::stock_poles(&mut game, 20);

    let before: Vec<i64> = game
        .state()
        .tower
        .floors
        .iter()
        .map(|floor| floor.panel.max)
        .collect();
    game.try_send(GameCommand::Reinforce)
        .expect("the city plates hulls, and there is scrap for it");
    let bonus = game.state().tower.shell_bonus;
    assert!(bonus > 0, "plating added nothing");

    for (floor, was) in game.state().tower.floors.iter().zip(&before) {
        assert_eq!(
            floor.panel.max,
            was + bonus,
            "floor {} was not plated",
            floor.index
        );
        assert_eq!(
            floor.panel.hp, floor.panel.max,
            "an undamaged panel came back reading as damaged"
        );
    }

    // And the next floor up arrives already plated.
    let top = game.state().tower.top_floor();
    game.try_send(GameCommand::BuildFloor)
        .expect("affordable with 60 poles");
    let fresh = game
        .state()
        .tower
        .floor(top + 1)
        .expect("the new floor is there");
    assert_eq!(
        fresh.panel.max,
        content.balance.siege.panel_hp + bonus,
        "a floor built after the plating came up bare"
    );
}

#[test]
fn plating_a_breach_does_not_close_it() {
    // New material is material, not a repair. A panel already breached
    // comes back plated and still breached, so shell work cannot be
    // used as a way to skip the repair loop it is supposed to make
    // survivable.
    let mut game = engine(4201);
    berth_at_the_enclave(&mut game);
    let scrap = item(&content(), "item.scrap");
    game.state_mut_for_test().shelve(scrap, 60);
    {
        let floor = game
            .state_mut_for_test()
            .tower
            .floor_mut(0)
            .expect("ground floor");
        floor.panel.hp = 0;
    }

    game.try_send(GameCommand::Reinforce).expect("affordable");
    let panel = game.state().tower.floor(0).expect("ground floor").panel;
    assert_eq!(panel.hp, 0, "plating quietly repaired a breach");
    assert!(panel.max > 0, "the breached floor was not plated at all");
}

#[test]
fn a_settlement_only_does_so_much_shell_work() {
    let mut game = engine(4202);
    berth_at_the_enclave(&mut game);
    let scrap = item(&content(), "item.scrap");
    game.state_mut_for_test().shelve(scrap, 60);

    let times = game.state().shell_work_left;
    assert!(times > 0, "the pack authors no shell work at all");
    for _ in 0..times {
        game.try_send(GameCommand::Reinforce)
            .expect("they will do it this many times");
    }
    let error = game
        .try_send(GameCommand::Reinforce)
        .expect_err("and no more");
    assert!(matches!(error, CommandError::NoShellWorkLeft));
}

#[test]
fn shell_work_costs_scrap_and_a_refusal_costs_nothing() {
    let mut game = engine(4203);
    berth_at_the_enclave(&mut game);
    let scrap = item(&content(), "item.scrap");

    let error = game
        .try_send(GameCommand::Reinforce)
        .expect_err("no scrap aboard");
    assert!(matches!(error, CommandError::InsufficientStock { .. }));
    assert_eq!(
        game.state().tower.shell_bonus,
        0,
        "a refused plating still plated the tower"
    );

    game.state_mut_for_test().shelve(scrap, 60);
    let held = game.state().stock_of(scrap);
    game.try_send(GameCommand::Reinforce).expect("affordable");
    assert!(game.state().stock_of(scrap) < held, "the plating was free");
}
