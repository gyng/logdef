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
    // §3.6's whole claim: a tower that never stops harvests at exactly
    // the M2 rate. 54 paces at 0.6 paces/tick is 90 ticks, the rate the
    // arm shipped with.
    let content = content();
    let cutter = content.room_idx("room.cutter_arm").expect("cutter arm");
    let Some(IntakeSource::Terrain { paces_per_item }) = content.room_rt(cutter).intake_source
    else {
        panic!("the cutter arm draws from the terrain");
    };
    let stride = content.balance.world.stride_paces_per_100_ticks;
    assert_eq!(paces_per_item * 100 / stride, 90);
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
