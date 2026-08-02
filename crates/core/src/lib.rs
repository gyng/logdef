//! Understory — deterministic simulation core.
//!
//! A walking garden-tower striding through the jungle that swallowed
//! the old world. This crate owns all of it: the world it walks
//! through, the rooms inside it, the crew hauling between them, and the
//! rules that make the same seed produce the same run forever.
//!
//! ## What is where
//!
//! | Module | Holds |
//! |---|---|
//! | [`fx`] | Q8.8 fixed point. The reason there is no floating point below the snapshot layer. |
//! | [`rng`] | Named random streams, with the cosmetic firewall. |
//! | [`ids`] | Interned content indices and runtime instance IDs. |
//! | [`content`] | The RON content pack, validated and interned. |
//! | [`state`] | `GameState`: world, tower, crew. |
//! | [`command`] | The only way state changes. |
//! | [`systems`] | The fixed-order tick. |
//! | [`snapshot`] | The presentation boundary — and the only place floats appear. |
//! | [`replay`] | Seed + commands + hashes. The save, the fixture, the shared seed. |
//! | [`engine`] | Ties it together. |
//!
//! ## The rules that do not bend
//!
//! 1. No floating point in `GameState` or any system.
//! 2. Ordered iteration everywhere — `Vec` and `BTreeMap`, never a hash map.
//! 3. Every mutation is a validated `GameCommand`.
//! 4. Same seed plus same commands equals the same state hash, natively and in wasm.
//!
//! See `docs/DECISIONS.md` for why, and `docs/SYSTEMS.md` for the
//! milestone-by-milestone spec.

pub mod command;
pub mod content;
pub mod engine;
pub mod fx;
pub mod ids;
pub mod replay;
pub mod rng;
pub mod snapshot;
pub mod state;
pub mod systems;

pub use engine::{GameEngine, verify_golden_replay, verify_replay};

#[cfg(test)]
mod tests;
