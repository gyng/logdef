use serde::{Deserialize, Serialize};

/// Deterministic RNG for reproducible simulation.
/// Same seed + same commands = same result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Derive a child RNG for a specific scope (chapter, encounter, etc.)
    /// without advancing the parent state unpredictably.
    pub fn derive(&mut self, scope: u64) -> Self {
        let child_seed = self.state.wrapping_mul(6364136223846793005).wrapping_add(scope);
        Self { state: child_seed }
    }

    /// Generate next u64 using xorshift64.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generate a float in [0, 1).
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Generate a random integer in [min, max] inclusive.
    pub fn range(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        let range = (max - min + 1) as u64;
        (self.next_u64() % range) as u32 + min
    }

    pub fn seed(&self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_output() {
        let mut rng_a = DeterministicRng::new(42);
        let mut rng_b = DeterministicRng::new(42);
        for _ in 0..100 {
            assert_eq!(rng_a.next_u64(), rng_b.next_u64());
        }
    }

    #[test]
    fn range_within_bounds() {
        let mut rng = DeterministicRng::new(12345);
        for _ in 0..1000 {
            let v = rng.range(5, 10);
            assert!((5..=10).contains(&v));
        }
    }

    #[test]
    fn f32_within_bounds() {
        let mut rng = DeterministicRng::new(99);
        for _ in 0..1000 {
            let v = rng.next_f32();
            assert!((0.0..1.0).contains(&v));
        }
    }
}
