//! Strongly-typed identifiers.
//!
//! Two families live here and they are not interchangeable:
//!
//! * **Indices** (`ItemIdx`, `RoomIdx`, `TerrainIdx`) point into the
//!   loaded content pack. Content ships as readable string IDs; the
//!   registry interns them into dense `u16` indices once at load. The
//!   string never enters the simulation, and the mapping is recorded in
//!   a save header so saves survive a content edit.
//! * **Runtime IDs** (`RoomId`, `CrewId`, `ShaftId`) identify instances
//!   the player created. They are allocated from monotonic counters in
//!   `GameState` — never from a pointer, a hash, or a vector position.

use serde::{Deserialize, Serialize};

macro_rules! index_type {
    ($name:ident, $what:literal) => {
        #[doc = concat!("Interned index of a ", $what, " definition in the content pack.")]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub u16);

        impl $name {
            #[inline]
            #[must_use]
            pub const fn get(self) -> usize {
                self.0 as usize
            }
        }
    };
}

macro_rules! runtime_id {
    ($name:ident, $what:literal) => {
        #[doc = concat!("Runtime identity of one ", $what, " instance.")]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub u32);
    };
}

index_type!(ItemIdx, "item");
index_type!(RoomIdx, "room");
index_type!(ShaftIdx, "shaft");
index_type!(TerrainIdx, "terrain band");
index_type!(DaypartIdx, "daypart");

runtime_id!(RoomId, "room");
runtime_id!(CrewId, "crew member");
runtime_id!(ShaftId, "shaft");

/// Floor index, counted from the ground.
pub type FloorIdx = u8;

/// Horizontal slot index within a floor, counted from the left edge.
pub type SlotIdx = u8;
