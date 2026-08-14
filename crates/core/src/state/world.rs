//! The world the tower walks through.
//!
//! Terrain is not a level; it is a stream. Bands are generated ahead of
//! the tower and pruned behind it, so a run of any length costs the
//! same memory. A band's character sets what the intake rooms can pull
//! out of it — shaded jungle is biomass-rich, an open ruin-field is
//! barren — which is the first thread of "your route is your power mix".

use serde::{Deserialize, Serialize};

use crate::content::{ApproachWeights, Content};
use crate::fx::{Paces, paces_from_int};
use crate::ids::{BranchIdx, RegionIdx, TerrainIdx};
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
/// One beat on the route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waypoint {
    pub at: Paces,
    /// Indexes `content.waypoints`.
    pub def: u16,
    /// Answered already. Stays in the list until it is pruned so the
    /// renderer can draw it going past as a thing that happened.
    pub taken: bool,
}

/// Floor on how close two beats may fall, and on how near a fork one
/// may be. Paces.
const MIN_WAYPOINT_INTERVAL_PACES: i64 = 400;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feature {
    pub at: Paces,
    /// Index into the owning band's `feature_kinds`.
    ///
    /// This used to be marked presentation-only. It is not, any more:
    /// `TerrainDef.ruin_kinds` says which kinds are ruins, and a ruin
    /// is a place the tower can stop and work at.
    pub kind: u8,
    /// 0..=255 size jitter, for parallax variety.
    pub scale: u8,
    /// Which depth layer to draw on: 0 far, 1 mid, 2 near.
    pub layer: u8,
    /// Scrap still in this ruin. Zero for anything that is not one, and
    /// for one that has been worked out.
    pub salvage: i64,
    /// Whether disturbing this ruin has already brought its wardens
    /// out. Once roused, a ruin does not rouse again — the second berth
    /// at a half-emptied ruin is safe, which is what makes coming back
    /// to one a real option rather than a repeated toll.
    pub roused: bool,
}

/// One region's rolled shape, decided at run start and fixed for the
/// life of the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionRoll {
    /// Absolute distance at which this region ends.
    pub end: Paces,
    /// Scales what every ruin in this region holds.
    pub ruin_richness_pct: i64,
}

/// A fork the generator has reached and the player has not yet crossed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingFork {
    /// Where the route splits.
    pub at: Paces,
    /// The two archetypes on offer. Always distinct.
    pub branches: [BranchIdx; 2],
    /// Which one the player has picked, if they have. Replaceable right
    /// up until the tower crosses the line.
    pub answer: Option<u8>,
}

/// A branch the tower is currently walking through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveBranch {
    pub def: BranchIdx,
    pub from: Paces,
    pub to: Paces,
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
    /// Beats scattered along the route, sorted by `at`, within the
    /// live window.
    ///
    /// **The answer to "the journey is a screensaver"** (`SYSTEMS.md`
    /// §6.14). Forks are rare by design and an enclave is a whole
    /// settlement; a waypoint is the small thing in between that comes
    /// into range, asks one question and goes past.
    pub waypoints: Vec<Waypoint>,
    /// Where the next one falls. Rolled forward as they are placed, the
    /// same way `next_fork_at` is.
    pub next_waypoint_at: Paces,
    /// Every region's rolled length and richness, in journey order.
    ///
    /// Rolled once, at run start, rather than on entry to each region.
    /// The generator streams `stream_ahead_paces` past a boundary, so
    /// rolling on entry would ask it to produce terrain for a region
    /// whose length had not been decided yet (`SYSTEMS.md` §3.2).
    pub journey: Vec<RegionRoll>,
    /// The region the tower is standing in. Derivable from `distance`
    /// and `journey`, and stored anyway because crossing a boundary is
    /// an event and not only a fact.
    pub region: RegionIdx,
    /// Where that region began, so the fork arithmetic stays local.
    pub region_start: Paces,
    /// The next fork position the generator has still to place, as an
    /// absolute distance. Past the region's last fork this sits beyond
    /// the region's end and simply never comes up.
    pub next_fork_at: Paces,
    pub fork: Option<PendingFork>,
    pub branch: Option<ActiveBranch>,
}

