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
