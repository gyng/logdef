use serde::{Deserialize, Serialize};

/// Type alias for simulation math. Can be swapped to fixed-point if
/// cross-platform determinism requires it (see implementation-decisions.md §19).
pub type Scalar = f32;

/// Maximum simulation ticks per frame call, to prevent spiral-of-death
/// when real_dt is large (e.g., after a tab switch).
pub const MAX_TICKS_PER_FRAME: u32 = 4;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Vec2 {
    pub x: Scalar,
    pub y: Scalar,
}

impl Vec2 {
    pub fn new(x: Scalar, y: Scalar) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

// Strongly-typed IDs to prevent mixing up indices.
macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub u32);
    };
}

id_type!(EntityId);
id_type!(EnemyId);
id_type!(ProjectileId);
id_type!(RunnerId);
id_type!(QuartersId);
id_type!(TransportId);
id_type!(BalconyId);
id_type!(NodeId);
id_type!(DestinationId);

/// Simulation tick rate: 30hz.
pub const FIXED_DT: Scalar = 1.0 / 30.0;