impl World {
    /// A world with the opening stretch already streamed in, so the
    /// first frame shows terrain rather than a void.
    #[must_use]
    pub fn new(rng: &mut Rng, content: &Content) -> Self {
        // The whole journey is rolled here, before a single band is
        // generated. Both draws come off the `world` stream: how long a
        // region is and how much its ruins hold are facts about the
        // run, and two people sharing a seed have to get the same
        // journey (`DECISIONS.md` §2).
        let mut journey = Vec::with_capacity(content.regions.len());
        let mut end = 0;
        for def in &content.regions {
            let length = rng.range(
                def.length_min_paces,
                def.length_max_paces.max(def.length_min_paces),
            );
            let ruin_richness_pct = rng.range(
                def.ruin_richness_min_pct,
                def.ruin_richness_max_pct.max(def.ruin_richness_min_pct),
            );
            end += paces_from_int(length);
            journey.push(RegionRoll {
                end,
                ruin_richness_pct,
            });
        }

        // Landmarks are placed once from the rolled journey rather than
        // drawn from the repeating beat stream. Their position is a
        // deterministic fraction of their owning region, so they can be
        // previewed, ignored, taken and then pruned without ever being
        // generated again.
        let mut waypoints = Vec::new();
        for (def, runtime) in content.waypoint_runtime.iter().enumerate() {
            let Some(region) = runtime.landmark_region else {
                continue;
            };
            let start = match region.get().checked_sub(1) {
                Some(previous) => journey.get(previous).map_or(0, |roll| roll.end),
                None => 0,
            };
            let Some(end) = journey.get(region.get()).map(|roll| roll.end) else {
                continue;
            };
            let at = start + (end - start) * runtime.landmark_permille / 1000;
            waypoints.push(Waypoint {
                at,
                def: u16::try_from(def).unwrap_or(u16::MAX),
                taken: false,
            });
        }

        let mut world = Self {
            distance: 0,
            bands: Vec::new(),
            features: Vec::new(),
            waypoints,
            // The first beat falls a short way in rather than at pace
            // zero: the opening five minutes are busy enough
            // (`SYSTEMS.md` §6.11) without a prompt in them.
            next_waypoint_at: paces_from_int(600),
            generated_to: 0,
            journey,
            region: RegionIdx(0),
            region_start: 0,
            next_fork_at: 0,
            fork: None,
            branch: None,
        };
        world.schedule_first_fork(content);
        world.generate_ahead(rng, content);
        world
    }

    /// The region a given distance falls in.
    ///
    /// A pure function of `distance` and the journey roll, which is the
    /// property that lets the generator work ahead of the tower without
    /// a second cursor. Past the end of the last region — the run's
    /// finish line — this clamps to the last one.
    #[must_use]
    pub fn region_at(&self, at: Paces) -> RegionIdx {
        for (i, roll) in self.journey.iter().enumerate() {
            if at < roll.end {
                return RegionIdx(i as u16);
            }
        }
        RegionIdx(self.journey.len().saturating_sub(1) as u16)
    }

    /// Where a region begins, absolutely.
    #[must_use]
    pub fn region_start_of(&self, region: RegionIdx) -> Paces {
        match region.get().checked_sub(1) {
            Some(previous) => self.journey.get(previous).map_or(0, |roll| roll.end),
            None => 0,
        }
    }

    /// Where the run's last region ends. Reaching it is one of the two
    /// ways a run finishes (`SYSTEMS.md` §3.7).
    #[must_use]
    pub fn journey_end(&self) -> Paces {
        self.journey.last().map_or(0, |roll| roll.end)
    }

