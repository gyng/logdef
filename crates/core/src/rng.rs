//! Deterministic RNG, split into named streams.
//!
//! One generator for the whole game is a trap: the moment a cosmetic
//! system draws a number, every economic roll downstream shifts. So the
//! run seed fans out into independent streams, and the rule is written
//! into the type — see [`RngStreams`].

use serde::{Deserialize, Serialize};

/// xorshift64 with a splitmix64 seed conditioner. Small, fast, and
/// identical on every target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}

impl Rng {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        // xorshift64 is stuck at zero and weak on low-entropy seeds, so
        // run the seed through splitmix64 first.
        Self {
            state: splitmix64(seed ^ 0x9E37_79B9_7F4A_7C15),
        }
    }

    /// Derive an independent child stream. Does not advance `self`.
    #[must_use]
    pub fn stream(&self, label: u64) -> Self {
        Self::new(self.state ^ splitmix64(label))
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform integer in `[min, max]`, inclusive. Rejection-sampled so
    /// the distribution has no modulo bias.
    pub fn range(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        let span = (max - min) as u64 + 1;
        let limit = u64::MAX - (u64::MAX % span);
        loop {
            let draw = self.next_u64();
            if draw < limit {
                return min + (draw % span) as i64;
            }
        }
    }

    /// True with probability `numerator / denominator`.
    pub fn chance(&mut self, numerator: i64, denominator: i64) -> bool {
        if denominator <= 0 {
            return false;
        }
        self.range(0, denominator - 1) < numerator
    }

    /// Index into a slice of length `len`. Returns `None` if empty.
    pub fn index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        Some(self.range(0, len as i64 - 1) as usize)
    }
}

fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The run's random streams.
///
/// **The cosmetic firewall.** `cosmetic` feeds presentation only —
/// idle animation phase, bark selection, visual jitter. No system may
/// read it and write anything another system reads. Adding a bark in a
/// later milestone must not move a single crate. `world` and `sim` are
/// economically live and may be read freely by simulation systems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RngStreams {
    /// Terrain generation, regions, route layout.
    pub world: Rng,
    /// Anything the economy rolls for.
    pub sim: Rng,
    /// Presentation only. Never feeds the economy.
    pub cosmetic: Rng,
}

impl RngStreams {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let root = Rng::new(seed);
        Self {
            world: root.stream(STREAM_WORLD),
            sim: root.stream(STREAM_SIM),
            cosmetic: root.stream(STREAM_COSMETIC),
        }
    }
}

const STREAM_WORLD: u64 = 0x776F_726C_6400_0001;
const STREAM_SIM: u64 = 0x7369_6D00_0000_0002;
const STREAM_COSMETIC: u64 = 0x636F_736D_0000_0003;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn zero_seed_is_not_degenerate() {
        let mut rng = Rng::new(0);
        let first = rng.next_u64();
        assert_ne!(first, 0);
        assert_ne!(rng.next_u64(), first);
    }

    #[test]
    fn range_stays_in_bounds_and_covers_them() {
        let mut rng = Rng::new(7);
        let mut saw_min = false;
        let mut saw_max = false;
        for _ in 0..5000 {
            let value = rng.range(3, 9);
            assert!((3..=9).contains(&value));
            saw_min |= value == 3;
            saw_max |= value == 9;
        }
        assert!(saw_min && saw_max);
    }

    #[test]
    fn degenerate_range_returns_min() {
        let mut rng = Rng::new(1);
        assert_eq!(rng.range(5, 5), 5);
        assert_eq!(rng.range(9, 2), 9);
    }

    #[test]
    fn streams_are_independent() {
        let streams = RngStreams::new(1234);
        // Drawing from cosmetic must not alter what world/sim produce.
        let mut a = streams.clone();
        let mut b = streams.clone();
        for _ in 0..100 {
            let _ = b.cosmetic.next_u64();
        }
        for _ in 0..100 {
            assert_eq!(a.world.next_u64(), b.world.next_u64());
            assert_eq!(a.sim.next_u64(), b.sim.next_u64());
        }
    }

    #[test]
    fn streams_differ_from_each_other() {
        let mut streams = RngStreams::new(99);
        let world = streams.world.next_u64();
        let sim = streams.sim.next_u64();
        let cosmetic = streams.cosmetic.next_u64();
        assert_ne!(world, sim);
        assert_ne!(sim, cosmetic);
        assert_ne!(world, cosmetic);
    }
}
