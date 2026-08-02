//! The world the tower walks through.
//!
//! Terrain is not a level; it is a stream. Bands are generated ahead of
//! the tower and pruned behind it, so a run of any length costs the
//! same memory. A band's character sets what the intake rooms can pull
//! out of it — shaded jungle is biomass-rich, an open ruin-field is
//! barren — which is the first thread of "your route is your power mix".

use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::fx::{Paces, paces_from_int};
use crate::ids::TerrainIdx;
use crate::rng::Rng;

/// One stretch of terrain with a single character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainBand {
    /// First pace covered, in Q8.8 pace units.
    pub start: Paces,
    /// Length in Q8.8 pace units. Always positive.
    pub length: Paces,
    pub kind: TerrainIdx,
}

impl TerrainBand {
    #[must_use]
    pub const fn end(&self) -> Paces {
        self.start + self.length
    }

    #[must_use]
    pub const fn contains(&self, at: Paces) -> bool {
        at >= self.start && at < self.end()
    }
}

/// A decorative landmark: a tree, a fern, a rock, a drowned ruin.
///
/// Drawn from the **world** stream rather than the cosmetic one,
/// because M3 turns ruins into berthing sites — where a ruin stands has
/// to be a fact about the run, not about the frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feature {
    pub at: Paces,
    /// Index into the owning band's `feature_kinds`. Presentation only.
    pub kind: u8,
    /// 0..=255 size jitter, for parallax variety.
    pub scale: u8,
    /// Which depth layer to draw on: 0 far, 1 mid, 2 near.
    pub layer: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    /// Total distance walked, Q8.8 paces.
    pub distance: Paces,
    /// Bands sorted by `start`, contiguous, covering the live window.
    pub bands: Vec<TerrainBand>,
    /// Features sorted by `at`, within the live window.
    pub features: Vec<Feature>,
    /// How far ahead the generator has produced.
    pub generated_to: Paces,
}

impl World {
    /// A world with the opening stretch already streamed in, so the
    /// first frame shows terrain rather than a void.
    #[must_use]
    pub fn new(rng: &mut Rng, content: &Content) -> Self {
        let mut world = Self {
            distance: 0,
            bands: Vec::new(),
            features: Vec::new(),
            generated_to: 0,
        };
        world.generate_ahead(rng, content);
        world
    }

    /// The band the tower is standing in right now.
    #[must_use]
    pub fn band_at(&self, at: Paces) -> Option<&TerrainBand> {
        self.bands.iter().find(|band| band.contains(at))
    }

    /// Yield multiplier, in percent, of the terrain under the tower.
    /// 100 means "as authored"; a barren ruin-field is lower.
    #[must_use]
    pub fn current_yield_pct(&self, content: &Content) -> i64 {
        self.band_at(self.distance).map_or(100, |band| {
            content.terrain_runtime[band.kind.get()].yield_pct
        })
    }

    /// Extend the terrain until it reaches `stream_ahead_paces` past
    /// the tower. Called every tick; usually a no-op.
    pub fn generate_ahead(&mut self, rng: &mut Rng, content: &Content) {
        let balance = &content.balance.world;
        let target = self.distance + paces_from_int(balance.stream_ahead_paces);

        while self.generated_to < target {
            let kind = pick_band_kind(rng, content, self.bands.last().map(|b| b.kind));
            let length_paces = rng.range(
                balance.band_min_paces,
                balance.band_max_paces.max(balance.band_min_paces),
            );
            let length = paces_from_int(length_paces);
            let band = TerrainBand {
                start: self.generated_to,
                length,
                kind,
            };
            self.scatter_features(rng, content, &band);
            self.generated_to = band.end();
            self.bands.push(band);
        }
    }

    /// Drop everything that has fallen fully behind the live window.
    pub fn prune_behind(&mut self, content: &Content) {
        let cutoff = self.distance - paces_from_int(content.balance.world.stream_behind_paces);
        if cutoff <= 0 {
            return;
        }
        self.bands.retain(|band| band.end() > cutoff);
        self.features.retain(|feature| feature.at > cutoff);
    }

    fn scatter_features(&mut self, rng: &mut Rng, content: &Content, band: &TerrainBand) {
        let def = content.terrain(band.kind);
        if def.feature_kinds.is_empty() || def.features_per_100_paces <= 0 {
            return;
        }
        let whole_paces = band.length >> crate::fx::FX_SHIFT;
        let count = (whole_paces * def.features_per_100_paces) / 100;
        for _ in 0..count {
            let offset = rng.range(0, whole_paces.max(1) - 1);
            let kind = rng.index(def.feature_kinds.len()).unwrap_or(0) as u8;
            self.features.push(Feature {
                at: band.start + paces_from_int(offset),
                kind,
                scale: (rng.range(0, 255)) as u8,
                layer: (rng.range(0, 2)) as u8,
            });
        }
        // Keep the render order stable regardless of draw order.
        self.features.sort_by_key(|f| (f.at, f.layer, f.kind));
    }
}

/// Weighted pick that never repeats the previous band's kind, so the
/// horizon always changes. Falls back to allowing a repeat if the pack
/// only defines one kind.
fn pick_band_kind(rng: &mut Rng, content: &Content, previous: Option<TerrainIdx>) -> TerrainIdx {
    let eligible: Vec<(TerrainIdx, i64)> = content
        .terrain_runtime
        .iter()
        .enumerate()
        .map(|(i, rt)| (TerrainIdx(i as u16), rt.weight))
        .filter(|(idx, weight)| *weight > 0 && Some(*idx) != previous)
        .collect();

    let pool = if eligible.is_empty() {
        content
            .terrain_runtime
            .iter()
            .enumerate()
            .map(|(i, rt)| (TerrainIdx(i as u16), rt.weight.max(1)))
            .collect()
    } else {
        eligible
    };

    let total: i64 = pool.iter().map(|(_, w)| *w).sum();
    let mut roll = rng.range(0, total - 1);
    for (idx, weight) in &pool {
        if roll < *weight {
            return *idx;
        }
        roll -= *weight;
    }
    pool[0].0
}
