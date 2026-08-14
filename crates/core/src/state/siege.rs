//! The siege: what is coming, and how badly you have provoked it.
//!
//! Creatures live on the terrain layer the tower is already walking
//! through — there is no arena, no encounter, no mode. A wave is a
//! demand spike on the circulation you already have, which is the whole
//! reason combat belongs in the same loop as the mill.
//!
//! Damage attaches to the things the player built rather than to an
//! abstract tower hit-point bar. A breached panel, a wrecked mill, a
//! severed shaft column: each is legible on the cross-section without a
//! readout, and each fails in a different way.

use serde::{Deserialize, Serialize};

use crate::fx::Paces;
use crate::ids::{EnemyId, EnemyIdx, FloorIdx, ShaftId, SlotIdx};

/// What a creature is chewing on.
///
/// Not an entity handle: rooms and shafts can be demolished by the
/// player mid-bite, and a stale handle would either panic or silently
/// damage whatever took its place. A coordinate re-resolves, and
/// re-resolving to nothing simply ends the attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageTarget {
    Panel { floor: FloorIdx },
    Room { floor: FloorIdx, slot: SlotIdx },
    Shaft { id: ShaftId },
    Heart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyState {
    /// Closing on the tower along the terrain layer.
    Approaching,
    /// In contact, working on something.
    Attacking { target: DamageTarget },
    /// Shot down, and fading. Held for `enemy_fade_ticks` rather than
    /// a single tick: a death that exists for one frame at 1x exists
    /// for none at 4x, and the player has to be able to see that the
    /// darts are working.
    Dying,
    /// Lost its grip and been left behind by the tower's stride. Fades
    /// out the same way a kill does, and deliberately does not count
    /// toward `repelled` — walking away from something is not the same
    /// as seeing it off, and the readout should not claim otherwise.
    Leaving,
}

impl EnemyState {
    /// On the way out, by either route.
    #[must_use]
    pub fn is_going(self) -> bool {
        matches!(self, Self::Dying | Self::Leaving)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enemy {
    pub id: EnemyId,
    pub def: EnemyIdx,
    /// World position in Q8.8 paces, on the same axis as the tower.
    pub at: Paces,
    pub hp: i64,
    pub state: EnemyState,
    /// Ticks until the next bite.
    pub attack_cooldown: u32,
    /// Ticks of holding on left before the tower's stride carries it
    /// out from under this one. Only counts down while the legs run.
    pub cling_left: u32,
    /// Ticks left of the fade-out, once it is on its way out.
    pub fade_left: u32,
}

/// A temporary physical restraint applied by an emplacement.
///
/// Kept beside the wave rather than on `Enemy` so fixtures that author
/// creatures directly cannot accidentally grant or omit hidden combat
/// state. One entry per enemy, sorted by id whenever it is changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemyControl {
    pub enemy: EnemyId,
    pub ticks_left: u32,
    pub speed_pct: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Siege {
    pub enemies: Vec<Enemy>,
    /// Nets, resonance pulses and root wards currently holding a
    /// creature. Cleared when their subject leaves the wave.
    pub controls: Vec<EnemyControl>,
    /// How much attention the tower has drawn, 0 to `provocation_max`.
    ///
    /// One knob, raised by harvesting hard and by burner smoke, bled off
    /// by walking quietly. It is the only difficulty dial in the game
    /// and the player turns it with their own behaviour rather than
    /// from a menu.
    pub provocation: i64,
    /// Sub-unit provocation accumulation, so a slow trickle still adds
    /// up instead of truncating away.
    pub provocation_acc: i64,
    /// Tick the next wave check happens on.
    pub next_wave_tick: u64,
    /// Creatures seen off. Reported, never celebrated.
    pub repelled: u64,
    /// The Heartseed is gone. The run is over.
    pub lost: bool,
}

impl Siege {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            enemies: Vec::new(),
            controls: Vec::new(),
            provocation: 0,
            provocation_acc: 0,
            next_wave_tick: 0,
            repelled: 0,
            lost: false,
        }
    }

    /// Raise provocation, capped. Harvesting hard and burning bamboo
    /// both feed this.
    pub fn provoke(&mut self, amount: i64, ceiling: i64) {
        self.provocation = (self.provocation + amount).clamp(0, ceiling);
    }
}

impl Default for Siege {
    fn default() -> Self {
        Self::new()
    }
}

/// Something with hit points that the player built.
///
/// Panels, rooms, and shafts all take damage and all get repaired, so
/// they share one shape rather than three near-identical ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    pub hp: i64,
    pub max: i64,
}

impl Health {
    #[must_use]
    pub const fn full(max: i64) -> Self {
        Self { hp: max, max }
    }

    /// Take damage. Returns whether this brought it to nothing.
    pub fn hurt(&mut self, amount: i64) -> bool {
        self.hp = (self.hp - amount).max(0);
        self.hp == 0
    }

    /// Put hit points back, up to full. Returns how many landed.
    pub fn heal(&mut self, amount: i64) -> i64 {
        let room = (self.max - self.hp).max(0);
        let taken = amount.min(room);
        self.hp += taken;
        taken
    }

    #[must_use]
    pub const fn is_broken(&self) -> bool {
        self.hp == 0
    }

    #[must_use]
    pub const fn is_hurt(&self) -> bool {
        self.hp < self.max
    }

    /// Health as per-mille of full, so the renderer needs no division
    /// and the simulation needs no float.
    #[must_use]
    pub const fn permille(&self) -> i64 {
        if self.max <= 0 {
            return 1000;
        }
        self.hp * 1000 / self.max
    }
}