    /// The furthest the tower may walk, if something is stopping it.
    ///
    /// Two reasons, deliberately collapsed into one answer so that
    /// every caller — the stride system, the charge it would spend,
    /// the renderer deciding which halted state to draw — asks the same
    /// question and cannot disagree about it:
    ///
    /// * an unanswered fork, where the ground ahead has not been
    ///   decided (`SYSTEMS.md` §3.3), and
    /// * the far edge of the last region, where the run is over.
    #[must_use]
    pub fn blocked_at(&self) -> Option<Paces> {
        if let Some(fork) = self.fork
            && fork.answer.is_none()
        {
            return Some(fork.at);
        }
        Some(self.journey_end())
    }

    /// Whether the tower is standing at whatever is stopping it, rather
    /// than merely somewhere behind it.
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.blocked_at().is_some_and(|at| self.distance >= at)
    }

    /// The palette terrain at a given distance is drawn from: the
    /// branch the tower is walking through, if that stretch is inside
    /// one, and otherwise the region's own.
    fn palette_at<'a>(&self, content: &'a Content, at: Paces) -> &'a [(TerrainIdx, i64)] {
        if let Some(branch) = self.branch
            && at >= branch.from
            && at < branch.to
        {
            return &content.branch_rt(branch.def).palette;
        }
        &content.region_rt(self.region_at(at)).palette
    }

    /// The palette at a distance, for tests that need to see which side
    /// of a branch boundary a pace falls on.
    #[cfg(test)]
    #[must_use]
    pub fn palette_at_for_test<'a>(
        &self,
        content: &'a Content,
        at: Paces,
    ) -> &'a [(TerrainIdx, i64)] {
        self.palette_at(content, at)
    }

    /// Put the region's first fork on the schedule, or push the marker
    /// past the region's end if it has none.
    fn schedule_first_fork(&mut self, content: &Content) {
        let start = self.region_start_of(self.region);
        self.region_start = start;
        let interval = content.region(self.region).fork_interval_paces;
        self.next_fork_at = if interval <= 0 {
            self.region_end(self.region)
        } else {
            start + paces_from_int(interval)
        };
        self.skip_forks_too_near_an_edge(content);
    }

    fn region_end(&self, region: RegionIdx) -> Paces {
        self.journey
            .get(region.get())
            .map_or(Paces::MAX, |roll| roll.end)
    }

    /// Walk the fork marker forward past any position that falls inside
    /// `fork_edge_margin_paces` of the region's ends or of its enclave,
    /// so a decision never competes with a boundary or a berth for the
    /// same stretch of horizon.
    pub fn skip_forks_too_near_an_edge(&mut self, content: &Content) {
        let region = self.region;
        let interval = content.region(region).fork_interval_paces;
        if interval <= 0 {
            return;
        }
        let margin = paces_from_int(content.balance.journey.fork_edge_margin_paces);
        let start = self.region_start_of(region);
        let end = self.region_end(region);
        let enclave_at = content
            .region(region)
            .enclave
            .as_ref()
            .map(|enclave| start + paces_from_int(enclave.at_paces));

        while self.next_fork_at < end {
            let too_near_start = self.next_fork_at - start < margin;
            let too_near_end = end - self.next_fork_at < margin;
            let too_near_enclave =
                enclave_at.is_some_and(|at| (self.next_fork_at - at).abs() < margin);
            // And it has to be somewhere the tower has not already
            // been. A fork scheduled behind the tower is opened the
            // instant the generator notices it, sits at a distance
            // already walked, and blocks the legs for the rest of the
            // run — the tower waiting for a decision about ground
            // behind it. Reachable whenever a region is entered from
            // anywhere but its first pace.
            let already_passed = self.next_fork_at <= self.distance;
            if !(too_near_start || too_near_end || too_near_enclave || already_passed) {
                return;
            }
            self.next_fork_at += paces_from_int(interval);
        }
    }

    /// Move the bookkeeping into a new region. Called by `stride` when
    /// the tower crosses a boundary — crossing is an event, not only a
    /// fact, and this is the one-time setup it fires.
    pub fn enter_region(&mut self, content: &Content, region: RegionIdx) {
        self.region = region;
        self.branch = None;
        self.fork = None;
        self.schedule_first_fork(content);
    }

    /// The band the tower is standing in right now.
    #[must_use]
    pub fn band_at(&self, at: Paces) -> Option<&TerrainBand> {
        self.bands.iter().find(|band| band.contains(at))
    }

    /// The nearest ruin with anything left in it, within `range_paces`
    /// of where the tower stands.
    ///
    /// This is the whole of berthing (`SYSTEMS.md` §3.4). There is no
    /// `Berth` command and no berthed flag: a tower that has stopped
    /// with a working rig whose reach covers a ruin is berthing, and
    /// one that walks on is not. Reach belongs to the rig rather than
    /// to the world, the way a dart battery's does, so a longer-reaching
    /// rig is a thing content can author.
    ///
    /// Returns an index rather than a reference, because the caller
    /// draws the ruin down. Ties go to the earlier feature and
    /// `features` is kept sorted, so two rigs of the same reach always
    /// pick the same ruin.
    #[must_use]
    pub fn ruin_in_reach(&self, range_paces: i64) -> Option<usize> {
        let reach = paces_from_int(range_paces);
        let mut best: Option<(usize, Paces)> = None;
        for (i, feature) in self.features.iter().enumerate() {
            if feature.salvage <= 0 {
                continue;
            }
            let gap = (feature.at - self.distance).abs();
            if gap > reach {
                continue;
            }
            if best.is_none_or(|(_, closest)| gap < closest) {
                best = Some((i, gap));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Where the enclave stands, absolutely, if the run has one.
    ///
    /// A region's enclave is authored as an offset from that region's
    /// own start, so this needs the journey roll to place it.
    #[must_use]
    pub fn enclave_at(&self, content: &Content) -> Option<Paces> {
        self.enclave_ahead(content).map(|(_, at)| at)
    }

    /// The next enclave the tower has not yet passed, and where it
    /// stands on the distance axis.
    ///
    /// **Was "the first enclave in the pack", which stopped being the
    /// same thing the moment M5 added a second.** One enclave meant one
    /// answer and `find_map` on the first `Some` was that answer; three
    /// mean the tower has to be told which one it is walking toward, or
    /// it stands at the coast berthed against a settlement two regions
    /// behind it. Skips anything already passed by more than a berth's
    /// reach, because there is no going back down the axis.
    #[must_use]
    pub fn enclave_ahead(&self, content: &Content) -> Option<(RegionIdx, Paces)> {
        let reach = paces_from_int(content.balance.journey.enclave_berth_paces);
        content.regions.iter().enumerate().find_map(|(i, region)| {
            let enclave = region.enclave.as_ref()?;
            let idx = RegionIdx(i as u16);
            let at = self.region_start_of(idx) + paces_from_int(enclave.at_paces);
            (at + reach >= self.distance).then_some((idx, at))
        })
    }

    /// Which enclave the tower is berthed at, if any.
    #[must_use]
    pub fn berthed_enclave(&self, content: &Content, strode: bool) -> Option<RegionIdx> {
        if strode {
            return None;
        }
        let reach = paces_from_int(content.balance.journey.enclave_berth_paces);
        self.enclave_ahead(content)
            .filter(|(_, at)| (*at - self.distance).abs() <= reach)
            .map(|(idx, _)| idx)
    }

    /// Whether the tower is berthed at the enclave — stopped, and near
    /// enough. Berthing is implicit here for the same reason it is at a
    /// ruin: there is no docking mechanic, only a tower that stopped in
    /// the right place.
    ///
    /// A tower that walks past has lost it. There is no going back down
    /// the axis, which is what makes deciding to stop cost something.
    #[must_use]
    pub fn at_enclave(&self, content: &Content, strode: bool) -> bool {
        self.berthed_enclave(content, strode).is_some()
    }

    /// Yield multiplier, in percent, of the terrain under the tower.
    /// 100 means "as authored"; a barren ruin-field is lower.
    #[must_use]
    pub fn current_yield_pct(&self, content: &Content) -> i64 {
        self.band_at(self.distance).map_or(100, |band| {
            content.terrain_runtime[band.kind.get()].yield_pct
        })
    }

    /// Threat pressure of the ground the tower has committed to.
    ///
    /// Regions establish the baseline and a live fork branch modifies it.
    /// Keeping the multiplication here gives simulation and presentation one
    /// authoritative answer instead of leaving the fork's advertised danger as
    /// decorative copy.
    #[must_use]
    pub fn current_threat_pct(&self, content: &Content) -> i64 {
        let region = content.region(self.region).threat_pct;
        let branch = self
            .branch
            .map_or(100, |active| content.branch(active.def).threat_pct);
        region * branch / 100
    }

    /// The authored specialist mix at the tower's current position.
    #[must_use]
    pub fn current_approaches(&self, content: &Content) -> ApproachWeights {
        self.branch
            .map_or(content.region(self.region).approaches, |active| {
                content.branch(active.def).approaches
            })
    }

    /// Extend the terrain until it reaches `stream_ahead_paces` past
    /// the tower. Called every tick; usually a no-op.
    pub fn generate_ahead(&mut self, rng: &mut Rng, content: &Content) {
        let balance = &content.balance.world;
        let target = self.distance + paces_from_int(balance.stream_ahead_paces);

        while self.generated_to < target {
            // Beats, placed as the horizon reaches them — before the
            // fork check, so a waypoint that falls short of a fork line
            // still gets laid down rather than being lost to the early
            // return below.
            self.scatter_waypoints(rng, content);

            // Place a fork as soon as the horizon reaches one, so the
            // player sees it coming with the whole streaming window to
            // answer in.
            if self.fork.is_none()
                && self.next_fork_at > self.region_start
                && self.generated_to >= self.next_fork_at
            {
                self.open_fork(rng, content);
            }

            // Past an unanswered fork there is nothing to produce: the
            // palette beyond depends on an answer that does not exist
            // yet. The generator stops at the line, which is *why* the
            // tower halts there — the ground it would walk onto has not
            // been decided (`SYSTEMS.md` §3.3).
            if let Some(fork) = self.fork
                && fork.answer.is_none()
                && self.generated_to >= fork.at
            {
                return;
            }

            let palette = self.palette_at(content, self.generated_to).to_vec();
            let kind = pick_band_kind(rng, &palette, self.bands.last().map(|b| b.kind));
            let length_paces = rng.range(
                balance.band_min_paces,
                balance.band_max_paces.max(balance.band_min_paces),
            );
            // A band must never straddle the fork line. If one did, the
            // stretch past the split would already have been drawn from
            // the palette on this side of it — and the generator would
            // have run past a decision that has not been made, which is
            // the one thing the halt depends on it not doing.
            let mut length = paces_from_int(length_paces);
            if self.fork.is_none() && self.next_fork_at > self.generated_to {
                length = length.min(self.next_fork_at - self.generated_to);
            }
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

    /// Draw the two archetypes this fork offers.
    ///
    /// Always distinct — the choice between two identical branches is
    /// not a choice. A region with fewer than two archetypes cannot
    /// fork at all, which content validation already forbids, but the
    /// generator declines rather than panicking if it ever happens.
    fn open_fork(&mut self, rng: &mut Rng, content: &Content) {
        let rt = content.region_rt(self.region);
        let count = usize::from(rt.branch_end.saturating_sub(rt.branch_first));
        if count < 2 {
            return;
        }
        let first = rng.index(count).unwrap_or(0);
        // Offset into the remaining archetypes, so the second is never
        // the first and every pair stays equally likely.
        let step = rng.index(count - 1).unwrap_or(0) + 1;
        let second = (first + step) % count;
        self.fork = Some(PendingFork {
            at: self.next_fork_at,
            branches: [
                BranchIdx(rt.branch_first + first as u16),
                BranchIdx(rt.branch_first + second as u16),
            ],
            answer: None,
        });
    }

    /// Commit to one of the pending fork's two branches.
    ///
    /// Legal from the moment the fork appears until the tower crosses
    /// it, and re-sendable: the last answer before the line is the one
    /// that counts. Changing an answer throws away the terrain already
    /// generated past the fork, because that terrain was drawn from the
    /// wrong palette — cheap, since it can only ever be the streaming
    /// window's worth.
    pub fn answer_fork(&mut self, content: &Content, branch: u8) -> bool {
        let Some(mut fork) = self.fork else {
            return false;
        };
        let Some(def) = fork.branches.get(usize::from(branch)).copied() else {
            return false;
        };
        if fork.answer == Some(branch) {
            return true;
        }
        fork.answer = Some(branch);
        self.fork = Some(fork);
        self.branch = Some(ActiveBranch {
            def,
            from: fork.at,
            to: fork.at + paces_from_int(content.branch(def).length_paces),
        });
        self.discard_generation_past(fork.at);
        true
    }

    /// Throw away everything generated at or past `at`, so it can be
    /// produced again from a different palette.
    fn discard_generation_past(&mut self, at: Paces) {
        if self.generated_to <= at {
            return;
        }
        // Whole bands only. Nothing straddles the line — the generator
        // cuts a band short rather than crossing one — so dropping
        // every band that ends past it leaves the terrain behind the
        // fork contiguous and complete.
        self.bands.retain(|band| band.end() <= at);
        self.features.retain(|feature| feature.at < at);
        self.generated_to = self.bands.last().map_or(at, TerrainBand::end).max(at);
    }

    /// Drop everything that has fallen fully behind the live window.
    pub fn prune_behind(&mut self, content: &Content) {
        let cutoff = self.distance - paces_from_int(content.balance.world.stream_behind_paces);
        if cutoff <= 0 {
            return;
        }
        self.bands.retain(|band| band.end() > cutoff);
        self.features.retain(|feature| feature.at > cutoff);
        // Pruned on the same window as everything else, so a run of any
        // length costs the same memory — `AGENTS.md` §VII's rule about
        // never accumulating unbounded history.
        self.waypoints.retain(|waypoint| waypoint.at > cutoff);
    }

    /// Lay down beats up to the generated horizon.
    ///
    /// **Off the `world` stream, not `cosmetic`** (`DECISIONS.md` §2).
    /// Where a waypoint falls and which one it is are facts about the
    /// run — a shared seed has to reproduce them — and the cosmetic
    /// stream is explicitly the one that may be perturbed by adding or
    /// editing content.
    fn scatter_waypoints(&mut self, rng: &mut Rng, content: &Content) {
        let ordinary: Vec<u16> = content
            .waypoint_runtime
            .iter()
            .enumerate()
            .filter(|(_, runtime)| runtime.landmark_region.is_none())
            .filter_map(|(index, _)| u16::try_from(index).ok())
            .collect();
        if ordinary.is_empty() {
            return;
        }
        while self.next_waypoint_at <= self.generated_to {
            let region = self.region_at(self.next_waypoint_at);
            let authored = content.region(region).waypoint_interval_paces;
            if authored <= 0 {
                // This region scatters none. Step past it rather than
                // spinning: the horizon still has to advance or
                // `generate_ahead` never terminates.
                self.next_waypoint_at += paces_from_int(MIN_WAYPOINT_INTERVAL_PACES);
                continue;
            }
            let interval = authored.max(MIN_WAYPOINT_INTERVAL_PACES);
            // Jittered either side of the interval rather than laid on a
            // grid: a beat you can time is a beat you stop reading.
            let step = rng.range(interval / 2, interval * 3 / 2);
            let at = self.next_waypoint_at;
            self.next_waypoint_at = at + paces_from_int(step);

            // Never on top of a fork line. A fork already halts the
            // tower and asks a question; stacking a second prompt on it
            // turns one decision into a queue.
            if let Some(fork) = self.fork
                && (fork.at - at).abs() < paces_from_int(MIN_WAYPOINT_INTERVAL_PACES)
            {
                // Retry just past the exclusion zone rather than silently
                // deleting a beat from this region's cadence.
                self.next_waypoint_at = at + paces_from_int(MIN_WAYPOINT_INTERVAL_PACES);
                continue;
            }
            // The fixed landmark was placed before streaming began.
            // Give it the same breathing room as any other beat so its
            // warning and choice never arrive as the second card in a
            // prompt queue.
            if self.waypoints.iter().any(|waypoint| {
                (waypoint.at - at).abs() < paces_from_int(MIN_WAYPOINT_INTERVAL_PACES)
            }) {
                self.next_waypoint_at = at + paces_from_int(MIN_WAYPOINT_INTERVAL_PACES);
                continue;
            }
            let Some(pick) = rng.index(ordinary.len()) else {
                break;
            };
            let def = ordinary[pick];
            self.waypoints.push(Waypoint {
                at,
                def,
                taken: false,
            });
        }
        self.waypoints.sort_by_key(|waypoint| waypoint.at);
    }

    fn scatter_features(&mut self, rng: &mut Rng, content: &Content, band: &TerrainBand) {
        let def = content.terrain(band.kind);
        if def.feature_kinds.is_empty() || def.features_per_100_paces <= 0 {
            return;
        }
        let rt = &content.terrain_runtime[band.kind.get()];
        // Richness belongs to the region the ruin stands in, not to the
        // region the tower happens to be in when the band is generated
        // — the generator runs ahead, and a ruin's worth is a fact
        // about where it is.
        let richness = self
            .journey
            .get(self.region_at(band.start).get())
            .map_or(100, |roll| roll.ruin_richness_pct);

        let whole_paces = band.length >> crate::fx::FX_SHIFT;
        let count = (whole_paces * def.features_per_100_paces) / 100;
        for _ in 0..count {
            let offset = rng.range(0, whole_paces.max(1) - 1);
            let kind = rng.index(def.feature_kinds.len()).unwrap_or(0) as u8;
            let is_ruin = rt
                .ruin_feature
                .get(usize::from(kind))
                .copied()
                .unwrap_or(false);
            // Every ruin is rolled and then scaled, rather than scaled
            // and then rolled, so a lean region still has the same
            // spread of good and poor ruins — just less in all of them.
            let salvage = if is_ruin {
                rng.range(rt.salvage_min, rt.salvage_max.max(rt.salvage_min)) * richness / 100
            } else {
                0
            };
            self.features.push(Feature {
                at: band.start + paces_from_int(offset),
                kind,
                scale: (rng.range(0, 255)) as u8,
                layer: (rng.range(0, 2)) as u8,
                salvage,
                roused: false,
            });
        }
        // Keep the render order stable regardless of draw order.
        self.features.sort_by_key(|f| (f.at, f.layer, f.kind));
    }
}

/// Weighted pick from a palette that never repeats the previous band's
/// kind, so the horizon always changes. Falls back to allowing a repeat
/// if the palette holds only one kind — which content validation
/// forbids, but the generator should not be the thing that panics if it
/// ever happens.
fn pick_band_kind(
    rng: &mut Rng,
    palette: &[(TerrainIdx, i64)],
    previous: Option<TerrainIdx>,
) -> TerrainIdx {
    let eligible: Vec<(TerrainIdx, i64)> = palette
        .iter()
        .copied()
        .filter(|(idx, weight)| *weight > 0 && Some(*idx) != previous)
        .collect();

    let pool = if eligible.is_empty() {
        palette
            .iter()
            .map(|(idx, weight)| (*idx, (*weight).max(1)))
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
